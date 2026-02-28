use actix_web::{web, HttpRequest, HttpResponse};
use actix_ws::Message;
use futures::StreamExt;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::handlers;
use crate::message::{IncomingMessage, OutgoingMessage};
use crate::WebSocketManager;
use nexus_types::accountability::Accountability;

/// WebSocket endpoint handler
/// Upgrades HTTP connection to WebSocket and manages the bidirectional stream
/// Mirrors api/src/websocket/controllers/rest.ts
pub async fn websocket_handler(
    req: HttpRequest,
    body: web::Payload,
    manager: web::Data<Arc<WebSocketManager>>,
) -> Result<HttpResponse, actix_web::Error> {
    let (response, mut session, mut msg_stream) = actix_ws::handle(&req, body)?;

    let connection_id = uuid::Uuid::new_v4().to_string();
    let manager = manager.get_ref().clone();
    let accountability: Arc<RwLock<Option<Accountability>>> = Arc::new(RwLock::new(None));

    // Subscribe to broadcast events for this connection
    let mut event_rx = manager.receiver();

    // Spawn the WebSocket message loop
    let conn_id_for_events = connection_id.clone();
    let manager_for_events = manager.clone();

    actix_web::rt::spawn(async move {
        // Clone session for the broadcast listener
        let mut broadcast_session = session.clone();
        let conn_id_broadcast = conn_id_for_events.clone();
        let manager_broadcast = manager_for_events.clone();

        // Spawn a separate task to forward broadcast events to this connection
        let broadcast_handle = tokio::spawn(async move {
            loop {
                match event_rx.recv().await {
                    Ok(event) => {
                        // Check if this connection has matching subscriptions
                        let subs = manager_broadcast.get_subscriptions(&conn_id_broadcast).await;
                        for sub in &subs {
                            if sub.collection == event.collection || sub.collection == "*" {
                                let msg = OutgoingMessage::subscription_event(
                                    &sub.uid,
                                    &event.action,
                                    event.payload.clone(),
                                );
                                if let Ok(json) = serde_json::to_string(&msg) {
                                    if broadcast_session.text(json).await.is_err() {
                                        return; // Connection closed
                                    }
                                }
                            }
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!(lagged = n, "WebSocket broadcast receiver lagged");
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });

        // Main message processing loop
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    match serde_json::from_str::<IncomingMessage>(&text) {
                        Ok(incoming) => {
                            let response = handlers::handle_message(
                                incoming,
                                &connection_id,
                                &accountability,
                                &manager,
                            )
                            .await;

                            if let Ok(json) = serde_json::to_string(&response) {
                                let _ = session.text(json).await;
                            }
                        }
                        Err(e) => {
                            let error = OutgoingMessage::error(
                                "INVALID_MESSAGE",
                                &format!("Failed to parse message: {}", e),
                            );
                            if let Ok(json) = serde_json::to_string(&error) {
                                let _ = session.text(json).await;
                            }
                        }
                    }
                }
                Message::Ping(bytes) => {
                    let _ = session.pong(&bytes).await;
                }
                Message::Close(_) => {
                    break;
                }
                _ => {}
            }
        }

        // Clean up
        broadcast_handle.abort();
        manager.unsubscribe_all(&connection_id).await;
        tracing::debug!(
            connection_id = %connection_id,
            "WebSocket connection closed"
        );
    });

    Ok(response)
}

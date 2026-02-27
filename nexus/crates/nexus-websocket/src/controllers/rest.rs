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

    // Spawn the WebSocket message loop
    actix_web::rt::spawn(async move {
        while let Some(Ok(msg)) = msg_stream.next().await {
            match msg {
                Message::Text(text) => {
                    // Parse incoming message
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

        // Clean up subscriptions on disconnect
        manager.unsubscribe_all(&connection_id).await;
        tracing::debug!(
            connection_id = %connection_id,
            "WebSocket connection closed"
        );
    });

    Ok(response)
}

pub mod subscribe;
pub mod heartbeat;

use crate::message::{IncomingMessage, OutgoingMessage};
use crate::WebSocketManager;
use nexus_types::accountability::Accountability;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Handle an incoming WebSocket message
pub async fn handle_message(
    msg: IncomingMessage,
    connection_id: &str,
    accountability: &Arc<RwLock<Option<Accountability>>>,
    manager: &Arc<WebSocketManager>,
) -> OutgoingMessage {
    match msg {
        IncomingMessage::Ping => OutgoingMessage::Pong,

        IncomingMessage::Auth {
            access_token,
            email: _email,
            password: _password,
        } => {
            if let Some(_token) = access_token {
                // TODO: Verify JWT and set accountability
                let acc = Accountability {
                    admin: false,
                    ..Default::default()
                };
                *accountability.write().await = Some(acc);
                OutgoingMessage::auth_ok()
            } else {
                OutgoingMessage::auth_error("Missing credentials")
            }
        }

        IncomingMessage::Subscribe {
            uid,
            collection,
            query,
            event: _event,
        } => {
            let acc = accountability.read().await;
            if acc.is_none() {
                return OutgoingMessage::auth_error("Not authenticated");
            }

            let uid = uid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
            subscribe::handle_subscribe(connection_id, &uid, &collection, query, manager).await
        }

        IncomingMessage::Unsubscribe { uid } => {
            manager.unsubscribe(connection_id, &uid).await;
            OutgoingMessage::Subscription {
                uid,
                status: "ok".to_string(),
                event: None,
                data: None,
            }
        }

        IncomingMessage::Items {
            uid,
            action,
            collection,
            query: _query,
            data: _data,
            id: _id,
        } => {
            let acc = accountability.read().await;
            if acc.is_none() {
                return OutgoingMessage::auth_error("Not authenticated");
            }

            // TODO: Execute CRUD operations via ItemsService
            OutgoingMessage::Items {
                uid,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "action": action,
                    "collection": collection,
                })),
                error: None,
            }
        }
    }
}

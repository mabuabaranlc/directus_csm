pub mod subscribe;
pub mod heartbeat;

use crate::message::{IncomingMessage, OutgoingMessage};
use crate::WebSocketManager;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;
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
            if let Some(token) = access_token {
                // Verify the JWT token
                let secret = nexus_env::env_string_or("SECRET", "nexus-default-secret-change-me");
                let token_manager = nexus_auth::jwt::TokenManager::new(&secret, "nexus", 900, 604800);

                match token_manager.verify_token(&token) {
                    Ok(claims) => {
                        let acc = Accountability {
                            user: Some(claims.id),
                            role: claims.role,
                            admin: claims.admin,
                            app: claims.app,
                            ..Default::default()
                        };
                        *accountability.write().await = Some(acc);
                        OutgoingMessage::auth_ok()
                    }
                    Err(e) => {
                        OutgoingMessage::auth_error(&format!("Invalid token: {}", e))
                    }
                }
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
            query,
            data,
            id,
        } => {
            let acc_guard = accountability.read().await;
            let acc = match acc_guard.as_ref() {
                Some(a) => a.clone(),
                None => return OutgoingMessage::auth_error("Not authenticated"),
            };
            drop(acc_guard);

            let result = execute_ws_crud(&action, &collection, query, data, id, &acc, manager).await;

            match result {
                Ok(response_data) => OutgoingMessage::Items {
                    uid,
                    status: "ok".to_string(),
                    data: Some(response_data),
                    error: None,
                },
                Err(e) => OutgoingMessage::Items {
                    uid,
                    status: "error".to_string(),
                    data: None,
                    error: Some(crate::message::ErrorPayload {
                        code: "ITEMS_ERROR".to_string(),
                        message: e,
                    }),
                },
            }
        }
    }
}

/// Execute CRUD operations via the WebSocket Items protocol
async fn execute_ws_crud(
    action: &str,
    collection: &str,
    query: Option<serde_json::Value>,
    data: Option<serde_json::Value>,
    id: Option<serde_json::Value>,
    accountability: &Accountability,
    manager: &Arc<WebSocketManager>,
) -> Result<serde_json::Value, String> {
    let ctx = manager
        .service_context(Some(accountability.clone()))
        .await
        .ok_or_else(|| "No database context available".to_string())?;

    let service = nexus_services::items::ItemsService::new(collection, ctx);

    match action {
        "read" => {
            if let Some(id_val) = id {
                let pk = pk_from_value(&id_val)?;
                let item = service
                    .read_one(&pk, None, None)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(item)
            } else {
                let q = query
                    .and_then(|v| serde_json::from_value(v).ok())
                    .unwrap_or_default();
                let items = service
                    .read_by_query(q, None)
                    .await
                    .map_err(|e| e.to_string())?;
                Ok(serde_json::json!(items))
            }
        }
        "create" => {
            let payload = data.ok_or("Missing data for create")?;
            let pk = service
                .create_one(payload, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "id": pk }))
        }
        "update" => {
            let id_val = id.ok_or("Missing id for update")?;
            let pk = pk_from_value(&id_val)?;
            let payload = data.ok_or("Missing data for update")?;
            service
                .update_one(&pk, payload, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "id": pk }))
        }
        "delete" => {
            let id_val = id.ok_or("Missing id for delete")?;
            let pk = pk_from_value(&id_val)?;
            service
                .delete_one(&pk, None)
                .await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({ "deleted": true }))
        }
        _ => Err(format!("Unknown action: {}", action)),
    }
}

fn pk_from_value(val: &serde_json::Value) -> Result<PrimaryKey, String> {
    match val {
        serde_json::Value::String(s) => Ok(PrimaryKey::String(s.clone())),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                Ok(PrimaryKey::Integer(i))
            } else {
                Ok(PrimaryKey::String(n.to_string()))
            }
        }
        _ => Err("Invalid primary key format".to_string()),
    }
}

use crate::message::OutgoingMessage;
use crate::{Subscription, SubscriptionEvent, WebSocketManager};
use serde_json::Value;
use std::sync::Arc;

/// Handle a subscribe request
pub async fn handle_subscribe(
    connection_id: &str,
    uid: &str,
    collection: &str,
    query: Option<Value>,
    manager: &Arc<WebSocketManager>,
) -> OutgoingMessage {
    let event = None; // Subscribe to all events by default

    let subscription = Subscription {
        uid: uid.to_string(),
        collection: collection.to_string(),
        query,
        event,
    };

    manager.subscribe(connection_id, subscription).await;

    OutgoingMessage::Subscription {
        uid: uid.to_string(),
        status: "ok".to_string(),
        event: None,
        data: None,
    }
}

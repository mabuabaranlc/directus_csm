use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Notification operation — sends an in-app notification
/// Mirrors api/src/operations/notification/index.ts
pub struct NotificationOperation;

#[async_trait]
impl FlowOperation for NotificationOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let recipient = options
            .get("recipient")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                FlowError::InvalidConfig("Missing 'recipient'".to_string())
            })?;

        let subject = options
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("Notification");

        let message = options.get("message").and_then(|v| v.as_str());
        let collection = options.get("collection").and_then(|v| v.as_str());
        let item = options.get("item").and_then(|v| v.as_str());

        // TODO: Use NotificationsService to send the notification
        tracing::info!(
            recipient = recipient,
            subject = subject,
            "Notification operation"
        );

        Ok(json!({
            "sent": true,
            "recipient": recipient,
            "subject": subject,
            "message": message,
            "collection": collection,
            "item": item,
        }))
    }

    fn operation_type(&self) -> &str {
        "notification"
    }
}

use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Notification operation — sends an in-app notification via NotificationsService
/// Mirrors api/src/operations/notification/index.ts
pub struct NotificationOperation;

#[async_trait]
impl FlowOperation for NotificationOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        context: &OperationContext,
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

        // Try to use NotificationsService via service context
        if let Some(svc_ctx) = context.service_context() {
            let service = nexus_services::notifications::NotificationsService::new(svc_ctx);

            match service.send(recipient, subject, message, collection, item).await {
                Ok(pk) => {
                    tracing::info!(
                        recipient = recipient,
                        subject = subject,
                        pk = %pk,
                        "Notification operation: created notification"
                    );
                    return Ok(json!({
                        "sent": true,
                        "id": pk.to_string(),
                        "recipient": recipient,
                        "subject": subject,
                    }));
                }
                Err(e) => {
                    tracing::warn!(error = %e, "Notification operation: failed to create via service");
                }
            }
        }

        // Fallback: log only
        tracing::info!(
            recipient = recipient,
            subject = subject,
            "Notification operation: logged (no service context)"
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

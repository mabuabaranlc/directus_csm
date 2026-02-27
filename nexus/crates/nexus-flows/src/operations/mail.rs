use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Mail operation — sends an email
/// Mirrors api/src/operations/mail/index.ts
pub struct MailOperation;

#[async_trait]
impl FlowOperation for MailOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let to = options
            .get("to")
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'to' recipient".to_string()))?;

        let subject = options
            .get("subject")
            .and_then(|v| v.as_str())
            .unwrap_or("(No subject)");

        let _body = options
            .get("body")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let _template = options.get("template").and_then(|v| v.as_str());

        // Build recipients list
        let recipients: Vec<String> = match to {
            Value::String(email) => vec![email.clone()],
            Value::Array(arr) => arr
                .iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect(),
            _ => {
                return Err(FlowError::InvalidConfig(
                    "Invalid 'to' format".to_string(),
                ));
            }
        };

        // TODO: Send email via lettre transport
        // For now, log the email and return metadata
        tracing::info!(
            to = ?recipients,
            subject = subject,
            "Mail operation: sending email"
        );

        Ok(json!({
            "sent": true,
            "to": recipients,
            "subject": subject,
        }))
    }

    fn operation_type(&self) -> &str {
        "mail"
    }
}

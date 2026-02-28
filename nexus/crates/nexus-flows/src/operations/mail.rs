use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};
use lettre::{Message, SmtpTransport, Transport};
use lettre::message::header::ContentType;

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

        let body_text = options
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

        let from = nexus_env::env_string_or("EMAIL_FROM", "no-reply@example.com");
        let transport_type = nexus_env::env_string_or("EMAIL_TRANSPORT", "sendmail");

        let mut sent_count = 0;
        let mut errors: Vec<String> = Vec::new();

        for recipient in &recipients {
            let email = Message::builder()
                .from(from.parse().map_err(|e: lettre::address::AddressError| {
                    FlowError::OperationFailed(format!("Invalid from address: {}", e))
                })?)
                .to(recipient.parse().map_err(|e: lettre::address::AddressError| {
                    FlowError::OperationFailed(format!("Invalid recipient address: {}", e))
                })?)
                .subject(subject)
                .header(ContentType::TEXT_HTML)
                .body(body_text.to_string())
                .map_err(|e| FlowError::OperationFailed(format!("Failed to build email: {}", e)))?;

            let send_result: Result<(), String> = if transport_type == "smtp" {
                let host = nexus_env::env_string_or("EMAIL_SMTP_HOST", "localhost");
                let port: u16 = nexus_env::env_number_or("EMAIL_SMTP_PORT", 587) as u16;
                let user = std::env::var("EMAIL_SMTP_USER").ok();
                let pass = std::env::var("EMAIL_SMTP_PASSWORD").ok();
                let secure = nexus_env::env_string_or("EMAIL_SMTP_SECURE", "true");

                let mailer = if secure == "true" || port == 465 {
                    let mut builder = SmtpTransport::relay(&host)
                        .map_err(|e| FlowError::Internal(e.to_string()))?
                        .port(port);
                    if let (Some(u), Some(p)) = (user, pass) {
                        builder = builder.credentials(lettre::transport::smtp::authentication::Credentials::new(u, p));
                    }
                    builder.build()
                } else {
                    let mut builder = SmtpTransport::builder_dangerous(&host).port(port);
                    if let (Some(u), Some(p)) = (user, pass) {
                        builder = builder.credentials(lettre::transport::smtp::authentication::Credentials::new(u, p));
                    }
                    builder.build()
                };
                mailer.send(&email).map(|_| ()).map_err(|e| e.to_string())
            } else {
                // Use sendmail transport
                let mailer = lettre::SendmailTransport::new();
                mailer.send(&email).map_err(|e| e.to_string())
            };

            match send_result {
                Ok(()) => sent_count += 1,
                Err(e) => {
                    tracing::warn!(recipient = recipient.as_str(), error = e.as_str(), "Failed to send email");
                    errors.push(format!("{}: {}", recipient, e));
                }
            }
        }

        tracing::info!(
            to = ?recipients,
            subject = subject,
            sent = sent_count,
            "Mail operation: sent emails"
        );

        Ok(json!({
            "sent": sent_count > 0,
            "count": sent_count,
            "to": recipients,
            "subject": subject,
            "errors": errors,
        }))
    }

    fn operation_type(&self) -> &str {
        "mail"
    }
}

use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::Value;

/// Log operation — logs a message to the server console
/// Mirrors api/src/operations/log/index.ts
pub struct LogOperation;

#[async_trait]
impl FlowOperation for LogOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let message = options
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("Flow log");

        tracing::info!(
            flow_id = %context.flow_id,
            message = message,
            data = %data,
            "Flow log operation"
        );

        Ok(data)
    }

    fn operation_type(&self) -> &str {
        "log"
    }
}

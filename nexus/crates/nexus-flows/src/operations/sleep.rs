use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::Value;

/// Sleep operation — pauses execution for a specified duration
/// Mirrors api/src/operations/sleep/index.ts
pub struct SleepOperation;

#[async_trait]
impl FlowOperation for SleepOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let milliseconds = options
            .get("milliseconds")
            .and_then(|v| v.as_u64())
            .unwrap_or(1000);

        // Cap at 60 seconds to prevent abuse
        let capped = milliseconds.min(60_000);

        tokio::time::sleep(tokio::time::Duration::from_millis(capped)).await;

        Ok(data)
    }

    fn operation_type(&self) -> &str {
        "sleep"
    }
}

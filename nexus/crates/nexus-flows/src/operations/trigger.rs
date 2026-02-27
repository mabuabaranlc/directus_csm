use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Trigger operation — triggers another flow
/// Mirrors api/src/operations/trigger/index.ts
pub struct TriggerOperation;

#[async_trait]
impl FlowOperation for TriggerOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let flow_id = options
            .get("flow")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'flow' ID".to_string()))?;

        let payload = options
            .get("payload")
            .cloned()
            .unwrap_or(data);

        // TODO: Use FlowManager to trigger the target flow
        // This requires passing a reference to FlowManager into the operation context
        tracing::info!(
            target_flow = flow_id,
            "Trigger operation: triggering another flow"
        );

        Ok(json!({
            "flow": flow_id,
            "payload": payload,
            "triggered": true,
        }))
    }

    fn operation_type(&self) -> &str {
        "trigger"
    }
}

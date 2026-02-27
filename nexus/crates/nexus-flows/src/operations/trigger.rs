use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Trigger operation — triggers another flow
/// Mirrors api/src/operations/trigger/index.ts
///
/// Note: Full cross-flow triggering requires FlowManager to be accessible
/// from OperationContext. Currently logs the trigger intent and returns
/// the payload for the next step in the current flow.
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

        tracing::info!(
            target_flow = flow_id,
            "Trigger operation: triggering flow"
        );

        // Return the trigger result — the FlowManager event loop will pick up
        // flows triggered by the Operation trigger type
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

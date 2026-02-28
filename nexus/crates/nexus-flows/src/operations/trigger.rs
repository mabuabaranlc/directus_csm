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
        context: &OperationContext,
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

        // If we have a service context, use the emitter to trigger the target flow
        if let Some(svc_ctx) = context.service_context() {
            svc_ctx.emitter.emit_action(
                "flows.trigger",
                json!({
                    "flow": flow_id,
                    "payload": payload,
                    "source_flow": context.flow_id,
                }),
                json!({}),
            );
        }

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

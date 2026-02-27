use crate::{FlowDefinition, FlowError, FlowOperation, OperationContext, OperationDefinition, TriggerType};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Flow manager — loads, registers, and executes flows
/// Mirrors api/src/flows.ts
pub struct FlowManager {
    /// Registered flow definitions indexed by ID
    flows: Arc<RwLock<HashMap<String, FlowDefinition>>>,
    /// Operation definitions indexed by ID
    operations_defs: Arc<RwLock<HashMap<String, OperationDefinition>>>,
    /// Registered operation implementations indexed by type name
    operation_handlers: Arc<RwLock<HashMap<String, Arc<dyn FlowOperation>>>>,
    /// Event-triggered flows indexed by event scope
    event_flows: Arc<RwLock<HashMap<String, Vec<String>>>>,
    /// Webhook-triggered flows indexed by flow ID
    webhook_flows: Arc<RwLock<HashMap<String, String>>>,
}

impl FlowManager {
    pub fn new() -> Self {
        Self {
            flows: Arc::new(RwLock::new(HashMap::new())),
            operations_defs: Arc::new(RwLock::new(HashMap::new())),
            operation_handlers: Arc::new(RwLock::new(HashMap::new())),
            event_flows: Arc::new(RwLock::new(HashMap::new())),
            webhook_flows: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Register an operation handler (built-in or from extensions)
    pub async fn register_operation(&self, handler: Arc<dyn FlowOperation>) {
        let op_type = handler.operation_type().to_string();
        self.operation_handlers
            .write()
            .await
            .insert(op_type, handler);
    }

    /// Load flows and operations from the database
    pub async fn load_flows(
        &self,
        flows: Vec<FlowDefinition>,
        operations: Vec<OperationDefinition>,
    ) {
        let mut flows_map = self.flows.write().await;
        let mut ops_map = self.operations_defs.write().await;
        let mut event_map = self.event_flows.write().await;
        let mut webhook_map = self.webhook_flows.write().await;

        // Clear existing
        flows_map.clear();
        ops_map.clear();
        event_map.clear();
        webhook_map.clear();

        for flow in flows {
            if flow.status != "active" {
                continue;
            }

            match &flow.trigger {
                TriggerType::Event => {
                    // Register event-triggered flow
                    if let Some(options) = &flow.options {
                        if let Some(scope) = options.get("scope").and_then(|v| v.as_str()) {
                            event_map
                                .entry(scope.to_string())
                                .or_default()
                                .push(flow.id.clone());
                        }
                    }
                }
                TriggerType::Webhook => {
                    webhook_map.insert(flow.id.clone(), flow.id.clone());
                }
                TriggerType::Schedule => {
                    // TODO: Register cron jobs with tokio-cron-scheduler
                    info!(flow_id = %flow.id, "Schedule flow registered (cron not yet active)");
                }
                _ => {}
            }

            flows_map.insert(flow.id.clone(), flow);
        }

        for op in operations {
            ops_map.insert(op.id.clone(), op);
        }

        info!(
            flows = flows_map.len(),
            operations = ops_map.len(),
            "Flows loaded"
        );
    }

    /// Execute a flow by ID with the given trigger payload
    pub async fn execute_flow(
        &self,
        flow_id: &str,
        trigger_payload: Value,
    ) -> Result<Value, FlowError> {
        let flows = self.flows.read().await;
        let flow = flows.get(flow_id).ok_or(FlowError::NotFound)?;

        if flow.status != "active" {
            return Err(FlowError::InvalidConfig("Flow is not active".to_string()));
        }

        let first_operation_id = flow
            .operation
            .as_ref()
            .ok_or_else(|| FlowError::InvalidConfig("Flow has no operations".to_string()))?;

        let mut ctx = OperationContext::new(flow_id, trigger_payload.clone());
        ctx.accountability = flow.accountability.clone();
        ctx.set("$trigger".to_string(), trigger_payload);

        // Execute operation chain
        let result = self
            .execute_operation(first_operation_id, &mut ctx)
            .await?;

        Ok(result)
    }

    /// Execute a single operation and follow resolve/reject chain
    async fn execute_operation(
        &self,
        operation_id: &str,
        ctx: &mut OperationContext,
    ) -> Result<Value, FlowError> {
        let ops_defs = self.operations_defs.read().await;
        let op_def = ops_defs
            .get(operation_id)
            .ok_or_else(|| FlowError::NotFound)?
            .clone();
        drop(ops_defs);

        let handlers = self.operation_handlers.read().await;
        let handler = handlers.get(&op_def.op_type).ok_or_else(|| {
            FlowError::OperationFailed(format!(
                "No handler registered for operation type: {}",
                op_def.op_type
            ))
        })?;
        let handler = handler.clone();
        drop(handlers);

        let options = op_def.options.as_ref().cloned().unwrap_or(Value::Null);

        // Get the last result for $last reference
        let data = ctx
            .data
            .get("$last")
            .cloned()
            .unwrap_or(ctx.trigger_payload.clone());

        // Execute the operation
        match handler.execute(data, &options, ctx).await {
            Ok(result) => {
                // Store result under operation key and $last
                ctx.set(op_def.key.clone(), result.clone());
                ctx.set("$last".to_string(), result.clone());

                // Follow resolve chain
                if let Some(resolve_id) = &op_def.resolve {
                    Box::pin(self.execute_operation(resolve_id, ctx)).await
                } else {
                    Ok(result)
                }
            }
            Err(e) => {
                warn!(
                    operation = %op_def.key,
                    error = %e,
                    "Operation failed"
                );

                let error_value = serde_json::json!({
                    "error": e.to_string(),
                    "operation": op_def.key,
                });
                ctx.set(op_def.key, error_value.clone());
                ctx.set("$last".to_string(), error_value);

                // Follow reject chain
                if let Some(reject_id) = &op_def.reject {
                    Box::pin(self.execute_operation(reject_id, ctx)).await
                } else {
                    Err(e)
                }
            }
        }
    }

    /// Get flows triggered by a specific event scope
    pub async fn get_event_flows(&self, scope: &str) -> Vec<String> {
        let event_flows = self.event_flows.read().await;
        event_flows.get(scope).cloned().unwrap_or_default()
    }

    /// Trigger all flows matching an event scope
    pub async fn trigger_event(
        &self,
        scope: &str,
        payload: Value,
    ) -> Vec<Result<Value, FlowError>> {
        let flow_ids = self.get_event_flows(scope).await;
        let mut results = Vec::new();

        for flow_id in flow_ids {
            let result = self.execute_flow(&flow_id, payload.clone()).await;
            results.push(result);
        }

        results
    }

    /// Execute a webhook-triggered flow
    pub async fn trigger_webhook(
        &self,
        flow_id: &str,
        payload: Value,
    ) -> Result<Value, FlowError> {
        let webhook_flows = self.webhook_flows.read().await;
        if !webhook_flows.contains_key(flow_id) {
            return Err(FlowError::NotFound);
        }
        drop(webhook_flows);

        self.execute_flow(flow_id, payload).await
    }
}

impl Default for FlowManager {
    fn default() -> Self {
        Self::new()
    }
}

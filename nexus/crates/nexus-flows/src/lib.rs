use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use thiserror::Error;

pub mod manager;
pub mod operations;

#[derive(Debug, Error)]
pub enum FlowError {
    #[error("Flow not found")]
    NotFound,
    #[error("Operation failed: {0}")]
    OperationFailed(String),
    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
    #[error("Timeout: operation exceeded time limit")]
    Timeout,
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Trigger types for flows
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    /// Triggered by an event (action/filter hook)
    Event,
    /// Triggered on a schedule (cron expression)
    Schedule,
    /// Triggered by a webhook HTTP request
    Webhook,
    /// Triggered by an operation within another flow
    Operation,
    /// Manually triggered
    Manual,
}

/// Flow definition — loaded from directus_flows table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FlowDefinition {
    pub id: String,
    pub name: String,
    pub status: String,
    pub trigger: TriggerType,
    pub accountability: Option<String>,
    pub options: Option<Value>,
    pub operation: Option<String>,
    pub description: Option<String>,
    pub color: Option<String>,
    pub icon: Option<String>,
}

/// Operation definition — loaded from directus_operations table
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OperationDefinition {
    pub id: String,
    pub name: Option<String>,
    pub key: String,
    #[serde(rename = "type")]
    pub op_type: String,
    pub options: Option<Value>,
    pub position_x: Option<i32>,
    pub position_y: Option<i32>,
    pub resolve: Option<String>,
    pub reject: Option<String>,
    pub flow: String,
}

/// Trait for implementing flow operations
#[async_trait]
pub trait FlowOperation: Send + Sync {
    /// Execute the operation
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        context: &OperationContext,
    ) -> Result<Value, FlowError>;

    /// Get the operation type identifier
    fn operation_type(&self) -> &str;
}

/// Context passed to operations during execution
#[derive(Debug, Clone)]
pub struct OperationContext {
    pub flow_id: String,
    pub accountability: Option<String>,
    pub data: HashMap<String, Value>,
    pub trigger_payload: Value,
    /// Service context for database operations, stored as opaque type to avoid
    /// DashMap lifetime issues with async_trait
    service_ctx: Option<std::sync::Arc<dyn std::any::Any + Send + Sync>>,
}

impl OperationContext {
    pub fn new(flow_id: &str, trigger_payload: Value) -> Self {
        Self {
            flow_id: flow_id.to_string(),
            accountability: None,
            data: HashMap::new(),
            trigger_payload,
            service_ctx: None,
        }
    }

    /// Create a context with a service context for database operations
    pub fn with_service_ctx(flow_id: &str, trigger_payload: Value, service_ctx: nexus_services::context::ServiceContext) -> Self {
        Self {
            flow_id: flow_id.to_string(),
            accountability: None,
            data: HashMap::new(),
            trigger_payload,
            service_ctx: Some(std::sync::Arc::new(service_ctx)),
        }
    }

    /// Get the service context for database operations
    pub fn service_context(&self) -> Option<nexus_services::context::ServiceContext> {
        self.service_ctx
            .as_ref()
            .and_then(|ctx| ctx.downcast_ref::<nexus_services::context::ServiceContext>())
            .cloned()
    }

    /// Get data from a previous operation by key ($last, $trigger, or operation key)
    pub fn get(&self, key: &str) -> Option<&Value> {
        self.data.get(key)
    }

    /// Store result of an operation
    pub fn set(&mut self, key: String, value: Value) {
        self.data.insert(key, value);
    }
}

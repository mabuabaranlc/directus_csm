use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Item Create operation — creates an item in a collection
/// Mirrors api/src/operations/item-create/index.ts
pub struct ItemCreateOperation;

#[async_trait]
impl FlowOperation for ItemCreateOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let collection = options
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'collection'".to_string()))?
            .to_string();

        let payload = options
            .get("payload")
            .cloned()
            .unwrap_or(json!({}));

        // Extract service context early to avoid lifetime issues
        let svc_ctx = context.service_context().ok_or_else(|| {
            FlowError::Internal("No service context available".to_string())
        })?;

        let service = nexus_services::items::ItemsService::new(&collection, svc_ctx);

        let pk = service.create_one(payload, None).await.map_err(|e| {
            FlowError::OperationFailed(format!("Failed to create item: {}", e))
        })?;

        match service.read_one(&pk, None, None).await {
            Ok(item) => Ok(item),
            Err(_) => Ok(json!({ "id": pk.to_string() })),
        }
    }

    fn operation_type(&self) -> &str {
        "item-create"
    }
}

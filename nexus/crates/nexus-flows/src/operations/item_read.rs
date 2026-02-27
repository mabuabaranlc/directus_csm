use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Item Read operation — reads items from a collection
/// Mirrors api/src/operations/item-read/index.ts
pub struct ItemReadOperation;

#[async_trait]
impl FlowOperation for ItemReadOperation {
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

        let key = options.get("key").and_then(|v| v.as_str()).map(String::from);
        let query_opt = options.get("query").cloned();

        let svc_ctx = context.service_context().ok_or_else(|| {
            FlowError::Internal("No service context available".to_string())
        })?;

        let service = nexus_services::items::ItemsService::new(&collection, svc_ctx);

        if let Some(key) = key {
            let pk = nexus_types::items::PrimaryKey::String(key);
            let item = service.read_one(&pk, None, None).await.map_err(|e| {
                FlowError::OperationFailed(format!("Failed to read item: {}", e))
            })?;
            Ok(item)
        } else {
            let query = query_opt
                .and_then(|q| serde_json::from_value(q).ok())
                .unwrap_or_default();

            let items = service.read_by_query(query, None).await.map_err(|e| {
                FlowError::OperationFailed(format!("Failed to read items: {}", e))
            })?;

            Ok(json!(items))
        }
    }

    fn operation_type(&self) -> &str {
        "item-read"
    }
}

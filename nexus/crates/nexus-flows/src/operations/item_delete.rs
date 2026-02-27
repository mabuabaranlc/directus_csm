use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Item Delete operation — deletes items from a collection
/// Mirrors api/src/operations/item-delete/index.ts
pub struct ItemDeleteOperation;

#[async_trait]
impl FlowOperation for ItemDeleteOperation {
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
        let keys: Option<Vec<String>> = options
            .get("keys")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_str().map(String::from)).collect());

        let svc_ctx = context.service_context().ok_or_else(|| {
            FlowError::Internal("No service context available".to_string())
        })?;

        let service = nexus_services::items::ItemsService::new(&collection, svc_ctx);

        if let Some(key) = key {
            let pk = nexus_types::items::PrimaryKey::String(key.clone());
            service.delete_one(&pk, None).await.map_err(|e| {
                FlowError::OperationFailed(format!("Failed to delete item: {}", e))
            })?;
            Ok(json!({ "key": key }))
        } else if let Some(keys) = keys {
            let pks: Vec<nexus_types::items::PrimaryKey> = keys
                .iter()
                .map(|s| nexus_types::items::PrimaryKey::String(s.clone()))
                .collect();
            service.delete_many(&pks, None).await.map_err(|e| {
                FlowError::OperationFailed(format!("Failed to delete items: {}", e))
            })?;
            Ok(json!({ "keys": keys }))
        } else {
            Err(FlowError::InvalidConfig("Missing 'key' or 'keys'".to_string()))
        }
    }

    fn operation_type(&self) -> &str {
        "item-delete"
    }
}

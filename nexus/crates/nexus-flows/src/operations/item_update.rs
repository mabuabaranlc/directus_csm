use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Item Update operation — updates items in a collection
/// Mirrors api/src/operations/item-update/index.ts
pub struct ItemUpdateOperation;

#[async_trait]
impl FlowOperation for ItemUpdateOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let collection = options
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'collection'".to_string()))?;

        let key = options.get("key").cloned().unwrap_or(Value::Null);
        let payload = options.get("payload").cloned().unwrap_or(json!({}));

        // TODO: Use ItemsService to update items
        tracing::info!(collection = collection, "Item update operation");

        Ok(json!({
            "collection": collection,
            "key": key,
            "payload": payload,
        }))
    }

    fn operation_type(&self) -> &str {
        "item-update"
    }
}

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
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let collection = options
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'collection'".to_string()))?;

        let key = options.get("key").cloned().unwrap_or(Value::Null);

        // TODO: Use ItemsService to delete items
        tracing::info!(collection = collection, "Item delete operation");

        Ok(json!({
            "collection": collection,
            "key": key,
        }))
    }

    fn operation_type(&self) -> &str {
        "item-delete"
    }
}

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
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let collection = options
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'collection'".to_string()))?;

        let payload = options
            .get("payload")
            .cloned()
            .unwrap_or(json!({}));

        // TODO: Use ItemsService to create the item
        // Need to inject ServiceContext into OperationContext
        tracing::info!(collection = collection, "Item create operation");

        Ok(json!({
            "collection": collection,
            "payload": payload,
        }))
    }

    fn operation_type(&self) -> &str {
        "item-create"
    }
}

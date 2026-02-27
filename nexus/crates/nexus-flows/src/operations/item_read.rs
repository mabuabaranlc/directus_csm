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
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let collection = options
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'collection'".to_string()))?;

        let query = options.get("query").cloned().unwrap_or(json!({}));

        // TODO: Use ItemsService to read items
        tracing::info!(collection = collection, "Item read operation");

        Ok(json!({
            "collection": collection,
            "query": query,
        }))
    }

    fn operation_type(&self) -> &str {
        "item-read"
    }
}

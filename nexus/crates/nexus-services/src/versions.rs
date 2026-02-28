use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing content versions (drafts)
/// Mirrors api/src/services/versions.ts
pub struct VersionsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl VersionsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_versions", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new content version (draft)
    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        let mut version_data = data;
        if let Some(obj) = version_data.as_object_mut() {
            if obj.get("id").is_none() {
                obj.insert("id".to_string(), json!(uuid::Uuid::new_v4().to_string()));
            }
            if let Some(user) = self.ctx.user_id() {
                obj.insert("user_created".to_string(), json!(user));
            }
            obj.insert(
                "date_created".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
        }
        self.items.create_one(version_data, None).await
    }

    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    pub async fn read_one(&self, pk: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(pk, None, None).await
    }

    pub async fn update_one(
        &self,
        pk: &PrimaryKey,
        data: Value,
    ) -> Result<PrimaryKey, ServiceError> {
        self.items.update_one(pk, data, None).await
    }

    /// Save data to a version without promoting it
    pub async fn save(&self, pk: &PrimaryKey, data: Value) -> Result<(), ServiceError> {
        let mut save_data = json!({});
        if let Some(obj) = save_data.as_object_mut() {
            obj.insert("delta".to_string(), data);
            if let Some(user) = self.ctx.user_id() {
                obj.insert("user_updated".to_string(), json!(user));
            }
            obj.insert(
                "date_updated".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
        }
        self.items.update_one(pk, save_data, None).await?;
        Ok(())
    }

    /// Promote a version to live (apply changes to the main item)
    pub async fn promote(&self, pk: &PrimaryKey) -> Result<Value, ServiceError> {
        let version = self.items.read_one(pk, None, None).await?;

        // Get the collection and item ID from the version record
        let collection = version
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::InvalidPayload("Version missing collection".to_string()))?;
        let item_id = version
            .get("item")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::InvalidPayload("Version missing item reference".to_string()))?;

        // Get the delta to apply
        let delta = version
            .get("delta")
            .cloned()
            .unwrap_or(Value::Object(serde_json::Map::new()));

        if delta.is_null() || delta.as_object().map_or(true, |o| o.is_empty()) {
            return Err(ServiceError::InvalidPayload("Version has no changes to promote".to_string()));
        }

        // Apply the delta to the main item
        let main_items = crate::items::ItemsService::new(collection, self.ctx.clone());
        let main_pk = PrimaryKey::String(item_id.to_string());
        main_items.update_one(&main_pk, delta, None).await?;

        // Create an activity record for the promotion
        let activity_items = crate::items::ItemsService::new("directus_activity", self.ctx.clone());
        let activity_data = json!({
            "action": "version_promote",
            "collection": collection,
            "item": item_id,
            "user": self.ctx.user_id(),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "comment": format!("Promoted version {}", pk),
        });
        let _ = activity_items.create_one(activity_data, None).await;

        // Read the updated main item and return it
        let updated = main_items.read_one(&main_pk, None, None).await?;
        Ok(updated)
    }

    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        self.items.delete_one(pk, None).await
    }
}

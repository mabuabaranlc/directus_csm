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
        // TODO: Apply delta to main item, create activity record
        Ok(version)
    }

    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        self.items.delete_one(pk, None).await
    }
}

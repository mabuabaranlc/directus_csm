use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing shared views (public links)
/// Mirrors api/src/services/shares.ts
pub struct SharesService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl SharesService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_shares", ctx.clone());
        Self { ctx, items }
    }

    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        let mut share_data = data;
        if let Some(obj) = share_data.as_object_mut() {
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
        self.items.create_one(share_data, None).await
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

    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        self.items.delete_one(pk, None).await
    }
}

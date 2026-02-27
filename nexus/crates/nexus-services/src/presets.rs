use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::Value;

/// Service for managing layout presets (saved views)
/// Mirrors api/src/services/presets.ts
pub struct PresetsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl PresetsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_presets", ctx.clone());
        Self { ctx, items }
    }

    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        self.items.create_one(data, None).await
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

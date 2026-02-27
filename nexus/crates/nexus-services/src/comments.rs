use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing item comments
/// Mirrors api/src/services/comments.ts
pub struct CommentsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl CommentsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_comments", ctx.clone());
        Self { ctx, items }
    }

    pub async fn create_one(
        &self,
        collection: &str,
        item: &str,
        comment: &str,
    ) -> Result<PrimaryKey, ServiceError> {
        let data = json!({
            "collection": collection,
            "item": item,
            "comment": comment,
            "user_created": self.ctx.user_id(),
            "date_created": chrono::Utc::now().to_rfc3339(),
        });
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

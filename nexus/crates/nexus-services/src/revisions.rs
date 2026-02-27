use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing data revisions (audit trail of changes)
/// Mirrors api/src/services/revisions.ts
pub struct RevisionsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl RevisionsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_revisions", ctx.clone());
        Self { ctx, items }
    }

    /// Create a revision record for a data change
    pub async fn create_revision(
        &self,
        collection: &str,
        item: &str,
        action: &str,
        data: &Value,
        delta: &Value,
        activity_id: Option<i64>,
    ) -> Result<PrimaryKey, ServiceError> {
        let revision = json!({
            "activity": activity_id,
            "collection": collection,
            "item": item,
            "data": data,
            "delta": delta,
            "version": null,
        });

        self.items.create_one(revision, None).await
    }

    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    pub async fn read_one(&self, pk: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(pk, None, None).await
    }
}

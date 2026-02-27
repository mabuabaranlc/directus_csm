use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for activity/audit log tracking
/// Mirrors api/src/services/activity.ts
pub struct ActivityService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl ActivityService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_activity", ctx.clone());
        Self { ctx, items }
    }

    /// Create an activity record
    pub async fn create_activity(
        &self,
        action: &str,
        collection: &str,
        item: &str,
        comment: Option<&str>,
    ) -> Result<PrimaryKey, ServiceError> {
        let data = json!({
            "action": action,
            "collection": collection,
            "item": item,
            "user": self.ctx.user_id(),
            "ip": self.ctx.accountability.as_ref().and_then(|a| a.ip.clone()),
            "user_agent": self.ctx.accountability.as_ref().and_then(|a| a.user_agent.clone()),
            "origin": self.ctx.accountability.as_ref().and_then(|a| a.origin.clone()),
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "comment": comment,
        });

        self.items
            .create_one(data, Some(nexus_types::items::MutationOptions {
                emit_events: Some(false),
                ..Default::default()
            }))
            .await
    }

    /// Read activity records by query
    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    /// Read a single activity record
    pub async fn read_one(&self, key: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(key, None, None).await
    }

    /// Create a comment on an activity record
    pub async fn create_comment(
        &self,
        collection: &str,
        item: &str,
        comment: &str,
    ) -> Result<PrimaryKey, ServiceError> {
        self.create_activity("comment", collection, item, Some(comment))
            .await
    }
}

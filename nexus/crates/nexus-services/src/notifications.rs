use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing in-app notifications
/// Mirrors api/src/services/notifications.ts
pub struct NotificationsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl NotificationsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_notifications", ctx.clone());
        Self { ctx, items }
    }

    /// Send a notification to a user
    pub async fn send(
        &self,
        recipient: &str,
        subject: &str,
        message: Option<&str>,
        collection: Option<&str>,
        item: Option<&str>,
    ) -> Result<PrimaryKey, ServiceError> {
        let mut notification = json!({
            "recipient": recipient,
            "subject": subject,
            "status": "inbox",
            "timestamp": chrono::Utc::now().to_rfc3339(),
        });

        if let Some(obj) = notification.as_object_mut() {
            if let Some(msg) = message {
                obj.insert("message".to_string(), json!(msg));
            }
            if let Some(coll) = collection {
                obj.insert("collection".to_string(), json!(coll));
            }
            if let Some(itm) = item {
                obj.insert("item".to_string(), json!(itm));
            }
            if let Some(user) = self.ctx.user_id() {
                obj.insert("sender".to_string(), json!(user));
            }
        }

        self.items.create_one(notification, None).await
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

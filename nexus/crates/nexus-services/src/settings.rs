use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use serde_json::Value;

/// Service for managing project settings (singleton)
/// Mirrors api/src/services/settings.ts
pub struct SettingsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl SettingsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_settings", ctx.clone());
        Self { ctx, items }
    }

    /// Read the project settings (singleton)
    pub async fn read(&self) -> Result<Value, ServiceError> {
        self.items.read_singleton(None, None).await
    }

    /// Update the project settings
    pub async fn update(&self, data: Value) -> Result<Value, ServiceError> {
        let pk = PrimaryKey::Integer(1);
        self.items.update_one(&pk, data.clone(), None).await?;
        self.read().await
    }
}

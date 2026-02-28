use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing automation flows
/// Mirrors api/src/services/flows.ts
pub struct FlowsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl FlowsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_flows", ctx.clone());
        Self { ctx, items }
    }

    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        let pk = self.items.create_one(data.clone(), None).await?;

        // Emit event so FlowManager can register triggers
        self.ctx.emitter.emit_action(
            "flows.create",
            json!({ "key": pk, "payload": data }),
            json!({ "accountability": self.ctx.accountability }),
        );

        Ok(pk)
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
        let result = self.items.update_one(pk, data.clone(), None).await?;

        // Emit event so FlowManager can re-register triggers
        self.ctx.emitter.emit_action(
            "flows.update",
            json!({ "keys": [pk], "payload": data }),
            json!({ "accountability": self.ctx.accountability }),
        );

        Ok(result)
    }

    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        let result = self.items.delete_one(pk, None).await?;

        // Emit event so FlowManager can unregister triggers
        self.ctx.emitter.emit_action(
            "flows.delete",
            json!({ "keys": [pk] }),
            json!({ "accountability": self.ctx.accountability }),
        );

        Ok(result)
    }
}

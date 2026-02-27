use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing roles
/// Mirrors api/src/services/roles.ts
pub struct RolesService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl RolesService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_roles", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new role
    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        self.items.create_one(data, None).await
    }

    /// Read all roles
    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    /// Read a single role
    pub async fn read_one(&self, key: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(key, None, None).await
    }

    /// Update a role
    pub async fn update_one(
        &self,
        key: &PrimaryKey,
        data: Value,
    ) -> Result<PrimaryKey, ServiceError> {
        self.items.update_one(key, data, None).await
    }

    /// Delete a role
    pub async fn delete_one(&self, key: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        // Check that we're not deleting a role with admin users
        let users_query = Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("role".to_string(), json!({ "_eq": key.to_string() }))]
                    .into_iter()
                    .collect(),
            )),
            limit: Some(1),
            ..Default::default()
        };

        let users_service = ItemsService::new("directus_users", self.ctx.clone());
        let users = users_service.read_by_query(users_query, None).await?;

        if !users.is_empty() {
            return Err(ServiceError::InvalidPayload(
                "Can't delete a role that still has users assigned.".to_string(),
            ));
        }

        self.items.delete_one(key, None).await
    }
}

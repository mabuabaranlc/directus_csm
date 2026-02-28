use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_auth::providers::local::LocalAuthProvider;
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing users
/// Extends ItemsService with user-specific logic (password hashing, validation)
/// Mirrors api/src/services/users.ts
pub struct UsersService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl UsersService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_users", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new user (with password hashing)
    pub async fn create_one(&self, mut data: Value) -> Result<PrimaryKey, ServiceError> {
        // Hash password if provided
        if let Some(password) = data.get("password").and_then(|v| v.as_str()) {
            if !password.is_empty() {
                let hashed = LocalAuthProvider::hash_password(password)
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("password".to_string(), json!(hashed));
                }
            }
        }

        // Set default status if not provided
        if data.get("status").is_none() {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("status".to_string(), json!("active"));
            }
        }

        // Set default provider if not provided
        if data.get("provider").is_none() {
            if let Some(obj) = data.as_object_mut() {
                obj.insert("provider".to_string(), json!("default"));
            }
        }

        self.items.create_one(data, None).await
    }

    /// Read users by query
    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        let mut items = self.items.read_by_query(query, None).await?;

        // Strip passwords from output
        for item in &mut items {
            if let Some(obj) = item.as_object_mut() {
                obj.remove("password");
            }
        }

        Ok(items)
    }

    /// Read a single user
    pub async fn read_one(&self, key: &PrimaryKey) -> Result<Value, ServiceError> {
        let mut item = self.items.read_one(key, None, None).await?;

        if let Some(obj) = item.as_object_mut() {
            obj.remove("password");
        }

        Ok(item)
    }

    /// Update a user (with password hashing if changed)
    pub async fn update_one(
        &self,
        key: &PrimaryKey,
        mut data: Value,
    ) -> Result<PrimaryKey, ServiceError> {
        // Hash password if changed
        if let Some(password) = data.get("password").and_then(|v| v.as_str()) {
            if !password.is_empty() {
                let hashed = LocalAuthProvider::hash_password(password)
                    .map_err(|e| ServiceError::Internal(e.to_string()))?;
                if let Some(obj) = data.as_object_mut() {
                    obj.insert("password".to_string(), json!(hashed));
                }
            }
        }

        self.items.update_one(key, data, None).await
    }

    /// Delete a user
    pub async fn delete_one(&self, key: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        // Check that we're not deleting the last admin
        let user = self.items.read_one(key, None, None).await?;
        let is_admin = user
            .get("admin_access")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if is_admin {
            // Count remaining admins
            let admin_query = Query {
                filter: Some(nexus_types::filter::Filter::Field(
                    [("admin_access".to_string(), json!({ "_eq": true }))]
                        .into_iter()
                        .collect(),
                )),
                ..Default::default()
            };

            let admins = self.items.read_by_query(admin_query, None).await?;
            if admins.len() <= 1 {
                return Err(ServiceError::InvalidPayload(
                    "You can't delete the last admin user.".to_string(),
                ));
            }
        }

        self.items.delete_one(key, None).await
    }

    /// Invite a user by email
    pub async fn invite_user(
        &self,
        email: &str,
        role: &str,
    ) -> Result<(), ServiceError> {
        let invite_token = uuid::Uuid::new_v4().to_string();

        let user_data = json!({
            "email": email,
            "role": role,
            "status": "invited",
            "invite_token": invite_token,
        });

        self.create_one(user_data).await?;

        // Emit invite event so mail hook/flow can send the email
        let public_url = nexus_env::env_string_or("PUBLIC_URL", "http://localhost:8055");
        self.ctx.emitter.emit_action(
            "users.invite",
            json!({
                "email": email,
                "role": role,
                "token": invite_token,
                "url": format!("{}/admin/accept-invite?token={}", public_url, invite_token),
            }),
            json!({ "accountability": self.ctx.accountability }),
        );

        Ok(())
    }

    /// Accept an invitation
    pub async fn accept_invite(
        &self,
        token: &str,
        password: &str,
    ) -> Result<(), ServiceError> {
        let query = Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("invite_token".to_string(), json!({ "_eq": token }))]
                    .into_iter()
                    .collect(),
            )),
            limit: Some(1),
            ..Default::default()
        };

        let users = self.items.read_by_query(query, None).await?;
        let user = users.into_iter().next().ok_or_else(|| {
            ServiceError::NotFound("Invalid invite token.".to_string())
        })?;

        let user_id = user
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::Internal("Missing user ID".to_string()))?;

        let hashed = LocalAuthProvider::hash_password(password)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let pk = PrimaryKey::String(user_id.to_string());
        self.items
            .update_one(
                &pk,
                json!({
                    "password": hashed,
                    "status": "active",
                    "invite_token": null,
                }),
                None,
            )
            .await?;

        Ok(())
    }
}

use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_auth::jwt::TokenManager;
use nexus_auth::providers::local::LocalAuthProvider;
use nexus_types::items::PrimaryKey;
use serde_json::{json, Value};

/// Authentication service — handles login, refresh, logout, and password management
/// Mirrors api/src/services/authentication.ts
pub struct AuthenticationService {
    pub ctx: ServiceContext,
    users: ItemsService,
    sessions: ItemsService,
    token_manager: TokenManager,
}

impl AuthenticationService {
    pub fn new(ctx: ServiceContext) -> Self {
        let secret = nexus_env::env_string_or("SECRET", "nexus-default-secret-change-me");
        let access_ttl = nexus_env::env_number_or("ACCESS_TOKEN_TTL", 900);
        let refresh_ttl = nexus_env::env_number_or("REFRESH_TOKEN_TTL", 604800);

        let token_manager = TokenManager::new(&secret, "nexus", access_ttl, refresh_ttl);
        let users = ItemsService::new("directus_users", ctx.clone());
        let sessions = ItemsService::new("directus_sessions", ctx.clone());

        Self {
            ctx,
            users,
            sessions,
            token_manager,
        }
    }

    /// Login with email and password
    pub async fn login(
        &self,
        email: &str,
        password: &str,
        mode: Option<&str>,
        otp: Option<&str>,
    ) -> Result<Value, ServiceError> {
        // Find the user by email
        let query = nexus_types::query::Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("email".to_string(), json!({ "_eq": email }))]
                    .into_iter()
                    .collect(),
            )),
            limit: Some(1),
            ..Default::default()
        };

        let users = self.users.read_by_query(query, None).await?;
        let user = users.into_iter().next().ok_or_else(|| {
            ServiceError::Forbidden("Invalid user credentials.".to_string())
        })?;

        // Check if user is active
        let status = user
            .get("status")
            .and_then(|v| v.as_str())
            .unwrap_or("draft");

        if status == "suspended" {
            return Err(ServiceError::Forbidden("User suspended.".to_string()));
        }

        if status != "active" {
            return Err(ServiceError::Forbidden("Invalid user credentials.".to_string()));
        }

        // Verify password
        let stored_hash = user
            .get("password")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::Forbidden("Invalid user credentials.".to_string()))?;

        LocalAuthProvider::verify_password_hash(password, stored_hash)
            .map_err(|_| ServiceError::Forbidden("Invalid user credentials.".to_string()))?;

        // Check TFA if enabled
        if let Some(tfa_secret) = user.get("tfa_secret").and_then(|v| v.as_str()) {
            if !tfa_secret.is_empty() {
                let provided_otp = otp.ok_or_else(|| {
                    ServiceError::Forbidden("OTP required.".to_string())
                })?;
                // TODO: Verify OTP against tfa_secret
                let _ = provided_otp;
            }
        }

        let user_id = user
            .get("id")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::Internal("Missing user ID".to_string()))?;

        let role = user.get("role").and_then(|v| v.as_str());
        let admin = user
            .get("admin_access")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let app = user
            .get("app_access")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // Generate tokens
        let access_token = self
            .token_manager
            .generate_access_token(user_id, role, admin, app)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let refresh_token = self.token_manager.generate_refresh_token();
        let expires = self.token_manager.access_token_ttl() * 1000; // Convert to ms

        // Store the session/refresh token
        let session_data = json!({
            "token": refresh_token,
            "user": user_id,
            "expires": chrono::Utc::now() + chrono::Duration::seconds(nexus_env::env_number_or("REFRESH_TOKEN_TTL", 604800)),
            "ip": self.ctx.accountability.as_ref().and_then(|a| a.ip.clone()),
            "user_agent": self.ctx.accountability.as_ref().and_then(|a| a.user_agent.clone()),
        });

        self.sessions.create_one(session_data, None).await?;

        // Emit login action
        self.ctx.emitter.emit_action(
            "auth.login",
            json!({ "user": user_id }),
            json!({ "accountability": self.ctx.accountability }),
        );

        Ok(json!({
            "access_token": access_token,
            "refresh_token": refresh_token,
            "expires": expires,
        }))
    }

    /// Refresh an access token using a refresh token
    pub async fn refresh(&self, refresh_token: &str) -> Result<Value, ServiceError> {
        // Find the session
        let query = nexus_types::query::Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("token".to_string(), json!({ "_eq": refresh_token }))]
                    .into_iter()
                    .collect(),
            )),
            limit: Some(1),
            ..Default::default()
        };

        let sessions = self.sessions.read_by_query(query, None).await?;
        let session = sessions.into_iter().next().ok_or_else(|| {
            ServiceError::Forbidden("Invalid refresh token.".to_string())
        })?;

        let user_id = session
            .get("user")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::Internal("Invalid session".to_string()))?;

        // Load the user
        let pk = PrimaryKey::String(user_id.to_string());
        let user = self.users.read_one(&pk, None, None).await?;

        let role = user.get("role").and_then(|v| v.as_str());
        let admin = user.get("admin_access").and_then(|v| v.as_bool()).unwrap_or(false);
        let app = user.get("app_access").and_then(|v| v.as_bool()).unwrap_or(false);

        // Generate new tokens
        let access_token = self
            .token_manager
            .generate_access_token(user_id, role, admin, app)
            .map_err(|e| ServiceError::Internal(e.to_string()))?;

        let new_refresh_token = self.token_manager.generate_refresh_token();
        let expires = self.token_manager.access_token_ttl() * 1000;

        // Update the session with the new refresh token
        if let Some(session_id) = session.get("id") {
            let session_pk = PrimaryKey::String(
                session_id.as_str().unwrap_or_default().to_string(),
            );
            self.sessions
                .update_one(
                    &session_pk,
                    json!({ "token": new_refresh_token }),
                    None,
                )
                .await?;
        }

        Ok(json!({
            "access_token": access_token,
            "refresh_token": new_refresh_token,
            "expires": expires,
        }))
    }

    /// Logout — invalidate the refresh token
    pub async fn logout(&self, refresh_token: &str) -> Result<(), ServiceError> {
        let query = nexus_types::query::Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("token".to_string(), json!({ "_eq": refresh_token }))]
                    .into_iter()
                    .collect(),
            )),
            ..Default::default()
        };

        self.sessions.delete_by_query(query, None).await?;
        Ok(())
    }

    /// Verify a JWT token and return the claims
    pub fn verify_token(
        &self,
        token: &str,
    ) -> Result<nexus_auth::jwt::Claims, ServiceError> {
        self.token_manager
            .verify_token(token)
            .map_err(|e| ServiceError::Forbidden(e.to_string()))
    }
}

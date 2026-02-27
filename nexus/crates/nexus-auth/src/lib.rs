pub mod jwt;
pub mod providers;

use async_trait::async_trait;
use serde_json::Value;

/// Result type for authentication operations
pub type AuthResult<T> = Result<T, AuthError>;

/// Login result containing tokens
#[derive(Debug, Clone)]
pub struct LoginResult {
    pub access_token: String,
    pub refresh_token: String,
    pub expires: i64,
}

/// Trait for authentication providers
/// Mirrors api/src/auth/drivers/ pattern
#[async_trait]
pub trait AuthProvider: Send + Sync {
    /// Get the user ID from login payload
    async fn get_user_id(&self, payload: &Value) -> AuthResult<String>;

    /// Verify a user's password
    async fn verify_password(&self, user_id: &str, password: &str) -> AuthResult<()>;

    /// Perform login and return tokens
    async fn login(&self, payload: Value) -> AuthResult<LoginResult>;

    /// Get the driver name
    fn driver_name(&self) -> &str;
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error("Invalid credentials")]
    InvalidCredentials,
    #[error("Token expired")]
    TokenExpired,
    #[error("Invalid token: {0}")]
    InvalidToken(String),
    #[error("User suspended")]
    UserSuspended,
    #[error("Invalid OTP")]
    InvalidOtp,
    #[error("Provider error: {0}")]
    ProviderError(String),
    #[error("Configuration error: {0}")]
    ConfigError(String),
}

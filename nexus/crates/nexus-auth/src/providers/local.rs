use crate::{AuthError, AuthProvider, AuthResult, LoginResult};
use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use async_trait::async_trait;
use serde_json::Value;

/// Local authentication provider using argon2 password hashing
pub struct LocalAuthProvider {
    // In a full implementation, this would hold a reference to the database
    // for looking up users and their password hashes
}

impl LocalAuthProvider {
    pub fn new() -> Self {
        Self {}
    }

    /// Hash a password using argon2
    pub fn hash_password(password: &str) -> AuthResult<String> {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();

        argon2
            .hash_password(password.as_bytes(), &salt)
            .map(|hash| hash.to_string())
            .map_err(|e| AuthError::ProviderError(format!("Failed to hash password: {}", e)))
    }

    /// Verify a password against a hash
    pub fn verify_password_hash(password: &str, hash: &str) -> AuthResult<()> {
        let parsed_hash = PasswordHash::new(hash)
            .map_err(|e| AuthError::ProviderError(format!("Invalid hash format: {}", e)))?;

        Argon2::default()
            .verify_password(password.as_bytes(), &parsed_hash)
            .map_err(|_| AuthError::InvalidCredentials)
    }
}

impl Default for LocalAuthProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AuthProvider for LocalAuthProvider {
    async fn get_user_id(&self, payload: &Value) -> AuthResult<String> {
        payload
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or(AuthError::InvalidCredentials)
    }

    async fn verify_password(&self, _user_id: &str, _password: &str) -> AuthResult<()> {
        // In full implementation: look up user's password hash from DB and verify
        Err(AuthError::ProviderError(
            "Database integration not yet implemented".to_string(),
        ))
    }

    async fn login(&self, _payload: Value) -> AuthResult<LoginResult> {
        // In full implementation: verify credentials, generate tokens
        Err(AuthError::ProviderError(
            "Database integration not yet implemented".to_string(),
        ))
    }

    fn driver_name(&self) -> &str {
        "local"
    }
}

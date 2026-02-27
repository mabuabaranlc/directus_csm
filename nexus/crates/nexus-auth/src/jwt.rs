use crate::{AuthError, AuthResult};
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};

/// JWT claims matching Directus token structure
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub id: String,
    pub role: Option<String>,
    pub admin: bool,
    pub app: bool,
    pub iss: String,
    pub iat: i64,
    pub exp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share_scope: Option<String>,
}

/// JWT token manager
pub struct TokenManager {
    secret: String,
    issuer: String,
    access_token_ttl: i64,
    _refresh_token_ttl: i64,
}

impl TokenManager {
    pub fn new(secret: &str, issuer: &str, access_ttl_secs: i64, refresh_ttl_secs: i64) -> Self {
        Self {
            secret: secret.to_string(),
            issuer: issuer.to_string(),
            access_token_ttl: access_ttl_secs,
            _refresh_token_ttl: refresh_ttl_secs,
        }
    }

    /// Generate an access token
    pub fn generate_access_token(
        &self,
        user_id: &str,
        role: Option<&str>,
        admin: bool,
        app: bool,
    ) -> AuthResult<String> {
        let now = Utc::now();
        let claims = Claims {
            id: user_id.to_string(),
            role: role.map(|r| r.to_string()),
            admin,
            app,
            iss: self.issuer.clone(),
            iat: now.timestamp(),
            exp: (now + Duration::seconds(self.access_token_ttl)).timestamp(),
            share: None,
            share_scope: None,
        };

        encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.secret.as_bytes()),
        )
        .map_err(|e| AuthError::InvalidToken(e.to_string()))
    }

    /// Generate a refresh token
    pub fn generate_refresh_token(&self) -> String {
        uuid::Uuid::new_v4().to_string()
    }

    /// Verify and decode an access token
    pub fn verify_token(&self, token: &str) -> AuthResult<Claims> {
        let mut validation = Validation::default();
        validation.set_issuer(&[&self.issuer]);

        decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.secret.as_bytes()),
            &validation,
        )
        .map(|data| data.claims)
        .map_err(|e| match e.kind() {
            jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
            _ => AuthError::InvalidToken(e.to_string()),
        })
    }

    pub fn access_token_ttl(&self) -> i64 {
        self.access_token_ttl
    }
}

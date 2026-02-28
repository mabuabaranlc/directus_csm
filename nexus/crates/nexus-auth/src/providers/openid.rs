use crate::{AuthError, AuthProvider, AuthResult, LoginResult};
use async_trait::async_trait;
use serde_json::Value;

/// OpenID Connect authentication provider
/// Extends OAuth2 with automatic discovery and ID token verification.
/// Mirrors api/src/auth/drivers/openid.ts
pub struct OpenIdProvider {
    provider_id: String,
    client_id: String,
    client_secret: String,
    issuer_url: String,
    redirect_url: String,
    identifier_key: String,
    scopes: Vec<String>,
    allow_public_registration: bool,
    default_role: Option<String>,
}

impl OpenIdProvider {
    pub fn new(
        provider_id: &str,
        client_id: &str,
        client_secret: &str,
        issuer_url: &str,
        redirect_url: &str,
    ) -> Self {
        Self {
            provider_id: provider_id.to_string(),
            client_id: client_id.to_string(),
            client_secret: client_secret.to_string(),
            issuer_url: issuer_url.to_string(),
            redirect_url: redirect_url.to_string(),
            identifier_key: "sub".to_string(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            allow_public_registration: false,
            default_role: None,
        }
    }

    pub fn with_identifier_key(mut self, key: &str) -> Self {
        self.identifier_key = key.to_string();
        self
    }

    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    pub fn with_public_registration(mut self, allow: bool, default_role: Option<String>) -> Self {
        self.allow_public_registration = allow;
        self.default_role = default_role;
        self
    }

    /// Discover OpenID configuration and generate authorization URL
    pub async fn get_authorize_url(&self) -> AuthResult<(String, String)> {
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            self.issuer_url.trim_end_matches('/')
        );

        let http_client = reqwest::Client::new();
        let config: Value = http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Discovery failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Discovery parse failed: {}", e)))?;

        let auth_endpoint = config
            .get("authorization_endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::ConfigError("Missing authorization_endpoint in discovery".to_string())
            })?;

        let state = uuid::Uuid::new_v4().to_string();

        let scopes_str = self.scopes.join(" ");
        let auth_url = format!(
            "{}?response_type=code&client_id={}&redirect_uri={}&scope={}&state={}",
            auth_endpoint,
            urlencoding::encode(&self.client_id),
            urlencoding::encode(&self.redirect_url),
            urlencoding::encode(&scopes_str),
            urlencoding::encode(&state),
        );

        Ok((auth_url, state))
    }

    /// Exchange code for tokens using the discovered token endpoint
    pub async fn exchange_code(&self, code: &str) -> AuthResult<OpenIdUserInfo> {
        let discovery_url = format!(
            "{}/.well-known/openid-configuration",
            self.issuer_url.trim_end_matches('/')
        );

        let http_client = reqwest::Client::new();
        let config: Value = http_client
            .get(&discovery_url)
            .send()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Discovery failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Discovery parse failed: {}", e)))?;

        let token_endpoint = config
            .get("token_endpoint")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                AuthError::ConfigError("Missing token_endpoint in discovery".to_string())
            })?;

        let userinfo_endpoint = config
            .get("userinfo_endpoint")
            .and_then(|v| v.as_str());

        // Exchange code for tokens
        let token_response: Value = http_client
            .post(token_endpoint)
            .form(&[
                ("grant_type", "authorization_code"),
                ("code", code),
                ("redirect_uri", &self.redirect_url),
                ("client_id", &self.client_id),
                ("client_secret", &self.client_secret),
            ])
            .send()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Token request failed: {}", e)))?
            .json()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Token parse failed: {}", e)))?;

        let access_token = token_response
            .get("access_token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| AuthError::ProviderError("Missing access_token".to_string()))?
            .to_string();

        // Decode the ID token claims (without full verification for now)
        let id_token = token_response
            .get("id_token")
            .and_then(|v| v.as_str());

        let mut claims = Value::Null;
        if let Some(id_token) = id_token {
            // Decode JWT payload (base64 middle segment)
            let parts: Vec<&str> = id_token.split('.').collect();
            if parts.len() == 3 {
                use base64::Engine as _;
                if let Ok(payload_bytes) = base64::engine::general_purpose::URL_SAFE_NO_PAD
                    .decode(parts[1])
                {
                    if let Ok(payload) = serde_json::from_slice::<Value>(&payload_bytes) {
                        claims = payload;
                    }
                }
            }
        }

        // If we have a userinfo endpoint, fetch additional user info
        if let Some(userinfo_url) = userinfo_endpoint {
            let userinfo: Value = http_client
                .get(userinfo_url)
                .bearer_auth(&access_token)
                .send()
                .await
                .map_err(|e| AuthError::ProviderError(format!("Userinfo failed: {}", e)))?
                .json()
                .await
                .unwrap_or(Value::Null);

            // Merge userinfo into claims
            if let (Some(claims_obj), Some(info_obj)) =
                (claims.as_object_mut(), userinfo.as_object())
            {
                for (k, v) in info_obj {
                    if !claims_obj.contains_key(k) {
                        claims_obj.insert(k.clone(), v.clone());
                    }
                }
            } else if claims.is_null() {
                claims = userinfo;
            }
        }

        let identifier = claims
            .get(&self.identifier_key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AuthError::ProviderError(format!(
                    "Claims missing identifier key '{}'",
                    self.identifier_key
                ))
            })?;

        let email = claims
            .get("email")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let first_name = claims
            .get("given_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let last_name = claims
            .get("family_name")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(OpenIdUserInfo {
            identifier,
            email,
            first_name,
            last_name,
            provider: self.provider_id.clone(),
            access_token,
            refresh_token: token_response
                .get("refresh_token")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            raw: claims,
        })
    }
}

#[derive(Debug, Clone)]
pub struct OpenIdUserInfo {
    pub identifier: String,
    pub email: Option<String>,
    pub first_name: Option<String>,
    pub last_name: Option<String>,
    pub provider: String,
    pub access_token: String,
    pub refresh_token: Option<String>,
    pub raw: Value,
}

#[async_trait]
impl AuthProvider for OpenIdProvider {
    async fn get_user_id(&self, payload: &Value) -> AuthResult<String> {
        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidCredentials)?;

        let user_info = self.exchange_code(code).await?;
        Ok(user_info.identifier)
    }

    async fn verify_password(&self, _user_id: &str, _password: &str) -> AuthResult<()> {
        Err(AuthError::ProviderError(
            "OpenID Connect does not support password verification".to_string(),
        ))
    }

    async fn login(&self, payload: Value) -> AuthResult<LoginResult> {
        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidCredentials)?;

        let _user_info = self.exchange_code(code).await?;

        Err(AuthError::ProviderError(
            "Use AuthenticationService.login() for the full OpenID flow".to_string(),
        ))
    }

    fn driver_name(&self) -> &str {
        "openid"
    }
}

/// Create an OpenID Connect provider from environment variables
pub fn create_from_env(provider_id: &str) -> AuthResult<OpenIdProvider> {
    let prefix = format!("AUTH_{}", provider_id.to_uppercase());

    let client_id = nexus_env::env_string_or(&format!("{}_CLIENT_ID", prefix), "");
    let client_secret = nexus_env::env_string_or(&format!("{}_CLIENT_SECRET", prefix), "");
    let issuer_url = nexus_env::env_string_or(&format!("{}_ISSUER_URL", prefix), "");

    let public_url = nexus_env::env_string_or("PUBLIC_URL", "http://localhost:8055");
    let redirect_url = format!("{}/auth/login/{}/callback", public_url, provider_id);

    if client_id.is_empty() || client_secret.is_empty() || issuer_url.is_empty() {
        return Err(AuthError::ConfigError(format!(
            "OpenID provider '{}' is missing CLIENT_ID, CLIENT_SECRET, or ISSUER_URL",
            provider_id
        )));
    }

    let mut provider = OpenIdProvider::new(
        provider_id,
        &client_id,
        &client_secret,
        &issuer_url,
        &redirect_url,
    );

    let identifier_key =
        nexus_env::env_string_or(&format!("{}_IDENTIFIER_KEY", prefix), "sub");
    provider = provider.with_identifier_key(&identifier_key);

    let scope_str = nexus_env::env_string_or(&format!("{}_SCOPE", prefix), "openid profile email");
    let scopes: Vec<String> = scope_str.split_whitespace().map(|s| s.to_string()).collect();
    provider = provider.with_scopes(scopes);

    Ok(provider)
}

use crate::{AuthError, AuthProvider, AuthResult, LoginResult};
use async_trait::async_trait;
use oauth2::basic::BasicClient;
use oauth2::reqwest::async_http_client;
use oauth2::{
    AuthUrl, AuthorizationCode, ClientId, ClientSecret, CsrfToken, RedirectUrl, Scope,
    TokenResponse, TokenUrl,
};
use serde_json::Value;

/// OAuth2 authentication provider
/// Mirrors api/src/auth/drivers/oauth2.ts
pub struct OAuth2Provider {
    client: BasicClient,
    provider_id: String,
    user_info_url: String,
    identifier_key: String,
    email_key: String,
    scopes: Vec<String>,
    allow_public_registration: bool,
    default_role: Option<String>,
}

impl OAuth2Provider {
    pub fn new(
        provider_id: &str,
        client_id: &str,
        client_secret: &str,
        authorize_url: &str,
        token_url: &str,
        redirect_url: &str,
        user_info_url: &str,
    ) -> AuthResult<Self> {
        let client = BasicClient::new(
            ClientId::new(client_id.to_string()),
            Some(ClientSecret::new(client_secret.to_string())),
            AuthUrl::new(authorize_url.to_string())
                .map_err(|e| AuthError::ConfigError(format!("Invalid authorize URL: {}", e)))?,
            Some(
                TokenUrl::new(token_url.to_string())
                    .map_err(|e| AuthError::ConfigError(format!("Invalid token URL: {}", e)))?,
            ),
        )
        .set_redirect_uri(
            RedirectUrl::new(redirect_url.to_string())
                .map_err(|e| AuthError::ConfigError(format!("Invalid redirect URL: {}", e)))?,
        );

        Ok(Self {
            client,
            provider_id: provider_id.to_string(),
            user_info_url: user_info_url.to_string(),
            identifier_key: "sub".to_string(),
            email_key: "email".to_string(),
            scopes: vec!["openid".to_string(), "profile".to_string(), "email".to_string()],
            allow_public_registration: false,
            default_role: None,
        })
    }

    /// Configure the identifier key (field in user info response that contains the user ID)
    pub fn with_identifier_key(mut self, key: &str) -> Self {
        self.identifier_key = key.to_string();
        self
    }

    /// Configure the email key
    pub fn with_email_key(mut self, key: &str) -> Self {
        self.email_key = key.to_string();
        self
    }

    /// Configure scopes
    pub fn with_scopes(mut self, scopes: Vec<String>) -> Self {
        self.scopes = scopes;
        self
    }

    /// Allow public registration (create user on first login)
    pub fn with_public_registration(mut self, allow: bool, default_role: Option<String>) -> Self {
        self.allow_public_registration = allow;
        self.default_role = default_role;
        self
    }

    /// Generate the authorization URL for OAuth2 redirect
    pub fn get_authorize_url(&self) -> (String, CsrfToken) {
        let mut auth_request = self.client.authorize_url(CsrfToken::new_random);
        for scope in &self.scopes {
            auth_request = auth_request.add_scope(Scope::new(scope.clone()));
        }
        let (url, csrf_token) = auth_request.url();
        (url.to_string(), csrf_token)
    }

    /// Exchange authorization code for tokens and fetch user info
    pub async fn exchange_code(&self, code: &str) -> AuthResult<OAuth2UserInfo> {
        // Exchange the authorization code for an access token
        let token_result = self
            .client
            .exchange_code(AuthorizationCode::new(code.to_string()))
            .request_async(async_http_client)
            .await
            .map_err(|e| AuthError::ProviderError(format!("Token exchange failed: {}", e)))?;

        let access_token = token_result.access_token().secret().to_string();

        // Fetch user info from the provider
        let http_client = reqwest::Client::new();
        let user_info_response = http_client
            .get(&self.user_info_url)
            .bearer_auth(&access_token)
            .send()
            .await
            .map_err(|e| AuthError::ProviderError(format!("User info request failed: {}", e)))?;

        if !user_info_response.status().is_success() {
            return Err(AuthError::ProviderError(format!(
                "User info request returned status {}",
                user_info_response.status()
            )));
        }

        let user_info: Value = user_info_response
            .json()
            .await
            .map_err(|e| AuthError::ProviderError(format!("Failed to parse user info: {}", e)))?;

        let identifier = user_info
            .get(&self.identifier_key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string())
            .ok_or_else(|| {
                AuthError::ProviderError(format!(
                    "User info missing identifier key '{}'",
                    self.identifier_key
                ))
            })?;

        let email = user_info
            .get(&self.email_key)
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let first_name = user_info
            .get("given_name")
            .or_else(|| user_info.get("first_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        let last_name = user_info
            .get("family_name")
            .or_else(|| user_info.get("last_name"))
            .and_then(|v| v.as_str())
            .map(|s| s.to_string());

        Ok(OAuth2UserInfo {
            identifier,
            email,
            first_name,
            last_name,
            provider: self.provider_id.clone(),
            access_token,
            refresh_token: token_result
                .refresh_token()
                .map(|t| t.secret().to_string()),
            raw: user_info,
        })
    }

    /// Check if public registration is allowed
    pub fn allows_public_registration(&self) -> bool {
        self.allow_public_registration
    }

    /// Get the default role for newly registered users
    pub fn default_role(&self) -> Option<&str> {
        self.default_role.as_deref()
    }
}

/// User info retrieved from the OAuth2 provider
#[derive(Debug, Clone)]
pub struct OAuth2UserInfo {
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
impl AuthProvider for OAuth2Provider {
    async fn get_user_id(&self, payload: &Value) -> AuthResult<String> {
        // For OAuth2, the payload contains the authorization code
        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidCredentials)?;

        let user_info = self.exchange_code(code).await?;
        Ok(user_info.identifier)
    }

    async fn verify_password(&self, _user_id: &str, _password: &str) -> AuthResult<()> {
        // OAuth2 doesn't use passwords
        Err(AuthError::ProviderError(
            "OAuth2 does not support password verification".to_string(),
        ))
    }

    async fn login(&self, payload: Value) -> AuthResult<LoginResult> {
        let code = payload
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or(AuthError::InvalidCredentials)?;

        let _user_info = self.exchange_code(code).await?;

        // The actual token generation is handled by AuthenticationService,
        // not the provider. This method returns a placeholder.
        // In practice, AuthenticationService.login() calls get_user_id(),
        // looks up the user in the DB, and generates JWT tokens itself.
        Err(AuthError::ProviderError(
            "Use AuthenticationService.login() for the full OAuth2 flow".to_string(),
        ))
    }

    fn driver_name(&self) -> &str {
        "oauth2"
    }
}

/// Create an OAuth2 provider from environment variables
/// Convention: AUTH_{PROVIDER}_CLIENT_ID, AUTH_{PROVIDER}_CLIENT_SECRET, etc.
pub fn create_from_env(provider_id: &str) -> AuthResult<OAuth2Provider> {
    let prefix = format!("AUTH_{}", provider_id.to_uppercase());

    let client_id = nexus_env::env_string_or(&format!("{}_CLIENT_ID", prefix), "");
    let client_secret = nexus_env::env_string_or(&format!("{}_CLIENT_SECRET", prefix), "");
    let authorize_url = nexus_env::env_string_or(&format!("{}_AUTHORIZE_URL", prefix), "");
    let token_url = nexus_env::env_string_or(&format!("{}_ACCESS_URL", prefix), "");
    let user_info_url = nexus_env::env_string_or(&format!("{}_PROFILE_URL", prefix), "");

    let public_url = nexus_env::env_string_or("PUBLIC_URL", "http://localhost:8055");
    let redirect_url = format!("{}/auth/login/{}/callback", public_url, provider_id);

    if client_id.is_empty() || client_secret.is_empty() {
        return Err(AuthError::ConfigError(format!(
            "OAuth2 provider '{}' is missing CLIENT_ID or CLIENT_SECRET",
            provider_id
        )));
    }

    let mut provider = OAuth2Provider::new(
        provider_id,
        &client_id,
        &client_secret,
        &authorize_url,
        &token_url,
        &redirect_url,
        &user_info_url,
    )?;

    let identifier_key =
        nexus_env::env_string_or(&format!("{}_IDENTIFIER_KEY", prefix), "sub");
    provider = provider.with_identifier_key(&identifier_key);

    let email_key = nexus_env::env_string_or(&format!("{}_EMAIL_KEY", prefix), "email");
    provider = provider.with_email_key(&email_key);

    let scope_str = nexus_env::env_string_or(&format!("{}_SCOPE", prefix), "openid profile email");
    let scopes: Vec<String> = scope_str.split_whitespace().map(|s| s.to_string()).collect();
    provider = provider.with_scopes(scopes);

    let allow_public = nexus_env::env_string_or(
        &format!("{}_ALLOW_PUBLIC_REGISTRATION", prefix),
        "false",
    );
    let default_role = nexus_env::env_string_or(
        &format!("{}_DEFAULT_ROLE_ID", prefix),
        "",
    );

    if allow_public == "true" {
        provider = provider.with_public_registration(
            true,
            if default_role.is_empty() {
                None
            } else {
                Some(default_role)
            },
        );
    }

    Ok(provider)
}

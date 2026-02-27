use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{Error, HttpMessage};
use futures_util::future::{ok, LocalBoxFuture, Ready};
use nexus_types::accountability::Accountability;
use std::rc::Rc;

use super::extract_token::extract_token;

/// Authentication middleware — resolves the request token into an Accountability struct
/// Mirrors api/src/middleware/authenticate.ts
///
/// Does NOT reject unauthenticated requests — just attaches accountability when possible.
/// Individual endpoints decide whether authentication is required.
pub struct Authentication;

impl<S, B> Transform<S, ServiceRequest> for Authentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Transform = AuthenticationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(AuthenticationMiddleware {
            service: Rc::new(service),
        })
    }
}

pub struct AuthenticationMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for AuthenticationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = self.service.clone();

        Box::pin(async move {
            let token = extract_token(&req);

            if let Some(token) = token {
                // Try to resolve the token into accountability
                // In a full implementation, this would verify the JWT and look up the user
                if let Some(accountability) = resolve_accountability(&token).await {
                    req.extensions_mut().insert(accountability);
                }
            }

            // If there's no token or it failed to resolve, pass through with no accountability.
            // Public endpoints work without auth; protected endpoints check req.extensions().
            service.call(req).await
        })
    }
}

/// Resolve a token into accountability information
/// In production, this verifies the JWT and loads user/role data from the DB
async fn resolve_accountability(token: &str) -> Option<Accountability> {
    // Decode token (without full verification for now — just parse claims)
    let secret = nexus_env::env_string_or("SECRET", "nexus-default-secret-change-me");
    let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::HS256);
    validation.set_issuer(&["nexus"]);
    validation.validate_exp = true;

    #[derive(serde::Deserialize)]
    struct Claims {
        id: Option<String>,
        role: Option<String>,
        admin_access: Option<bool>,
        app_access: Option<bool>,
        session: Option<String>,
    }

    let token_data = jsonwebtoken::decode::<Claims>(token, &key, &validation).ok()?;
    let claims = token_data.claims;

    Some(Accountability {
        user: claims.id,
        role: claims.role.clone(),
        roles: claims.role.into_iter().collect(),
        admin: claims.admin_access.unwrap_or(false),
        app: claims.app_access.unwrap_or(false),
        session: claims.session,
        ..Default::default()
    })
}

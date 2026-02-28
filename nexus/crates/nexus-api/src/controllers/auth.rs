use actix_web::{web, HttpRequest, HttpResponse};
use serde::Deserialize;
use serde_json::json;

use crate::app_state::AppState;
use nexus_services::authentication::AuthenticationService;
use nexus_services::items::ServiceError;
use nexus_types::accountability::Accountability;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/login", web::post().to(login))
        .route("/refresh", web::post().to(refresh))
        .route("/logout", web::post().to(logout))
        .route("/password/request", web::post().to(password_request))
        .route("/password/reset", web::post().to(password_reset));
}

#[derive(Debug, Deserialize)]
struct LoginRequest {
    email: String,
    password: String,
    #[serde(default)]
    mode: Option<String>,
    #[serde(default)]
    otp: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RefreshRequest {
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    mode: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LogoutRequest {
    #[serde(default)]
    refresh_token: Option<String>,
}

async fn login(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<LoginRequest>,
) -> HttpResponse {
    // Build accountability from request IP and user-agent
    let ip = req
        .connection_info()
        .realip_remote_addr()
        .map(String::from);
    let user_agent = req
        .headers()
        .get("user-agent")
        .and_then(|v| v.to_str().ok())
        .map(String::from);

    let accountability = Accountability {
        user: None,
        role: None,
        admin: false,
        app: false,
        ip,
        user_agent,
        roles: Vec::new(),
        share: None,
        origin: None,
        session: None,
    };

    let ctx = state.service_context(Some(accountability)).await;
    let service = AuthenticationService::new(ctx);

    let mode = body.mode.as_deref().unwrap_or("json");

    match service
        .login(
            &body.email,
            &body.password,
            Some(mode),
            body.otp.as_deref(),
        )
        .await
    {
        Ok(tokens) => {
            match mode {
                "cookie" => {
                    // Set refresh token as httpOnly cookie
                    let refresh_token = tokens.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("");
                    let refresh_ttl = nexus_env::env_number_or("REFRESH_TOKEN_TTL", 604800);

                    let cookie = actix_web::cookie::Cookie::build("directus_refresh_token", refresh_token)
                        .path("/")
                        .http_only(true)
                        .secure(true)
                        .same_site(actix_web::cookie::SameSite::Lax)
                        .max_age(actix_web::cookie::time::Duration::seconds(refresh_ttl))
                        .finish();

                    HttpResponse::Ok()
                        .cookie(cookie)
                        .json(json!({
                            "data": {
                                "access_token": tokens.get("access_token"),
                                "expires": tokens.get("expires"),
                            }
                        }))
                }
                _ => HttpResponse::Ok().json(json!({ "data": tokens })),
            }
        }
        Err(e) => auth_error_response(&e),
    }
}

async fn refresh(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<RefreshRequest>,
) -> HttpResponse {
    let mode = body.mode.as_deref().unwrap_or("json");

    // Get refresh token from body or cookie
    let refresh_token = body.refresh_token.clone().or_else(|| {
        req.cookie("directus_refresh_token").map(|c| c.value().to_string())
    });

    let refresh_token = match refresh_token {
        Some(t) if !t.is_empty() => t,
        _ => return HttpResponse::BadRequest().json(json!({
            "errors": [{ "message": "Missing refresh token" }]
        })),
    };

    let ctx = state.service_context(None).await;
    let service = AuthenticationService::new(ctx);

    match service.refresh(&refresh_token).await {
        Ok(tokens) => {
            match mode {
                "cookie" => {
                    let new_refresh = tokens.get("refresh_token").and_then(|v| v.as_str()).unwrap_or("");
                    let refresh_ttl = nexus_env::env_number_or("REFRESH_TOKEN_TTL", 604800);

                    let cookie = actix_web::cookie::Cookie::build("directus_refresh_token", new_refresh)
                        .path("/")
                        .http_only(true)
                        .secure(true)
                        .same_site(actix_web::cookie::SameSite::Lax)
                        .max_age(actix_web::cookie::time::Duration::seconds(refresh_ttl))
                        .finish();

                    HttpResponse::Ok()
                        .cookie(cookie)
                        .json(json!({
                            "data": {
                                "access_token": tokens.get("access_token"),
                                "expires": tokens.get("expires"),
                            }
                        }))
                }
                _ => HttpResponse::Ok().json(json!({ "data": tokens })),
            }
        }
        Err(e) => auth_error_response(&e),
    }
}

async fn logout(
    state: web::Data<AppState>,
    body: web::Json<LogoutRequest>,
) -> HttpResponse {
    if let Some(ref token) = body.refresh_token {
        let ctx = state.service_context(None).await;
        let service = AuthenticationService::new(ctx);
        let _ = service.logout(token).await;
    }
    HttpResponse::NoContent().finish()
}

async fn password_request(
    state: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let email = match body.get("email").and_then(|v| v.as_str()) {
        Some(e) if !e.is_empty() => e.to_string(),
        _ => return HttpResponse::NoContent().finish(), // Don't leak user existence
    };

    let ctx = state.service_context(None).await;
    let users = nexus_services::items::ItemsService::new("directus_users", ctx.clone());

    // Look up user by email — always return 204 regardless to avoid leaking existence
    let query = nexus_types::query::Query {
        filter: Some(nexus_types::filter::Filter::Field(
            [("email".to_string(), json!({ "_eq": email }))]
                .into_iter()
                .collect(),
        )),
        limit: Some(1),
        ..Default::default()
    };

    if let Ok(results) = users.read_by_query(query, None).await {
        if let Some(user) = results.into_iter().next() {
            if let Some(user_id) = user.get("id").and_then(|v| v.as_str()) {
                // Generate a reset token and store it on the user
                let reset_token = uuid::Uuid::new_v4().to_string();
                let pk = nexus_types::items::PrimaryKey::String(user_id.to_string());
                let _ = users
                    .update_one(
                        &pk,
                        json!({
                            "password_reset_token": reset_token,
                            "password_reset_expiry": (chrono::Utc::now() + chrono::Duration::hours(1)).to_rfc3339(),
                        }),
                        None,
                    )
                    .await;

                // Emit event so mail hook/flow can send the email
                ctx.emitter.emit_action(
                    "auth.password_request",
                    json!({
                        "user": user_id,
                        "email": email,
                        "token": reset_token,
                    }),
                    json!({}),
                );
            }
        }
    }

    HttpResponse::NoContent().finish()
}

async fn password_reset(
    state: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let token = match body.get("token").and_then(|v| v.as_str()) {
        Some(t) if !t.is_empty() => t.to_string(),
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "errors": [{ "message": "Missing token", "extensions": { "code": "INVALID_PAYLOAD" } }]
            }))
        }
    };
    let password = match body.get("password").and_then(|v| v.as_str()) {
        Some(p) if !p.is_empty() => p.to_string(),
        _ => {
            return HttpResponse::BadRequest().json(json!({
                "errors": [{ "message": "Missing password", "extensions": { "code": "INVALID_PAYLOAD" } }]
            }))
        }
    };

    let ctx = state.service_context(None).await;
    let users = nexus_services::items::ItemsService::new("directus_users", ctx);

    // Find user by reset token
    let query = nexus_types::query::Query {
        filter: Some(nexus_types::filter::Filter::Field(
            [("password_reset_token".to_string(), json!({ "_eq": token }))]
                .into_iter()
                .collect(),
        )),
        limit: Some(1),
        ..Default::default()
    };

    let results = match users.read_by_query(query, None).await {
        Ok(r) => r,
        Err(_) => {
            return HttpResponse::Forbidden().json(json!({
                "errors": [{ "message": "Invalid or expired reset token", "extensions": { "code": "INVALID_TOKEN" } }]
            }))
        }
    };

    let user = match results.into_iter().next() {
        Some(u) => u,
        None => {
            return HttpResponse::Forbidden().json(json!({
                "errors": [{ "message": "Invalid or expired reset token", "extensions": { "code": "INVALID_TOKEN" } }]
            }))
        }
    };

    // Check expiry
    if let Some(expiry_str) = user
        .get("password_reset_expiry")
        .and_then(|v| v.as_str())
    {
        if let Ok(expiry) = chrono::DateTime::parse_from_rfc3339(expiry_str) {
            if chrono::Utc::now() > expiry {
                return HttpResponse::Forbidden().json(json!({
                    "errors": [{ "message": "Reset token has expired", "extensions": { "code": "TOKEN_EXPIRED" } }]
                }));
            }
        }
    }

    let user_id = match user.get("id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => {
            return HttpResponse::InternalServerError().json(json!({
                "errors": [{ "message": "Internal error" }]
            }))
        }
    };

    // Hash the new password
    let hashed = match nexus_auth::providers::local::LocalAuthProvider::hash_password(&password) {
        Ok(h) => h,
        Err(e) => {
            return HttpResponse::InternalServerError().json(json!({
                "errors": [{ "message": e.to_string() }]
            }))
        }
    };

    // Update the user's password and clear the reset token
    let pk = nexus_types::items::PrimaryKey::String(user_id);
    let _ = users
        .update_one(
            &pk,
            json!({
                "password": hashed,
                "password_reset_token": null,
                "password_reset_expiry": null,
            }),
            None,
        )
        .await;

    HttpResponse::NoContent().finish()
}

fn auth_error_response(e: &ServiceError) -> HttpResponse {
    match e {
        ServiceError::Forbidden(msg) => {
            HttpResponse::Unauthorized().json(json!({
                "errors": [{
                    "message": msg,
                    "extensions": { "code": "INVALID_CREDENTIALS" }
                }]
            }))
        }
        _ => {
            HttpResponse::InternalServerError().json(json!({
                "errors": [{
                    "message": e.to_string(),
                    "extensions": { "code": "INTERNAL_SERVER_ERROR" }
                }]
            }))
        }
    }
}

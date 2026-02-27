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
    refresh_token: String,
    #[serde(default, rename = "mode")]
    _mode: Option<String>,
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

    match service
        .login(
            &body.email,
            &body.password,
            body.mode.as_deref(),
            body.otp.as_deref(),
        )
        .await
    {
        Ok(tokens) => HttpResponse::Ok().json(json!({ "data": tokens })),
        Err(e) => auth_error_response(&e),
    }
}

async fn refresh(
    state: web::Data<AppState>,
    body: web::Json<RefreshRequest>,
) -> HttpResponse {
    let ctx = state.service_context(None).await;
    let service = AuthenticationService::new(ctx);

    match service.refresh(&body.refresh_token).await {
        Ok(tokens) => HttpResponse::Ok().json(json!({ "data": tokens })),
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
    _state: web::Data<AppState>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let _email = body.get("email").and_then(|v| v.as_str()).unwrap_or("");
    // In a full implementation: look up user, generate reset token, send email
    // For now, always return 204 to not leak user existence
    HttpResponse::NoContent().finish()
}

async fn password_reset(
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let _token = body.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let _password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
    // In a full implementation: verify reset token, update password
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

use actix_web::{web, HttpResponse};
use serde::Deserialize;
use serde_json::json;

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

async fn login(body: web::Json<LoginRequest>) -> HttpResponse {
    // TODO: Implement actual authentication
    HttpResponse::Ok().json(json!({
        "data": {
            "access_token": "placeholder",
            "expires": 900000,
            "refresh_token": "placeholder"
        }
    }))
}

async fn refresh() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "data": {
            "access_token": "placeholder",
            "expires": 900000,
            "refresh_token": "placeholder"
        }
    }))
}

async fn logout() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

async fn password_request() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

async fn password_reset() -> HttpResponse {
    HttpResponse::NoContent().finish()
}

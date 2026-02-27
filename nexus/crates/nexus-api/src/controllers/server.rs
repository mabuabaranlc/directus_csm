use actix_web::{web, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/info", web::get().to(server_info))
        .route("/ping", web::get().to(ping))
        .route("/health", web::get().to(health));
}

async fn server_info() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "data": {
            "project": {
                "project_name": "Nexus",
                "project_descriptor": null,
            },
            "nexus": {
                "version": env!("CARGO_PKG_VERSION"),
            },
            "node": {
                "version": "rust",
                "uptime": 0,
            }
        }
    }))
}

async fn ping() -> HttpResponse {
    HttpResponse::Ok().body("pong")
}

async fn health() -> HttpResponse {
    HttpResponse::Ok().json(json!({
        "status": "ok"
    }))
}

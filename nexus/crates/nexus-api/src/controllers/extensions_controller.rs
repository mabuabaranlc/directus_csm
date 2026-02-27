use actix_web::{web, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(list_extensions));
}

async fn list_extensions() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": [] }))
}

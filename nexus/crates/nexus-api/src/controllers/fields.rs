use actix_web::{web, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_all_fields))
        .route("/{collection}", web::get().to(read_fields))
        .route("/{collection}", web::post().to(create_field))
        .route("/{collection}/{field}", web::get().to(read_field))
        .route("/{collection}/{field}", web::patch().to(update_field))
        .route("/{collection}/{field}", web::delete().to(delete_field));
}

async fn read_all_fields() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": [] }))
}

async fn read_fields(path: web::Path<String>) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": [] }))
}

async fn create_field(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": body.into_inner() }))
}

async fn read_field(path: web::Path<(String, String)>) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": {} }))
}

async fn update_field(
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": body.into_inner() }))
}

async fn delete_field(path: web::Path<(String, String)>) -> HttpResponse {
    HttpResponse::NoContent().finish()
}

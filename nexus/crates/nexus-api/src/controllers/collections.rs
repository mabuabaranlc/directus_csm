use actix_web::{web, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_collections))
        .route("", web::post().to(create_collection))
        .route("/{collection}", web::get().to(read_collection))
        .route("/{collection}", web::patch().to(update_collection))
        .route("/{collection}", web::delete().to(delete_collection));
}

async fn read_collections() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": [] }))
}

async fn create_collection(body: web::Json<serde_json::Value>) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": body.into_inner() }))
}

async fn read_collection(path: web::Path<String>) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": {} }))
}

async fn update_collection(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": body.into_inner() }))
}

async fn delete_collection(path: web::Path<String>) -> HttpResponse {
    HttpResponse::NoContent().finish()
}

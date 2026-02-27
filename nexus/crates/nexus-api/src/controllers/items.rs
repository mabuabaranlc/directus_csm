use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_items))
        .route("", web::post().to(create_item))
        .route("/{pk}", web::get().to(read_item))
        .route("/{pk}", web::patch().to(update_item))
        .route("/{pk}", web::delete().to(delete_item));
}

async fn read_items(
    path: web::Path<String>,
    req: HttpRequest,
) -> HttpResponse {
    let collection = path.into_inner();
    // TODO: Parse query params, run through ItemsService
    HttpResponse::Ok().json(json!({
        "data": [],
        "meta": {
            "total_count": 0,
            "filter_count": 0
        }
    }))
}

async fn create_item(
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let collection = path.into_inner();
    // TODO: Run through ItemsService
    HttpResponse::Ok().json(json!({
        "data": body.into_inner()
    }))
}

async fn read_item(
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (collection, pk) = path.into_inner();
    // TODO: Run through ItemsService
    HttpResponse::Ok().json(json!({
        "data": {}
    }))
}

async fn update_item(
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let (collection, pk) = path.into_inner();
    // TODO: Run through ItemsService
    HttpResponse::Ok().json(json!({
        "data": body.into_inner()
    }))
}

async fn delete_item(
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (collection, pk) = path.into_inner();
    // TODO: Run through ItemsService
    HttpResponse::NoContent().finish()
}

use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use nexus_services::collections::CollectionsService;
use nexus_types::accountability::Accountability;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_collections))
        .route("", web::post().to(create_collection))
        .route("/{collection}", web::get().to(read_collection))
        .route("/{collection}", web::patch().to(update_collection))
        .route("/{collection}", web::delete().to(delete_collection));
}

async fn read_collections(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = CollectionsService::new(ctx);

    match service.read_all().await {
        Ok(collections) => HttpResponse::Ok().json(json!({ "data": collections })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn create_collection(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = CollectionsService::new(ctx);

    match service.create_one(body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_collection(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = CollectionsService::new(ctx);
    let collection = path.into_inner();

    match service.read_one(&collection).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_collection(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = CollectionsService::new(ctx);
    let collection = path.into_inner();

    match service.update_one(&collection, body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_collection(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = CollectionsService::new(ctx);
    let collection = path.into_inner();

    match service.delete_one(&collection).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

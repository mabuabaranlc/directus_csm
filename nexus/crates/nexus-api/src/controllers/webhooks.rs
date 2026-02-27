use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::webhooks::WebhooksService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_many))
        .route("", web::post().to(create_one))
        .route("/{pk}", web::get().to(read_one))
        .route("/{pk}", web::patch().to(update_one))
        .route("/{pk}", web::delete().to(delete_one));
}

async fn read_many(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = WebhooksService::new(ctx);
    let query = sanitize_query(req.query_string());
    match service.read_by_query(query).await {
        Ok(items) => HttpResponse::Ok().json(json!({ "data": items })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

async fn create_one(state: web::Data<AppState>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = WebhooksService::new(ctx);
    match service.create_one(body.into_inner()).await {
        Ok(pk) => match service.read_one(&pk).await {
            Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
            Err(_) => HttpResponse::Ok().json(json!({ "data": { "id": pk.to_string() } })),
        },
        Err(e) => HttpResponse::BadRequest().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

async fn read_one(state: web::Data<AppState>, req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = WebhooksService::new(ctx);
    let pk = parse_pk(&path.into_inner());
    match service.read_one(&pk).await {
        Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
        Err(e) => HttpResponse::NotFound().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

async fn update_one(state: web::Data<AppState>, req: HttpRequest, path: web::Path<String>, body: web::Json<serde_json::Value>) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = WebhooksService::new(ctx);
    let pk = parse_pk(&path.into_inner());
    match service.update_one(&pk, body.into_inner()).await {
        Ok(_) => match service.read_one(&pk).await {
            Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
            Err(_) => HttpResponse::Ok().json(json!({ "data": {} })),
        },
        Err(e) => HttpResponse::BadRequest().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

async fn delete_one(state: web::Data<AppState>, req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = WebhooksService::new(ctx);
    let pk = parse_pk(&path.into_inner());
    match service.delete_one(&pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::BadRequest().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

fn parse_pk(s: &str) -> PrimaryKey {
    if let Ok(id) = s.parse::<i64>() { PrimaryKey::Integer(id) } else { PrimaryKey::String(s.to_string()) }
}

use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::revisions::RevisionsService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_many))
        .route("/{pk}", web::get().to(read_one));
}

async fn read_many(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RevisionsService::new(ctx);
    let query = sanitize_query(req.query_string());
    match service.read_by_query(query).await {
        Ok(items) => HttpResponse::Ok().json(json!({ "data": items })),
        Err(e) => HttpResponse::InternalServerError().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

async fn read_one(state: web::Data<AppState>, req: HttpRequest, path: web::Path<String>) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RevisionsService::new(ctx);
    let pk = parse_pk(&path.into_inner());
    match service.read_one(&pk).await {
        Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
        Err(e) => HttpResponse::NotFound().json(json!({ "errors": [{ "message": e.to_string() }] })),
    }
}

fn parse_pk(s: &str) -> PrimaryKey {
    if let Ok(id) = s.parse::<i64>() { PrimaryKey::Integer(id) } else { PrimaryKey::String(s.to_string()) }
}

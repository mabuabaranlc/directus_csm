use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::roles::RolesService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_roles))
        .route("", web::post().to(create_role))
        .route("/{pk}", web::get().to(read_role))
        .route("/{pk}", web::patch().to(update_role))
        .route("/{pk}", web::delete().to(delete_role));
}

async fn read_roles(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RolesService::new(ctx);
    let query = sanitize_query(req.query_string());

    match service.read_by_query(query).await {
        Ok(roles) => HttpResponse::Ok().json(json!({ "data": roles })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn create_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RolesService::new(ctx);

    match service.create_one(body.into_inner()).await {
        Ok(pk) => HttpResponse::Ok().json(json!({ "data": { "id": pk } })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RolesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.read_one(&pk).await {
        Ok(role) => HttpResponse::Ok().json(json!({ "data": role })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RolesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.update_one(&pk, body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "data": {} })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_role(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RolesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.delete_one(&pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

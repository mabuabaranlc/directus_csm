use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::notifications::NotificationsService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_notifications))
        .route("/{pk}", web::get().to(read_notification))
        .route("/{pk}", web::patch().to(update_notification))
        .route("/{pk}", web::delete().to(delete_notification));
}

async fn read_notifications(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = NotificationsService::new(ctx);
    let query = sanitize_query(req.query_string());

    match service.read_by_query(query).await {
        Ok(items) => HttpResponse::Ok().json(json!({ "data": items })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_notification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = NotificationsService::new(ctx);

    let pk_str = path.into_inner();
    let pk = if let Ok(id) = pk_str.parse::<i64>() {
        PrimaryKey::Integer(id)
    } else {
        PrimaryKey::String(pk_str)
    };

    match service.read_one(&pk).await {
        Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_notification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = NotificationsService::new(ctx);

    let pk_str = path.into_inner();
    let pk = if let Ok(id) = pk_str.parse::<i64>() {
        PrimaryKey::Integer(id)
    } else {
        PrimaryKey::String(pk_str)
    };

    match service.update_one(&pk, body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "data": {} })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_notification(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = NotificationsService::new(ctx);

    let pk_str = path.into_inner();
    let pk = if let Ok(id) = pk_str.parse::<i64>() {
        PrimaryKey::Integer(id)
    } else {
        PrimaryKey::String(pk_str)
    };

    match service.delete_one(&pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

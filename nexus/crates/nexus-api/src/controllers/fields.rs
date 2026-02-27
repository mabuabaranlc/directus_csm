use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use nexus_services::fields::FieldsService;
use nexus_types::accountability::Accountability;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_all_fields))
        .route("/{collection}", web::get().to(read_fields))
        .route("/{collection}", web::post().to(create_field))
        .route("/{collection}/{field}", web::get().to(read_field))
        .route("/{collection}/{field}", web::patch().to(update_field))
        .route("/{collection}/{field}", web::delete().to(delete_field));
}

async fn read_all_fields(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);

    match service.read_all().await {
        Ok(fields) => HttpResponse::Ok().json(json!({ "data": fields })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_fields(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);
    let collection = path.into_inner();

    match service.read_for_collection(&collection).await {
        Ok(fields) => HttpResponse::Ok().json(json!({ "data": fields })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn create_field(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);
    let collection = path.into_inner();

    match service.create_field(&collection, body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_field(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);
    let (collection, field) = path.into_inner();

    match service.read_one(&collection, &field).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_field(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);
    let (collection, field) = path.into_inner();

    match service.update_field(&collection, &field, body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_field(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FieldsService::new(ctx);
    let (collection, field) = path.into_inner();

    match service.delete_field(&collection, &field).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

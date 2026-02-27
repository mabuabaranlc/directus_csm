use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::items::ItemsService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_items))
        .route("", web::post().to(create_item))
        .route("/{pk}", web::get().to(read_item))
        .route("/{pk}", web::patch().to(update_item))
        .route("/{pk}", web::delete().to(delete_item));
}

async fn read_items(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let collection = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = ItemsService::new(&collection, ctx);
    let query = sanitize_query(req.query_string());

    match service.read_by_query(query, None).await {
        Ok(items) => HttpResponse::Ok().json(json!({
            "data": items
        })),
        Err(e) => error_response(&e),
    }
}

async fn create_item(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let collection = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = ItemsService::new(&collection, ctx);

    let data = body.into_inner();

    // Support both single item and array of items
    if let Some(arr) = data.as_array() {
        match service.create_many(arr.clone(), None).await {
            Ok(keys) => HttpResponse::Ok().json(json!({ "data": keys.iter().map(|k| k.to_string()).collect::<Vec<_>>() })),
            Err(e) => error_response(&e),
        }
    } else {
        match service.create_one(data, None).await {
            Ok(pk) => {
                // Read back the created item to return full data
                match service.read_one(&pk, None, None).await {
                    Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
                    Err(_) => HttpResponse::Ok().json(json!({ "data": { "id": pk.to_string() } })),
                }
            }
            Err(e) => error_response(&e),
        }
    }
}

async fn read_item(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (collection, pk_str) = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = ItemsService::new(&collection, ctx);
    let pk = parse_pk(&pk_str);
    let query = sanitize_query(req.query_string());

    match service.read_one(&pk, Some(query), None).await {
        Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
        Err(e) => error_response(&e),
    }
}

async fn update_item(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let (collection, pk_str) = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = ItemsService::new(&collection, ctx);
    let pk = parse_pk(&pk_str);

    match service.update_one(&pk, body.into_inner(), None).await {
        Ok(_) => {
            // Read back the updated item
            match service.read_one(&pk, None, None).await {
                Ok(item) => HttpResponse::Ok().json(json!({ "data": item })),
                Err(_) => HttpResponse::Ok().json(json!({ "data": {} })),
            }
        }
        Err(e) => error_response(&e),
    }
}

async fn delete_item(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<(String, String)>,
) -> HttpResponse {
    let (collection, pk_str) = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = ItemsService::new(&collection, ctx);
    let pk = parse_pk(&pk_str);

    match service.delete_one(&pk, None).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => error_response(&e),
    }
}

fn parse_pk(s: &str) -> PrimaryKey {
    if let Ok(id) = s.parse::<i64>() {
        PrimaryKey::Integer(id)
    } else {
        PrimaryKey::String(s.to_string())
    }
}

fn error_response(e: &nexus_services::items::ServiceError) -> HttpResponse {
    use nexus_services::items::ServiceError;
    let (status, code) = match e {
        ServiceError::NotFound(_) => (actix_web::http::StatusCode::NOT_FOUND, "RECORD_NOT_UNIQUE"),
        ServiceError::Forbidden(_) => (actix_web::http::StatusCode::FORBIDDEN, "FORBIDDEN"),
        ServiceError::InvalidPayload(_) => (actix_web::http::StatusCode::BAD_REQUEST, "INVALID_PAYLOAD"),
        ServiceError::Database(_) | ServiceError::Internal(_) => {
            (actix_web::http::StatusCode::INTERNAL_SERVER_ERROR, "INTERNAL_SERVER_ERROR")
        }
    };
    HttpResponse::build(status).json(json!({
        "errors": [{
            "message": e.to_string(),
            "extensions": { "code": code }
        }]
    }))
}

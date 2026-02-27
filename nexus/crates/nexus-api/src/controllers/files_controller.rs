use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::files::FilesService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_files))
        .route("", web::post().to(upload_file))
        .route("/{pk}", web::get().to(read_file))
        .route("/{pk}", web::patch().to(update_file))
        .route("/{pk}", web::delete().to(delete_file))
        .route("/import", web::post().to(import_file));
}

async fn read_files(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);
    let query = sanitize_query(req.query_string());

    match service.read_by_query(query).await {
        Ok(files) => HttpResponse::Ok().json(json!({ "data": files })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn upload_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);

    match service.upload(body.into_inner(), None).await {
        Ok(pk) => HttpResponse::Ok().json(json!({ "data": { "id": pk } })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.read_one(&pk).await {
        Ok(file) => HttpResponse::Ok().json(json!({ "data": file })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.update_one(&pk, body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "data": {} })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.delete_one(&pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn import_file(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);

    let url = body.get("url").and_then(|v| v.as_str()).unwrap_or("");
    let data = body.get("data").cloned().unwrap_or(json!({}));

    match service.import_one(url, data).await {
        Ok(pk) => HttpResponse::Ok().json(json!({ "data": { "id": pk } })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

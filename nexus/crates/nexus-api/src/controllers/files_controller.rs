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
    payload: actix_multipart::Multipart,
) -> HttpResponse {
    use futures_util::StreamExt;

    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let storage = state.storage.as_ref().cloned();
    let mut service = FilesService::new(ctx);
    if let Some(s) = storage {
        service = service.with_storage(s);
    }

    let mut file_bytes: Option<Vec<u8>> = None;
    let mut file_data = serde_json::Map::new();
    let mut payload = payload;

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(e) => {
                return HttpResponse::BadRequest().json(json!({
                    "errors": [{ "message": format!("Multipart error: {}", e) }]
                }));
            }
        };

        let field_name = field
            .content_disposition()
            .and_then(|cd| cd.get_name().map(|s| s.to_string()))
            .unwrap_or_default();

        if field_name == "file" {
            // Binary file field
            let content_type = field.content_type().map(|m| m.to_string());
            let filename: Option<String> = field
                .content_disposition()
                .and_then(|cd| cd.get_filename().map(|s| s.to_string()));

            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                match chunk {
                    Ok(b) => bytes.extend_from_slice(&b),
                    Err(e) => {
                        return HttpResponse::BadRequest().json(json!({
                            "errors": [{ "message": format!("Read error: {}", e) }]
                        }));
                    }
                }
            }

            if let Some(ct) = content_type {
                file_data.insert("type".to_string(), json!(ct));
            }
            if let Some(fname) = filename {
                file_data.insert("filename_download".to_string(), json!(&fname));
                if !file_data.contains_key("filename_disk") {
                    file_data.insert("filename_disk".to_string(), json!(&fname));
                }
            }

            file_bytes = Some(bytes);
        } else {
            // Metadata field
            let mut bytes = Vec::new();
            while let Some(chunk) = field.next().await {
                if let Ok(b) = chunk {
                    bytes.extend_from_slice(&b);
                }
            }
            if let Ok(val) = String::from_utf8(bytes) {
                // Try to parse as JSON, else use as string
                let json_val = serde_json::from_str::<serde_json::Value>(&val)
                    .unwrap_or(json!(val));
                file_data.insert(field_name, json_val);
            }
        }
    }

    match service.upload(json!(file_data), file_bytes).await {
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

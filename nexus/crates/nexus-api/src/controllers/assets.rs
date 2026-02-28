use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use nexus_services::files::FilesService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

/// Assets controller — serves file contents (images, documents, etc.)
/// Mirrors api/src/controllers/assets.ts
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/{pk}", web::get().to(get_asset));
}

async fn get_asset(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = FilesService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    // Read file content via the storage driver
    match service.read_file_content(&pk).await {
        Ok((bytes, content_type)) => {
            // Get metadata for the download filename
            let filename = service
                .read_one(&pk)
                .await
                .ok()
                .and_then(|f| {
                    f.get("filename_download")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "file".to_string());

            HttpResponse::Ok()
                .content_type(content_type)
                .insert_header((
                    "Content-Disposition",
                    format!("inline; filename=\"{}\"", filename),
                ))
                .insert_header(("Cache-Control", "public, max-age=31536000"))
                .body(bytes)
        }
        Err(_) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": "Asset not found" }]
        })),
    }
}

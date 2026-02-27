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

    // Look up file metadata
    match service.read_one(&pk).await {
        Ok(file) => {
            // In a full implementation, this would:
            // 1. Read the file from storage
            // 2. Apply image transformations (width, height, quality, format, fit)
            // 3. Set proper Content-Type and Cache-Control headers
            // 4. Stream the file content

            let filename = file
                .get("filename_download")
                .and_then(|v| v.as_str())
                .unwrap_or("file");
            let content_type = file
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("application/octet-stream");

            HttpResponse::Ok()
                .content_type(content_type)
                .insert_header(("Content-Disposition", format!("inline; filename=\"{}\"", filename)))
                .insert_header(("Cache-Control", "public, max-age=31536000"))
                .body("") // TODO: Serve actual file content from storage
        }
        Err(_) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": "Asset not found" }]
        })),
    }
}

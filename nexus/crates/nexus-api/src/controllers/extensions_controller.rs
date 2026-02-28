use actix_web::{web, HttpResponse};
use nexus_extensions::ExtensionManager;
use serde_json::json;
use std::sync::Arc;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(list_extensions));
}

async fn list_extensions(
    ext_manager: Option<web::Data<Arc<ExtensionManager>>>,
) -> HttpResponse {
    let extensions: Vec<serde_json::Value> = match ext_manager {
        Some(manager) => manager
            .list()
            .into_iter()
            .map(|manifest| {
                json!({
                    "name": manifest.name,
                    "type": manifest.extension_type,
                    "version": manifest.version,
                    "description": manifest.description,
                    "enabled": true,
                })
            })
            .collect(),
        None => Vec::new(),
    };

    HttpResponse::Ok().json(json!({ "data": extensions }))
}

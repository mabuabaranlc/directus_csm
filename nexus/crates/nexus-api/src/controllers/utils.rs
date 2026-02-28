use actix_web::{web, HttpRequest, HttpMessage, HttpResponse};
use serde_json::json;
use uuid::Uuid;

use crate::app_state::AppState;
use nexus_auth::providers::local::LocalAuthProvider;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/random/string", web::get().to(random_string))
        .route("/hash/generate", web::post().to(hash_generate))
        .route("/hash/verify", web::post().to(hash_verify))
        .route("/sort/{collection}", web::post().to(sort));
}

async fn random_string() -> HttpResponse {
    HttpResponse::Ok().json(json!({ "data": Uuid::new_v4().to_string() }))
}

async fn hash_generate(body: web::Json<serde_json::Value>) -> HttpResponse {
    let string = body.get("string").and_then(|v| v.as_str()).unwrap_or("");
    match LocalAuthProvider::hash_password(string) {
        Ok(hash) => HttpResponse::Ok().json(json!({ "data": hash })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": format!("Hash generation failed: {}", e) }]
        })),
    }
}

async fn hash_verify(body: web::Json<serde_json::Value>) -> HttpResponse {
    let string = body.get("string").and_then(|v| v.as_str()).unwrap_or("");
    let hash = body.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    let matches = LocalAuthProvider::verify_password_hash(string, hash).is_ok();
    HttpResponse::Ok().json(json!({ "data": matches }))
}

async fn sort(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let collection = path.into_inner();
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = nexus_services::items::ItemsService::new(&collection, ctx);

    let item_pk = match body.get("item").and_then(|v| v.as_str()) {
        Some(pk) => PrimaryKey::String(pk.to_string()),
        None => return HttpResponse::BadRequest().json(json!({
            "errors": [{ "message": "Missing 'item' in request body" }]
        })),
    };

    let to = match body.get("to") {
        Some(v) => v.clone(),
        None => return HttpResponse::BadRequest().json(json!({
            "errors": [{ "message": "Missing 'to' in request body" }]
        })),
    };

    match service.update_one(&item_pk, json!({ "sort": to }), None).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

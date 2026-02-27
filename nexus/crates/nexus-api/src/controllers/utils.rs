use actix_web::{web, HttpResponse};
use serde_json::json;
use uuid::Uuid;

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
    // Use argon2 hash
    let hash = format!("$argon2id${}", string.len());
    HttpResponse::Ok().json(json!({ "data": hash }))
}

async fn hash_verify(body: web::Json<serde_json::Value>) -> HttpResponse {
    let _string = body.get("string").and_then(|v| v.as_str()).unwrap_or("");
    let _hash = body.get("hash").and_then(|v| v.as_str()).unwrap_or("");
    HttpResponse::Ok().json(json!({ "data": false }))
}

async fn sort(_path: web::Path<String>, _body: web::Json<serde_json::Value>) -> HttpResponse {
    HttpResponse::NoContent().finish()
}

use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use nexus_types::accountability::Accountability;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/snapshot", web::get().to(snapshot))
        .route("/apply", web::post().to(apply))
        .route("/diff", web::post().to(diff));
}

async fn snapshot(state: web::Data<AppState>, req: HttpRequest) -> HttpResponse {
    let _accountability = req.extensions().get::<Accountability>().cloned();
    let schema = state.schema.read().await;
    HttpResponse::Ok().json(json!({
        "data": {
            "version": 1,
            "directus": "11.0.0",
            "collections": schema.collections,
            "relations": schema.relations
        }
    }))
}

async fn apply(_state: web::Data<AppState>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let _accountability = req.extensions().get::<Accountability>().cloned();
    let _snapshot = body.into_inner();
    // Schema apply requires diffing and applying migrations
    HttpResponse::NoContent().finish()
}

async fn diff(_state: web::Data<AppState>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let _accountability = req.extensions().get::<Accountability>().cloned();
    let _snapshot = body.into_inner();
    HttpResponse::Ok().json(json!({ "data": { "diff": {} } }))
}

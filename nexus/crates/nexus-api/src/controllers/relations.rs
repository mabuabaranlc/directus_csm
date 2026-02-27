use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use nexus_services::relations::RelationsService;
use nexus_types::accountability::Accountability;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_relations))
        .route("", web::post().to(create_relation))
        .route("/{collection}", web::get().to(read_collection_relations))
        .route("/{pk}", web::patch().to(update_relation))
        .route("/{pk}", web::delete().to(delete_relation));
}

async fn read_relations(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RelationsService::new(ctx);

    match service.read_all().await {
        Ok(relations) => HttpResponse::Ok().json(json!({ "data": relations })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn create_relation(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RelationsService::new(ctx);

    match service.create_one(body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_collection_relations(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RelationsService::new(ctx);
    let collection = path.into_inner();

    match service.read_for_collection(&collection).await {
        Ok(relations) => HttpResponse::Ok().json(json!({ "data": relations })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_relation(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RelationsService::new(ctx);

    let pk: i64 = match path.into_inner().parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "errors": [{ "message": "Invalid relation ID" }]
            }));
        }
    };

    match service.update_one(pk, body.into_inner()).await {
        Ok(result) => HttpResponse::Ok().json(json!({ "data": result })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_relation(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = RelationsService::new(ctx);

    let pk: i64 = match path.into_inner().parse() {
        Ok(id) => id,
        Err(_) => {
            return HttpResponse::BadRequest().json(json!({
                "errors": [{ "message": "Invalid relation ID" }]
            }));
        }
    };

    match service.delete_one(pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

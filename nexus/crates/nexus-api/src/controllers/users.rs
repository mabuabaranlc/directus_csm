use actix_web::{web, HttpRequest, HttpResponse, HttpMessage};
use serde_json::json;

use crate::app_state::AppState;
use crate::middleware::sanitize_query::sanitize_query;
use nexus_services::users::UsersService;
use nexus_types::accountability::Accountability;
use nexus_types::items::PrimaryKey;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(read_users))
        .route("", web::post().to(create_user))
        .route("/me", web::get().to(read_me))
        .route("/me", web::patch().to(update_me))
        .route("/{pk}", web::get().to(read_user))
        .route("/{pk}", web::patch().to(update_user))
        .route("/{pk}", web::delete().to(delete_user))
        .route("/invite", web::post().to(invite_user))
        .route("/invite/accept", web::post().to(accept_invite));
}

async fn read_users(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);
    let query = sanitize_query(req.query_string());

    match service.read_by_query(query).await {
        Ok(users) => HttpResponse::Ok().json(json!({ "data": users })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn create_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);

    match service.create_one(body.into_inner()).await {
        Ok(pk) => HttpResponse::Ok().json(json!({ "data": { "id": pk } })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn read_me(
    state: web::Data<AppState>,
    req: HttpRequest,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let user_id = accountability.as_ref().and_then(|a| a.user.clone());

    if let Some(user_id) = user_id {
        let ctx = state.service_context(accountability).await;
        let service = UsersService::new(ctx);
        let pk = PrimaryKey::String(user_id);
        match service.read_one(&pk).await {
            Ok(user) => HttpResponse::Ok().json(json!({ "data": user })),
            Err(e) => HttpResponse::NotFound().json(json!({
                "errors": [{ "message": e.to_string() }]
            })),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "errors": [{ "message": "You must be logged in to access this." }]
        }))
    }
}

async fn update_me(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let user_id = accountability.as_ref().and_then(|a| a.user.clone());

    if let Some(user_id) = user_id {
        let ctx = state.service_context(accountability).await;
        let service = UsersService::new(ctx);
        let pk = PrimaryKey::String(user_id);
        match service.update_one(&pk, body.into_inner()).await {
            Ok(_) => HttpResponse::Ok().json(json!({ "data": {} })),
            Err(e) => HttpResponse::InternalServerError().json(json!({
                "errors": [{ "message": e.to_string() }]
            })),
        }
    } else {
        HttpResponse::Unauthorized().json(json!({
            "errors": [{ "message": "You must be logged in to access this." }]
        }))
    }
}

async fn read_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.read_one(&pk).await {
        Ok(user) => HttpResponse::Ok().json(json!({ "data": user })),
        Err(e) => HttpResponse::NotFound().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn update_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.update_one(&pk, body.into_inner()).await {
        Ok(_) => HttpResponse::Ok().json(json!({ "data": {} })),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn delete_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    path: web::Path<String>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);
    let pk = PrimaryKey::String(path.into_inner());

    match service.delete_one(&pk).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn invite_user(
    state: web::Data<AppState>,
    req: HttpRequest,
    body: web::Json<serde_json::Value>,
) -> HttpResponse {
    let accountability = req.extensions().get::<Accountability>().cloned();
    let ctx = state.service_context(accountability).await;
    let service = UsersService::new(ctx);

    let email = body.get("email").and_then(|v| v.as_str()).unwrap_or("");
    let role = body.get("role").and_then(|v| v.as_str()).unwrap_or("");

    match service.invite_user(email, role).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

async fn accept_invite(
    body: web::Json<serde_json::Value>,
    state: web::Data<AppState>,
) -> HttpResponse {
    let ctx = state.service_context(None).await;
    let service = UsersService::new(ctx);

    let token = body.get("token").and_then(|v| v.as_str()).unwrap_or("");
    let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");

    match service.accept_invite(token, password).await {
        Ok(_) => HttpResponse::NoContent().finish(),
        Err(e) => HttpResponse::InternalServerError().json(json!({
            "errors": [{ "message": e.to_string() }]
        })),
    }
}

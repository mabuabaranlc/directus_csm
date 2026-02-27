use actix_web::dev::{ServiceRequest, ServiceResponse};
use actix_web::HttpMessage;

/// Extract bearer token from Authorization header, query param, or cookie
/// Mirrors api/src/middleware/extract-token.ts
pub fn extract_token(req: &ServiceRequest) -> Option<String> {
    // 1. Authorization: Bearer <token>
    if let Some(auth) = req.headers().get("Authorization") {
        if let Ok(auth_str) = auth.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                return Some(token.to_string());
            }
        }
    }

    // 2. Query parameter: ?access_token=<token>
    let query = req.query_string();
    for pair in query.split('&') {
        if let Some(token) = pair.strip_prefix("access_token=") {
            return Some(token.to_string());
        }
    }

    // 3. Cookie: directus_session_token or session_token
    if let Some(cookie) = req.cookie("directus_session_token") {
        return Some(cookie.value().to_string());
    }
    if let Some(cookie) = req.cookie("session_token") {
        return Some(cookie.value().to_string());
    }

    None
}

pub mod rest;

use actix_web::web;

/// Configure WebSocket routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/websocket", web::get().to(rest::websocket_handler));
}

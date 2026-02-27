pub mod controllers;
pub mod middleware;
pub mod openapi;

use actix_web::web;

/// Configure all Nexus API routes
pub fn configure_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(
        web::scope("/api")
            // System routes
            .service(web::scope("/server").configure(controllers::server::configure))
            // Auth routes
            .service(web::scope("/auth").configure(controllers::auth::configure))
            // Dynamic CRUD routes
            .service(web::scope("/items/{collection}").configure(controllers::items::configure))
            // Schema management routes
            .service(web::scope("/collections").configure(controllers::collections::configure))
            .service(web::scope("/fields").configure(controllers::fields::configure))
            // Documentation routes
            .service(web::scope("/docs").configure(openapi::configure)),
    );
}

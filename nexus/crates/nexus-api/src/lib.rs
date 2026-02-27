pub mod app_state;
pub mod controllers;
pub mod middleware;
pub mod openapi;

use actix_web::web;

/// Configure all Nexus API routes
/// Mirrors the full set of Directus REST endpoints
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
            .service(web::scope("/relations").configure(controllers::relations::configure))
            // User management routes
            .service(web::scope("/users").configure(controllers::users::configure))
            .service(web::scope("/roles").configure(controllers::roles::configure))
            // Permissions routes
            .service(web::scope("/permissions").configure(controllers::permissions::configure))
            // Activity / audit log
            .service(web::scope("/activity").configure(controllers::activity::configure))
            // Files and assets
            .service(web::scope("/files").configure(controllers::files_controller::configure))
            .service(web::scope("/assets").configure(controllers::assets::configure))
            // Settings (singleton)
            .service(web::scope("/settings").configure(controllers::settings_controller::configure))
            // Notifications
            .service(web::scope("/notifications").configure(controllers::notifications_controller::configure))
            // Documentation routes
            .service(web::scope("/docs").configure(openapi::configure)),
    );
}

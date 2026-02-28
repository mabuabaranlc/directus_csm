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
            // Permissions and policies
            .service(web::scope("/permissions").configure(controllers::permissions::configure))
            .service(web::scope("/policies").configure(controllers::policies::configure))
            // Activity / audit log
            .service(web::scope("/activity").configure(controllers::activity::configure))
            .service(web::scope("/revisions").configure(controllers::revisions::configure))
            // Files and assets
            .service(web::scope("/files").configure(controllers::files_controller::configure))
            .service(web::scope("/folders").configure(controllers::folders::configure))
            .service(web::scope("/assets").configure(controllers::assets::configure))
            // Settings (singleton)
            .service(web::scope("/settings").configure(controllers::settings_controller::configure))
            // Notifications
            .service(web::scope("/notifications").configure(controllers::notifications_controller::configure))
            // Insights
            .service(web::scope("/dashboards").configure(controllers::dashboards::configure))
            .service(web::scope("/panels").configure(controllers::panels::configure))
            // Presets (saved views)
            .service(web::scope("/presets").configure(controllers::presets::configure))
            // Automation
            .service(web::scope("/flows").configure(controllers::flows::configure))
            .service(web::scope("/operations").configure(controllers::operations_controller::configure))
            .service(web::scope("/webhooks").configure(controllers::webhooks::configure))
            // Collaboration
            .service(web::scope("/shares").configure(controllers::shares::configure))
            .service(web::scope("/translations").configure(controllers::translations::configure))
            .service(web::scope("/versions").configure(controllers::versions::configure))
            .service(web::scope("/comments").configure(controllers::comments::configure))
            // Extensions
            .service(web::scope("/extensions").configure(controllers::extensions_controller::configure))
            // Schema snapshot/apply
            .service(web::scope("/schema").configure(controllers::schema::configure))
            // Utilities
            .service(web::scope("/utils").configure(controllers::utils::configure))
            // GraphQL endpoint
            .service(web::scope("/graphql").configure(controllers::graphql::configure))
            // Documentation routes
            .service(web::scope("/docs").configure(openapi::configure)),
    );
}

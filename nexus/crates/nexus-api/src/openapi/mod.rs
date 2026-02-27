use actix_web::{web, HttpResponse};

/// Configure documentation routes
pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("/swagger", web::get().to(swagger_ui))
        .route("/rapidoc", web::get().to(rapidoc_ui))
        .route("/redoc", web::get().to(redoc_ui))
        .route("/openapi.json", web::get().to(openapi_spec));
}

async fn swagger_ui() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<!DOCTYPE html>
<html>
<head>
    <title>Nexus API - Swagger UI</title>
    <link rel="stylesheet" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        SwaggerUIBundle({ url: '/api/docs/openapi.json', dom_id: '#swagger-ui' });
    </script>
</body>
</html>"#)
}

async fn rapidoc_ui() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<!DOCTYPE html>
<html>
<head>
    <title>Nexus API - RapiDoc</title>
    <script type="module" src="https://unpkg.com/rapidoc/dist/rapidoc-min.js"></script>
</head>
<body>
    <rapi-doc spec-url="/api/docs/openapi.json" theme="dark" render-style="read" show-header="false"></rapi-doc>
</body>
</html>"#)
}

async fn redoc_ui() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html")
        .body(r#"<!DOCTYPE html>
<html>
<head>
    <title>Nexus API - ReDoc</title>
</head>
<body>
    <redoc spec-url="/api/docs/openapi.json"></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#)
}

async fn openapi_spec() -> HttpResponse {
    // TODO: Generate dynamic OpenAPI spec from schema
    HttpResponse::Ok().json(serde_json::json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Nexus API",
            "description": "Nexus CMS REST API — Directus-compatible",
            "version": env!("CARGO_PKG_VERSION")
        },
        "servers": [
            { "url": "/", "description": "Current server" }
        ],
        "paths": {
            "/api/server/ping": {
                "get": {
                    "summary": "Ping",
                    "responses": { "200": { "description": "pong" } }
                }
            },
            "/api/server/info": {
                "get": {
                    "summary": "Server info",
                    "responses": { "200": { "description": "Server information" } }
                }
            },
            "/api/auth/login": {
                "post": {
                    "summary": "Login",
                    "requestBody": {
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "email": { "type": "string" },
                                        "password": { "type": "string" }
                                    },
                                    "required": ["email", "password"]
                                }
                            }
                        }
                    },
                    "responses": { "200": { "description": "Auth tokens" } }
                }
            },
            "/api/items/{collection}": {
                "get": {
                    "summary": "List items",
                    "parameters": [
                        { "name": "collection", "in": "path", "required": true, "schema": { "type": "string" } },
                        { "name": "fields", "in": "query", "schema": { "type": "string" } },
                        { "name": "filter", "in": "query", "schema": { "type": "object" } },
                        { "name": "sort", "in": "query", "schema": { "type": "string" } },
                        { "name": "limit", "in": "query", "schema": { "type": "integer" } },
                        { "name": "offset", "in": "query", "schema": { "type": "integer" } },
                        { "name": "search", "in": "query", "schema": { "type": "string" } }
                    ],
                    "responses": { "200": { "description": "List of items" } }
                },
                "post": {
                    "summary": "Create item",
                    "responses": { "200": { "description": "Created item" } }
                }
            }
        }
    }))
}

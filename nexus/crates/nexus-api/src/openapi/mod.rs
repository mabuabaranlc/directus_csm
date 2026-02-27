use actix_web::{web, HttpResponse};
use crate::app_state::AppState;
use nexus_types::fields::FieldType;
use nexus_types::schema::SchemaOverview;
use serde_json::{json, Map, Value};

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

/// Generate the full dynamic OpenAPI spec from schema + static system endpoints
async fn openapi_spec(state: web::Data<AppState>) -> HttpResponse {
    let schema = state.schema.read().await.clone();
    let spec = build_openapi_spec(&schema);
    HttpResponse::Ok().json(spec)
}

fn build_openapi_spec(schema: &SchemaOverview) -> Value {
    let mut paths = Map::new();
    let mut schemas = Map::new();

    // Static system endpoints
    insert_system_paths(&mut paths);

    // Dynamic per-collection endpoints
    for (name, col) in &schema.collections {
        if name.starts_with("nexus_") {
            continue;
        }

        let (col_schema, col_input_schema) = build_collection_schemas(col);
        let schema_ref = format!("{}Items", capitalize(name));
        let input_ref = format!("{}Input", capitalize(name));
        schemas.insert(schema_ref.clone(), col_schema);
        schemas.insert(input_ref.clone(), col_input_schema);

        let list_path = format!("/api/items/{}", name);
        paths.insert(list_path, json!({
            "get": {
                "tags": [name],
                "summary": format!("List {}", name),
                "operationId": format!("list_{}", name),
                "parameters": item_query_parameters(),
                "responses": {
                    "200": {
                        "description": format!("List of {} items", name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": {
                                            "type": "array",
                                            "items": { "$ref": format!("#/components/schemas/{}", schema_ref) }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    "401": { "$ref": "#/components/responses/UnauthorizedError" }
                },
                "security": [{ "BearerAuth": [] }]
            },
            "post": {
                "tags": [name],
                "summary": format!("Create {}", name),
                "operationId": format!("create_{}", name),
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": { "$ref": format!("#/components/schemas/{}", input_ref) }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": format!("Created {} item", name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": { "$ref": format!("#/components/schemas/{}", schema_ref) }
                                    }
                                }
                            }
                        }
                    },
                    "401": { "$ref": "#/components/responses/UnauthorizedError" }
                },
                "security": [{ "BearerAuth": [] }]
            }
        }));

        let item_path = format!("/api/items/{}/{{id}}", name);
        paths.insert(item_path, json!({
            "get": {
                "tags": [name],
                "summary": format!("Get {} by ID", name),
                "operationId": format!("get_{}_by_id", name),
                "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
                "responses": {
                    "200": {
                        "description": format!("{} item", name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": { "$ref": format!("#/components/schemas/{}", schema_ref) }
                                    }
                                }
                            }
                        }
                    },
                    "401": { "$ref": "#/components/responses/UnauthorizedError" },
                    "404": { "$ref": "#/components/responses/NotFoundError" }
                },
                "security": [{ "BearerAuth": [] }]
            },
            "patch": {
                "tags": [name],
                "summary": format!("Update {}", name),
                "operationId": format!("update_{}", name),
                "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
                "requestBody": {
                    "content": {
                        "application/json": {
                            "schema": { "$ref": format!("#/components/schemas/{}", input_ref) }
                        }
                    }
                },
                "responses": {
                    "200": {
                        "description": format!("Updated {} item", name),
                        "content": {
                            "application/json": {
                                "schema": {
                                    "type": "object",
                                    "properties": {
                                        "data": { "$ref": format!("#/components/schemas/{}", schema_ref) }
                                    }
                                }
                            }
                        }
                    },
                    "401": { "$ref": "#/components/responses/UnauthorizedError" },
                    "404": { "$ref": "#/components/responses/NotFoundError" }
                },
                "security": [{ "BearerAuth": [] }]
            },
            "delete": {
                "tags": [name],
                "summary": format!("Delete {}", name),
                "operationId": format!("delete_{}", name),
                "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
                "responses": {
                    "204": { "description": "Deleted" },
                    "401": { "$ref": "#/components/responses/UnauthorizedError" },
                    "404": { "$ref": "#/components/responses/NotFoundError" }
                },
                "security": [{ "BearerAuth": [] }]
            }
        }));
    }

    insert_system_schemas(&mut schemas);

    json!({
        "openapi": "3.1.0",
        "info": {
            "title": "Nexus API",
            "description": "Nexus CMS REST API — Directus-compatible. Auto-generated from the current database schema.",
            "version": env!("CARGO_PKG_VERSION"),
            "contact": { "name": "Nexus CMS" }
        },
        "servers": [{ "url": "/", "description": "Current server" }],
        "tags": build_tags(schema),
        "paths": Value::Object(paths),
        "components": {
            "schemas": Value::Object(schemas),
            "securitySchemes": {
                "BearerAuth": { "type": "http", "scheme": "bearer", "bearerFormat": "JWT" },
                "QueryAuth": { "type": "apiKey", "in": "query", "name": "access_token" }
            },
            "responses": {
                "UnauthorizedError": {
                    "description": "Authentication required",
                    "content": { "application/json": { "schema": { "type": "object", "properties": { "errors": { "type": "array", "items": { "type": "object", "properties": { "message": { "type": "string" }, "extensions": { "type": "object", "properties": { "code": { "type": "string" } } } } } } } } } }
                },
                "NotFoundError": {
                    "description": "Item not found",
                    "content": { "application/json": { "schema": { "type": "object", "properties": { "errors": { "type": "array", "items": { "type": "object", "properties": { "message": { "type": "string" }, "extensions": { "type": "object", "properties": { "code": { "type": "string" } } } } } } } } } }
                }
            }
        }
    })
}

fn build_tags(schema: &SchemaOverview) -> Value {
    let mut tags: Vec<Value> = vec![
        json!({ "name": "Server", "description": "Server information" }),
        json!({ "name": "Authentication", "description": "Login, logout, token refresh" }),
        json!({ "name": "Users", "description": "User management" }),
        json!({ "name": "Roles", "description": "Role management" }),
        json!({ "name": "Permissions", "description": "Permission management" }),
        json!({ "name": "Collections", "description": "Collection management" }),
        json!({ "name": "Fields", "description": "Field management" }),
        json!({ "name": "Relations", "description": "Relation management" }),
        json!({ "name": "Files", "description": "File management" }),
        json!({ "name": "Assets", "description": "Asset delivery" }),
        json!({ "name": "Activity", "description": "Activity log" }),
        json!({ "name": "Settings", "description": "Project settings" }),
        json!({ "name": "Notifications", "description": "User notifications" }),
    ];
    for name in schema.collections.keys() {
        if !name.starts_with("nexus_") {
            tags.push(json!({ "name": name, "description": format!("Items in '{}'", name) }));
        }
    }
    Value::Array(tags)
}

fn field_type_to_openapi(ft: &FieldType) -> Value {
    match ft {
        FieldType::Integer | FieldType::BigInteger => json!({ "type": "integer" }),
        FieldType::Float | FieldType::Decimal => json!({ "type": "number" }),
        FieldType::Boolean => json!({ "type": "boolean" }),
        FieldType::Json => json!({ "type": "object" }),
        FieldType::Uuid => json!({ "type": "string", "format": "uuid" }),
        FieldType::Date => json!({ "type": "string", "format": "date" }),
        FieldType::DateTime | FieldType::Timestamp => json!({ "type": "string", "format": "date-time" }),
        FieldType::Time => json!({ "type": "string", "format": "time" }),
        FieldType::Binary => json!({ "type": "string", "format": "binary" }),
        FieldType::Geometry | FieldType::GeometryPoint | FieldType::GeometryLineString
        | FieldType::GeometryPolygon | FieldType::GeometryMultiPoint
        | FieldType::GeometryMultiLineString | FieldType::GeometryMultiPolygon => {
            json!({ "type": "object", "description": "GeoJSON geometry" })
        }
        _ => json!({ "type": "string" }),
    }
}

fn build_collection_schemas(
    col: &nexus_types::schema::CollectionOverview,
) -> (Value, Value) {
    let mut properties = Map::new();
    let mut input_properties = Map::new();
    let mut required = Vec::new();

    for (fname, field) in &col.fields {
        if field.alias { continue; }
        let prop = field_type_to_openapi(&field.field_type);
        properties.insert(fname.clone(), prop.clone());
        if fname != &col.primary {
            input_properties.insert(fname.clone(), prop);
            if !field.nullable && field.default_value.is_none() {
                required.push(Value::String(fname.clone()));
            }
        }
    }

    let schema = json!({ "type": "object", "properties": Value::Object(properties) });
    let mut input = json!({ "type": "object", "properties": Value::Object(input_properties) });
    if !required.is_empty() {
        input.as_object_mut().unwrap().insert("required".to_string(), Value::Array(required));
    }
    (schema, input)
}

fn item_query_parameters() -> Value {
    json!([
        { "name": "fields", "in": "query", "description": "Comma-separated fields to return", "schema": { "type": "string" } },
        { "name": "filter", "in": "query", "description": "Filter rules (JSON)", "schema": { "type": "string" } },
        { "name": "sort", "in": "query", "description": "Sort field(s), prefix with - for descending", "schema": { "type": "string" } },
        { "name": "limit", "in": "query", "description": "Max items to return", "schema": { "type": "integer", "default": 100 } },
        { "name": "offset", "in": "query", "description": "Pagination offset", "schema": { "type": "integer", "default": 0 } },
        { "name": "search", "in": "query", "description": "Full-text search", "schema": { "type": "string" } },
        { "name": "deep", "in": "query", "description": "Deep filter for relational fields (JSON)", "schema": { "type": "string" } },
        { "name": "aggregate", "in": "query", "description": "Aggregate functions (JSON)", "schema": { "type": "string" } }
    ])
}

fn insert_system_paths(paths: &mut Map<String, Value>) {
    paths.insert("/api/server/ping".into(), json!({
        "get": { "tags": ["Server"], "summary": "Ping", "operationId": "serverPing", "responses": { "200": { "description": "pong" } } }
    }));
    paths.insert("/api/server/info".into(), json!({
        "get": { "tags": ["Server"], "summary": "Server Info", "operationId": "serverInfo",
            "responses": { "200": { "description": "Server information", "content": { "application/json": { "schema": { "type": "object", "properties": { "data": { "$ref": "#/components/schemas/ServerInfo" } } } } } } } }
    }));
    paths.insert("/api/auth/login".into(), json!({
        "post": { "tags": ["Authentication"], "summary": "Login", "operationId": "login",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "required": ["email", "password"],
                "properties": { "email": { "type": "string", "format": "email" }, "password": { "type": "string", "format": "password" },
                    "mode": { "type": "string", "enum": ["json", "cookie", "session"], "default": "json" }, "otp": { "type": "string" } } } } } },
            "responses": { "200": { "description": "Auth tokens", "content": { "application/json": { "schema": { "type": "object",
                "properties": { "data": { "type": "object", "properties": { "access_token": { "type": "string" }, "expires": { "type": "integer" }, "refresh_token": { "type": "string" } } } } } } } } } }
    }));
    paths.insert("/api/auth/refresh".into(), json!({
        "post": { "tags": ["Authentication"], "summary": "Refresh token", "operationId": "refresh",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "required": ["refresh_token"],
                "properties": { "refresh_token": { "type": "string" }, "mode": { "type": "string" } } } } } },
            "responses": { "200": { "description": "New tokens" } } }
    }));
    paths.insert("/api/auth/logout".into(), json!({
        "post": { "tags": ["Authentication"], "summary": "Logout", "operationId": "logout",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "properties": { "refresh_token": { "type": "string" } } } } } },
            "responses": { "204": { "description": "Logged out" } }, "security": [{ "BearerAuth": [] }] }
    }));
    paths.insert("/api/auth/password/request".into(), json!({
        "post": { "tags": ["Authentication"], "summary": "Request password reset", "operationId": "passwordRequest",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "required": ["email"], "properties": { "email": { "type": "string", "format": "email" } } } } } },
            "responses": { "204": { "description": "OK" } } }
    }));
    paths.insert("/api/auth/password/reset".into(), json!({
        "post": { "tags": ["Authentication"], "summary": "Reset password", "operationId": "passwordReset",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "required": ["token", "password"], "properties": { "token": { "type": "string" }, "password": { "type": "string" } } } } } },
            "responses": { "204": { "description": "Password reset" } } }
    }));

    insert_crud_paths(paths, "Users", "users", "User");
    paths.insert("/api/users/me".into(), json!({
        "get": { "tags": ["Users"], "summary": "Get current user", "operationId": "getMe",
            "responses": { "200": { "description": "Current user", "content": { "application/json": { "schema": { "type": "object", "properties": { "data": { "$ref": "#/components/schemas/User" } } } } } } },
            "security": [{ "BearerAuth": [] }] },
        "patch": { "tags": ["Users"], "summary": "Update current user", "operationId": "updateMe",
            "responses": { "200": { "description": "Updated" } }, "security": [{ "BearerAuth": [] }] }
    }));

    insert_crud_paths(paths, "Roles", "roles", "Role");
    insert_crud_paths(paths, "Permissions", "permissions", "Permission");
    insert_crud_paths(paths, "Collections", "collections", "Collection");
    insert_crud_paths(paths, "Fields", "fields", "Field");
    insert_crud_paths(paths, "Relations", "relations", "Relation");
    insert_crud_paths(paths, "Activity", "activity", "Activity");
    insert_crud_paths(paths, "Notifications", "notifications", "Notification");
    insert_crud_paths(paths, "Files", "files", "File");

    paths.insert("/api/files/import".into(), json!({
        "post": { "tags": ["Files"], "summary": "Import file from URL", "operationId": "importFile",
            "requestBody": { "content": { "application/json": { "schema": { "type": "object", "required": ["url"],
                "properties": { "url": { "type": "string", "format": "uri" }, "data": { "type": "object" } } } } } },
            "responses": { "200": { "description": "Imported file" } }, "security": [{ "BearerAuth": [] }] }
    }));

    paths.insert("/api/assets/{id}".into(), json!({
        "get": { "tags": ["Assets"], "summary": "Get asset", "operationId": "getAsset",
            "parameters": [
                { "name": "id", "in": "path", "required": true, "schema": { "type": "string" } },
                { "name": "key", "in": "query", "description": "Transformation preset key", "schema": { "type": "string" } },
                { "name": "width", "in": "query", "schema": { "type": "integer" } },
                { "name": "height", "in": "query", "schema": { "type": "integer" } },
                { "name": "fit", "in": "query", "schema": { "type": "string", "enum": ["cover", "contain", "inside", "outside"] } },
                { "name": "quality", "in": "query", "schema": { "type": "integer", "minimum": 1, "maximum": 100 } },
                { "name": "format", "in": "query", "schema": { "type": "string", "enum": ["jpg", "png", "webp", "avif", "tiff"] } },
                { "name": "download", "in": "query", "schema": { "type": "boolean" } }
            ],
            "responses": { "200": { "description": "Asset binary", "content": { "*/*": { "schema": { "type": "string", "format": "binary" } } } },
                "404": { "$ref": "#/components/responses/NotFoundError" } } }
    }));

    paths.insert("/api/settings".into(), json!({
        "get": { "tags": ["Settings"], "summary": "Get settings", "operationId": "getSettings",
            "responses": { "200": { "description": "Project settings", "content": { "application/json": { "schema": { "type": "object", "properties": { "data": { "$ref": "#/components/schemas/Settings" } } } } } } },
            "security": [{ "BearerAuth": [] }] },
        "patch": { "tags": ["Settings"], "summary": "Update settings", "operationId": "updateSettings",
            "responses": { "200": { "description": "Updated settings" } }, "security": [{ "BearerAuth": [] }] }
    }));
}

fn insert_crud_paths(paths: &mut Map<String, Value>, tag: &str, endpoint: &str, schema_name: &str) {
    let list_path = format!("/api/{}", endpoint);
    let item_path = format!("/api/{}/{{id}}", endpoint);

    paths.insert(list_path, json!({
        "get": { "tags": [tag], "summary": format!("List {}", tag), "operationId": format!("list{}", tag),
            "parameters": item_query_parameters(),
            "responses": { "200": { "description": format!("List of {}", tag.to_lowercase()),
                "content": { "application/json": { "schema": { "type": "object", "properties": { "data": { "type": "array", "items": { "$ref": format!("#/components/schemas/{}", schema_name) } } } } } } },
                "401": { "$ref": "#/components/responses/UnauthorizedError" } },
            "security": [{ "BearerAuth": [] }] },
        "post": { "tags": [tag], "summary": format!("Create {}", schema_name), "operationId": format!("create{}", schema_name),
            "requestBody": { "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{}", schema_name) } } } },
            "responses": { "200": { "description": format!("Created {}", schema_name.to_lowercase()) } },
            "security": [{ "BearerAuth": [] }] }
    }));

    paths.insert(item_path, json!({
        "get": { "tags": [tag], "summary": format!("Get {} by ID", schema_name), "operationId": format!("get{}ById", schema_name),
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "200": { "description": schema_name, "content": { "application/json": { "schema": { "type": "object", "properties": { "data": { "$ref": format!("#/components/schemas/{}", schema_name) } } } } } },
                "404": { "$ref": "#/components/responses/NotFoundError" } },
            "security": [{ "BearerAuth": [] }] },
        "patch": { "tags": [tag], "summary": format!("Update {}", schema_name), "operationId": format!("update{}", schema_name),
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "requestBody": { "content": { "application/json": { "schema": { "$ref": format!("#/components/schemas/{}", schema_name) } } } },
            "responses": { "200": { "description": format!("Updated {}", schema_name.to_lowercase()) } },
            "security": [{ "BearerAuth": [] }] },
        "delete": { "tags": [tag], "summary": format!("Delete {}", schema_name), "operationId": format!("delete{}", schema_name),
            "parameters": [{ "name": "id", "in": "path", "required": true, "schema": { "type": "string" } }],
            "responses": { "204": { "description": "Deleted" } },
            "security": [{ "BearerAuth": [] }] }
    }));
}

fn insert_system_schemas(schemas: &mut Map<String, Value>) {
    schemas.insert("ServerInfo".into(), json!({
        "type": "object", "properties": { "project": { "type": "object", "properties": {
            "project_name": { "type": "string" }, "project_color": { "type": "string" }, "default_language": { "type": "string" }, "public_note": { "type": "string" } } } }
    }));
    schemas.insert("User".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "string", "format": "uuid" }, "first_name": { "type": "string" }, "last_name": { "type": "string" },
            "email": { "type": "string", "format": "email" }, "location": { "type": "string" }, "title": { "type": "string" },
            "description": { "type": "string" }, "tags": { "type": "array", "items": { "type": "string" } },
            "avatar": { "type": "string", "format": "uuid" }, "language": { "type": "string" },
            "status": { "type": "string", "enum": ["draft", "invited", "unverified", "active", "suspended", "archived"] },
            "role": { "type": "string", "format": "uuid" }, "token": { "type": "string" },
            "last_access": { "type": "string", "format": "date-time" }, "provider": { "type": "string" }, "appearance": { "type": "string" } }
    }));
    schemas.insert("Role".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "string", "format": "uuid" }, "name": { "type": "string" }, "icon": { "type": "string" },
            "description": { "type": "string" }, "parent": { "type": "string", "format": "uuid" },
            "enforce_tfa": { "type": "boolean" }, "admin_access": { "type": "boolean" }, "app_access": { "type": "boolean" } }
    }));
    schemas.insert("Permission".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "policy": { "type": "string", "format": "uuid" }, "collection": { "type": "string" },
            "action": { "type": "string", "enum": ["create", "read", "update", "delete", "share"] },
            "permissions": { "type": "object" }, "validation": { "type": "object" }, "presets": { "type": "object" }, "fields": { "type": "string" } }
    }));
    schemas.insert("Collection".into(), json!({
        "type": "object", "properties": {
            "collection": { "type": "string" }, "icon": { "type": "string" }, "note": { "type": "string" },
            "display_template": { "type": "string" }, "hidden": { "type": "boolean" }, "singleton": { "type": "boolean" },
            "sort_field": { "type": "string" }, "accountability": { "type": "string" }, "versioning": { "type": "boolean" } }
    }));
    schemas.insert("Field".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "collection": { "type": "string" }, "field": { "type": "string" },
            "special": { "type": "string" }, "interface": { "type": "string" }, "display": { "type": "string" },
            "readonly": { "type": "boolean" }, "hidden": { "type": "boolean" }, "required": { "type": "boolean" },
            "sort": { "type": "integer" }, "width": { "type": "string" }, "note": { "type": "string" } }
    }));
    schemas.insert("Relation".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "many_collection": { "type": "string" }, "many_field": { "type": "string" },
            "one_collection": { "type": "string" }, "one_field": { "type": "string" }, "junction_field": { "type": "string" } }
    }));
    schemas.insert("Activity".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "action": { "type": "string" }, "user": { "type": "string", "format": "uuid" },
            "timestamp": { "type": "string", "format": "date-time" }, "ip": { "type": "string" }, "user_agent": { "type": "string" },
            "collection": { "type": "string" }, "item": { "type": "string" }, "comment": { "type": "string" } }
    }));
    schemas.insert("File".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "string", "format": "uuid" }, "storage": { "type": "string" }, "filename_disk": { "type": "string" },
            "filename_download": { "type": "string" }, "title": { "type": "string" }, "type": { "type": "string" },
            "folder": { "type": "string", "format": "uuid" }, "uploaded_by": { "type": "string", "format": "uuid" },
            "created_on": { "type": "string", "format": "date-time" }, "filesize": { "type": "integer" },
            "width": { "type": "integer" }, "height": { "type": "integer" }, "description": { "type": "string" },
            "tags": { "type": "array", "items": { "type": "string" } }, "metadata": { "type": "object" } }
    }));
    schemas.insert("Settings".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "project_name": { "type": "string" }, "project_url": { "type": "string" },
            "project_color": { "type": "string" }, "project_logo": { "type": "string", "format": "uuid" },
            "public_note": { "type": "string" }, "auth_login_attempts": { "type": "integer" },
            "storage_asset_transform": { "type": "string" }, "custom_css": { "type": "string" },
            "default_language": { "type": "string" }, "default_appearance": { "type": "string" } }
    }));
    schemas.insert("Notification".into(), json!({
        "type": "object", "properties": {
            "id": { "type": "integer" }, "timestamp": { "type": "string", "format": "date-time" }, "status": { "type": "string" },
            "recipient": { "type": "string", "format": "uuid" }, "sender": { "type": "string", "format": "uuid" },
            "subject": { "type": "string" }, "message": { "type": "string" }, "collection": { "type": "string" }, "item": { "type": "string" } }
    }));
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

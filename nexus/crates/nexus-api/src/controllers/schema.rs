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

async fn apply(state: web::Data<AppState>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let _accountability = req.extensions().get::<Accountability>().cloned();
    let snapshot = body.into_inner();

    let target_schema: nexus_types::schema::SchemaOverview = match serde_json::from_value(snapshot) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({
            "errors": [{ "message": format!("Invalid snapshot: {}", e) }]
        })),
    };

    let current_schema = state.schema.read().await;
    let db = &state.db;
    let mut ddl_statements: Vec<String> = Vec::new();

    // Create new collections
    for (name, target_col) in &target_schema.collections {
        if !current_schema.collections.contains_key(name) {
            let mut col_defs: Vec<String> = Vec::new();
            for (field_name, field) in &target_col.fields {
                let db_type = field.db_type.clone().unwrap_or_else(|| map_field_type_sql(&field.field_type));
                let mut col_def = format!("{} {}", db.quote_identifier(field_name), db_type);
                if !field.nullable { col_def.push_str(" NOT NULL"); }
                if field_name == &target_col.primary { col_def.push_str(" PRIMARY KEY"); }
                col_defs.push(col_def);
            }
            ddl_statements.push(format!("CREATE TABLE {} ({})", db.quote_identifier(name), col_defs.join(", ")));
        }
    }

    // Add new fields
    for (name, target_col) in &target_schema.collections {
        if let Some(current_col) = current_schema.collections.get(name) {
            for (field_name, field) in &target_col.fields {
                if !current_col.fields.contains_key(field_name) {
                    let db_type = field.db_type.clone().unwrap_or_else(|| map_field_type_sql(&field.field_type));
                    let nullable = if field.nullable { "" } else { " NOT NULL" };
                    ddl_statements.push(format!("ALTER TABLE {} ADD COLUMN {} {}{}", db.quote_identifier(name), db.quote_identifier(field_name), db_type, nullable));
                }
            }
        }
    }

    // Drop user collections not in target
    for name in current_schema.collections.keys() {
        if !target_schema.collections.contains_key(name) && !name.starts_with("directus_") {
            ddl_statements.push(format!("DROP TABLE IF EXISTS {}", db.quote_identifier(name)));
        }
    }
    drop(current_schema);

    for stmt in &ddl_statements {
        if let Err(e) = db.execute(stmt, &[]).await {
            return HttpResponse::InternalServerError().json(json!({
                "errors": [{ "message": format!("DDL failed: {}", e) }]
            }));
        }
    }

    // Refresh cached schema
    if let Ok(new_schema) = nexus_database::helpers::schema::SchemaInspector::snapshot(db.as_ref()).await {
        *state.schema.write().await = std::sync::Arc::new(new_schema);
    }

    HttpResponse::NoContent().finish()
}

async fn diff(state: web::Data<AppState>, req: HttpRequest, body: web::Json<serde_json::Value>) -> HttpResponse {
    let _accountability = req.extensions().get::<Accountability>().cloned();
    let snapshot = body.into_inner();

    let target_schema: nexus_types::schema::SchemaOverview = match serde_json::from_value(snapshot) {
        Ok(s) => s,
        Err(e) => return HttpResponse::BadRequest().json(json!({
            "errors": [{ "message": format!("Invalid snapshot: {}", e) }]
        })),
    };

    let current_schema = state.schema.read().await;
    let mut collections_to_create = Vec::new();
    let mut collections_to_delete = Vec::new();
    let mut fields_to_create = Vec::new();
    let mut fields_to_delete = Vec::new();

    for name in target_schema.collections.keys() {
        if !current_schema.collections.contains_key(name) {
            collections_to_create.push(name.clone());
        }
    }
    for name in current_schema.collections.keys() {
        if !target_schema.collections.contains_key(name) && !name.starts_with("directus_") {
            collections_to_delete.push(name.clone());
        }
    }
    for (name, target_col) in &target_schema.collections {
        if let Some(current_col) = current_schema.collections.get(name) {
            for field_name in target_col.fields.keys() {
                if !current_col.fields.contains_key(field_name) {
                    fields_to_create.push(json!({ "collection": name, "field": field_name }));
                }
            }
            for field_name in current_col.fields.keys() {
                if !target_col.fields.contains_key(field_name) {
                    fields_to_delete.push(json!({ "collection": name, "field": field_name }));
                }
            }
        }
    }

    HttpResponse::Ok().json(json!({
        "data": { "diff": {
            "collections": { "create": collections_to_create, "delete": collections_to_delete },
            "fields": { "create": fields_to_create, "delete": fields_to_delete }
        }}
    }))
}

fn map_field_type_sql(ft: &nexus_types::fields::FieldType) -> String {
    use nexus_types::fields::FieldType;
    match ft {
        FieldType::Integer => "integer".to_string(),
        FieldType::BigInteger => "bigint".to_string(),
        FieldType::Float | FieldType::Decimal => "real".to_string(),
        FieldType::Boolean => "boolean".to_string(),
        FieldType::String | FieldType::Hash | FieldType::Csv => "varchar(255)".to_string(),
        FieldType::Text => "text".to_string(),
        FieldType::Date => "date".to_string(),
        FieldType::Time => "time".to_string(),
        FieldType::DateTime | FieldType::Timestamp => "timestamp".to_string(),
        FieldType::Json => "jsonb".to_string(),
        FieldType::Uuid => "uuid".to_string(),
        FieldType::Binary => "bytea".to_string(),
        _ => "text".to_string(),
    }
}

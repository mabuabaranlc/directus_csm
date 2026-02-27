use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing collections (tables)
/// Mirrors api/src/services/collections.ts
pub struct CollectionsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl CollectionsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_collections", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new collection (CREATE TABLE + metadata)
    pub async fn create_one(&self, data: Value) -> Result<Value, ServiceError> {
        let collection_name = data
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                ServiceError::InvalidPayload("Missing 'collection' field".to_string())
            })?;

        let schema = data.get("schema");
        let meta = data.get("meta");

        // Create the actual database table if schema is provided
        if schema.is_some() {
            let fields = data
                .get("fields")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();

            let mut column_defs = Vec::new();

            for field in &fields {
                if let Some(field_schema) = field.get("schema") {
                    let field_name = field
                        .get("field")
                        .and_then(|v| v.as_str())
                        .unwrap_or("id");

                    let data_type = field_schema
                        .get("data_type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("varchar(255)");

                    let mut col_def = format!(
                        "{} {}",
                        self.ctx.db.quote_identifier(field_name),
                        data_type
                    );

                    if field_schema
                        .get("is_primary_key")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        col_def.push_str(" PRIMARY KEY");
                    }

                    if field_schema
                        .get("has_auto_increment")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                    {
                        match self.ctx.db.dialect() {
                            nexus_database::Dialect::Postgres | nexus_database::Dialect::CockroachDB => {
                                col_def = format!(
                                    "{} SERIAL PRIMARY KEY",
                                    self.ctx.db.quote_identifier(field_name)
                                );
                            }
                            nexus_database::Dialect::MySQL => {
                                col_def.push_str(" AUTO_INCREMENT");
                            }
                            nexus_database::Dialect::SQLite => {
                                col_def = format!(
                                    "{} INTEGER PRIMARY KEY AUTOINCREMENT",
                                    self.ctx.db.quote_identifier(field_name)
                                );
                            }
                            _ => {}
                        }
                    }

                    if !field_schema
                        .get("is_nullable")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(true)
                    {
                        col_def.push_str(" NOT NULL");
                    }

                    if let Some(default) = field_schema.get("default_value") {
                        if !default.is_null() {
                            col_def.push_str(&format!(" DEFAULT {}", sql_default_value(default)));
                        }
                    }

                    column_defs.push(col_def);
                }
            }

            if !column_defs.is_empty() {
                let sql = format!(
                    "CREATE TABLE {} ({})",
                    self.ctx.db.quote_identifier(collection_name),
                    column_defs.join(", ")
                );

                self.ctx
                    .db
                    .execute(&sql, &[])
                    .await
                    .map_err(|e| ServiceError::Database(e.to_string()))?;
            }
        }

        // Insert metadata into directus_collections
        if let Some(meta) = meta {
            let mut meta_record = meta.clone();
            if let Some(obj) = meta_record.as_object_mut() {
                obj.insert("collection".to_string(), json!(collection_name));
            }
            self.items.create_one(meta_record, None).await?;
        }

        Ok(data)
    }

    /// Read all collections
    pub async fn read_all(&self) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(Query::default(), None).await
    }

    /// Read a single collection by name
    pub async fn read_one(&self, collection: &str) -> Result<Value, ServiceError> {
        let pk = nexus_types::items::PrimaryKey::String(collection.to_string());
        self.items.read_one(&pk, None, None).await
    }

    /// Update collection metadata
    pub async fn update_one(
        &self,
        collection: &str,
        data: Value,
    ) -> Result<Value, ServiceError> {
        let pk = nexus_types::items::PrimaryKey::String(collection.to_string());
        self.items.update_one(&pk, data.clone(), None).await?;
        Ok(data)
    }

    /// Delete a collection (DROP TABLE + metadata)
    pub async fn delete_one(&self, collection: &str) -> Result<(), ServiceError> {
        // Drop the table
        let sql = format!("DROP TABLE IF EXISTS {}", self.ctx.db.quote_identifier(collection));
        self.ctx
            .db
            .execute(&sql, &[])
            .await
            .map_err(|e| ServiceError::Database(e.to_string()))?;

        // Delete metadata
        let pk = nexus_types::items::PrimaryKey::String(collection.to_string());
        self.items.delete_one(&pk, None).await?;

        Ok(())
    }
}

fn sql_default_value(val: &Value) -> String {
    match val {
        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
        Value::Null => "NULL".to_string(),
        _ => format!("'{}'", val),
    }
}

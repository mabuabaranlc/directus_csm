use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing fields (columns) in collections
/// Mirrors api/src/services/fields.ts
pub struct FieldsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl FieldsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_fields", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new field in a collection
    pub async fn create_field(
        &self,
        collection: &str,
        data: Value,
    ) -> Result<Value, ServiceError> {
        let field_name = data
            .get("field")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::InvalidPayload("Missing 'field' name".to_string()))?;

        // Add the column to the actual table if schema is provided
        if let Some(schema) = data.get("schema") {
            let data_type = schema
                .get("data_type")
                .and_then(|v| v.as_str())
                .unwrap_or("varchar(255)");

            let mut sql = format!(
                "ALTER TABLE {} ADD COLUMN {} {}",
                self.ctx.db.quote_identifier(collection),
                self.ctx.db.quote_identifier(field_name),
                data_type
            );

            if !schema
                .get("is_nullable")
                .and_then(|v| v.as_bool())
                .unwrap_or(true)
            {
                sql.push_str(" NOT NULL");
            }

            if let Some(default) = schema.get("default_value") {
                if !default.is_null() {
                    let default_str = match default {
                        Value::String(s) => format!("'{}'", s.replace('\'', "''")),
                        Value::Number(n) => n.to_string(),
                        Value::Bool(b) => if *b { "TRUE" } else { "FALSE" }.to_string(),
                        _ => "NULL".to_string(),
                    };
                    sql.push_str(&format!(" DEFAULT {}", default_str));
                }
            }

            self.ctx
                .db
                .execute(&sql, &[])
                .await
                .map_err(|e| ServiceError::Database(e.to_string()))?;
        }

        // Insert field metadata
        if data.get("meta").is_some() || data.get("type").is_some() {
            let mut meta = data.get("meta").cloned().unwrap_or(json!({}));
            if let Some(obj) = meta.as_object_mut() {
                obj.insert("collection".to_string(), json!(collection));
                obj.insert("field".to_string(), json!(field_name));
            }
            self.items.create_one(meta, None).await?;
        }

        Ok(data)
    }

    /// Read all fields across all collections
    pub async fn read_all(&self) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(Query::default(), None).await
    }

    /// Read fields for a specific collection
    pub async fn read_for_collection(
        &self,
        collection: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let query = Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("collection".to_string(), json!({ "_eq": collection }))]
                    .into_iter()
                    .collect(),
            )),
            ..Default::default()
        };

        self.items.read_by_query(query, None).await
    }

    /// Read a single field
    pub async fn read_one(
        &self,
        collection: &str,
        field: &str,
    ) -> Result<Value, ServiceError> {
        let query = Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [
                    ("collection".to_string(), json!({ "_eq": collection })),
                    ("field".to_string(), json!({ "_eq": field })),
                ]
                .into_iter()
                .collect(),
            )),
            limit: Some(1),
            ..Default::default()
        };

        let items = self.items.read_by_query(query, None).await?;
        items
            .into_iter()
            .next()
            .ok_or_else(|| ServiceError::NotFound(format!("Field {}.{} not found", collection, field)))
    }

    /// Update a field's metadata and/or schema
    pub async fn update_field(
        &self,
        collection: &str,
        field: &str,
        data: Value,
    ) -> Result<Value, ServiceError> {
        // Update the column type if schema changes are requested
        if let Some(schema) = data.get("schema") {
            if let Some(new_type) = schema.get("data_type").and_then(|v| v.as_str()) {
                let sql = format!(
                    "ALTER TABLE {} ALTER COLUMN {} TYPE {}",
                    self.ctx.db.quote_identifier(collection),
                    self.ctx.db.quote_identifier(field),
                    new_type
                );
                let _ = self.ctx.db.execute(&sql, &[]).await;
            }
        }

        // Update metadata
        if let Some(meta) = data.get("meta") {
            let existing = self.read_one(collection, field).await;
            if let Ok(existing) = existing {
                if let Some(id) = existing.get("id").and_then(|v| v.as_i64()) {
                    let pk = PrimaryKey::Integer(id);
                    self.items.update_one(&pk, meta.clone(), None).await?;
                }
            }
        }

        Ok(data)
    }

    /// Delete a field from a collection
    pub async fn delete_field(
        &self,
        collection: &str,
        field: &str,
    ) -> Result<(), ServiceError> {
        // Drop the column
        let sql = format!(
            "ALTER TABLE {} DROP COLUMN {}",
            self.ctx.db.quote_identifier(collection),
            self.ctx.db.quote_identifier(field)
        );
        let _ = self.ctx.db.execute(&sql, &[]).await;

        // Delete metadata
        let existing = self.read_one(collection, field).await;
        if let Ok(existing) = existing {
            if let Some(id) = existing.get("id").and_then(|v| v.as_i64()) {
                let pk = PrimaryKey::Integer(id);
                self.items.delete_one(&pk, None).await?;
            }
        }

        Ok(())
    }
}

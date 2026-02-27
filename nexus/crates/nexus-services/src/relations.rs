use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use nexus_types::relations::Relation;
use serde_json::{json, Value};

/// Service for managing relations between collections
/// Mirrors api/src/services/relations.ts
pub struct RelationsService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl RelationsService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_relations", ctx.clone());
        Self { ctx, items }
    }

    /// Create a new relation
    pub async fn create_one(&self, data: Value) -> Result<Value, ServiceError> {
        let many_collection = data
            .get("collection")
            .or_else(|| data.get("many_collection"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::InvalidPayload("Missing collection".to_string()))?;

        let many_field = data
            .get("field")
            .or_else(|| data.get("many_field"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::InvalidPayload("Missing field".to_string()))?;

        // Create the foreign key constraint if schema info is provided
        if let Some(related_collection) = data
            .get("related_collection")
            .or_else(|| data.get("schema").and_then(|s| s.get("foreign_key_table")))
            .and_then(|v| v.as_str())
        {
            // Get the primary key of the related collection
            let related_pk = self
                .ctx
                .schema
                .collections
                .get(related_collection)
                .map(|c| c.primary.as_str())
                .unwrap_or("id");

            let constraint_name = format!("{}_{}_foreign", many_collection, many_field);

            let sql = format!(
                "ALTER TABLE {} ADD CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {} ({})",
                self.ctx.db.quote_identifier(many_collection),
                self.ctx.db.quote_identifier(&constraint_name),
                self.ctx.db.quote_identifier(many_field),
                self.ctx.db.quote_identifier(related_collection),
                self.ctx.db.quote_identifier(related_pk)
            );

            // Don't fail on constraint creation errors (table might not exist yet)
            let _ = self.ctx.db.execute(&sql, &[]).await;
        }

        // Insert relation metadata
        let mut meta = data.get("meta").cloned().unwrap_or(json!({}));
        if let Some(obj) = meta.as_object_mut() {
            obj.insert("many_collection".to_string(), json!(many_collection));
            obj.insert("many_field".to_string(), json!(many_field));
            if let Some(one_collection) = data.get("related_collection") {
                obj.insert("one_collection".to_string(), one_collection.clone());
            }
        }

        self.items.create_one(meta, None).await?;

        Ok(data)
    }

    /// Read all relations
    pub async fn read_all(&self) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(Query::default(), None).await
    }

    /// Read relations for a specific collection
    pub async fn read_for_collection(
        &self,
        collection: &str,
    ) -> Result<Vec<Value>, ServiceError> {
        let query = Query {
            filter: Some(nexus_types::filter::Filter::Field(
                [("many_collection".to_string(), json!({ "_eq": collection }))]
                    .into_iter()
                    .collect(),
            )),
            ..Default::default()
        };

        self.items.read_by_query(query, None).await
    }

    /// Update a relation
    pub async fn update_one(&self, id: i64, data: Value) -> Result<Value, ServiceError> {
        let pk = PrimaryKey::Integer(id);
        self.items.update_one(&pk, data.clone(), None).await?;
        Ok(data)
    }

    /// Delete a relation
    pub async fn delete_one(&self, id: i64) -> Result<(), ServiceError> {
        let pk = PrimaryKey::Integer(id);
        self.items.delete_one(&pk, None).await?;
        Ok(())
    }
}

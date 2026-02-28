use crate::{DataBackend, Item, NoSqlError, PrimaryKey};
use async_trait::async_trait;
use mongodb::{bson, Client, Database};
use nexus_types::filter::{Filter, LogicalFilter};
use nexus_types::query::Query;
use serde_json::Value;

pub struct MongoBackend {
    db: Database,
}

impl MongoBackend {
    pub async fn new(uri: &str, database: &str) -> Result<Self, NoSqlError> {
        let client = Client::with_uri_str(uri)
            .await
            .map_err(|e| NoSqlError::Connection(e.to_string()))?;

        let db = client.database(database);
        Ok(Self { db })
    }
}

#[async_trait]
impl DataBackend for MongoBackend {
    async fn create(
        &self,
        collection: &str,
        data: Vec<Item>,
    ) -> Result<Vec<PrimaryKey>, NoSqlError> {
        let coll = self.db.collection::<bson::Document>(collection);

        let docs: Vec<bson::Document> = data
            .iter()
            .map(|item| {
                bson::to_document(item).map_err(|e| NoSqlError::Query(e.to_string()))
            })
            .collect::<Result<Vec<_>, _>>()?;

        let result = coll
            .insert_many(docs)
            .await
            .map_err(|e| NoSqlError::Query(e.to_string()))?;

        let keys: Vec<PrimaryKey> = result
            .inserted_ids
            .values()
            .map(|id| serde_json::to_value(id.to_string()).unwrap_or_default())
            .collect();

        Ok(keys)
    }

    async fn read(
        &self,
        collection: &str,
        query: &Query,
    ) -> Result<Vec<Item>, NoSqlError> {
        let coll = self.db.collection::<bson::Document>(collection);

        // Convert Nexus filter to MongoDB filter document
        let filter = if let Some(ref nexus_filter) = query.filter {
            filter_to_bson(nexus_filter)?
        } else {
            bson::doc! {}
        };

        let mut find_options = mongodb::options::FindOptions::default();

        if let Some(limit) = query.limit {
            find_options.limit = Some(limit);
        }

        if let Some(offset) = query.offset {
            find_options.skip = Some(offset as u64);
        }

        if let Some(ref sort) = query.sort {
            let mut sort_doc = bson::Document::new();
            for field in sort {
                if let Some(stripped) = field.strip_prefix('-') {
                    sort_doc.insert(stripped, -1);
                } else {
                    sort_doc.insert(field.as_str(), 1);
                }
            }
            find_options.sort = Some(sort_doc);
        }

        let mut cursor = coll
            .find(filter)
            .with_options(find_options)
            .await
            .map_err(|e| NoSqlError::Query(e.to_string()))?;

        let mut items = Vec::new();
        use futures_util::StreamExt;
        while let Some(doc) = cursor.next().await {
            let doc = doc.map_err(|e| NoSqlError::Query(e.to_string()))?;
            let item: Item =
                serde_json::to_value(&doc).map_err(|e| NoSqlError::Query(e.to_string()))?;
            items.push(item);
        }

        Ok(items)
    }

    async fn update(
        &self,
        collection: &str,
        keys: &[PrimaryKey],
        data: Item,
    ) -> Result<Vec<PrimaryKey>, NoSqlError> {
        let coll = self.db.collection::<bson::Document>(collection);
        let update_doc =
            bson::to_document(&data).map_err(|e| NoSqlError::Query(e.to_string()))?;

        for key in keys {
            let id_str = key.as_str().unwrap_or_default();
            let filter = if let Ok(oid) = bson::oid::ObjectId::parse_str(id_str) {
                bson::doc! { "_id": oid }
            } else {
                bson::doc! { "_id": id_str }
            };

            coll.update_one(filter, bson::doc! { "$set": update_doc.clone() })
                .await
                .map_err(|e| NoSqlError::Query(e.to_string()))?;
        }

        Ok(keys.to_vec())
    }

    async fn delete(
        &self,
        collection: &str,
        keys: &[PrimaryKey],
    ) -> Result<Vec<PrimaryKey>, NoSqlError> {
        let coll = self.db.collection::<bson::Document>(collection);

        for key in keys {
            let id_str = key.as_str().unwrap_or_default();
            let filter = if let Ok(oid) = bson::oid::ObjectId::parse_str(id_str) {
                bson::doc! { "_id": oid }
            } else {
                bson::doc! { "_id": id_str }
            };

            coll.delete_one(filter)
                .await
                .map_err(|e| NoSqlError::Query(e.to_string()))?;
        }

        Ok(keys.to_vec())
    }

    async fn introspect_schema(
        &self,
    ) -> Result<nexus_types::schema::SchemaOverview, NoSqlError> {
        // MongoDB schema introspection by sampling documents
        let collection_names = self
            .db
            .list_collection_names()
            .await
            .map_err(|e| NoSqlError::Schema(e.to_string()))?;

        let mut collections = std::collections::HashMap::new();

        for name in &collection_names {
            collections.insert(
                name.clone(),
                nexus_types::schema::CollectionOverview {
                    collection: name.clone(),
                    primary: "_id".to_string(),
                    fields: std::collections::HashMap::new(),
                    is_singleton: false,
                    accountability: None,
                },
            );
        }

        Ok(nexus_types::schema::SchemaOverview {
            collections,
            relations: Vec::new(),
        })
    }

    fn supports_transactions(&self) -> bool {
        true // MongoDB 4.0+ supports transactions
    }

    fn supports_relations(&self) -> bool {
        false // MongoDB doesn't have native relations
    }
}

/// Convert a Nexus Filter to a MongoDB BSON filter document
fn filter_to_bson(filter: &Filter) -> Result<bson::Document, NoSqlError> {
    match filter {
        Filter::Logical(logical) => match logical {
            LogicalFilter::And { _and } => {
                let conditions: Vec<bson::Bson> = _and
                    .iter()
                    .map(|f| filter_to_bson(f).map(bson::Bson::Document))
                    .collect::<Result<Vec<_>, _>>()?;
                if conditions.is_empty() {
                    Ok(bson::doc! {})
                } else {
                    Ok(bson::doc! { "$and": conditions })
                }
            }
            LogicalFilter::Or { _or } => {
                let conditions: Vec<bson::Bson> = _or
                    .iter()
                    .map(|f| filter_to_bson(f).map(bson::Bson::Document))
                    .collect::<Result<Vec<_>, _>>()?;
                if conditions.is_empty() {
                    Ok(bson::doc! {})
                } else {
                    Ok(bson::doc! { "$or": conditions })
                }
            }
        },
        Filter::Field(field_map) => {
            let mut doc = bson::Document::new();
            for (field, operators) in field_map {
                if let Some(ops) = operators.as_object() {
                    let field_doc = ops_to_bson(ops)?;
                    doc.insert(field.clone(), field_doc);
                }
            }
            Ok(doc)
        }
    }
}

/// Convert operator map { "_eq": val, "_gt": val } to BSON for a single field
fn ops_to_bson(
    ops: &serde_json::Map<String, Value>,
) -> Result<bson::Bson, NoSqlError> {
    // If there's only _eq, use direct value match
    if ops.len() == 1 {
        if let Some(val) = ops.get("_eq") {
            return Ok(json_to_bson(val));
        }
    }

    let mut doc = bson::Document::new();

    for (op, val) in ops {
        match op.as_str() {
            "_eq" => {
                if val.is_null() {
                    doc.insert("$eq", bson::Bson::Null);
                } else {
                    // For single _eq, return the value directly
                    return Ok(json_to_bson(val));
                }
            }
            "_neq" => {
                doc.insert("$ne", json_to_bson(val));
            }
            "_lt" => {
                doc.insert("$lt", json_to_bson(val));
            }
            "_lte" => {
                doc.insert("$lte", json_to_bson(val));
            }
            "_gt" => {
                doc.insert("$gt", json_to_bson(val));
            }
            "_gte" => {
                doc.insert("$gte", json_to_bson(val));
            }
            "_in" => {
                if let Some(arr) = val.as_array() {
                    let bson_arr: Vec<bson::Bson> = arr.iter().map(json_to_bson).collect();
                    doc.insert("$in", bson_arr);
                }
            }
            "_nin" => {
                if let Some(arr) = val.as_array() {
                    let bson_arr: Vec<bson::Bson> = arr.iter().map(json_to_bson).collect();
                    doc.insert("$nin", bson_arr);
                }
            }
            "_null" => {
                if val.as_bool().unwrap_or(false) {
                    doc.insert("$eq", bson::Bson::Null);
                } else {
                    doc.insert("$ne", bson::Bson::Null);
                }
            }
            "_nnull" => {
                if val.as_bool().unwrap_or(false) {
                    doc.insert("$ne", bson::Bson::Null);
                } else {
                    doc.insert("$eq", bson::Bson::Null);
                }
            }
            "_contains" | "_icontains" => {
                let s = val.as_str().unwrap_or_default();
                let options = if op == "_icontains" { "i" } else { "" };
                doc.insert(
                    "$regex",
                    bson::Bson::String(regex::escape(s)),
                );
                if !options.is_empty() {
                    doc.insert("$options", bson::Bson::String(options.to_string()));
                }
            }
            "_ncontains" => {
                let s = val.as_str().unwrap_or_default();
                doc.insert(
                    "$not",
                    bson::doc! { "$regex": regex::escape(s) },
                );
            }
            "_starts_with" | "_istarts_with" => {
                let s = val.as_str().unwrap_or_default();
                let options = if op == "_istarts_with" { "i" } else { "" };
                doc.insert(
                    "$regex",
                    bson::Bson::String(format!("^{}", regex::escape(s))),
                );
                if !options.is_empty() {
                    doc.insert("$options", bson::Bson::String(options.to_string()));
                }
            }
            "_ends_with" | "_iends_with" => {
                let s = val.as_str().unwrap_or_default();
                let options = if op == "_iends_with" { "i" } else { "" };
                doc.insert(
                    "$regex",
                    bson::Bson::String(format!("{}$", regex::escape(s))),
                );
                if !options.is_empty() {
                    doc.insert("$options", bson::Bson::String(options.to_string()));
                }
            }
            "_between" => {
                if let Some(arr) = val.as_array() {
                    if arr.len() == 2 {
                        doc.insert("$gte", json_to_bson(&arr[0]));
                        doc.insert("$lte", json_to_bson(&arr[1]));
                    }
                }
            }
            "_nbetween" => {
                if let Some(arr) = val.as_array() {
                    if arr.len() == 2 {
                        // NOT BETWEEN a AND b → $lt a OR $gt b
                        return Ok(bson::Bson::Document(bson::doc! {
                            "$not": {
                                "$gte": json_to_bson(&arr[0]),
                                "$lte": json_to_bson(&arr[1]),
                            }
                        }));
                    }
                }
            }
            "_empty" => {
                if val.as_bool().unwrap_or(false) {
                    return Ok(bson::Bson::Document(bson::doc! {
                        "$in": [bson::Bson::Null, bson::Bson::String("".to_string())]
                    }));
                } else {
                    doc.insert("$nin", vec![bson::Bson::Null, bson::Bson::String("".to_string())]);
                }
            }
            "_nempty" => {
                if val.as_bool().unwrap_or(false) {
                    doc.insert("$nin", vec![bson::Bson::Null, bson::Bson::String("".to_string())]);
                } else {
                    return Ok(bson::Bson::Document(bson::doc! {
                        "$in": [bson::Bson::Null, bson::Bson::String("".to_string())]
                    }));
                }
            }
            "_regex" => {
                let pattern = val.as_str().unwrap_or_default();
                doc.insert("$regex", bson::Bson::String(pattern.to_string()));
            }
            _ => {
                // Unknown operator, skip
            }
        }
    }

    Ok(bson::Bson::Document(doc))
}

/// Convert a serde_json::Value to a bson::Bson value
fn json_to_bson(val: &Value) -> bson::Bson {
    match val {
        Value::Null => bson::Bson::Null,
        Value::Bool(b) => bson::Bson::Boolean(*b),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                bson::Bson::Int64(i)
            } else if let Some(f) = n.as_f64() {
                bson::Bson::Double(f)
            } else {
                bson::Bson::String(n.to_string())
            }
        }
        Value::String(s) => {
            // Try to parse as ObjectId if it looks like one
            if s.len() == 24 {
                if let Ok(oid) = bson::oid::ObjectId::parse_str(s) {
                    return bson::Bson::ObjectId(oid);
                }
            }
            bson::Bson::String(s.clone())
        }
        Value::Array(arr) => {
            bson::Bson::Array(arr.iter().map(json_to_bson).collect())
        }
        Value::Object(map) => {
            let mut doc = bson::Document::new();
            for (k, v) in map {
                doc.insert(k.clone(), json_to_bson(v));
            }
            bson::Bson::Document(doc)
        }
    }
}

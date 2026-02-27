use crate::{DataBackend, Item, NoSqlError, PrimaryKey};
use async_trait::async_trait;
use mongodb::{bson, Client, Database};
use nexus_types::query::Query;

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
        let filter = if let Some(ref _filter) = query.filter {
            // TODO: Implement full filter translation
            bson::doc! {}
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

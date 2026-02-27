use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing file metadata and uploads
/// Mirrors api/src/services/files.ts
pub struct FilesService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl FilesService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_files", ctx.clone());
        Self { ctx, items }
    }

    /// Upload and create file metadata
    pub async fn upload(
        &self,
        data: Value,
        stream: Option<Vec<u8>>,
    ) -> Result<PrimaryKey, ServiceError> {
        let mut file_data = data;

        // Generate a UUID for the file if not provided
        if file_data.get("id").is_none() {
            let id = uuid::Uuid::new_v4().to_string();
            if let Some(obj) = file_data.as_object_mut() {
                obj.insert("id".to_string(), json!(id));
            }
        }

        // Set uploaded_on timestamp
        if let Some(obj) = file_data.as_object_mut() {
            obj.insert(
                "uploaded_on".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            // Set uploaded_by from accountability
            if let Some(user) = self.ctx.user_id() {
                obj.insert("uploaded_by".to_string(), json!(user));
            }
        }

        // If we have file content, store it
        if let Some(_bytes) = stream {
            // TODO: Store file via StorageDriver
            // The storage location depends on env config STORAGE_LOCATIONS
        }

        self.items.create_one(file_data, None).await
    }

    /// Read file metadata by query
    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    /// Read a single file's metadata
    pub async fn read_one(&self, pk: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(pk, None, None).await
    }

    /// Update file metadata
    pub async fn update_one(
        &self,
        pk: &PrimaryKey,
        data: Value,
    ) -> Result<PrimaryKey, ServiceError> {
        self.items.update_one(pk, data, None).await
    }

    /// Delete a file
    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        // TODO: Also delete from storage
        self.items.delete_one(pk, None).await
    }

    /// Import a file from a URL
    pub async fn import_one(&self, url: &str, data: Value) -> Result<PrimaryKey, ServiceError> {
        let mut file_data = data;

        if let Some(obj) = file_data.as_object_mut() {
            obj.insert("id".to_string(), json!(uuid::Uuid::new_v4().to_string()));
            obj.insert(
                "uploaded_on".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            if let Some(user) = self.ctx.user_id() {
                obj.insert("uploaded_by".to_string(), json!(user));
            }
        }

        // TODO: Download file from URL and store via StorageDriver

        self.items.create_one(file_data, None).await
    }
}

use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_storage::StorageDriver;
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};
use std::pin::Pin;
use std::sync::Arc;
use tokio::io::AsyncReadExt as _;
use tracing::warn;

/// Service for managing file metadata and uploads
/// Mirrors api/src/services/files.ts
pub struct FilesService {
    pub ctx: ServiceContext,
    items: ItemsService,
    storage: Option<Arc<dyn StorageDriver>>,
}

impl FilesService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_files", ctx.clone());
        Self {
            ctx,
            items,
            storage: None,
        }
    }

    /// Set the storage driver for file operations
    pub fn with_storage(mut self, driver: Arc<dyn StorageDriver>) -> Self {
        self.storage = Some(driver);
        self
    }

    /// Upload and create file metadata
    pub async fn upload(
        &self,
        data: Value,
        stream: Option<Vec<u8>>,
    ) -> Result<PrimaryKey, ServiceError> {
        let mut file_data = data;

        // Generate a UUID for the file if not provided
        let file_id = if let Some(id) = file_data.get("id").and_then(|v| v.as_str()) {
            id.to_string()
        } else {
            let id = uuid::Uuid::new_v4().to_string();
            if let Some(obj) = file_data.as_object_mut() {
                obj.insert("id".to_string(), json!(&id));
            }
            id
        };

        // Set uploaded_on timestamp
        if let Some(obj) = file_data.as_object_mut() {
            obj.insert(
                "uploaded_on".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            if let Some(user) = self.ctx.user_id() {
                obj.insert("uploaded_by".to_string(), json!(user));
            }
        }

        // If we have file content, store it via the storage driver
        if let Some(bytes) = stream {
            if let Some(ref storage) = self.storage {
                let filename = file_data
                    .get("filename_disk")
                    .and_then(|v| v.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| format!("{}", file_id));

                let content_type = file_data
                    .get("type")
                    .and_then(|v| v.as_str())
                    .unwrap_or("application/octet-stream")
                    .to_string();

                // Store the file size
                if let Some(obj) = file_data.as_object_mut() {
                    obj.insert("filesize".to_string(), json!(bytes.len()));
                    if obj.get("filename_disk").is_none() {
                        obj.insert("filename_disk".to_string(), json!(&filename));
                    }
                }

                let cursor_reader: Pin<Box<dyn tokio::io::AsyncRead + Send>> =
                    Box::pin(std::io::Cursor::new(bytes));

                if let Err(e) = storage
                    .write(
                        &filename,
                        cursor_reader,
                        Some(&content_type),
                    )
                    .await
                {
                    warn!(error = %e, "Failed to store file, metadata will still be created");
                }
            }
        }

        self.items.create_one(file_data, None).await
    }

    /// Read file content by primary key
    pub async fn read_file_content(
        &self,
        pk: &PrimaryKey,
    ) -> Result<(Vec<u8>, String), ServiceError> {
        let file_meta = self.items.read_one(pk, None, None).await?;

        let filename = file_meta
            .get("filename_disk")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ServiceError::NotFound("File has no disk filename".to_string()))?;

        let content_type = file_meta
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("application/octet-stream")
            .to_string();

        let storage = self.storage.as_ref().ok_or_else(|| {
            ServiceError::Internal("No storage driver configured".to_string())
        })?;

        let mut reader = storage.read(filename, None).await.map_err(|e| {
            ServiceError::Internal(format!("Storage read failed: {}", e))
        })?;

        let mut bytes = Vec::new();
        reader.read_to_end(&mut bytes).await.map_err(|e| {
            ServiceError::Internal(format!("Failed to read file content: {}", e))
        })?;

        Ok((bytes, content_type))
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

    /// Delete a file (metadata + storage)
    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        // Read the file metadata first to get the storage path
        if let Ok(file_meta) = self.items.read_one(pk, None, None).await {
            if let Some(filename) = file_meta.get("filename_disk").and_then(|v| v.as_str()) {
                if let Some(ref storage) = self.storage {
                    if let Err(e) = storage.delete(filename).await {
                        warn!(error = %e, file = %filename, "Failed to delete file from storage");
                    }
                }
            }
        }

        self.items.delete_one(pk, None).await
    }

    /// Import a file from a URL
    pub async fn import_one(&self, url: &str, data: Value) -> Result<PrimaryKey, ServiceError> {
        let mut file_data = data;

        let file_id = uuid::Uuid::new_v4().to_string();
        if let Some(obj) = file_data.as_object_mut() {
            obj.insert("id".to_string(), json!(&file_id));
            obj.insert(
                "uploaded_on".to_string(),
                json!(chrono::Utc::now().to_rfc3339()),
            );
            if let Some(user) = self.ctx.user_id() {
                obj.insert("uploaded_by".to_string(), json!(user));
            }
        }

        // Download file from URL
        let response = reqwest::get(url)
            .await
            .map_err(|e| ServiceError::Internal(format!("Failed to download: {}", e)))?;

        let content_type = response
            .headers()
            .get("content-type")
            .and_then(|h| h.to_str().ok())
            .unwrap_or("application/octet-stream")
            .to_string();

        let bytes = response
            .bytes()
            .await
            .map_err(|e| ServiceError::Internal(format!("Failed to read response: {}", e)))?;

        if let Some(obj) = file_data.as_object_mut() {
            obj.insert("filesize".to_string(), json!(bytes.len()));
            obj.insert("type".to_string(), json!(&content_type));
            if obj.get("filename_disk").is_none() {
                obj.insert("filename_disk".to_string(), json!(&file_id));
            }
        }

        // Store via storage driver
        if let Some(ref storage) = self.storage {
            let filename = file_data
                .get("filename_disk")
                .and_then(|v| v.as_str())
                .unwrap_or(&file_id);

            let reader: Pin<Box<dyn tokio::io::AsyncRead + Send>> =
                Box::pin(std::io::Cursor::new(bytes.to_vec()));

            if let Err(e) = storage.write(filename, reader, Some(&content_type)).await {
                warn!(error = %e, "Failed to store imported file");
            }
        }

        self.items.create_one(file_data, None).await
    }
}

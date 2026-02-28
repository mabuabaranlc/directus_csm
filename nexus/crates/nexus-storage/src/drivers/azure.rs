use crate::{FileStat, ReadOptions, StorageDriver, StorageError};
use async_trait::async_trait;
use azure_storage::prelude::*;
use azure_storage_blobs::prelude::*;
use chrono::{DateTime, Utc};
use futures::{Stream, StreamExt};
use nexus_env::env_string;
use std::pin::Pin;
use tokio::io::AsyncRead;

pub struct AzureDriver {
    container_client: ContainerClient,
    account_name: String,
    container_name: String,
}

impl AzureDriver {
    /// Create a new AzureDriver from environment variables.
    ///
    /// Required env vars:
    ///   - `STORAGE_AZURE_CONTAINER`
    ///   - `STORAGE_AZURE_ACCOUNT_NAME`
    ///   - `STORAGE_AZURE_ACCESS_KEY`
    pub fn new_from_env() -> Result<Self, StorageError> {
        let container_name = env_string("STORAGE_AZURE_CONTAINER").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_AZURE_CONTAINER environment variable is required".to_string(),
            )
        })?;

        let account_name = env_string("STORAGE_AZURE_ACCOUNT_NAME").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_AZURE_ACCOUNT_NAME environment variable is required".to_string(),
            )
        })?;

        let access_key = env_string("STORAGE_AZURE_ACCESS_KEY").ok_or_else(|| {
            StorageError::Backend(
                "STORAGE_AZURE_ACCESS_KEY environment variable is required".to_string(),
            )
        })?;

        let storage_credentials =
            StorageCredentials::access_key(account_name.clone(), access_key);
        let service_client =
            BlobServiceClient::new(account_name.clone(), storage_credentials);
        let container_client = service_client.container_client(&container_name);

        Ok(Self {
            container_client,
            account_name,
            container_name,
        })
    }

    /// Create an AzureDriver with an explicit ContainerClient.
    pub fn new(
        container_client: ContainerClient,
        account_name: String,
        container_name: String,
    ) -> Self {
        Self {
            container_client,
            account_name,
            container_name,
        }
    }

    /// Normalize path: strip leading slash for blob key consistency.
    fn normalize_key(path: &str) -> &str {
        path.strip_prefix('/').unwrap_or(path)
    }

    /// Build the full URL for a blob (used as copy source).
    fn blob_url(&self, blob_name: &str) -> String {
        format!(
            "https://{}.blob.core.windows.net/{}/{}",
            self.account_name, self.container_name, blob_name
        )
    }

    /// Get a BlobClient for the given path.
    fn blob_client(&self, path: &str) -> BlobClient {
        let key = Self::normalize_key(path);
        self.container_client.blob_client(key)
    }

    /// Map an Azure SDK error to a StorageError.
    ///
    /// Attempts to extract HTTP status codes first, then falls back to
    /// string matching on the error message.
    fn map_error(path: &str, err: azure_core::error::Error) -> StorageError {
        // Try to extract structured HTTP error information
        if let Some(http_err) = err.as_http_error() {
            let status = http_err.status();
            if status == azure_core::StatusCode::NotFound {
                return StorageError::NotFound(path.to_string());
            }
            if status == azure_core::StatusCode::Forbidden
                || status == azure_core::StatusCode::Unauthorized
            {
                return StorageError::PermissionDenied(path.to_string());
            }
        }

        // Fall back to string matching for cases where HTTP error is not directly accessible
        let msg = err.to_string();
        if msg.contains("BlobNotFound")
            || msg.contains("ContainerNotFound")
            || msg.contains("404")
            || msg.contains("NotFound")
        {
            StorageError::NotFound(path.to_string())
        } else if msg.contains("AuthorizationFailure")
            || msg.contains("AuthenticationFailed")
            || msg.contains("AccessDenied")
            || msg.contains("403")
            || msg.contains("401")
        {
            StorageError::PermissionDenied(path.to_string())
        } else {
            StorageError::Backend(format!("Azure Blob Storage error: {}", err))
        }
    }
}

#[async_trait]
impl StorageDriver for AzureDriver {
    async fn read(
        &self,
        path: &str,
        options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError> {
        let blob_client = self.blob_client(path);

        let mut request = blob_client.get();

        // Apply byte range if provided
        if let Some(ref opts) = options {
            if let Some((start, end)) = opts.range {
                let range = match end {
                    Some(e) => azure_core::request_options::Range::new(start, e),
                    None => azure_core::request_options::Range::new(start, u64::MAX),
                };
                request = request.range(range);
            }
        }

        // Collect all chunks from the pageable stream into a single byte buffer
        let mut all_bytes = Vec::new();
        let mut stream = request.into_stream();

        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| Self::map_error(path, e))?;
            let data = chunk
                .data
                .collect()
                .await
                .map_err(|e| StorageError::Backend(format!("Failed to read blob data: {}", e)))?;
            all_bytes.extend_from_slice(&data);
        }

        let cursor = std::io::Cursor::new(all_bytes);
        Ok(Box::pin(cursor))
    }

    async fn write(
        &self,
        path: &str,
        mut content: Pin<Box<dyn AsyncRead + Send>>,
        content_type: Option<&str>,
    ) -> Result<(), StorageError> {
        let blob_client = self.blob_client(path);

        // Read the entire content into memory for the PutBlockBlob call.
        // For very large files, consider using block list uploads instead.
        let mut buf = Vec::new();
        tokio::io::AsyncReadExt::read_to_end(&mut content, &mut buf)
            .await
            .map_err(|e| StorageError::Backend(format!("Failed to read content: {}", e)))?;

        let body = azure_core::Body::from(buf);
        let mut builder = blob_client.put_block_blob(body);

        if let Some(ct) = content_type {
            builder = builder.content_type(ct.to_string());
        }

        builder
            .await
            .map_err(|e| Self::map_error(path, e))?;

        Ok(())
    }

    async fn delete(&self, path: &str) -> Result<(), StorageError> {
        let blob_client = self.blob_client(path);

        blob_client
            .delete()
            .await
            .map_err(|e| Self::map_error(path, e))?;

        Ok(())
    }

    async fn stat(&self, path: &str) -> Result<FileStat, StorageError> {
        let blob_client = self.blob_client(path);

        let response = blob_client
            .get_properties()
            .await
            .map_err(|e| Self::map_error(path, e))?;

        let properties = &response.blob.properties;

        let size = properties.content_length;

        // Convert time::OffsetDateTime to chrono::DateTime<Utc>
        let last_modified = properties.last_modified;
        let modified: DateTime<Utc> = DateTime::from_timestamp(
            last_modified.unix_timestamp(),
            last_modified.nanosecond() as u32,
        )
        .unwrap_or_else(Utc::now);

        let content_type = {
            let ct = properties.content_type.clone();
            if ct.is_empty() {
                None
            } else {
                Some(ct)
            }
        };

        Ok(FileStat {
            size,
            modified,
            content_type,
        })
    }

    async fn exists(&self, path: &str) -> Result<bool, StorageError> {
        match self.stat(path).await {
            Ok(_) => Ok(true),
            Err(StorageError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    async fn mv(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        self.copy(src, dest).await?;
        self.delete(src).await?;
        Ok(())
    }

    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError> {
        let src_key = Self::normalize_key(src);
        let dest_client = self.blob_client(dest);

        let source_url = self.blob_url(src_key);
        let source: url::Url = source_url
            .parse()
            .map_err(|e| StorageError::Backend(format!("Invalid source URL: {}", e)))?;

        dest_client
            .copy_from_url(source)
            .await
            .map_err(|e| Self::map_error(src, e))?;

        Ok(())
    }

    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>> {
        let container_client = self.container_client.clone();
        let prefix = prefix.map(|p| Self::normalize_key(p).to_string());

        Box::pin(async_stream::try_stream! {
            let mut builder = container_client.list_blobs();

            if let Some(ref p) = prefix {
                builder = builder.prefix(p.clone());
            }

            let mut stream = builder.into_stream();

            while let Some(page_result) = stream.next().await {
                let page = page_result.map_err(|e| {
                    StorageError::Backend(format!("Azure ListBlobs failed: {}", e))
                })?;

                for blob in page.blobs.blobs() {
                    yield blob.name.clone();
                }
            }
        })
    }
}

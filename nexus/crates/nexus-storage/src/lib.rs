pub mod drivers;

use async_trait::async_trait;
use futures::Stream;
use std::pin::Pin;
use tokio::io::{AsyncRead, AsyncWrite};

/// File metadata
#[derive(Debug, Clone)]
pub struct FileStat {
    pub size: u64,
    pub modified: chrono::DateTime<chrono::Utc>,
    pub content_type: Option<String>,
}

/// Read options for partial reads
#[derive(Debug, Clone, Default)]
pub struct ReadOptions {
    pub range: Option<(u64, Option<u64>)>,
}

/// Trait for storage driver implementations
/// Mirrors packages/storage/src/index.ts Driver interface
#[async_trait]
pub trait StorageDriver: Send + Sync {
    /// Read a file
    async fn read(
        &self,
        path: &str,
        options: Option<ReadOptions>,
    ) -> Result<Pin<Box<dyn AsyncRead + Send>>, StorageError>;

    /// Write a file
    async fn write(
        &self,
        path: &str,
        content: Pin<Box<dyn AsyncRead + Send>>,
        content_type: Option<&str>,
    ) -> Result<(), StorageError>;

    /// Delete a file
    async fn delete(&self, path: &str) -> Result<(), StorageError>;

    /// Get file metadata
    async fn stat(&self, path: &str) -> Result<FileStat, StorageError>;

    /// Check if a file exists
    async fn exists(&self, path: &str) -> Result<bool, StorageError>;

    /// Move a file
    async fn mv(&self, src: &str, dest: &str) -> Result<(), StorageError>;

    /// Copy a file
    async fn copy(&self, src: &str, dest: &str) -> Result<(), StorageError>;

    /// List files with an optional prefix
    fn list(
        &self,
        prefix: Option<&str>,
    ) -> Pin<Box<dyn Stream<Item = Result<String, StorageError>> + Send>>;
}

/// TUS protocol extension for resumable uploads
#[async_trait]
pub trait TusDriver: StorageDriver {
    async fn create_chunk(
        &self,
        key: &str,
        content: &[u8],
        offset: u64,
    ) -> Result<u64, StorageError>;

    async fn finish_chunks(&self, key: &str) -> Result<(), StorageError>;

    async fn delete_chunks(&self, key: &str) -> Result<(), StorageError>;
}

#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("File not found: {0}")]
    NotFound(String),
    #[error("Permission denied: {0}")]
    PermissionDenied(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Storage backend error: {0}")]
    Backend(String),
}

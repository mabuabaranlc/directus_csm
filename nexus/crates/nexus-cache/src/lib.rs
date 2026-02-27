pub mod memory;
pub mod redis_store;

use async_trait::async_trait;
use std::time::Duration;

/// Trait for cache storage backends
#[async_trait]
pub trait CacheStore: Send + Sync {
    /// Get a value by key
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError>;

    /// Set a value with optional TTL
    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<(), CacheError>;

    /// Delete a key
    async fn delete(&self, key: &str) -> Result<(), CacheError>;

    /// Check if a key exists
    async fn has(&self, key: &str) -> Result<bool, CacheError>;

    /// Clear all entries
    async fn clear(&self) -> Result<(), CacheError>;
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("Cache connection error: {0}")]
    Connection(String),
    #[error("Serialization error: {0}")]
    Serialization(String),
    #[error("Key not found: {0}")]
    NotFound(String),
}

/// Cache manager holding all cache instances
pub struct CacheManager {
    pub data: Box<dyn CacheStore>,
    pub system: Box<dyn CacheStore>,
    pub schema: Box<dyn CacheStore>,
    pub lock: Box<dyn CacheStore>,
}

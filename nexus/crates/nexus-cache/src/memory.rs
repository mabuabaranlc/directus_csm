use crate::{CacheError, CacheStore};
use async_trait::async_trait;
use moka::future::Cache;
use std::time::Duration;

pub struct MemoryCache {
    cache: Cache<String, Vec<u8>>,
}

impl MemoryCache {
    pub fn new(max_capacity: u64, default_ttl: Duration) -> Self {
        Self {
            cache: Cache::builder()
                .max_capacity(max_capacity)
                .time_to_live(default_ttl)
                .build(),
        }
    }
}

#[async_trait]
impl CacheStore for MemoryCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        Ok(self.cache.get(key).await)
    }

    async fn set(&self, key: &str, value: &[u8], _ttl: Option<Duration>) -> Result<(), CacheError> {
        self.cache.insert(key.to_string(), value.to_vec()).await;
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        self.cache.remove(key).await;
        Ok(())
    }

    async fn has(&self, key: &str) -> Result<bool, CacheError> {
        Ok(self.cache.get(key).await.is_some())
    }

    async fn clear(&self) -> Result<(), CacheError> {
        self.cache.invalidate_all();
        Ok(())
    }
}

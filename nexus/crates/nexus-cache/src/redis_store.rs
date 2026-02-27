use crate::{CacheError, CacheStore};
use async_trait::async_trait;
use redis::AsyncCommands;
use std::time::Duration;

pub struct RedisCache {
    client: redis::Client,
    prefix: String,
}

impl RedisCache {
    pub fn new(url: &str, prefix: &str) -> Result<Self, CacheError> {
        let client = redis::Client::open(url)
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        Ok(Self {
            client,
            prefix: prefix.to_string(),
        })
    }

    fn prefixed_key(&self, key: &str) -> String {
        format!("{}:{}", self.prefix, key)
    }

    async fn get_connection(&self) -> Result<redis::aio::MultiplexedConnection, CacheError> {
        self.client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))
    }
}

#[async_trait]
impl CacheStore for RedisCache {
    async fn get(&self, key: &str) -> Result<Option<Vec<u8>>, CacheError> {
        let mut conn = self.get_connection().await?;
        let result: Option<Vec<u8>> = conn
            .get(self.prefixed_key(key))
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        Ok(result)
    }

    async fn set(&self, key: &str, value: &[u8], ttl: Option<Duration>) -> Result<(), CacheError> {
        let mut conn = self.get_connection().await?;
        let pk = self.prefixed_key(key);
        if let Some(ttl) = ttl {
            conn.set_ex::<_, _, ()>(&pk, value, ttl.as_secs())
                .await
                .map_err(|e| CacheError::Connection(e.to_string()))?;
        } else {
            conn.set::<_, _, ()>(&pk, value)
                .await
                .map_err(|e| CacheError::Connection(e.to_string()))?;
        }
        Ok(())
    }

    async fn delete(&self, key: &str) -> Result<(), CacheError> {
        let mut conn = self.get_connection().await?;
        conn.del::<_, ()>(self.prefixed_key(key))
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        Ok(())
    }

    async fn has(&self, key: &str) -> Result<bool, CacheError> {
        let mut conn = self.get_connection().await?;
        let exists: bool = conn
            .exists(self.prefixed_key(key))
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;
        Ok(exists)
    }

    async fn clear(&self) -> Result<(), CacheError> {
        let mut conn = self.get_connection().await?;
        let pattern = format!("{}:*", self.prefix);
        let keys: Vec<String> = redis::cmd("KEYS")
            .arg(&pattern)
            .query_async(&mut conn)
            .await
            .map_err(|e| CacheError::Connection(e.to_string()))?;

        if !keys.is_empty() {
            conn.del::<_, ()>(keys)
                .await
                .map_err(|e| CacheError::Connection(e.to_string()))?;
        }
        Ok(())
    }
}

use crate::{BusError, MessageBus, MessageReceiver};
use async_trait::async_trait;

pub struct RedisBus {
    _client: redis::Client,
    prefix: String,
}

impl RedisBus {
    pub fn new(url: &str, prefix: &str) -> Result<Self, BusError> {
        let client = redis::Client::open(url)
            .map_err(|e| BusError::Connection(e.to_string()))?;
        Ok(Self {
            _client: client,
            prefix: prefix.to_string(),
        })
    }

    fn prefixed_channel(&self, channel: &str) -> String {
        format!("{}:{}", self.prefix, channel)
    }
}

#[async_trait]
impl MessageBus for RedisBus {
    async fn publish(&self, channel: &str, message: &[u8]) -> Result<(), BusError> {
        let mut conn = self
            ._client
            .get_multiplexed_async_connection()
            .await
            .map_err(|e| BusError::Connection(e.to_string()))?;

        redis::cmd("PUBLISH")
            .arg(self.prefixed_channel(channel))
            .arg(message)
            .query_async::<()>(&mut conn)
            .await
            .map_err(|e| BusError::Connection(e.to_string()))?;

        Ok(())
    }

    async fn subscribe(
        &self,
        _channel: &str,
    ) -> Result<Box<dyn MessageReceiver>, BusError> {
        // Redis pub/sub requires a dedicated connection via get_async_pubsub
        // For now, return a placeholder — full implementation requires redis PubSub API
        Err(BusError::Connection(
            "Redis pub/sub subscribe not yet fully implemented".to_string(),
        ))
    }

    async fn unsubscribe(&self, _channel: &str) -> Result<(), BusError> {
        Ok(())
    }
}

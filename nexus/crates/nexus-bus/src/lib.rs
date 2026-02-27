pub mod local;
pub mod redis_bus;

use async_trait::async_trait;

/// Trait for inter-process messaging
#[async_trait]
pub trait MessageBus: Send + Sync {
    /// Publish a message to a channel
    async fn publish(&self, channel: &str, message: &[u8]) -> Result<(), BusError>;

    /// Subscribe to a channel, returns a receiver
    async fn subscribe(
        &self,
        channel: &str,
    ) -> Result<Box<dyn MessageReceiver>, BusError>;

    /// Unsubscribe from a channel
    async fn unsubscribe(&self, channel: &str) -> Result<(), BusError>;
}

/// Trait for receiving messages from a channel
#[async_trait]
pub trait MessageReceiver: Send + Sync {
    /// Receive the next message (blocks until available)
    async fn recv(&mut self) -> Result<Vec<u8>, BusError>;
}

#[derive(Debug, thiserror::Error)]
pub enum BusError {
    #[error("Bus connection error: {0}")]
    Connection(String),
    #[error("Channel closed")]
    ChannelClosed,
    #[error("Serialization error: {0}")]
    Serialization(String),
}

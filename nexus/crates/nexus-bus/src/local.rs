use crate::{BusError, MessageBus, MessageReceiver};
use async_trait::async_trait;
use std::collections::HashMap;
use tokio::sync::{broadcast, RwLock};

pub struct LocalBus {
    channels: RwLock<HashMap<String, broadcast::Sender<Vec<u8>>>>,
}

impl LocalBus {
    pub fn new() -> Self {
        Self {
            channels: RwLock::new(HashMap::new()),
        }
    }

    async fn get_or_create_channel(&self, channel: &str) -> broadcast::Sender<Vec<u8>> {
        let channels = self.channels.read().await;
        if let Some(sender) = channels.get(channel) {
            return sender.clone();
        }
        drop(channels);

        let mut channels = self.channels.write().await;
        let (tx, _) = broadcast::channel(256);
        channels.insert(channel.to_string(), tx.clone());
        tx
    }
}

impl Default for LocalBus {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MessageBus for LocalBus {
    async fn publish(&self, channel: &str, message: &[u8]) -> Result<(), BusError> {
        let sender = self.get_or_create_channel(channel).await;
        let _ = sender.send(message.to_vec());
        Ok(())
    }

    async fn subscribe(
        &self,
        channel: &str,
    ) -> Result<Box<dyn MessageReceiver>, BusError> {
        let sender = self.get_or_create_channel(channel).await;
        let receiver = sender.subscribe();
        Ok(Box::new(LocalReceiver { receiver }))
    }

    async fn unsubscribe(&self, _channel: &str) -> Result<(), BusError> {
        Ok(())
    }
}

struct LocalReceiver {
    receiver: broadcast::Receiver<Vec<u8>>,
}

#[async_trait]
impl MessageReceiver for LocalReceiver {
    async fn recv(&mut self) -> Result<Vec<u8>, BusError> {
        self.receiver
            .recv()
            .await
            .map_err(|_| BusError::ChannelClosed)
    }
}

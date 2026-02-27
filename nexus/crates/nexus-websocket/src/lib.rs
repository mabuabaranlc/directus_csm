use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

pub mod controllers;
pub mod handlers;
pub mod message;

#[derive(Debug, Error)]
pub enum WebSocketError {
    #[error("Not authenticated")]
    NotAuthenticated,
    #[error("Forbidden")]
    Forbidden,
    #[error("Invalid message: {0}")]
    InvalidMessage(String),
    #[error("Subscription not found")]
    SubscriptionNotFound,
    #[error("Connection closed")]
    ConnectionClosed,
    #[error("Internal error: {0}")]
    Internal(String),
}

/// WebSocket connection manager — tracks active connections and subscriptions
/// Mirrors api/src/websocket/
pub struct WebSocketManager {
    /// Active subscriptions indexed by connection ID
    subscriptions: Arc<RwLock<HashMap<String, Vec<Subscription>>>>,
    /// Broadcast channel for sending events to all subscribers
    event_tx: broadcast::Sender<WebSocketEvent>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
        }
    }

    /// Register a new subscription for a connection
    pub async fn subscribe(
        &self,
        connection_id: &str,
        subscription: Subscription,
    ) -> broadcast::Receiver<WebSocketEvent> {
        let mut subs = self.subscriptions.write().await;
        subs.entry(connection_id.to_string())
            .or_default()
            .push(subscription);
        self.event_tx.subscribe()
    }

    /// Remove all subscriptions for a connection (on disconnect)
    pub async fn unsubscribe_all(&self, connection_id: &str) {
        let mut subs = self.subscriptions.write().await;
        subs.remove(connection_id);
    }

    /// Remove a specific subscription by UID
    pub async fn unsubscribe(&self, connection_id: &str, uid: &str) {
        let mut subs = self.subscriptions.write().await;
        if let Some(conn_subs) = subs.get_mut(connection_id) {
            conn_subs.retain(|s| s.uid != uid);
        }
    }

    /// Broadcast a data change event to all relevant subscribers
    pub fn broadcast(&self, event: WebSocketEvent) {
        let _ = self.event_tx.send(event);
    }

    /// Get the number of active connections
    pub async fn connection_count(&self) -> usize {
        self.subscriptions.read().await.len()
    }

    /// Get a receiver for events
    pub fn receiver(&self) -> broadcast::Receiver<WebSocketEvent> {
        self.event_tx.subscribe()
    }
}

impl Default for WebSocketManager {
    fn default() -> Self {
        Self::new()
    }
}

/// A subscription for real-time data changes
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Subscription {
    /// Unique identifier for this subscription
    pub uid: String,
    /// Collection being subscribed to
    pub collection: String,
    /// Optional filter for items
    pub query: Option<Value>,
    /// Event types to listen for
    pub event: Option<SubscriptionEvent>,
}

/// Events that can be subscribed to
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SubscriptionEvent {
    Create,
    Update,
    Delete,
}

/// Events broadcast through the WebSocket system
#[derive(Debug, Clone)]
pub struct WebSocketEvent {
    pub event_type: String,
    pub collection: String,
    pub action: String,
    pub payload: Value,
    pub keys: Vec<String>,
}

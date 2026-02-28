use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::{broadcast, RwLock};

use nexus_services::context::ServiceContext;
use nexus_types::accountability::Accountability;
use nexus_types::schema::SchemaOverview;

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
    /// Database backend for WS CRUD operations
    db: Option<Arc<dyn nexus_database::DatabaseBackend>>,
    /// Schema reference for building service contexts
    schema: Option<Arc<RwLock<Arc<SchemaOverview>>>>,
    /// Emitter for events
    emitter: Option<Arc<nexus_emitter::Emitter>>,
}

impl WebSocketManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1024);

        Self {
            subscriptions: Arc::new(RwLock::new(HashMap::new())),
            event_tx,
            db: None,
            schema: None,
            emitter: None,
        }
    }

    /// Set the database backend for WS CRUD operations
    pub fn with_db(mut self, db: Arc<dyn nexus_database::DatabaseBackend>) -> Self {
        self.db = Some(db);
        self
    }

    /// Set the schema for building service contexts
    pub fn with_schema(mut self, schema: Arc<RwLock<Arc<SchemaOverview>>>) -> Self {
        self.schema = Some(schema);
        self
    }

    /// Set the emitter for events
    pub fn with_emitter(mut self, emitter: Arc<nexus_emitter::Emitter>) -> Self {
        self.emitter = Some(emitter);
        self
    }

    /// Build a ServiceContext for WS CRUD operations
    pub async fn service_context(
        &self,
        accountability: Option<Accountability>,
    ) -> Option<ServiceContext> {
        let db = self.db.as_ref()?.clone();
        let schema = {
            let schema_lock = self.schema.as_ref()?;
            schema_lock.read().await.clone()
        };
        let emitter = self
            .emitter
            .as_ref()
            .cloned()
            .unwrap_or_else(|| Arc::new(nexus_emitter::Emitter::new()));

        Some(ServiceContext::new(db, schema, accountability, None, emitter))
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

    /// Get subscriptions for a connection
    pub async fn get_subscriptions(&self, connection_id: &str) -> Vec<Subscription> {
        let subs = self.subscriptions.read().await;
        subs.get(connection_id).cloned().unwrap_or_default()
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

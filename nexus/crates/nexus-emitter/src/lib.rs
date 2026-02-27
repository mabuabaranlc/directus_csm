use serde_json::Value;
use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Type alias for async filter handler functions
/// Filter handlers receive and can modify data before it's processed
pub type FilterHandler = Arc<
    dyn Fn(Value, Value) -> Pin<Box<dyn Future<Output = Result<Value, Box<dyn std::error::Error + Send + Sync>>> + Send>>
        + Send
        + Sync,
>;

/// Type alias for async action handler functions
/// Action handlers are fire-and-forget, spawned as background tasks
pub type ActionHandler = Arc<
    dyn Fn(Value, Value) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync,
>;

/// Type alias for async init handler functions
pub type InitHandler =
    Arc<dyn Fn() -> Pin<Box<dyn Future<Output = Result<(), Box<dyn std::error::Error + Send + Sync>>> + Send>> + Send + Sync>;

/// Event emitter with three channels: filter, action, init
/// Mirrors the Directus Emitter from api/src/emitter.ts
pub struct Emitter {
    filter_handlers: RwLock<HashMap<String, Vec<FilterHandler>>>,
    action_handlers: RwLock<HashMap<String, Vec<ActionHandler>>>,
    init_handlers: RwLock<HashMap<String, Vec<InitHandler>>>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            filter_handlers: RwLock::new(HashMap::new()),
            action_handlers: RwLock::new(HashMap::new()),
            init_handlers: RwLock::new(HashMap::new()),
        }
    }

    /// Register a filter handler for an event
    pub async fn on_filter(&self, event: &str, handler: FilterHandler) {
        self.filter_handlers
            .write()
            .await
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    /// Register an action handler for an event
    pub async fn on_action(&self, event: &str, handler: ActionHandler) {
        self.action_handlers
            .write()
            .await
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    /// Register an init handler for an event
    pub async fn on_init(&self, event: &str, handler: InitHandler) {
        self.init_handlers
            .write()
            .await
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    /// Emit a filter event — handlers are chained, each modifying the payload
    pub async fn emit_filter(
        &self,
        event: &str,
        mut payload: Value,
        meta: Value,
    ) -> Result<Value, Box<dyn std::error::Error + Send + Sync>> {
        let handlers = self.filter_handlers.read().await;

        // Check exact match
        if let Some(event_handlers) = handlers.get(event) {
            for handler in event_handlers.iter() {
                payload = handler(payload, meta.clone()).await?;
            }
        }

        // Check wildcard patterns
        for (pattern, pattern_handlers) in handlers.iter() {
            if pattern.contains('*') && matches_wildcard(pattern, event) {
                for handler in pattern_handlers.iter() {
                    payload = handler(payload, meta.clone()).await?;
                }
            }
        }

        Ok(payload)
    }

    /// Emit an action event — handlers are spawned as background tasks
    pub fn emit_action(&self, event: &str, payload: Value, meta: Value) {
        // Try to get a read lock without blocking; if we can't, skip
        // This is fire-and-forget so it's acceptable
        if let Ok(handlers) = self.action_handlers.try_read() {
            if let Some(event_handlers) = handlers.get(event) {
                for handler in event_handlers.iter() {
                    let handler = handler.clone();
                    let payload = payload.clone();
                    let meta = meta.clone();
                    tokio::spawn(async move {
                        handler(payload, meta).await;
                    });
                }
            }
        }
    }

    /// Emit an init event — handlers are awaited sequentially
    pub async fn emit_init(
        &self,
        event: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let handlers = self.init_handlers.read().await;
        if let Some(event_handlers) = handlers.get(event) {
            for handler in event_handlers.iter() {
                handler().await?;
            }
        }
        Ok(())
    }
}

impl Default for Emitter {
    fn default() -> Self {
        Self::new()
    }
}

/// Simple wildcard pattern matching (supports * as segment wildcard)
fn matches_wildcard(pattern: &str, event: &str) -> bool {
    let pattern_parts: Vec<&str> = pattern.split('.').collect();
    let event_parts: Vec<&str> = event.split('.').collect();

    if pattern_parts.len() != event_parts.len() {
        return false;
    }

    pattern_parts
        .iter()
        .zip(event_parts.iter())
        .all(|(p, e)| *p == "*" || p == e)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_wildcard_matching() {
        assert!(matches_wildcard("items.*", "items.create"));
        assert!(matches_wildcard("items.*.before", "items.create.before"));
        assert!(!matches_wildcard("items.*", "items.create.before"));
        assert!(!matches_wildcard("items.create", "items.update"));
        assert!(matches_wildcard("*.*", "items.create"));
    }
}

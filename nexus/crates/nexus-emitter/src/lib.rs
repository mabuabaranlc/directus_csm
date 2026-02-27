use async_trait::async_trait;
use dashmap::DashMap;
use serde_json::Value;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

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
    filter_handlers: DashMap<String, Vec<FilterHandler>>,
    action_handlers: DashMap<String, Vec<ActionHandler>>,
    init_handlers: DashMap<String, Vec<InitHandler>>,
}

impl Emitter {
    pub fn new() -> Self {
        Self {
            filter_handlers: DashMap::new(),
            action_handlers: DashMap::new(),
            init_handlers: DashMap::new(),
        }
    }

    /// Register a filter handler for an event
    pub fn on_filter(&self, event: &str, handler: FilterHandler) {
        self.filter_handlers
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    /// Register an action handler for an event
    pub fn on_action(&self, event: &str, handler: ActionHandler) {
        self.action_handlers
            .entry(event.to_string())
            .or_default()
            .push(handler);
    }

    /// Register an init handler for an event
    pub fn on_init(&self, event: &str, handler: InitHandler) {
        self.init_handlers
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
        // Check exact match
        if let Some(handlers) = self.filter_handlers.get(event) {
            for handler in handlers.iter() {
                payload = handler(payload, meta.clone()).await?;
            }
        }

        // Check wildcard patterns
        for entry in self.filter_handlers.iter() {
            let pattern = entry.key();
            if pattern.contains('*') && matches_wildcard(pattern, event) {
                for handler in entry.value().iter() {
                    payload = handler(payload, meta.clone()).await?;
                }
            }
        }

        Ok(payload)
    }

    /// Emit an action event — handlers are spawned as background tasks
    pub fn emit_action(&self, event: &str, payload: Value, meta: Value) {
        if let Some(handlers) = self.action_handlers.get(event) {
            for handler in handlers.iter() {
                let handler = handler.clone();
                let payload = payload.clone();
                let meta = meta.clone();
                tokio::spawn(async move {
                    handler(payload, meta).await;
                });
            }
        }
    }

    /// Emit an init event — handlers are awaited sequentially
    pub async fn emit_init(
        &self,
        event: &str,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(handlers) = self.init_handlers.get(event) {
            for handler in handlers.iter() {
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

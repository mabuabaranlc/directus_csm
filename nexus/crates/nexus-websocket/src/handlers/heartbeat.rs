use std::sync::Arc;
use crate::WebSocketManager;
use tokio::time::{interval, Duration, Instant};
use std::collections::HashMap;
use tokio::sync::RwLock;

/// Track last activity per connection
static LAST_ACTIVITY: once_cell::sync::Lazy<RwLock<HashMap<String, Instant>>> =
    once_cell::sync::Lazy::new(|| RwLock::new(HashMap::new()));

/// Record activity for a connection
pub async fn record_activity(connection_id: &str) {
    LAST_ACTIVITY
        .write()
        .await
        .insert(connection_id.to_string(), Instant::now());
}

/// Remove a connection from activity tracking
pub async fn remove_connection(connection_id: &str) {
    LAST_ACTIVITY.write().await.remove(connection_id);
}

/// Start the heartbeat check task
/// Periodically checks for stale connections and cleans them up
pub async fn start_heartbeat(manager: Arc<WebSocketManager>) {
    let mut heartbeat_interval = interval(Duration::from_secs(30));
    let stale_timeout = Duration::from_secs(300); // 5 minutes

    loop {
        heartbeat_interval.tick().await;

        let count = manager.connection_count().await;
        if count > 0 {
            tracing::debug!(
                connections = count,
                "WebSocket heartbeat check"
            );
        }

        // Check for stale connections
        let now = Instant::now();
        let stale: Vec<String> = {
            let activity = LAST_ACTIVITY.read().await;
            activity
                .iter()
                .filter(|(_, last)| now.duration_since(**last) > stale_timeout)
                .map(|(id, _)| id.clone())
                .collect()
        };

        for conn_id in &stale {
            tracing::info!(connection_id = %conn_id, "Disconnecting stale WebSocket connection");
            manager.unsubscribe_all(conn_id).await;
            LAST_ACTIVITY.write().await.remove(conn_id);
        }
    }
}

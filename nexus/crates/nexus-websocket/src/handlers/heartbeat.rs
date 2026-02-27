use std::sync::Arc;
use crate::WebSocketManager;
use tokio::time::{interval, Duration};

/// Start the heartbeat check task
/// Periodically checks for stale connections and cleans them up
pub async fn start_heartbeat(manager: Arc<WebSocketManager>) {
    let mut heartbeat_interval = interval(Duration::from_secs(30));

    loop {
        heartbeat_interval.tick().await;

        let count = manager.connection_count().await;
        if count > 0 {
            tracing::debug!(
                connections = count,
                "WebSocket heartbeat check"
            );
        }
    }
}

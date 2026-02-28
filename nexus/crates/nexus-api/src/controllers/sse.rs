use actix_web::{web, HttpResponse};
use serde_json::json;
use std::sync::Arc;
use tokio::sync::broadcast;

use nexus_websocket::WebSocketManager;

pub fn configure(cfg: &mut web::ServiceConfig) {
    cfg.route("", web::get().to(sse_handler));
}

/// SSE endpoint — streams real-time data change events to clients
/// Mirrors the Directus /api/server/ping + SSE behavior
async fn sse_handler(
    ws_manager: web::Data<Arc<WebSocketManager>>,
) -> HttpResponse {
    let mut event_rx = ws_manager.receiver();

    let stream = async_stream::stream! {
        // Send initial connected event
        yield Ok::<_, actix_web::Error>(
            actix_web::web::Bytes::from("event: init\ndata: {\"type\":\"connected\"}\n\n")
        );

        loop {
            match event_rx.recv().await {
                Ok(event) => {
                    let data = json!({
                        "type": "subscription",
                        "event": event.action,
                        "collection": event.collection,
                        "data": event.payload,
                        "keys": event.keys,
                    });
                    let msg = format!("event: message\ndata: {}\n\n", data);
                    yield Ok(actix_web::web::Bytes::from(msg));
                }
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    let msg = format!(
                        "event: warning\ndata: {{\"message\":\"Lagged {} events\"}}\n\n",
                        n
                    );
                    yield Ok(actix_web::web::Bytes::from(msg));
                }
                Err(broadcast::error::RecvError::Closed) => {
                    break;
                }
            }
        }
    };

    HttpResponse::Ok()
        .content_type("text/event-stream")
        .insert_header(("Cache-Control", "no-cache"))
        .insert_header(("Connection", "keep-alive"))
        .insert_header(("X-Accel-Buffering", "no"))
        .streaming(stream)
}

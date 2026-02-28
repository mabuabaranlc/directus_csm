use crate::context::ServiceContext;
use crate::items::{ItemsService, ServiceError};
use nexus_types::items::PrimaryKey;
use nexus_types::query::Query;
use serde_json::{json, Value};

/// Service for managing webhooks
/// Mirrors api/src/services/webhooks.ts
pub struct WebhooksService {
    pub ctx: ServiceContext,
    items: ItemsService,
}

impl WebhooksService {
    pub fn new(ctx: ServiceContext) -> Self {
        let items = ItemsService::new("directus_webhooks", ctx.clone());
        Self { ctx, items }
    }

    pub async fn create_one(&self, data: Value) -> Result<PrimaryKey, ServiceError> {
        self.items.create_one(data, None).await
    }

    pub async fn read_by_query(&self, query: Query) -> Result<Vec<Value>, ServiceError> {
        self.items.read_by_query(query, None).await
    }

    pub async fn read_one(&self, pk: &PrimaryKey) -> Result<Value, ServiceError> {
        self.items.read_one(pk, None, None).await
    }

    pub async fn update_one(
        &self,
        pk: &PrimaryKey,
        data: Value,
    ) -> Result<PrimaryKey, ServiceError> {
        self.items.update_one(pk, data, None).await
    }

    pub async fn delete_one(&self, pk: &PrimaryKey) -> Result<PrimaryKey, ServiceError> {
        self.items.delete_one(pk, None).await
    }

    /// Deliver webhooks matching an event/collection scope
    /// Called from the emitter when data changes occur
    pub async fn dispatch_webhooks(
        &self,
        event: &str,
        collection: &str,
        payload: &Value,
    ) -> Result<(), ServiceError> {
        // Query active webhooks matching event and collection
        let query = Query {
            filter: Some(nexus_types::filter::Filter::Logical(
                nexus_types::filter::LogicalFilter::And {
                    _and: vec![
                        nexus_types::filter::Filter::Field(
                            [("status".to_string(), json!({ "_eq": "active" }))]
                                .into_iter()
                                .collect(),
                        ),
                    ],
                },
            )),
            ..Default::default()
        };

        let webhooks = self.items.read_by_query(query, None).await?;

        let client = reqwest::Client::new();

        for webhook in webhooks {
            let wh_actions = webhook
                .get("actions")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let wh_collections = webhook
                .get("collections")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let url = webhook.get("url").and_then(|v| v.as_str()).unwrap_or("");

            // Check if this webhook matches the event and collection
            let actions: Vec<&str> = wh_actions.split(',').map(|s| s.trim()).collect();
            let collections: Vec<&str> = wh_collections.split(',').map(|s| s.trim()).collect();

            let action_matches = actions.iter().any(|a| *a == event || *a == "*");
            let collection_matches =
                collections.iter().any(|c| *c == collection || *c == "*") || collections.is_empty();

            if !action_matches || !collection_matches || url.is_empty() {
                continue;
            }

            let method = webhook
                .get("method")
                .and_then(|v| v.as_str())
                .unwrap_or("POST");

            let body = json!({
                "event": event,
                "collection": collection,
                "payload": payload,
                "timestamp": chrono::Utc::now().to_rfc3339(),
            });

            // Fire and forget — don't block the request
            let url = url.to_string();
            let method = method.to_string();
            let client = client.clone();

            tokio::spawn(async move {
                let req = match method.to_uppercase().as_str() {
                    "GET" => client.get(&url),
                    _ => client.post(&url).json(&body),
                };

                match req
                    .header("Content-Type", "application/json")
                    .header("User-Agent", "Nexus-Webhook/1.0")
                    .timeout(std::time::Duration::from_secs(15))
                    .send()
                    .await
                {
                    Ok(resp) => {
                        tracing::debug!(
                            url = %url,
                            status = %resp.status(),
                            "Webhook delivered"
                        );
                    }
                    Err(e) => {
                        tracing::warn!(url = %url, error = %e, "Webhook delivery failed");
                    }
                }
            });
        }

        Ok(())
    }
}

use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Request operation — makes an HTTP request to an external URL
/// Mirrors api/src/operations/request/index.ts
pub struct RequestOperation;

#[async_trait]
impl FlowOperation for RequestOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let url = options
            .get("url")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'url'".to_string()))?;

        let method = options
            .get("method")
            .and_then(|v| v.as_str())
            .unwrap_or("GET")
            .to_uppercase();

        let headers_opt = options.get("headers");
        let body = options.get("body").cloned();

        let client = reqwest::Client::new();

        let mut request = match method.as_str() {
            "GET" => client.get(url),
            "POST" => client.post(url),
            "PUT" => client.put(url),
            "PATCH" => client.patch(url),
            "DELETE" => client.delete(url),
            "HEAD" => client.head(url),
            _ => {
                return Err(FlowError::InvalidConfig(format!(
                    "Unsupported HTTP method: {}",
                    method
                )));
            }
        };

        // Set headers
        if let Some(Value::Object(headers)) = headers_opt {
            for (key, value) in headers {
                if let Some(val_str) = value.as_str() {
                    request = request.header(key.as_str(), val_str);
                }
            }
        }

        // Set body
        if let Some(body_value) = body {
            request = request.json(&body_value);
        }

        match request.send().await {
            Ok(response) => {
                let status = response.status().as_u16();
                let response_headers: serde_json::Map<String, Value> = response
                    .headers()
                    .iter()
                    .map(|(k, v)| {
                        (
                            k.as_str().to_string(),
                            json!(v.to_str().unwrap_or("")),
                        )
                    })
                    .collect();

                let body_text = response.text().await.unwrap_or_default();

                // Try to parse as JSON, fall back to string
                let body_value: Value =
                    serde_json::from_str(&body_text).unwrap_or(json!(body_text));

                Ok(json!({
                    "status": status,
                    "statusText": "",
                    "headers": response_headers,
                    "data": body_value,
                }))
            }
            Err(e) => Err(FlowError::OperationFailed(format!(
                "HTTP request failed: {}",
                e
            ))),
        }
    }

    fn operation_type(&self) -> &str {
        "request"
    }
}

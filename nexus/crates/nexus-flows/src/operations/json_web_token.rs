use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// JWT operation — sign, verify, or decode JSON Web Tokens
/// Mirrors api/src/operations/json-web-token/index.ts
pub struct JwtOperation;

#[async_trait]
impl FlowOperation for JwtOperation {
    async fn execute(
        &self,
        _data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let operation = options
            .get("operation")
            .and_then(|v| v.as_str())
            .unwrap_or("sign");

        let secret = options
            .get("secret")
            .and_then(|v| v.as_str())
            .unwrap_or_default();

        match operation {
            "sign" => {
                let payload = options.get("payload").cloned().unwrap_or(json!({}));
                let key = jsonwebtoken::EncodingKey::from_secret(secret.as_bytes());
                let header = jsonwebtoken::Header::default();

                // Convert payload to claims map
                let claims: std::collections::HashMap<String, Value> =
                    serde_json::from_value(payload).unwrap_or_default();

                match jsonwebtoken::encode(&header, &claims, &key) {
                    Ok(token) => Ok(json!({ "token": token })),
                    Err(e) => Err(FlowError::OperationFailed(format!(
                        "JWT sign failed: {}",
                        e
                    ))),
                }
            }
            "verify" => {
                let token = options
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");
                let key = jsonwebtoken::DecodingKey::from_secret(secret.as_bytes());
                let validation = jsonwebtoken::Validation::default();

                match jsonwebtoken::decode::<std::collections::HashMap<String, Value>>(
                    token, &key, &validation,
                ) {
                    Ok(data) => Ok(json!({
                        "valid": true,
                        "payload": data.claims,
                    })),
                    Err(e) => Ok(json!({
                        "valid": false,
                        "error": e.to_string(),
                    })),
                }
            }
            "decode" => {
                let token = options
                    .get("token")
                    .and_then(|v| v.as_str())
                    .unwrap_or("");

                // Decode without verification
                let key = jsonwebtoken::DecodingKey::from_secret(b"");
                let mut validation = jsonwebtoken::Validation::default();
                validation.insecure_disable_signature_validation();
                validation.validate_exp = false;

                match jsonwebtoken::decode::<std::collections::HashMap<String, Value>>(
                    token, &key, &validation,
                ) {
                    Ok(data) => Ok(json!({
                        "header": {},
                        "payload": data.claims,
                    })),
                    Err(e) => Err(FlowError::OperationFailed(format!(
                        "JWT decode failed: {}",
                        e
                    ))),
                }
            }
            _ => Err(FlowError::InvalidConfig(format!(
                "Unknown JWT operation: {}",
                operation
            ))),
        }
    }

    fn operation_type(&self) -> &str {
        "json-web-token"
    }
}

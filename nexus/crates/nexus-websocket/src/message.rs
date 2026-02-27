use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Incoming WebSocket message types
/// Mirrors the Directus WebSocket protocol
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum IncomingMessage {
    /// Authentication message
    Auth {
        access_token: Option<String>,
        email: Option<String>,
        password: Option<String>,
    },
    /// Subscribe to collection changes
    Subscribe {
        uid: Option<String>,
        collection: String,
        #[serde(default)]
        query: Option<Value>,
        event: Option<String>,
    },
    /// Unsubscribe from a subscription
    Unsubscribe {
        uid: String,
    },
    /// Ping/heartbeat
    Ping,
    /// Items request (CRUD over WebSocket)
    Items {
        uid: Option<String>,
        action: String,
        collection: String,
        #[serde(default)]
        query: Option<Value>,
        #[serde(default)]
        data: Option<Value>,
        #[serde(default)]
        id: Option<Value>,
    },
}

/// Outgoing WebSocket message types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "lowercase")]
pub enum OutgoingMessage {
    /// Authentication result
    Auth {
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorPayload>,
    },
    /// Subscription confirmation
    Subscription {
        uid: String,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        event: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
    },
    /// Pong response
    Pong,
    /// Items response
    Items {
        #[serde(skip_serializing_if = "Option::is_none")]
        uid: Option<String>,
        status: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<Value>,
        #[serde(skip_serializing_if = "Option::is_none")]
        error: Option<ErrorPayload>,
    },
    /// Error message
    Error {
        error: ErrorPayload,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorPayload {
    pub code: String,
    pub message: String,
}

impl OutgoingMessage {
    pub fn auth_ok() -> Self {
        Self::Auth {
            status: "ok".to_string(),
            error: None,
        }
    }

    pub fn auth_error(message: &str) -> Self {
        Self::Auth {
            status: "error".to_string(),
            error: Some(ErrorPayload {
                code: "AUTH_FAILED".to_string(),
                message: message.to_string(),
            }),
        }
    }

    pub fn subscription_event(uid: &str, event: &str, data: Value) -> Self {
        Self::Subscription {
            uid: uid.to_string(),
            status: "ok".to_string(),
            event: Some(event.to_string()),
            data: Some(data),
        }
    }

    pub fn error(code: &str, message: &str) -> Self {
        Self::Error {
            error: ErrorPayload {
                code: code.to_string(),
                message: message.to_string(),
            },
        }
    }
}

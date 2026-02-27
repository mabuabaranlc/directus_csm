use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Items in user-defined collections are dynamic JSON
pub type Item = Value;

/// Primary keys can be strings, numbers, or UUIDs
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(untagged)]
pub enum PrimaryKey {
    String(String),
    Integer(i64),
}

impl std::fmt::Display for PrimaryKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PrimaryKey::String(s) => write!(f, "{}", s),
            PrimaryKey::Integer(n) => write!(f, "{}", n),
        }
    }
}

impl From<String> for PrimaryKey {
    fn from(s: String) -> Self {
        PrimaryKey::String(s)
    }
}

impl From<&str> for PrimaryKey {
    fn from(s: &str) -> Self {
        PrimaryKey::String(s.to_string())
    }
}

impl From<i64> for PrimaryKey {
    fn from(n: i64) -> Self {
        PrimaryKey::Integer(n)
    }
}

/// Options for mutation operations
#[derive(Debug, Clone, Default)]
pub struct MutationOptions {
    pub emit_events: Option<bool>,
    pub bypass_limits: Option<bool>,
    pub auto_purge_cache: Option<bool>,
    pub auto_purge_system_cache: Option<bool>,
}

/// Options for query operations
#[derive(Debug, Clone, Default)]
pub struct QueryOptions {
    pub strip_non_requested: Option<bool>,
    pub permissions_action: Option<String>,
    pub emit_events: Option<bool>,
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShareScope {
    pub collection: String,
    pub item: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Accountability {
    pub role: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    pub user: Option<String>,
    #[serde(default)]
    pub admin: bool,
    #[serde(default)]
    pub app: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub share: Option<String>,
    pub ip: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub user_agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub origin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<String>,
}

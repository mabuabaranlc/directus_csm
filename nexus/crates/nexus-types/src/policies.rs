use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub description: Option<String>,
    pub enforce_tfa: Option<bool>,
    pub ip_access: Option<Vec<String>>,
    #[serde(default)]
    pub app_access: bool,
    #[serde(default)]
    pub admin_access: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Globals {
    pub enforce_tfa: bool,
    pub app_access: bool,
    pub admin_access: bool,
}

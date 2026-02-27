use crate::filter::Filter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Preset {
    pub id: Option<i64>,
    pub bookmark: Option<String>,
    #[serde(default = "default_icon")]
    pub icon: String,
    pub color: Option<String>,
    pub user: Option<String>,
    pub role: Option<String>,
    pub collection: String,
    pub search: Option<String>,
    pub filter: Option<Filter>,
    pub layout: Option<String>,
    pub layout_query: Option<HashMap<String, Value>>,
    pub layout_options: Option<HashMap<String, Value>>,
    pub refresh_interval: Option<i32>,
}

fn default_icon() -> String {
    "bookmark".to_string()
}

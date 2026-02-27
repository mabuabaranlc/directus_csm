use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContentVersion {
    pub id: String,
    pub key: String,
    pub name: Option<String>,
    pub collection: String,
    pub item: String,
    pub hash: String,
    pub date_created: String,
    pub date_updated: Option<String>,
    pub user_created: Option<String>,
    pub user_updated: Option<String>,
    pub delta: Option<Value>,
}

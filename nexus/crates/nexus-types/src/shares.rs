use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Share {
    pub id: String,
    pub name: String,
    pub collection: String,
    pub item: String,
    pub role: String,
    pub password: Option<String>,
    pub user_created: String,
    pub date_created: String,
    pub date_start: Option<String>,
    pub date_end: Option<String>,
    #[serde(default)]
    pub times_used: i32,
    pub max_uses: Option<i32>,
}

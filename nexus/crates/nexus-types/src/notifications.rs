use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub id: String,
    pub status: String,
    pub timestamp: String,
    pub recipient: String,
    pub sender: Option<String>,
    pub subject: String,
    pub message: Option<String>,
    pub collection: Option<String>,
    pub item: Option<String>,
}

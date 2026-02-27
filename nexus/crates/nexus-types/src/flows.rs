use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TriggerType {
    Event,
    Schedule,
    Operation,
    Webhook,
    Manual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlowStatus {
    Active,
    Inactive,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flow {
    pub id: String,
    pub name: Option<String>,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub description: Option<String>,
    pub status: FlowStatus,
    pub trigger: Option<TriggerType>,
    #[serde(default)]
    pub options: Value,
    pub operation: Option<String>,
    pub accountability: Option<String>,
    pub date_created: Option<String>,
    pub user_created: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    pub id: String,
    pub name: Option<String>,
    pub key: String,
    #[serde(rename = "type")]
    pub operation_type: String,
    pub position_x: i32,
    pub position_y: i32,
    #[serde(default)]
    pub options: Value,
    pub resolve: Option<String>,
    pub reject: Option<String>,
    pub flow: String,
    pub date_created: Option<String>,
    pub user_created: Option<String>,
}

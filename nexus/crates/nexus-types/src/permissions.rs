use crate::filter::Filter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PermissionsAction {
    Create,
    Read,
    Update,
    Delete,
    Share,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Permission {
    pub id: Option<i64>,
    pub policy: Option<String>,
    pub collection: String,
    pub action: PermissionsAction,
    pub permissions: Option<Filter>,
    pub validation: Option<Filter>,
    pub presets: Option<HashMap<String, Value>>,
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AccessLevel {
    None,
    Partial,
    Full,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionPermissions {
    pub create: ActionAccess,
    pub read: ActionAccess,
    pub update: ActionAccess,
    pub delete: ActionAccess,
    pub share: ActionAccess,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionAccess {
    pub access: AccessLevel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presets: Option<HashMap<String, Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GlobalAccess {
    pub admin: bool,
    pub app: bool,
}

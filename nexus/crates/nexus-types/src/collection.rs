use crate::fields::Column;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionMeta {
    pub collection: String,
    pub note: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub singleton: bool,
    pub icon: Option<String>,
    pub color: Option<String>,
    pub translations: Option<Vec<CollectionTranslation>>,
    pub display_template: Option<String>,
    pub preview_url: Option<String>,
    #[serde(default)]
    pub versioning: bool,
    pub sort_field: Option<String>,
    pub archive_field: Option<String>,
    pub archive_value: Option<String>,
    pub unarchive_value: Option<String>,
    #[serde(default)]
    pub archive_app_filter: bool,
    pub item_duplication_fields: Option<Vec<String>>,
    pub accountability: Option<String>,
    pub system: Option<bool>,
    pub sort: Option<i32>,
    pub group: Option<String>,
    #[serde(default = "default_collapse")]
    pub collapse: String,
}

fn default_collapse() -> String {
    "open".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionTranslation {
    pub language: String,
    pub translation: String,
    pub singular: String,
    pub plural: String,
}

/// Database table schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Table {
    pub name: String,
    pub comment: Option<String>,
    pub schema: Option<String>,
    pub collation: Option<String>,
    pub engine: Option<String>,
    pub owner: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    pub collection: String,
    pub meta: Option<CollectionMeta>,
    pub schema: Option<Table>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionType {
    Alias,
    Table,
    Unknown,
}

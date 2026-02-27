use crate::fields::FieldType;
use crate::filter::Filter;
use crate::relations::Relation;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldOverview {
    pub field: String,
    pub default_value: Option<serde_json::Value>,
    pub nullable: bool,
    pub generated: bool,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub db_type: Option<String>,
    pub precision: Option<i32>,
    pub scale: Option<i32>,
    #[serde(default)]
    pub special: Vec<String>,
    pub note: Option<String>,
    pub validation: Option<Filter>,
    pub alias: bool,
    #[serde(default)]
    pub searchable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionOverview {
    pub collection: String,
    pub primary: String,
    #[serde(default)]
    pub fields: HashMap<String, FieldOverview>,
    #[serde(default)]
    pub is_singleton: bool,
    pub accountability: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SchemaOverview {
    pub collections: HashMap<String, CollectionOverview>,
    pub relations: Vec<Relation>,
}

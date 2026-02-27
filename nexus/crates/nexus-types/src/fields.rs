use crate::filter::Filter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// All supported field types
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FieldType {
    BigInteger,
    Boolean,
    Date,
    DateTime,
    Decimal,
    Float,
    Integer,
    Json,
    String,
    Text,
    Time,
    Timestamp,
    Binary,
    Uuid,
    Alias,
    Hash,
    Csv,
    Geometry,
    #[serde(rename = "geometry.Point")]
    GeometryPoint,
    #[serde(rename = "geometry.LineString")]
    GeometryLineString,
    #[serde(rename = "geometry.Polygon")]
    GeometryPolygon,
    #[serde(rename = "geometry.MultiPoint")]
    GeometryMultiPoint,
    #[serde(rename = "geometry.MultiLineString")]
    GeometryMultiLineString,
    #[serde(rename = "geometry.MultiPolygon")]
    GeometryMultiPolygon,
    Unknown,
}

/// Field width in the admin UI
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Width {
    Half,
    HalfLeft,
    HalfRight,
    Full,
    Fill,
}

/// Functions that can be applied to fields
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FieldFunction {
    Year,
    Month,
    Week,
    Day,
    Weekday,
    Hour,
    Minute,
    Second,
    Count,
}

/// Field metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldMeta {
    pub id: Option<i64>,
    pub collection: String,
    pub field: String,
    pub group: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    pub interface: Option<String>,
    pub display: Option<String>,
    pub options: Option<HashMap<String, Value>>,
    pub display_options: Option<HashMap<String, Value>>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub required: bool,
    pub sort: Option<i32>,
    pub special: Option<Vec<String>>,
    pub translations: Option<Vec<FieldTranslation>>,
    pub width: Option<Width>,
    pub note: Option<String>,
    pub conditions: Option<Vec<Condition>>,
    pub validation: Option<Filter>,
    pub validation_message: Option<String>,
    #[serde(default)]
    pub searchable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FieldTranslation {
    pub language: String,
    pub translation: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Condition {
    pub name: String,
    pub rule: HashMap<String, Value>,
    pub readonly: Option<bool>,
    pub hidden: Option<bool>,
    pub options: Option<HashMap<String, Value>>,
    pub required: Option<bool>,
}

/// Database column schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Column {
    pub name: String,
    pub table: String,
    pub data_type: String,
    pub default_value: Option<Value>,
    pub max_length: Option<i32>,
    pub numeric_precision: Option<i32>,
    pub numeric_scale: Option<i32>,
    pub is_nullable: bool,
    pub is_unique: bool,
    pub is_indexed: bool,
    pub is_primary_key: bool,
    pub is_generated: bool,
    pub has_auto_increment: bool,
    pub foreign_key_table: Option<String>,
    pub foreign_key_column: Option<String>,
    pub comment: Option<String>,
    pub schema: Option<String>,
}

/// A field with its metadata and schema
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
    pub collection: String,
    pub field: String,
    #[serde(rename = "type")]
    pub field_type: FieldType,
    pub schema: Option<Column>,
    pub meta: Option<FieldMeta>,
}

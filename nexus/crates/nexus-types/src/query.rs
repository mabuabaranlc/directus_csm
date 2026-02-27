use crate::filter::Filter;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExportFormat {
    Json,
    Csv,
    CsvUtf8,
    Xml,
    Yaml,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Query {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub export: Option<ExportFormat>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub group: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub aggregate: Option<Aggregate>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deep: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias: Option<HashMap<String, String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Aggregate {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub avg: Option<Vec<String>>,
    #[serde(rename = "avgDistinct", skip_serializing_if = "Option::is_none")]
    pub avg_distinct: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<Vec<String>>,
    #[serde(rename = "countDistinct", skip_serializing_if = "Option::is_none")]
    pub count_distinct: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sum: Option<Vec<String>>,
    #[serde(rename = "sumDistinct", skip_serializing_if = "Option::is_none")]
    pub sum_distinct: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<Vec<String>>,
    #[serde(rename = "countAll", skip_serializing_if = "Option::is_none")]
    pub count_all: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DeepQuery {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _fields: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _sort: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _filter: Option<Filter>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _limit: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _offset: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _page: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _search: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _aggregate: Option<Aggregate>,
}

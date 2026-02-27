use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;

/// Filter operators supported by Nexus
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FilterOperator {
    Eq,
    Neq,
    Lt,
    Lte,
    Gt,
    Gte,
    In,
    Nin,
    Null,
    Nnull,
    Contains,
    Ncontains,
    Icontains,
    Between,
    Nbetween,
    Empty,
    Nempty,
    Intersects,
    Nintersects,
    IntersectsBbox,
    NintersectsBbox,
    StartsWith,
    NstartsWith,
    IstartsWith,
    NistartsWith,
    EndsWith,
    NendsWith,
    IendsWith,
    NiendsWith,
    Regex,
}

/// A filter can be either a logical filter (_and/_or) or a field filter
/// This uses serde_json::Value for maximum flexibility with the recursive structure
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Filter {
    Logical(LogicalFilter),
    Field(HashMap<String, Value>),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LogicalFilter {
    And {
        _and: Vec<Filter>,
    },
    Or {
        _or: Vec<Filter>,
    },
}

/// Field-level filter operators
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct FieldFilterOperator {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _eq: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _neq: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _lt: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _lte: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _gt: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _gte: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _in: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nin: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _null: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nnull: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _contains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _ncontains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _icontains: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _starts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nstarts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _istarts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nistarts_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _ends_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nends_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _iends_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _niends_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _between: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nbetween: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _empty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nempty: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _intersects: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nintersects: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _intersects_bbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _nintersects_bbox: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _regex: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub _submitted: Option<bool>,
}

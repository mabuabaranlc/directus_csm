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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_filter_operator_serialization() {
        let op = FilterOperator::Eq;
        let serialized = serde_json::to_string(&op).unwrap();
        assert_eq!(serialized, "\"eq\"");

        let op = FilterOperator::Icontains;
        let serialized = serde_json::to_string(&op).unwrap();
        assert_eq!(serialized, "\"icontains\"");
    }

    #[test]
    fn test_filter_operator_deserialization() {
        let op: FilterOperator = serde_json::from_str("\"eq\"").unwrap();
        assert_eq!(op, FilterOperator::Eq);

        let op: FilterOperator = serde_json::from_str("\"between\"").unwrap();
        assert_eq!(op, FilterOperator::Between);

        let op: FilterOperator = serde_json::from_str("\"starts_with\"").unwrap();
        assert_eq!(op, FilterOperator::StartsWith);
    }

    #[test]
    fn test_filter_operator_roundtrip() {
        let operators = vec![
            FilterOperator::Eq, FilterOperator::Neq, FilterOperator::Lt,
            FilterOperator::Lte, FilterOperator::Gt, FilterOperator::Gte,
            FilterOperator::In, FilterOperator::Nin, FilterOperator::Null,
            FilterOperator::Nnull, FilterOperator::Contains, FilterOperator::Ncontains,
            FilterOperator::Icontains, FilterOperator::Between, FilterOperator::Nbetween,
            FilterOperator::Empty, FilterOperator::Nempty, FilterOperator::Regex,
        ];

        for op in operators {
            let json = serde_json::to_string(&op).unwrap();
            let deserialized: FilterOperator = serde_json::from_str(&json).unwrap();
            assert_eq!(op, deserialized);
        }
    }

    #[test]
    fn test_field_filter_simple_eq() {
        let json_str = r#"{ "title": { "_eq": "hello" } }"#;
        let filter: Filter = serde_json::from_str(json_str).unwrap();

        match filter {
            Filter::Field(map) => {
                assert!(map.contains_key("title"));
                let title_filter = &map["title"];
                assert_eq!(title_filter["_eq"], json!("hello"));
            }
            _ => panic!("Expected Field filter"),
        }
    }

    #[test]
    fn test_logical_and_filter() {
        let json_str = r#"{ "_and": [{ "status": { "_eq": "published" } }, { "year": { "_gte": 2020 } }] }"#;
        let filter: Filter = serde_json::from_str(json_str).unwrap();

        match filter {
            Filter::Logical(LogicalFilter::And { _and }) => {
                assert_eq!(_and.len(), 2);
            }
            _ => panic!("Expected Logical And filter"),
        }
    }

    #[test]
    fn test_logical_or_filter() {
        let json_str = r#"{ "_or": [{ "status": { "_eq": "draft" } }, { "status": { "_eq": "published" } }] }"#;
        let filter: Filter = serde_json::from_str(json_str).unwrap();

        match filter {
            Filter::Logical(LogicalFilter::Or { _or }) => {
                assert_eq!(_or.len(), 2);
            }
            _ => panic!("Expected Logical Or filter"),
        }
    }

    #[test]
    fn test_filter_serialization_roundtrip() {
        let original = json!({
            "_and": [
                { "name": { "_contains": "test" } },
                { "age": { "_gte": 18 } }
            ]
        });

        let filter: Filter = serde_json::from_value(original.clone()).unwrap();
        let serialized = serde_json::to_value(&filter).unwrap();

        // Verify the structure survived roundtrip
        assert!(serialized["_and"].is_array());
        assert_eq!(serialized["_and"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn test_field_filter_operator_defaults() {
        let ffo = FieldFilterOperator::default();
        assert!(ffo._eq.is_none());
        assert!(ffo._neq.is_none());
        assert!(ffo._in.is_none());
        assert!(ffo._contains.is_none());
        assert!(ffo._between.is_none());
        assert!(ffo._regex.is_none());
    }

    #[test]
    fn test_field_filter_operator_deserialization() {
        let json_str = r#"{ "_eq": "test", "_null": false, "_in": [1, 2, 3] }"#;
        let ffo: FieldFilterOperator = serde_json::from_str(json_str).unwrap();
        assert_eq!(ffo._eq, Some(json!("test")));
        assert_eq!(ffo._null, Some(false));
        assert_eq!(ffo._in, Some(vec![json!(1), json!(2), json!(3)]));
        assert!(ffo._neq.is_none());
    }

    #[test]
    fn test_field_filter_skip_serializing_none() {
        let ffo = FieldFilterOperator {
            _eq: Some(json!("hello")),
            ..Default::default()
        };
        let serialized = serde_json::to_value(&ffo).unwrap();
        let obj = serialized.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["_eq"], json!("hello"));
    }
}

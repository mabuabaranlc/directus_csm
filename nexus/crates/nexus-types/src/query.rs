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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_query_default() {
        let q = Query::default();
        assert!(q.fields.is_none());
        assert!(q.sort.is_none());
        assert!(q.filter.is_none());
        assert!(q.limit.is_none());
        assert!(q.offset.is_none());
        assert!(q.page.is_none());
        assert!(q.search.is_none());
        assert!(q.aggregate.is_none());
        assert!(q.deep.is_none());
        assert!(q.alias.is_none());
    }

    #[test]
    fn test_query_deserialization() {
        let json_str = r#"{
            "fields": ["id", "title", "status"],
            "sort": ["-date_created", "title"],
            "limit": 25,
            "offset": 0,
            "search": "hello"
        }"#;
        let q: Query = serde_json::from_str(json_str).unwrap();
        assert_eq!(q.fields.unwrap(), vec!["id", "title", "status"]);
        assert_eq!(q.sort.unwrap(), vec!["-date_created", "title"]);
        assert_eq!(q.limit.unwrap(), 25);
        assert_eq!(q.offset.unwrap(), 0);
        assert_eq!(q.search.unwrap(), "hello");
    }

    #[test]
    fn test_query_with_filter() {
        let json_str = r#"{
            "fields": ["*"],
            "filter": { "status": { "_eq": "published" } },
            "limit": 10
        }"#;
        let q: Query = serde_json::from_str(json_str).unwrap();
        assert!(q.filter.is_some());
        assert_eq!(q.limit.unwrap(), 10);
    }

    #[test]
    fn test_query_skip_serializing_none() {
        let q = Query {
            limit: Some(10),
            ..Default::default()
        };
        let serialized = serde_json::to_value(&q).unwrap();
        let obj = serialized.as_object().unwrap();
        assert_eq!(obj.len(), 1);
        assert_eq!(obj["limit"], json!(10));
    }

    #[test]
    fn test_query_roundtrip() {
        let original = json!({
            "fields": ["id", "name"],
            "limit": 50,
            "offset": 10,
            "sort": ["name"]
        });
        let q: Query = serde_json::from_value(original).unwrap();
        let serialized = serde_json::to_value(&q).unwrap();
        assert_eq!(serialized["fields"], json!(["id", "name"]));
        assert_eq!(serialized["limit"], json!(50));
        assert_eq!(serialized["offset"], json!(10));
        assert_eq!(serialized["sort"], json!(["name"]));
    }

    #[test]
    fn test_aggregate_deserialization() {
        let json_str = r#"{
            "count": ["*"],
            "avg": ["price"],
            "countDistinct": ["category"],
            "sum": ["quantity"],
            "min": ["price"],
            "max": ["price"]
        }"#;
        let agg: Aggregate = serde_json::from_str(json_str).unwrap();
        assert_eq!(agg.count.unwrap(), vec!["*"]);
        assert_eq!(agg.avg.unwrap(), vec!["price"]);
        assert_eq!(agg.count_distinct.unwrap(), vec!["category"]);
        assert_eq!(agg.sum.unwrap(), vec!["quantity"]);
        assert_eq!(agg.min.unwrap(), vec!["price"]);
        assert_eq!(agg.max.unwrap(), vec!["price"]);
    }

    #[test]
    fn test_aggregate_camel_case_rename() {
        let agg = Aggregate {
            avg_distinct: Some(vec!["price".to_string()]),
            sum_distinct: Some(vec!["qty".to_string()]),
            count_all: Some(vec!["*".to_string()]),
            ..Default::default()
        };
        let serialized = serde_json::to_value(&agg).unwrap();
        assert!(serialized.get("avgDistinct").is_some());
        assert!(serialized.get("sumDistinct").is_some());
        assert!(serialized.get("countAll").is_some());
        // Snake case should NOT appear
        assert!(serialized.get("avg_distinct").is_none());
        assert!(serialized.get("sum_distinct").is_none());
        assert!(serialized.get("count_all").is_none());
    }

    #[test]
    fn test_export_format_serialization() {
        assert_eq!(serde_json::to_string(&ExportFormat::Json).unwrap(), "\"json\"");
        assert_eq!(serde_json::to_string(&ExportFormat::Csv).unwrap(), "\"csv\"");
        assert_eq!(serde_json::to_string(&ExportFormat::CsvUtf8).unwrap(), "\"csv_utf8\"");
        assert_eq!(serde_json::to_string(&ExportFormat::Xml).unwrap(), "\"xml\"");
        assert_eq!(serde_json::to_string(&ExportFormat::Yaml).unwrap(), "\"yaml\"");
    }

    #[test]
    fn test_export_format_deserialization() {
        let f: ExportFormat = serde_json::from_str("\"json\"").unwrap();
        assert_eq!(f, ExportFormat::Json);
        let f: ExportFormat = serde_json::from_str("\"csv_utf8\"").unwrap();
        assert_eq!(f, ExportFormat::CsvUtf8);
    }

    #[test]
    fn test_deep_query_deserialization() {
        let json_str = r#"{
            "_fields": ["id", "name"],
            "_limit": 5,
            "_sort": ["-name"],
            "_filter": { "active": { "_eq": true } }
        }"#;
        let dq: DeepQuery = serde_json::from_str(json_str).unwrap();
        assert_eq!(dq._fields.unwrap(), vec!["id", "name"]);
        assert_eq!(dq._limit.unwrap(), 5);
        assert_eq!(dq._sort.unwrap(), vec!["-name"]);
        assert!(dq._filter.is_some());
    }

    #[test]
    fn test_query_with_aggregate() {
        let json_str = r#"{
            "aggregate": {
                "count": ["*"],
                "avg": ["salary"]
            },
            "group": ["department"]
        }"#;
        let q: Query = serde_json::from_str(json_str).unwrap();
        assert!(q.aggregate.is_some());
        let agg = q.aggregate.unwrap();
        assert_eq!(agg.count.unwrap(), vec!["*"]);
        assert_eq!(agg.avg.unwrap(), vec!["salary"]);
        assert_eq!(q.group.unwrap(), vec!["department"]);
    }
}

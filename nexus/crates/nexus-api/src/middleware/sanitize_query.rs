use nexus_types::filter::Filter;
use nexus_types::query::{Aggregate, Query};
use serde_json::Value;
use std::collections::HashMap;

/// Parse and sanitize query parameters from the HTTP request
/// Mirrors api/src/middleware/sanitize-query.ts
pub fn sanitize_query(query_string: &str) -> Query {
    let params = parse_query_string(query_string);
    let mut query = Query::default();

    // fields
    if let Some(fields_str) = params.get("fields") {
        let fields: Vec<String> = fields_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !fields.is_empty() {
            query.fields = Some(fields);
        }
    }

    // sort
    if let Some(sort_str) = params.get("sort") {
        let sort: Vec<String> = sort_str
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        if !sort.is_empty() {
            query.sort = Some(sort);
        }
    }

    // filter (JSON)
    if let Some(filter_str) = params.get("filter") {
        if let Ok(filter_value) = serde_json::from_str::<Value>(filter_str) {
            if let Ok(filter) = serde_json::from_value::<Filter>(filter_value) {
                query.filter = Some(filter);
            }
        }
    }

    // limit
    if let Some(limit_str) = params.get("limit") {
        if let Ok(limit) = limit_str.parse::<i64>() {
            query.limit = Some(limit);
        }
    }

    // offset
    if let Some(offset_str) = params.get("offset") {
        if let Ok(offset) = offset_str.parse::<i64>() {
            query.offset = Some(offset);
        }
    }

    // page
    if let Some(page_str) = params.get("page") {
        if let Ok(page) = page_str.parse::<i64>() {
            query.page = Some(page);
        }
    }

    // search
    if let Some(search) = params.get("search") {
        if !search.is_empty() {
            query.search = Some(search.clone());
        }
    }

    // deep (JSON for nested query params)
    if let Some(deep_str) = params.get("deep") {
        if let Ok(deep_value) = serde_json::from_str::<Value>(deep_str) {
            query.deep = Some(deep_value);
        }
    }

    // aggregate
    if let Some(agg_str) = params.get("aggregate") {
        if let Ok(agg) = serde_json::from_str::<Aggregate>(agg_str) {
            query.aggregate = Some(agg);
        }
    }

    // alias
    if let Some(alias_str) = params.get("alias") {
        if let Ok(alias_value) = serde_json::from_str::<HashMap<String, String>>(alias_str) {
            query.alias = Some(alias_value);
        }
    }

    query
}

fn parse_query_string(qs: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    let qs = if let Some(stripped) = qs.strip_prefix('?') {
        stripped
    } else {
        qs
    };

    for pair in qs.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = urlencoding::decode(key).unwrap_or_default().to_string();
            let value = urlencoding::decode(value).unwrap_or_default().to_string();
            result.insert(key, value);
        }
    }

    result
}

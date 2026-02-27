use serde_json::Value;

pub fn format_labels(value: &Value) -> String {
    match value {
        Value::Array(arr) => {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect::<Vec<_>>()
                .join(", ")
        }
        Value::String(s) => s.clone(),
        _ => "—".to_string(),
    }
}

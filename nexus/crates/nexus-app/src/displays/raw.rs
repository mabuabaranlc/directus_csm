use serde_json::Value;

pub fn format_raw(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => "—".to_string(),
        Value::Array(arr) => format!("{} items", arr.len()),
        Value::Object(_) => "{...}".to_string(),
    }
}

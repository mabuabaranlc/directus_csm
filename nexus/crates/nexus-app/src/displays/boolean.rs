use serde_json::Value;

pub fn format_boolean(value: &Value) -> String {
    match value {
        Value::Bool(true) => "True".to_string(),
        Value::Bool(false) => "False".to_string(),
        Value::Number(n) => {
            if n.as_i64() == Some(1) { "True".to_string() }
            else { "False".to_string() }
        }
        Value::String(s) if s == "true" || s == "1" => "True".to_string(),
        _ => "—".to_string(),
    }
}

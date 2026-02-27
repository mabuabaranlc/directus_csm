use serde_json::Value;

pub fn format_datetime(value: &Value) -> String {
    match value {
        Value::String(s) => {
            // Return as-is; full date formatting would use chrono or js-sys Date
            if s.len() > 19 {
                s[..19].replace('T', " ")
            } else {
                s.replace('T', " ")
            }
        }
        Value::Null => "—".to_string(),
        _ => value.to_string(),
    }
}

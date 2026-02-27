use serde_json::Value;

pub fn format_value(value: &Value) -> String {
    match value {
        Value::String(s) => {
            if s.len() > 100 {
                format!("{}...", &s[..100])
            } else {
                s.clone()
            }
        }
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                if f.fract() == 0.0 {
                    format!("{}", f as i64)
                } else {
                    format!("{:.2}", f)
                }
            } else {
                n.to_string()
            }
        }
        Value::Bool(b) => if *b { "Yes" } else { "No" }.to_string(),
        Value::Null => "—".to_string(),
        _ => serde_json::to_string(value).unwrap_or_else(|_| "—".to_string()),
    }
}

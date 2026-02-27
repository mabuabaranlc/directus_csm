use leptos::prelude::*;
use serde_json::Value;
use crate::components::avatar::Avatar;

#[component]
pub fn UserDisplay(
    value: Value,
) -> impl IntoView {
    let (name, _email) = match &value {
        Value::Object(obj) => {
            let first = obj.get("first_name").and_then(|v| v.as_str()).unwrap_or("");
            let last = obj.get("last_name").and_then(|v| v.as_str()).unwrap_or("");
            let email = obj.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let name = format!("{} {}", first, last).trim().to_string();
            (if name.is_empty() { email.clone() } else { name }, email)
        }
        Value::String(s) => (s.clone(), String::new()),
        _ => ("—".to_string(), String::new()),
    };

    view! {
        <span class="display-user">
            <Avatar name=name.clone()/>
            <span>{name}</span>
        </span>
    }
}

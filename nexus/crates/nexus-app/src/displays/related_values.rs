use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn RelatedValuesDisplay(
    value: Value,
    #[prop(optional, into)] template: Option<String>,
) -> impl IntoView {
    let display = match &value {
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().take(5).map(|item| {
                match item {
                    Value::Object(obj) => {
                        if let Some(tmpl) = &template {
                            let mut result = tmpl.clone();
                            for (key, val) in obj {
                                let placeholder = format!("{{{{{}}}}}", key);
                                result = result.replace(&placeholder, &val.as_str().unwrap_or("").to_string());
                            }
                            result
                        } else {
                            obj.values().next()
                                .and_then(|v| v.as_str())
                                .unwrap_or("?")
                                .to_string()
                        }
                    }
                    Value::String(s) => s.clone(),
                    _ => "?".to_string(),
                }
            }).collect();
            let suffix = if arr.len() > 5 { format!(" +{} more", arr.len() - 5) } else { String::new() };
            format!("{}{}", items.join(", "), suffix)
        }
        Value::Object(obj) => {
            obj.values().next()
                .and_then(|v| v.as_str())
                .unwrap_or("—")
                .to_string()
        }
        _ => "—".to_string(),
    };

    view! { <span class="display-related">{display}</span> }
}

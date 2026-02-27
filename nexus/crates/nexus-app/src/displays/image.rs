use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn ImageDisplay(
    value: Value,
) -> impl IntoView {
    let src = match &value {
        Value::String(id) => format!("/assets/{}", id),
        Value::Object(obj) => {
            obj.get("id")
                .and_then(|v| v.as_str())
                .map(|id| format!("/assets/{}", id))
                .unwrap_or_default()
        }
        _ => String::new(),
    };

    if src.is_empty() {
        view! { <span>"—"</span> }.into_any()
    } else {
        view! { <img class="display-image" src=src alt=""/> }.into_any()
    }
}

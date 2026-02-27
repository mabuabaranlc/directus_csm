use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn TranslationsDisplay(
    value: Value,
) -> impl IntoView {
    let count = match &value {
        Value::Array(arr) => arr.len(),
        _ => 0,
    };

    view! {
        <span class="display-translations">
            {if count > 0 {
                format!("{} translation{}", count, if count == 1 { "" } else { "s" })
            } else {
                "—".to_string()
            }}
        </span>
    }
}

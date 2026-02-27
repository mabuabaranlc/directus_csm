use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn ColorDisplay(
    value: Value,
) -> impl IntoView {
    match &value {
        Value::String(s) => {
            let color = s.clone();
            view! {
                <span class="display-color">
                    <span class="color-swatch" style=format!("background-color: {}", color)></span>
                    {color}
                </span>
            }.into_any()
        }
        _ => view! { <span>"—"</span> }.into_any(),
    }
}

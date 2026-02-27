use leptos::prelude::*;
use serde_json::Value;
use crate::components::icon::Icon;

#[component]
pub fn IconDisplay(
    value: Value,
) -> impl IntoView {
    match &value {
        Value::String(s) => {
            let icon_name = s.clone();
            view! { <Icon name=icon_name/> }.into_any()
        }
        _ => view! { <span>"—"</span> }.into_any(),
    }
}

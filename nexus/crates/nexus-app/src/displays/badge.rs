use leptos::prelude::*;
use serde_json::Value;
use crate::components::badge::Badge;

#[component]
pub fn BadgeDisplay(
    value: Value,
    #[prop(optional, into)] color: Option<String>,
) -> impl IntoView {
    let label = match &value {
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        _ => "—".to_string(),
    };

    view! {
        <Badge label=label color=color.unwrap_or_else(|| "var(--primary)".into())/>
    }
}

use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn MetricPanel(
    value: Value,
    #[prop(optional, into)] label: Option<String>,
    #[prop(optional, into)] prefix: Option<String>,
    #[prop(optional, into)] suffix: Option<String>,
) -> impl IntoView {
    let display = match &value {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        _ => "—".to_string(),
    };

    view! {
        <div class="panel-metric">
            {label.map(|l| view! { <span class="metric-label">{l}</span> })}
            <div class="metric-value">
                {prefix.map(|p| view! { <span class="metric-prefix">{p}</span> })}
                <span class="metric-number">{display}</span>
                {suffix.map(|s| view! { <span class="metric-suffix">{s}</span> })}
            </div>
        </div>
    }
}

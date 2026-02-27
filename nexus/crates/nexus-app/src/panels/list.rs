use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn ListPanel(
    items: Vec<Value>,
    #[prop(optional, into)] _display_template: Option<String>,
) -> impl IntoView {
    view! {
        <div class="panel-list">
            <For
                each=move || items.clone()
                key=|item| item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                children=|item| {
                    let display = match &item {
                        Value::Object(obj) => {
                            obj.values().next()
                                .and_then(|v| v.as_str())
                                .unwrap_or("\u{2014}")
                                .to_string()
                        }
                        Value::String(s) => s.clone(),
                        _ => "\u{2014}".to_string(),
                    };
                    view! {
                        <div class="panel-list-item">
                            {display}
                        </div>
                    }
                }
            />
        </div>
    }
}

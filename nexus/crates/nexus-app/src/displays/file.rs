use leptos::prelude::*;
use serde_json::Value;
use crate::components::icon::Icon;

#[component]
pub fn FileDisplay(
    value: Value,
) -> impl IntoView {
    let display = match &value {
        Value::String(s) => s.clone(),
        Value::Object(obj) => {
            obj.get("filename_download")
                .or_else(|| obj.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("File")
                .to_string()
        }
        Value::Null => return view! { <span>"—"</span> }.into_any(),
        _ => "—".to_string(),
    };

    view! {
        <span class="display-file">
            <Icon name="attach_file"/>
            {display}
        </span>
    }.into_any()
}

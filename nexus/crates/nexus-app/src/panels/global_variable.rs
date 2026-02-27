use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn GlobalVariablePanel(
    #[prop(into)] field_key: String,
    value: RwSignal<Value>,
    #[prop(optional, into)] label: Option<String>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    };

    view! {
        <div class="panel-variable">
            {label.map(|l| view! { <label>{l}</label> })}
            <input
                type="text"
                prop:value=display_value
                on:input=move |ev| {
                    value.set(Value::String(event_target_value(&ev)));
                }
            />
        </div>
    }
}

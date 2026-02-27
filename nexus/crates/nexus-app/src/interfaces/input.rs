use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn InputInterface(
    value: RwSignal<Value>,
    #[prop(optional, into, default = "text".into())] input_type: String,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    #[prop(optional)] max_length: Option<usize>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        other => other.to_string(),
    };

    view! {
        <div class="interface-input">
            <input
                type=input_type
                placeholder=placeholder.unwrap_or_default()
                disabled=disabled.unwrap_or(false)
                maxlength=max_length.map(|m| m.to_string()).unwrap_or_default()
                prop:value=display_value
                on:input=move |ev| {
                    let val = event_target_value(&ev);
                    let json_val = Value::String(val);
                    value.set(json_val.clone());
                    on_change.run(json_val);
                }
            />
            {max_length.map(|max| view! {
                <span class="char-count">
                    {move || format!("{}/{}", display_value().len(), max)}
                </span>
            })}
        </div>
    }
}

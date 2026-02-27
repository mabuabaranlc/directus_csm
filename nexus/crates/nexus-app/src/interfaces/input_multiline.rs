use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn InputMultilineInterface(
    value: RwSignal<Value>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    };

    view! {
        <div class="interface-textarea">
            <textarea
                placeholder=placeholder.unwrap_or_default()
                disabled=disabled.unwrap_or(false)
                prop:value=display_value
                on:input=move |ev| {
                    let val = Value::String(event_target_value(&ev));
                    value.set(val.clone());
                    on_change.run(val);
                }
            ></textarea>
        </div>
    }
}

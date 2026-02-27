use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn ColorInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Null => "#000000".to_string(),
        other => other.to_string(),
    };

    view! {
        <div class="interface-color">
            <input
                type="color"
                disabled=disabled.unwrap_or(false)
                prop:value=display_value
                on:input=move |ev| {
                    let val = Value::String(event_target_value(&ev));
                    value.set(val.clone());
                    on_change.run(val);
                }
            />
            <input
                type="text"
                class="color-hex"
                prop:value=display_value
                on:input=move |ev| {
                    let val = Value::String(event_target_value(&ev));
                    value.set(val.clone());
                    on_change.run(val);
                }
            />
        </div>
    }
}

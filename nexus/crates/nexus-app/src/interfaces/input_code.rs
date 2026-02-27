use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn InputCodeInterface(
    value: RwSignal<Value>,
    #[prop(optional, into)] language: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => serde_json::to_string_pretty(&other).unwrap_or_default(),
    };

    view! {
        <div class="interface-code">
            <div class="code-header">
                {language.map(|l| view! { <span class="code-language">{l}</span> })}
            </div>
            <textarea
                class="code-editor"
                spellcheck="false"
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

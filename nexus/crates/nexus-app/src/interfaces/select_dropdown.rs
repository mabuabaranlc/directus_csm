use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn SelectDropdownInterface(
    value: RwSignal<Value>,
    #[prop(optional)] choices: Option<Vec<(String, String)>>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    #[prop(optional)] allow_other: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    };

    let options = choices.unwrap_or_default();

    view! {
        <div class="interface-select">
            <select
                disabled=disabled.unwrap_or(false)
                prop:value=display_value
                on:change=move |ev| {
                    let val = Value::String(event_target_value(&ev));
                    value.set(val.clone());
                    on_change.run(val);
                }
            >
                <option value="" disabled=true selected=move || value.get().is_null()>
                    {placeholder.clone().unwrap_or_else(|| "Select...".into())}
                </option>
                {options.into_iter().map(|(val, label)| view! {
                    <option value=val>{label}</option>
                }).collect::<Vec<_>>()}
            </select>
        </div>
    }
}

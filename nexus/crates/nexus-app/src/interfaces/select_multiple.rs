use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn SelectMultipleInterface(
    value: RwSignal<Value>,
    #[prop(optional)] choices: Option<Vec<(String, String)>>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let selected = move || -> Vec<String> {
        match value.get() {
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            _ => vec![],
        }
    };

    let options = choices.unwrap_or_default();

    let toggle = move |val: String| {
        let mut current = selected();
        if current.contains(&val) {
            current.retain(|v| v != &val);
        } else {
            current.push(val);
        }
        let new_val = Value::Array(current.into_iter().map(Value::String).collect());
        value.set(new_val.clone());
        on_change.run(new_val);
    };

    view! {
        <div class="interface-select-multiple">
            {options.into_iter().map(|(val, label)| {
                let v = val.clone();
                let v2 = val.clone();
                view! {
                    <label class="select-option">
                        <input
                            type="checkbox"
                            disabled=disabled.unwrap_or(false)
                            prop:checked=move || selected().contains(&v)
                            on:change=move |_| toggle(v2.clone())
                        />
                        <span>{label}</span>
                    </label>
                }
            }).collect::<Vec<_>>()}
        </div>
    }
}

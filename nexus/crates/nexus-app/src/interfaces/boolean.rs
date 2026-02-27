use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn BooleanInterface(
    value: RwSignal<Value>,
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let is_checked = move || value.get().as_bool().unwrap_or(false);

    view! {
        <div class="interface-boolean">
            <label class="toggle-label">
                <input
                    type="checkbox"
                    disabled=disabled.unwrap_or(false)
                    prop:checked=is_checked
                    on:change=move |_| {
                        let new_val = Value::Bool(!is_checked());
                        value.set(new_val.clone());
                        on_change.run(new_val);
                    }
                />
                <span class="toggle-switch"></span>
                {label.map(|l| view! { <span class="toggle-text">{l}</span> })}
            </label>
        </div>
    }
}

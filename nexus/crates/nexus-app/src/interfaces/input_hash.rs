use leptos::prelude::*;
use serde_json::Value;
use crate::components::icon::Icon;

#[component]
pub fn InputHashInterface(
    value: RwSignal<Value>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let has_value = move || !value.get().is_null() && value.get() != Value::String(String::new());

    view! {
        <div class="interface-hash">
            <Show
                when=has_value
                fallback=move || view! {
                    <input
                        type="password"
                        placeholder=placeholder.clone().unwrap_or_else(|| "Enter value to hash...".into())
                        disabled=disabled.unwrap_or(false)
                        on:input=move |ev| {
                            let val = Value::String(event_target_value(&ev));
                            on_change.run(val);
                        }
                    />
                }
            >
                <div class="hash-display">
                    <Icon name="lock"/>
                    <span>"**********"</span>
                    <button on:click=move |_| {
                        value.set(Value::Null);
                    }>
                        <Icon name="edit"/>
                    </button>
                </div>
            </Show>
        </div>
    }
}

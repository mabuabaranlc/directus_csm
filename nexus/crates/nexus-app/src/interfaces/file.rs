use leptos::prelude::*;
use serde_json::Value;
use crate::components::icon::Icon;

#[component]
pub fn FileInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let has_file = move || !value.get().is_null() && value.get() != Value::String(String::new());

    let file_id = move || match value.get() {
        Value::String(s) => s,
        _ => String::new(),
    };

    view! {
        <div class="interface-file">
            <Show
                when=has_file
                fallback=move || view! {
                    <button class="file-upload-btn" disabled=disabled.unwrap_or(false)>
                        <Icon name="attach_file"/>
                        " Select File"
                    </button>
                }
            >
                <div class="file-preview">
                    <Icon name="description"/>
                    <span class="file-id">{file_id}</span>
                    <button class="file-remove" on:click=move |_| {
                        value.set(Value::Null);
                        on_change.run(Value::Null);
                    }>
                        <Icon name="close"/>
                    </button>
                </div>
            </Show>
        </div>
    }
}

use leptos::prelude::*;
use serde_json::Value;
use crate::components::icon::Icon;

#[component]
pub fn FileImageInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let has_image = move || !value.get().is_null() && value.get() != Value::String(String::new());

    let image_id = move || match value.get() {
        Value::String(s) => s,
        _ => String::new(),
    };

    view! {
        <div class="interface-file-image">
            <Show
                when=has_image
                fallback=move || view! {
                    <div class="image-upload-area" class:disabled=disabled.unwrap_or(false)>
                        <Icon name="add_photo_alternate"/>
                        <p>"Click or drag image here"</p>
                    </div>
                }
            >
                <div class="image-preview">
                    <img src=move || format!("/assets/{}", image_id()) alt="Preview"/>
                    <button class="image-remove" on:click=move |_| {
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

use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn InputRichTextInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let display_value = move || match value.get() {
        Value::String(s) => s,
        Value::Null => String::new(),
        other => other.to_string(),
    };

    // Simple contenteditable fallback — a full WYSIWYG editor would need a JS interop library
    view! {
        <div class="interface-rich-text">
            <div class="rich-text-toolbar">
                <button type="button" title="Bold"><b>"B"</b></button>
                <button type="button" title="Italic"><i>"I"</i></button>
                <button type="button" title="Underline"><u>"U"</u></button>
            </div>
            <textarea
                class="rich-text-editor"
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

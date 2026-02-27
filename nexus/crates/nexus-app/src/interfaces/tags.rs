use leptos::prelude::*;
use serde_json::Value;
use crate::components::chip::Chip;

#[component]
pub fn TagsInterface(
    value: RwSignal<Value>,
    #[prop(optional)] placeholder: Option<String>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let input = RwSignal::new(String::new());

    let tags = move || -> Vec<String> {
        match value.get() {
            Value::Array(arr) => arr.iter().filter_map(|v| v.as_str().map(String::from)).collect(),
            Value::String(s) => s.split(',').map(|t| t.trim().to_string()).filter(|t| !t.is_empty()).collect(),
            _ => vec![],
        }
    };

    let update_tags = move |new_tags: Vec<String>| {
        let val = Value::Array(new_tags.into_iter().map(Value::String).collect());
        value.set(val.clone());
        on_change.run(val);
    };

    let add_tag = move || {
        let tag = input.get().trim().to_string();
        if !tag.is_empty() {
            let mut current = tags();
            if !current.contains(&tag) {
                current.push(tag);
                update_tags(current);
            }
            input.set(String::new());
        }
    };

    let tag_chips = move || {
        tags().into_iter().map(|tag| {
            let tag_display = tag.clone();
            let tag_remove = tag.clone();
            view! {
                <Chip
                    label=tag_display
                    close=Callback::new(move |_| {
                        let mut current = tags();
                        current.retain(|t| t != &tag_remove);
                        update_tags(current);
                    })
                />
            }
        }).collect::<Vec<_>>()
    };

    view! {
        <div class="interface-tags">
            <div class="tags-list">
                {tag_chips}
            </div>
            <input
                type="text"
                placeholder=placeholder.unwrap_or_else(|| "Add tag...".into())
                disabled=disabled.unwrap_or(false)
                prop:value=move || input.get()
                on:input=move |ev| input.set(event_target_value(&ev))
                on:keydown=move |ev: web_sys::KeyboardEvent| {
                    if ev.key() == "Enter" {
                        ev.prevent_default();
                        add_tag();
                    }
                }
            />
        </div>
    }
}

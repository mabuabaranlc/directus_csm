use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn TranslationsInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let _ = disabled;
    let _ = on_change;

    let entries = move || -> Vec<(String, String)> {
        match value.get() {
            Value::Array(arr) => arr.iter().filter_map(|item| {
                let lang = item.get("language")?.as_str()?.to_string();
                let text = item.get("value").or_else(|| item.get("text"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some((lang, text))
            }).collect(),
            _ => vec![],
        }
    };

    view! {
        <div class="interface-translations">
            <For
                each=move || entries()
                key=|(lang, _)| lang.clone()
                children=|entry| {
                    view! {
                        <div class="translation-row">
                            <span class="lang-code">{entry.0}</span>
                            <span class="lang-value">{entry.1}</span>
                        </div>
                    }
                }
            />
            <Show when=move || entries().is_empty()>
                <p class="no-translations">"No translations"</p>
            </Show>
        </div>
    }
}

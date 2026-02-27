use leptos::prelude::*;
use serde_json::Value;
use crate::components::card::Card;
use crate::components::pagination::Pagination;

#[component]
pub fn CardsLayout(
    items: Signal<Vec<Value>>,
    #[prop(optional, into)] title_field: Option<String>,
    #[prop(optional, into)] subtitle_field: Option<String>,
    #[prop(optional, into)] image_field: Option<String>,
    page: RwSignal<usize>,
    total: Signal<usize>,
    per_page: Signal<usize>,
) -> impl IntoView {
    let tf = title_field.unwrap_or_default();
    let sf = subtitle_field.unwrap_or_default();
    let _imgf = image_field.unwrap_or_default();

    view! {
        <div class="layout-cards">
            <div class="cards-grid">
                <For
                    each=move || items.get()
                    key=|item| item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                    children=move |item| {
                        let title = if !tf.is_empty() {
                            item.get(&tf).and_then(|v| v.as_str()).unwrap_or("").to_string()
                        } else {
                            "Untitled".to_string()
                        };
                        let subtitle = if !sf.is_empty() {
                            item.get(&sf).and_then(|v| v.as_str()).map(String::from)
                        } else {
                            None
                        };
                        view! {
                            <Card>
                                <h3>{title}</h3>
                                {subtitle.map(|s| view! { <p>{s}</p> })}
                            </Card>
                        }
                    }
                />
            </div>
            <Pagination page=page total=total per_page=per_page/>
        </div>
    }
}

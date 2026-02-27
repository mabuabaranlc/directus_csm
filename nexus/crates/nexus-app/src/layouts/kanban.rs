use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn KanbanLayout(
    items: Signal<Vec<Value>>,
    #[prop(into)] group_field: String,
) -> impl IntoView {
    let grouped = move || {
        let mut groups: std::collections::BTreeMap<String, Vec<Value>> = std::collections::BTreeMap::new();
        for item in items.get() {
            let group = item.get(&group_field)
                .and_then(|v| v.as_str())
                .unwrap_or("Ungrouped")
                .to_string();
            groups.entry(group).or_default().push(item);
        }
        groups.into_iter().collect::<Vec<_>>()
    };

    view! {
        <div class="layout-kanban">
            <div class="kanban-columns">
                <For
                    each=grouped
                    key=|(group, _)| group.clone()
                    children=|group_data| {
                        let group_name = group_data.0.clone();
                        let group_items = group_data.1.clone();
                        let count = group_items.len();
                        view! {
                            <div class="kanban-column">
                                <div class="kanban-column-header">
                                    <h3>{group_name}</h3>
                                    <span class="kanban-count">{count}</span>
                                </div>
                                <div class="kanban-cards">
                                    <For
                                        each=move || group_items.clone()
                                        key=|item| item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                                        children=|item| {
                                            let id_str = item.get("id").and_then(|v| v.as_str()).unwrap_or("?").to_string();
                                            view! {
                                                <div class="kanban-card">
                                                    <span>{id_str}</span>
                                                </div>
                                            }
                                        }
                                    />
                                </div>
                            </div>
                        }
                    }
                />
            </div>
        </div>
    }
}

use leptos::prelude::*;
use serde_json::Value;

/// Simplified DataTable component used by collection pages
#[component]
pub fn DataTable(
    columns: Vec<(String, String)>,
    rows: Vec<Value>,
    sort_field: RwSignal<String>,
    sort_desc: RwSignal<bool>,
) -> impl IntoView {
    let header_cols = columns.clone();
    let body_cols = columns.clone();
    let row_data = rows.clone();

    let headers = header_cols.into_iter().map(|(key, label)| {
        let key_click = key.clone();
        view! {
            <th
                class="sortable"
                on:click=move |_| {
                    let current = sort_field.get();
                    if current == key_click {
                        sort_desc.update(|v| *v = !*v);
                    } else {
                        sort_field.set(key_click.clone());
                        sort_desc.set(false);
                    }
                }
            >
                {label}
            </th>
        }
    }).collect::<Vec<_>>();

    let body_rows = row_data.into_iter().enumerate().map(|(_i, row)| {
        let cells = body_cols.iter().map(|(key, _)| {
            let val = row.get(key)
                .map(|v| match v {
                    Value::String(s) => s.clone(),
                    Value::Null => String::new(),
                    other => other.to_string(),
                })
                .unwrap_or_default();
            view! { <td>{val}</td> }
        }).collect::<Vec<_>>();

        view! {
            <tr class="table-row">
                {cells}
            </tr>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="table-container">
            <table class="data-table">
                <thead>
                    <tr>{headers}</tr>
                </thead>
                <tbody>
                    {body_rows}
                </tbody>
            </table>
        </div>
    }
}

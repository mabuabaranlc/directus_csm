use leptos::prelude::*;
use serde_json::Value;
use crate::components::table::DataTable;
use crate::components::pagination::Pagination;

#[component]
pub fn TabularLayout(
    columns: Vec<(String, String)>,
    items: Signal<Vec<Value>>,
    page: RwSignal<usize>,
    total: Signal<usize>,
    per_page: Signal<usize>,
    sort_field: RwSignal<String>,
    sort_desc: RwSignal<bool>,
) -> impl IntoView {
    view! {
        <div class="layout-tabular">
            <DataTable
                columns=columns
                rows=items.get()
                sort_field=sort_field
                sort_desc=sort_desc
            />
            <Pagination page=page total=total per_page=per_page/>
        </div>
    }
}

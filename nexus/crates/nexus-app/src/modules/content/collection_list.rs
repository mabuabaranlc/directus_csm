use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use crate::api::client::ApiClient;
use crate::stores::fields::FieldsStore;
use crate::stores::collections::CollectionsStore;
use crate::components::table::DataTable;
use crate::components::pagination::Pagination;
use crate::components::search_input::SearchInput;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn CollectionListPage() -> impl IntoView {
    let params = use_params_map();
    let collection = move || params.get().get("collection").unwrap_or_default();

    let items = RwSignal::new(Vec::<Value>::new());
    let total = RwSignal::new(0usize);
    let page = RwSignal::new(1usize);
    let per_page = RwSignal::new(25usize);
    let search = RwSignal::new(String::new());
    let loading = RwSignal::new(true);
    let sort_field = RwSignal::new(String::new());
    let sort_desc = RwSignal::new(false);

    let fields_store = expect_context::<FieldsStore>();
    let collections_store = expect_context::<CollectionsStore>();
    let client = ApiClient::new();

    // Fetch items when collection, page, or search changes
    Effect::new(move || {
        let coll = collection();
        let p = page.get();
        let pp = per_page.get();
        let s = search.get();
        let sf = sort_field.get();
        let sd = sort_desc.get();
        let client = client.clone();

        loading.set(true);

        leptos::task::spawn_local(async move {
            let mut path = format!(
                "/items/{}?limit={}&offset={}&meta=total_count",
                coll, pp, (p - 1) * pp
            );
            if !s.is_empty() {
                path.push_str(&format!("&search={}", urlencoding::encode(&s)));
            }
            if !sf.is_empty() {
                let prefix = if sd { "-" } else { "" };
                path.push_str(&format!("&sort={}{}", prefix, sf));
            }

            match client.get::<Value>(&path).await {
                Ok(resp) => {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        items.set(data.clone());
                    }
                    if let Some(meta) = resp.get("meta") {
                        if let Some(tc) = meta.get("total_count").and_then(|v| v.as_u64()) {
                            total.set(tc as usize);
                        }
                    }
                }
                Err(_) => {
                    items.set(vec![]);
                }
            }
            loading.set(false);
        });
    });

    let visible_fields = move || {
        let coll = collection();
        fields_store.visible_for_collection(&coll)
    };

    let columns = move || -> Vec<(String, String)> {
        visible_fields()
            .into_iter()
            .take(6)
            .map(|f| {
                let label = f.display_name();
                (f.field, label)
            })
            .collect()
    };

    let collection_name = move || {
        let coll = collection();
        collections_store.get_collection(&coll)
            .map(|c| c.collection.clone())
            .unwrap_or_else(|| coll.clone())
    };

    view! {
        <div class="collection-list module-page">
            <div class="page-header">
                <h1>{collection_name}</h1>
                <div class="page-header-actions">
                    <SearchInput value=search/>
                    <A href=format!("/content/{}/+", collection()) attr:class="btn btn-primary">
                        <Icon name="add"/>
                        " Create Item"
                    </A>
                </div>
            </div>
            <div class="page-body">
                <Show when=move || loading.get()>
                    <Loading/>
                </Show>
                <Show when=move || !loading.get() && items.get().is_empty()>
                    <EmptyState
                        icon="box".to_string()
                        title="No Items"
                        description="This collection is empty.".to_string()
                    />
                </Show>
                <Show when=move || !loading.get() && !items.get().is_empty()>
                    <DataTable
                        columns=columns()
                        rows=items.get()
                        sort_field=sort_field
                        sort_desc=sort_desc
                    />
                    <Pagination
                        page=page
                        total=Signal::derive(move || total.get())
                        per_page=Signal::derive(move || per_page.get())
                    />
                </Show>
            </div>
        </div>
    }
}

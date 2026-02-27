use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use crate::api::client::ApiClient;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use crate::components::pagination::Pagination;
use crate::components::search_input::SearchInput;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn FilesPage() -> impl IntoView {
    let params = use_params_map();
    let file_id = move || params.get().get("id").map(|s| s.to_string());

    let files = RwSignal::new(Vec::<Value>::new());
    let total = RwSignal::new(0usize);
    let page = RwSignal::new(1usize);
    let per_page = RwSignal::new(25usize);
    let search = RwSignal::new(String::new());
    let loading = RwSignal::new(true);
    let view_mode = RwSignal::new("grid".to_string());
    let client = ApiClient::new();

    Effect::new(move || {
        let p = page.get();
        let pp = per_page.get();
        let s = search.get();
        let client = client.clone();

        loading.set(true);
        leptos::task::spawn_local(async move {
            let mut path = format!("/files?limit={}&offset={}&meta=total_count", pp, (p - 1) * pp);
            if !s.is_empty() {
                path.push_str(&format!("&search={}", urlencoding::encode(&s)));
            }

            match client.get::<Value>(&path).await {
                Ok(resp) => {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        files.set(data.clone());
                    }
                    if let Some(meta) = resp.get("meta") {
                        if let Some(tc) = meta.get("total_count").and_then(|v| v.as_u64()) {
                            total.set(tc as usize);
                        }
                    }
                }
                Err(_) => files.set(vec![]),
            }
            loading.set(false);
        });
    });

    view! {
        <div class="files-page module-page">
            <div class="page-header">
                <h1>"File Library"</h1>
                <div class="page-header-actions">
                    <SearchInput value=search/>
                    <div class="view-toggle">
                        <button
                            class:active=move || view_mode.get() == "grid"
                            on:click=move |_| view_mode.set("grid".into())
                        >
                            <Icon name="grid_view"/>
                        </button>
                        <button
                            class:active=move || view_mode.get() == "list"
                            on:click=move |_| view_mode.set("list".into())
                        >
                            <Icon name="list"/>
                        </button>
                    </div>
                    <button class="btn btn-primary">
                        <Icon name="add"/>
                        " Upload File"
                    </button>
                </div>
            </div>
            <div class="page-body">
                <Show when=move || loading.get()>
                    <Loading/>
                </Show>
                <Show when=move || !loading.get() && files.get().is_empty()>
                    <EmptyState
                        icon="folder".to_string()
                        title="No Files"
                        description="Upload files to get started.".to_string()
                    />
                </Show>
                <Show when=move || !loading.get() && !files.get().is_empty()>
                    <Show when=move || view_mode.get() == "grid">
                        <div class="files-grid">
                            <For
                                each=move || files.get()
                                key=|f| f.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                                children=|file| {
                                    let title = file.get("title")
                                        .or_else(|| file.get("filename_download"))
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("Untitled")
                                        .to_string();
                                    let file_type = file.get("type")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let id = file.get("id")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string();
                                    let is_image = file_type.starts_with("image/");
                                    view! {
                                        <div class="file-card">
                                            <div class="file-preview">
                                                {if is_image {
                                                    view! { <img src=format!("/assets/{}", id) alt=title.clone()/> }.into_any()
                                                } else {
                                                    view! { <Icon name="description"/> }.into_any()
                                                }}
                                            </div>
                                            <div class="file-info">
                                                <span class="file-title">{title}</span>
                                            </div>
                                        </div>
                                    }
                                }
                            />
                        </div>
                    </Show>
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

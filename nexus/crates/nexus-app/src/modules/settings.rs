use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use crate::api::client::ApiClient;
use crate::components::icon::Icon;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use serde_json::Value;

#[derive(Clone)]
#[allow(dead_code)]
struct SettingsModule {
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    description: &'static str,
}

const SETTINGS_MODULES: &[SettingsModule] = &[
    SettingsModule { id: "data-model", icon: "database", label: "Data Model", description: "Configure collections and fields" },
    SettingsModule { id: "roles", icon: "supervisor_account", label: "Access Control", description: "Manage roles and permissions" },
    SettingsModule { id: "presets", icon: "bookmark", label: "Presets & Bookmarks", description: "Manage default collection presets" },
    SettingsModule { id: "translations", icon: "translate", label: "Translation Strings", description: "Manage custom translations" },
    SettingsModule { id: "flows", icon: "bolt", label: "Flows", description: "Automation workflows and triggers" },
    SettingsModule { id: "webhooks", icon: "webhook", label: "Webhooks", description: "HTTP callbacks for events" },
    SettingsModule { id: "extensions", icon: "extension", label: "Extensions", description: "Manage installed extensions" },
    SettingsModule { id: "project", icon: "public", label: "Project Settings", description: "General project configuration" },
];

#[component]
pub fn SettingsPage() -> impl IntoView {
    let params = use_params_map();
    let sub_route = move || params.get().get("rest").unwrap_or_default();

    view! {
        <div class="settings-page module-page">
            <div class="page-header">
                <h1>"Settings"</h1>
            </div>
            <div class="page-body settings-body">
                <nav class="settings-nav">
                    {SETTINGS_MODULES.iter().map(|m| {
                        let id = m.id;
                        let icon = m.icon.to_string();
                        let label = m.label;
                        view! {
                            <A href=format!("/settings/{}", id) attr:class="settings-nav-item">
                                <Icon name=icon/>
                                <span>{label}</span>
                            </A>
                        }
                    }).collect::<Vec<_>>()}
                </nav>
                <div class="settings-content">
                    <Show
                        when=move || !sub_route().is_empty()
                        fallback=|| view! {
                            <div class="settings-welcome">
                                <Icon name="settings"/>
                                <h2>"Settings"</h2>
                                <p>"Select a module from the sidebar to get started."</p>
                            </div>
                        }
                    >
                        {move || {
                            let route = sub_route();
                            match route.as_str() {
                                "data-model" => view! { <DataModelSettings/> }.into_any(),
                                "roles" => view! { <CrudListSettings endpoint="/roles" label="Roles" icon="supervisor_account"/> }.into_any(),
                                "presets" => view! { <CrudListSettings endpoint="/presets" label="Presets" icon="bookmark"/> }.into_any(),
                                "translations" => view! { <CrudListSettings endpoint="/translations" label="Translations" icon="translate"/> }.into_any(),
                                "flows" => view! { <CrudListSettings endpoint="/flows" label="Flows" icon="bolt"/> }.into_any(),
                                "webhooks" => view! { <CrudListSettings endpoint="/webhooks" label="Webhooks" icon="webhook"/> }.into_any(),
                                "extensions" => view! { <ExtensionsSettings/> }.into_any(),
                                "project" => view! { <ProjectSettings/> }.into_any(),
                                _ => view! { <p>"Unknown settings module"</p> }.into_any(),
                            }
                        }}
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Data Model settings — shows collections and their fields
#[component]
fn DataModelSettings() -> impl IntoView {
    let collections = RwSignal::new(Vec::<Value>::new());
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    Effect::new({
        let client = client.clone();
        move || {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(resp) = client.get::<Value>("/collections").await {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        collections.set(data.clone());
                    }
                }
                loading.set(false);
            });
        }
    });

    view! {
        <div class="settings-data-model">
            <div class="settings-header">
                <h2>"Data Model"</h2>
                <button class="btn btn-primary">
                    <Icon name="add"/>
                    " Create Collection"
                </button>
            </div>
            <Show when=move || loading.get()>
                <Loading/>
            </Show>
            <Show when=move || !loading.get()>
                <div class="collections-list">
                    <For
                        each=move || collections.get()
                        key=|c| c.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_string()
                        children=|coll| {
                            let name = coll.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let icon = coll.get("meta").and_then(|m| m.get("icon")).and_then(|v| v.as_str()).unwrap_or("folder").to_string();
                            let note = coll.get("meta").and_then(|m| m.get("note")).and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let is_system = name.starts_with("directus_");
                            view! {
                                <div class=if is_system { "collection-item system" } else { "collection-item" }>
                                    <Icon name=icon/>
                                    <div class="collection-info">
                                        <span class="collection-name">{name.clone()}</span>
                                        <span class="collection-note">{note}</span>
                                    </div>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Generic CRUD list settings for roles, presets, translations, flows, webhooks
#[component]
fn CrudListSettings(
    endpoint: &'static str,
    label: &'static str,
    icon: &'static str,
) -> impl IntoView {
    let items = RwSignal::new(Vec::<Value>::new());
    let loading = RwSignal::new(true);
    let client = ApiClient::new();
    let endpoint = endpoint.to_string();
    let icon = icon.to_string();

    Effect::new({
        let client = client.clone();
        let endpoint = endpoint.clone();
        move || {
            let client = client.clone();
            let endpoint = endpoint.clone();
            leptos::task::spawn_local(async move {
                if let Ok(resp) = client.get::<Value>(&endpoint).await {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        items.set(data.clone());
                    }
                }
                loading.set(false);
            });
        }
    });

    view! {
        <div class="settings-crud-list">
            <div class="settings-header">
                <h2>{label}</h2>
                <button class="btn btn-primary">
                    <Icon name="add"/>
                    {format!(" Create {}", label)}
                </button>
            </div>
            <Show when=move || loading.get()>
                <Loading/>
            </Show>
            <Show when=move || !loading.get() && items.get().is_empty()>
                <EmptyState
                    icon=icon.clone()
                    title=format!("No {} Found", label)
                    description=format!("Create your first {} to get started.", label.to_lowercase()).to_string()
                />
            </Show>
            <Show when=move || !loading.get() && !items.get().is_empty()>
                <div class="crud-items-list">
                    <For
                        each=move || items.get()
                        key=|item| item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                        children=|item| {
                            let name = item.get("name")
                                .or(item.get("description"))
                                .or(item.get("key"))
                                .and_then(|v| v.as_str())
                                .unwrap_or("Untitled")
                                .to_string();
                            let id = item.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            view! {
                                <div class="crud-item">
                                    <span class="item-name">{name}</span>
                                    <span class="item-id">{id}</span>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Extensions settings
#[component]
fn ExtensionsSettings() -> impl IntoView {
    let extensions = RwSignal::new(Vec::<Value>::new());
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    Effect::new({
        let client = client.clone();
        move || {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(resp) = client.get::<Value>("/extensions").await {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        extensions.set(data.clone());
                    }
                }
                loading.set(false);
            });
        }
    });

    view! {
        <div class="settings-extensions">
            <div class="settings-header">
                <h2>"Extensions"</h2>
            </div>
            <Show when=move || loading.get()>
                <Loading/>
            </Show>
            <Show when=move || !loading.get() && extensions.get().is_empty()>
                <EmptyState
                    icon="extension".to_string()
                    title="No Extensions Installed"
                    description="Install extensions to add custom functionality.".to_string()
                />
            </Show>
            <Show when=move || !loading.get() && !extensions.get().is_empty()>
                <div class="extensions-list">
                    <For
                        each=move || extensions.get()
                        key=|ext| ext.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string()
                        children=|ext| {
                            let name = ext.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let ext_type = ext.get("type").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let version = ext.get("version").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let enabled = ext.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
                            view! {
                                <div class="extension-item">
                                    <Icon name="extension"/>
                                    <div class="extension-info">
                                        <span class="extension-name">{name}</span>
                                        <span class="extension-meta">{format!("{} v{}", ext_type, version)}</span>
                                    </div>
                                    <span class=if enabled { "status-badge active" } else { "status-badge inactive" }>
                                        {if enabled { "Enabled" } else { "Disabled" }}
                                    </span>
                                </div>
                            }
                        }
                    />
                </div>
            </Show>
        </div>
    }
}

/// Project settings
#[component]
fn ProjectSettings() -> impl IntoView {
    let settings = RwSignal::new(Option::<Value>::None);
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    Effect::new({
        let client = client.clone();
        move || {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(resp) = client.get::<Value>("/settings").await {
                    if let Some(data) = resp.get("data") {
                        settings.set(Some(data.clone()));
                    }
                }
                loading.set(false);
            });
        }
    });

    view! {
        <div class="settings-project">
            <div class="settings-header">
                <h2>"Project Settings"</h2>
            </div>
            <Show when=move || loading.get()>
                <Loading/>
            </Show>
            <Show when=move || !loading.get()>
                {move || {
                    let s = settings.get().unwrap_or(Value::Object(serde_json::Map::new()));
                    let project_name = s.get("project_name").and_then(|v| v.as_str()).unwrap_or("Nexus").to_string();
                    let project_url = s.get("project_url").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let project_color = s.get("project_color").and_then(|v| v.as_str()).unwrap_or("#6644FF").to_string();

                    view! {
                        <div class="project-form">
                            <div class="form-field">
                                <label>"Project Name"</label>
                                <input type="text" prop:value=project_name/>
                            </div>
                            <div class="form-field">
                                <label>"Project URL"</label>
                                <input type="url" prop:value=project_url/>
                            </div>
                            <div class="form-field">
                                <label>"Project Color"</label>
                                <input type="color" prop:value=project_color/>
                            </div>
                        </div>
                    }
                }}
            </Show>
        </div>
    }
}

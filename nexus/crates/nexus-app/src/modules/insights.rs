use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use crate::api::client::ApiClient;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn InsightsPage() -> impl IntoView {
    let params = use_params_map();
    let dashboard_id = move || params.get().get("dashboard").map(|s| s.to_string());

    let dashboards = RwSignal::new(Vec::<Value>::new());
    let current_dashboard = RwSignal::new(Option::<Value>::None);
    let panels = RwSignal::new(Vec::<Value>::new());
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    // Fetch dashboards list
    Effect::new({
        let client = client.clone();
        move || {
            let client = client.clone();
            leptos::task::spawn_local(async move {
                if let Ok(resp) = client.get::<Value>("/dashboards?sort=name").await {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        dashboards.set(data.clone());
                    }
                }
            });
        }
    });

    // Fetch dashboard detail and panels
    Effect::new(move || {
        let id = dashboard_id();
        let client = client.clone();

        if let Some(id) = id {
            loading.set(true);
            leptos::task::spawn_local(async move {
                match client.get::<Value>(&format!("/dashboards/{}", id)).await {
                    Ok(resp) => {
                        if let Some(data) = resp.get("data") {
                            current_dashboard.set(Some(data.clone()));
                        }
                    }
                    Err(_) => current_dashboard.set(None),
                }

                if let Ok(resp) = client.get::<Value>(&format!("/panels?filter[dashboard][_eq]={}&sort=position_x,position_y", id)).await {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        panels.set(data.clone());
                    }
                }
                loading.set(false);
            });
        } else {
            current_dashboard.set(None);
            panels.set(vec![]);
            loading.set(false);
        }
    });

    view! {
        <div class="insights-page module-page">
            <div class="page-header">
                <h1>"Insights"</h1>
                <div class="page-header-actions">
                    <button class="btn btn-primary">
                        <Icon name="add"/>
                        " Create Dashboard"
                    </button>
                </div>
            </div>
            <div class="page-body insights-body">
                <nav class="insights-nav">
                    <For
                        each=move || dashboards.get()
                        key=|d| d.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                        children=|dashboard| {
                            let id = dashboard.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                            let name = dashboard.get("name").and_then(|v| v.as_str()).unwrap_or("Untitled").to_string();
                            let icon = dashboard.get("icon").and_then(|v| v.as_str()).unwrap_or("dashboard").to_string();
                            view! {
                                <A href=format!("/insights/{}", id) attr:class="insights-nav-item">
                                    <Icon name=icon/>
                                    <span>{name}</span>
                                </A>
                            }
                        }
                    />
                </nav>
                <div class="insights-content">
                    <Show
                        when=move || dashboard_id().is_some()
                        fallback=|| view! {
                            <EmptyState
                                icon="insights".to_string()
                                title="No Dashboard Selected"
                                description="Select or create a dashboard to get started.".to_string()
                            />
                        }
                    >
                        <Show when=move || loading.get()>
                            <Loading/>
                        </Show>
                        <Show when=move || !loading.get()>
                            <div class="panels-grid">
                                <For
                                    each=move || panels.get()
                                    key=|p| p.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                                    children=|panel| {
                                        let panel_type = panel.get("type")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("label")
                                            .to_string();
                                        let name = panel.get("name")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("")
                                            .to_string();
                                        let options = panel.get("options").cloned().unwrap_or(Value::Null);
                                        let color = panel.get("color")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("#6644FF")
                                            .to_string();
                                        let icon = panel.get("icon")
                                            .and_then(|v| v.as_str())
                                            .unwrap_or("insert_chart")
                                            .to_string();
                                        let width = panel.get("width")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(6);
                                        let height = panel.get("height")
                                            .and_then(|v| v.as_i64())
                                            .unwrap_or(6);

                                        let panel_body = match panel_type.as_str() {
                                            "metric" => {
                                                let collection = options.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                let func = options.get("function").and_then(|v| v.as_str()).unwrap_or("count").to_string();
                                                let field = options.get("field").and_then(|v| v.as_str()).unwrap_or("id").to_string();
                                                view! {
                                                    <div class="panel-metric">
                                                        <Icon name=icon.clone()/>
                                                        <span class="metric-label">{format!("{}({}.{})", func, collection, field)}</span>
                                                        <PanelDataFetcher collection=collection func=func field=field/>
                                                    </div>
                                                }.into_any()
                                            }
                                            "list" => {
                                                let collection = options.get("collection").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                view! {
                                                    <div class="panel-list">
                                                        <PanelListFetcher collection=collection/>
                                                    </div>
                                                }.into_any()
                                            }
                                            "label" => {
                                                let text = options.get("text").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                                view! {
                                                    <div class="panel-label" style=format!("color: {}", color)>
                                                        <p>{text}</p>
                                                    </div>
                                                }.into_any()
                                            }
                                            _ => {
                                                view! {
                                                    <div class="panel-generic">
                                                        <Icon name=icon.clone()/>
                                                        <span>{format!("{} panel", panel_type)}</span>
                                                    </div>
                                                }.into_any()
                                            }
                                        };

                                        view! {
                                            <div
                                                class="panel-card"
                                                style=format!("grid-column: span {}; grid-row: span {}", width, height)
                                            >
                                                <div class="panel-header">
                                                    <Icon name=icon/>
                                                    <span class="panel-name">{name}</span>
                                                    <span class="panel-type">{panel_type}</span>
                                                </div>
                                                <div class="panel-body">
                                                    {panel_body}
                                                </div>
                                            </div>
                                        }
                                    }
                                />
                            </div>
                        </Show>
                    </Show>
                </div>
            </div>
        </div>
    }
}

/// Fetches and displays a single metric value (count, sum, etc.)
#[component]
fn PanelDataFetcher(
    collection: String,
    func: String,
    field: String,
) -> impl IntoView {
    let result = RwSignal::new(String::from("--"));
    let client = ApiClient::new();

    Effect::new(move || {
        let client = client.clone();
        let collection = collection.clone();
        let func = func.clone();
        let field = field.clone();
        leptos::task::spawn_local(async move {
            let endpoint = format!("/items/{}?aggregate[{}]={}&limit=0", collection, func, field);
            if let Ok(resp) = client.get::<Value>(&endpoint).await {
                if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                    if let Some(first) = data.first() {
                        let key = format!("{}_{}", func, field);
                        if let Some(val) = first.get(&key).or(first.get(&func)) {
                            result.set(match val {
                                Value::Number(n) => n.to_string(),
                                Value::String(s) => s.clone(),
                                _ => val.to_string(),
                            });
                        }
                    }
                }
            }
        });
    });

    view! {
        <span class="metric-value">{move || result.get()}</span>
    }
}

/// Fetches and displays a list of items
#[component]
fn PanelListFetcher(collection: String) -> impl IntoView {
    let items = RwSignal::new(Vec::<Value>::new());
    let client = ApiClient::new();

    Effect::new(move || {
        let client = client.clone();
        let collection = collection.clone();
        leptos::task::spawn_local(async move {
            let endpoint = format!("/items/{}?limit=10", collection);
            if let Ok(resp) = client.get::<Value>(&endpoint).await {
                if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                    items.set(data.clone());
                }
            }
        });
    });

    view! {
        <ul class="panel-list-items">
            <For
                each=move || items.get()
                key=|item| {
                    item.get("id")
                        .map(|v| v.to_string())
                        .unwrap_or_default()
                }
                children=|item| {
                    let display = item.get("name")
                        .or(item.get("title"))
                        .or(item.get("id"))
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .unwrap_or_default();
                    view! { <li>{display}</li> }
                }
            />
        </ul>
    }
}

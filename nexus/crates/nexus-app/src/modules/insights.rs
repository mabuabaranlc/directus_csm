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
                                        view! {
                                            <div class="panel-card">
                                                <div class="panel-header">
                                                    <span class="panel-name">{name}</span>
                                                    <span class="panel-type">{panel_type}</span>
                                                </div>
                                                <div class="panel-body">
                                                    <p>"Panel content"</p>
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

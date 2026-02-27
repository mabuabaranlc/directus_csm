use leptos::prelude::*;
use crate::api::client::ApiClient;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use crate::components::pagination::Pagination;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn ActivityPage() -> impl IntoView {
    let activities = RwSignal::new(Vec::<Value>::new());
    let total = RwSignal::new(0usize);
    let page = RwSignal::new(1usize);
    let per_page = RwSignal::new(50usize);
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    Effect::new(move || {
        let p = page.get();
        let pp = per_page.get();
        let client = client.clone();

        loading.set(true);
        leptos::task::spawn_local(async move {
            let path = format!(
                "/activity?limit={}&offset={}&sort=-timestamp&meta=total_count",
                pp, (p - 1) * pp
            );

            match client.get::<Value>(&path).await {
                Ok(resp) => {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        activities.set(data.clone());
                    }
                    if let Some(meta) = resp.get("meta") {
                        if let Some(tc) = meta.get("total_count").and_then(|v| v.as_u64()) {
                            total.set(tc as usize);
                        }
                    }
                }
                Err(_) => activities.set(vec![]),
            }
            loading.set(false);
        });
    });

    let action_icon = |action: &str| -> &str {
        match action {
            "create" => "add",
            "update" => "edit",
            "delete" => "delete",
            "login" => "login",
            "comment" => "chat_bubble",
            _ => "info",
        }
    };

    view! {
        <div class="activity-page module-page">
            <div class="page-header">
                <h1>"Activity Log"</h1>
            </div>
            <div class="page-body">
                <Show when=move || loading.get()>
                    <Loading/>
                </Show>
                <Show when=move || !loading.get() && activities.get().is_empty()>
                    <EmptyState
                        icon="history".to_string()
                        title="No Activity"
                        description="No activity has been recorded yet.".to_string()
                    />
                </Show>
                <Show when=move || !loading.get() && !activities.get().is_empty()>
                    <div class="activity-list">
                        <For
                            each=move || activities.get()
                            key=|a| a.get("id").and_then(|v| v.as_u64()).unwrap_or(0)
                            children=move |activity| {
                                let action = activity.get("action")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("unknown")
                                    .to_string();
                                let collection = activity.get("collection")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let timestamp = activity.get("timestamp")
                                    .and_then(|v| v.as_str())
                                    .unwrap_or("")
                                    .to_string();
                                let icon = action_icon(&action).to_string();
                                view! {
                                    <div class="activity-item">
                                        <div class="activity-icon">
                                            <Icon name=icon/>
                                        </div>
                                        <div class="activity-info">
                                            <span class="activity-action">{action}</span>
                                            <span class="activity-collection">{collection}</span>
                                        </div>
                                        <span class="activity-time">{timestamp}</span>
                                    </div>
                                }
                            }
                        />
                    </div>
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

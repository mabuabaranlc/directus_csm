pub mod user_detail;

use leptos::prelude::*;
use leptos_router::components::A;
use crate::api::client::ApiClient;
use crate::components::loading::Loading;
use crate::components::empty_state::EmptyState;
use crate::components::pagination::Pagination;
use crate::components::search_input::SearchInput;
use crate::components::avatar::Avatar;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn UsersPage() -> impl IntoView {
    let users = RwSignal::new(Vec::<Value>::new());
    let total = RwSignal::new(0usize);
    let page = RwSignal::new(1usize);
    let per_page = RwSignal::new(25usize);
    let search = RwSignal::new(String::new());
    let loading = RwSignal::new(true);
    let client = ApiClient::new();

    Effect::new(move || {
        let p = page.get();
        let pp = per_page.get();
        let s = search.get();
        let client = client.clone();

        loading.set(true);
        leptos::task::spawn_local(async move {
            let mut path = format!("/users?limit={}&offset={}&meta=total_count", pp, (p - 1) * pp);
            if !s.is_empty() {
                path.push_str(&format!("&search={}", urlencoding::encode(&s)));
            }

            match client.get::<Value>(&path).await {
                Ok(resp) => {
                    if let Some(data) = resp.get("data").and_then(|d| d.as_array()) {
                        users.set(data.clone());
                    }
                    if let Some(meta) = resp.get("meta") {
                        if let Some(tc) = meta.get("total_count").and_then(|v| v.as_u64()) {
                            total.set(tc as usize);
                        }
                    }
                }
                Err(_) => users.set(vec![]),
            }
            loading.set(false);
        });
    });

    view! {
        <div class="users-page module-page">
            <div class="page-header">
                <h1>"User Directory"</h1>
                <div class="page-header-actions">
                    <SearchInput value=search/>
                    <A href="/users/+" attr:class="btn btn-primary">
                        <Icon name="person_add"/>
                        " Create User"
                    </A>
                </div>
            </div>
            <div class="page-body">
                <Show when=move || loading.get()>
                    <Loading/>
                </Show>
                <Show when=move || !loading.get() && users.get().is_empty()>
                    <EmptyState
                        icon="people".to_string()
                        title="No Users"
                        description="No users found.".to_string()
                    />
                </Show>
                <Show when=move || !loading.get() && !users.get().is_empty()>
                    <div class="users-list">
                        <For
                            each=move || users.get()
                            key=|u| u.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string()
                            children=|user| {
                                let id = user.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let first = user.get("first_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let last = user.get("last_name").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let email = user.get("email").and_then(|v| v.as_str()).unwrap_or("").to_string();
                                let display = if !first.is_empty() || !last.is_empty() {
                                    format!("{} {}", first, last).trim().to_string()
                                } else {
                                    email.clone()
                                };
                                view! {
                                    <A href=format!("/users/{}", id) attr:class="user-row">
                                        <Avatar name=display.clone()/>
                                        <div class="user-info">
                                            <span class="user-name">{display}</span>
                                            <span class="user-email">{email}</span>
                                        </div>
                                    </A>
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

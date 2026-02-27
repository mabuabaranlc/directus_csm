use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_navigate};
use crate::api::client::ApiClient;
use crate::components::button::Button;
use crate::components::loading::Loading;
use crate::components::avatar::Avatar;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn UserDetailPage() -> impl IntoView {
    let params = use_params_map();
    let user_id = move || params.get().get("id").unwrap_or_default();

    let user = RwSignal::new(Option::<Value>::None);
    let edits = RwSignal::new(serde_json::Map::new());
    let loading = RwSignal::new(true);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);
    let client = ApiClient::new();

    let is_new = move || user_id() == "+";

    Effect::new({
        let client = client.clone();
        move || {
            let id = user_id();
            let client = client.clone();

            if id == "+" {
                user.set(Some(Value::Object(serde_json::Map::new())));
                loading.set(false);
                return;
            }

            loading.set(true);
            leptos::task::spawn_local(async move {
                match client.get::<Value>(&format!("/users/{}", id)).await {
                    Ok(resp) => {
                        if let Some(data) = resp.get("data") {
                            user.set(Some(data.clone()));
                        }
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        }
    });

    let get_field = move |key: &str| -> String {
        let e = edits.get();
        if let Some(v) = e.get(key) {
            return v.as_str().unwrap_or("").to_string();
        }
        user.get()
            .and_then(|u| u.get(key).and_then(|v| v.as_str()).map(String::from))
            .unwrap_or_default()
    };

    let set_field = move |key: String, val: String| {
        edits.update(|m| { m.insert(key, Value::String(val)); });
    };

    let save = move |_: web_sys::MouseEvent| {
        let id = user_id();
        let changes = Value::Object(edits.get());
        let client = client.clone();
        let navigate = use_navigate();

        saving.set(true);
        error.set(None);

        leptos::task::spawn_local(async move {
            let result = if id == "+" {
                client.post::<Value, Value>("/users", &changes).await
            } else {
                client.patch::<Value, Value>(&format!("/users/{}", id), &changes).await
            };

            match result {
                Ok(resp) => {
                    if id == "+" {
                        if let Some(data) = resp.get("data") {
                            if let Some(new_id) = data.get("id").and_then(|v| v.as_str()) {
                                navigate(&format!("/users/{}", new_id), Default::default());
                            }
                        }
                    } else {
                        user.set(resp.get("data").cloned());
                        edits.set(serde_json::Map::new());
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let display_name = move || {
        let first = get_field("first_name");
        let last = get_field("last_name");
        let name = format!("{} {}", first, last).trim().to_string();
        if name.is_empty() { get_field("email") } else { name }
    };

    let has_edits = move || !edits.get().is_empty();

    let save_cb = Callback::new(save);

    view! {
        <div class="user-detail module-page">
            <div class="page-header">
                <div class="page-header-left">
                    <a href="/users" class="back-btn"><Icon name="arrow_back"/></a>
                    <h1>{move || if is_new() { "Create User".to_string() } else { display_name() }}</h1>
                </div>
                <div class="page-header-actions">
                    <Button kind="primary" on_click=save_cb disabled=Signal::derive(move || !has_edits()) loading=saving>
                        <Icon name="check"/>
                        " Save"
                    </Button>
                </div>
            </div>
            <div class="page-body">
                <Show when=move || error.get().is_some()>
                    <div class="error-banner">{move || error.get().unwrap_or_default()}</div>
                </Show>
                <Show when=move || loading.get()>
                    <Loading/>
                </Show>
                <Show when=move || !loading.get()>
                    <div class="user-form">
                        <div class="user-avatar-section">
                            <Avatar name=display_name() size="large"/>
                        </div>
                        <div class="form-grid">
                            <div class="form-field">
                                <label>"First Name"</label>
                                <input
                                    type="text"
                                    prop:value=move || get_field("first_name")
                                    on:input=move |ev| set_field("first_name".into(), event_target_value(&ev))
                                />
                            </div>
                            <div class="form-field">
                                <label>"Last Name"</label>
                                <input
                                    type="text"
                                    prop:value=move || get_field("last_name")
                                    on:input=move |ev| set_field("last_name".into(), event_target_value(&ev))
                                />
                            </div>
                            <div class="form-field">
                                <label>"Email"</label>
                                <input
                                    type="email"
                                    prop:value=move || get_field("email")
                                    on:input=move |ev| set_field("email".into(), event_target_value(&ev))
                                />
                            </div>
                            <div class="form-field">
                                <label>"Title"</label>
                                <input
                                    type="text"
                                    prop:value=move || get_field("title")
                                    on:input=move |ev| set_field("title".into(), event_target_value(&ev))
                                />
                            </div>
                            <div class="form-field">
                                <label>"Description"</label>
                                <textarea
                                    prop:value=move || get_field("description")
                                    on:input=move |ev| set_field("description".into(), event_target_value(&ev))
                                ></textarea>
                            </div>
                            <Show when=move || is_new()>
                                <div class="form-field">
                                    <label>"Password"</label>
                                    <input
                                        type="password"
                                        prop:value=move || get_field("password")
                                        on:input=move |ev| set_field("password".into(), event_target_value(&ev))
                                    />
                                </div>
                            </Show>
                        </div>
                    </div>
                </Show>
            </div>
        </div>
    }
}

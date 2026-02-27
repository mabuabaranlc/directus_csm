use leptos::prelude::*;
use leptos_router::hooks::{use_params_map, use_navigate};
use crate::api::client::ApiClient;
use crate::stores::fields::FieldsStore;
use crate::stores::collections::CollectionsStore;
use crate::components::button::Button;
use crate::components::loading::Loading;
use crate::components::icon::Icon;
use serde_json::Value;

#[component]
pub fn ItemDetailPage() -> impl IntoView {
    let params = use_params_map();
    let collection = move || params.get().get("collection").unwrap_or_default();
    let pk = move || params.get().get("pk").unwrap_or_default();

    let item = RwSignal::new(Option::<Value>::None);
    let edits = RwSignal::new(serde_json::Map::new());
    let loading = RwSignal::new(true);
    let saving = RwSignal::new(false);
    let error = RwSignal::new(Option::<String>::None);

    let fields_store = expect_context::<FieldsStore>();
    let collections_store = expect_context::<CollectionsStore>();
    let client = ApiClient::new();

    let is_new = move || pk() == "+";

    // Fetch item
    Effect::new({
        let client = client.clone();
        move || {
            let coll = collection();
            let primary = pk();
            let client = client.clone();

            if primary == "+" {
                item.set(Some(Value::Object(serde_json::Map::new())));
                loading.set(false);
                return;
            }

            loading.set(true);
            leptos::task::spawn_local(async move {
                let path = format!("/items/{}/{}", coll, primary);
                match client.get::<Value>(&path).await {
                    Ok(resp) => {
                        if let Some(data) = resp.get("data") {
                            item.set(Some(data.clone()));
                        }
                    }
                    Err(e) => error.set(Some(e)),
                }
                loading.set(false);
            });
        }
    });

    let client_for_delete = client.clone();

    let save = move |_: web_sys::MouseEvent| {
        let coll = collection();
        let primary = pk();
        let changes = Value::Object(edits.get());
        let client = client.clone();
        let navigate = use_navigate();

        saving.set(true);
        error.set(None);

        leptos::task::spawn_local(async move {
            let result = if primary == "+" {
                client.post::<Value, Value>(&format!("/items/{}", coll), &changes).await
            } else {
                client.patch::<Value, Value>(&format!("/items/{}/{}", coll, primary), &changes).await
            };

            match result {
                Ok(resp) => {
                    if primary == "+" {
                        if let Some(data) = resp.get("data") {
                            if let Some(id) = data.get("id").and_then(|v| v.as_str()) {
                                navigate(&format!("/content/{}/{}", coll, id), Default::default());
                            }
                        }
                    } else {
                        item.set(resp.get("data").cloned());
                        edits.set(serde_json::Map::new());
                    }
                }
                Err(e) => error.set(Some(e)),
            }
            saving.set(false);
        });
    };

    let delete_item = move |_: web_sys::MouseEvent| {
        let coll = collection();
        let primary = pk();
        let client = client_for_delete.clone();
        let navigate = use_navigate();

        leptos::task::spawn_local(async move {
            if client.delete(&format!("/items/{}/{}", coll, primary)).await.is_ok() {
                navigate(&format!("/content/{}", coll), Default::default());
            }
        });
    };

    let visible_fields = move || {
        fields_store.visible_for_collection(&collection())
    };

    let collection_name = move || {
        collections_store.get_collection(&collection())
            .map(|c| c.collection.clone())
            .unwrap_or_else(|| collection())
    };

    let has_edits = move || !edits.get().is_empty();

    let save_cb = Callback::new(save);
    let delete_cb = Callback::new(delete_item);

    view! {
        <div class="item-detail module-page">
            <div class="page-header">
                <div class="page-header-left">
                    <a href=move || format!("/content/{}", collection()) class="back-btn">
                        <Icon name="arrow_back"/>
                    </a>
                    <h1>{move || if is_new() {
                        format!("Create {}", collection_name())
                    } else {
                        format!("{}: {}", collection_name(), pk())
                    }}</h1>
                </div>
                <div class="page-header-actions">
                    <Show when=move || !is_new()>
                        <Button kind="danger" on_click=delete_cb>
                            <Icon name="delete"/>
                        </Button>
                    </Show>
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
                {move || {
                    if loading.get() || item.get().is_none() {
                        return view! { <div></div> }.into_any();
                    }
                    let fields = visible_fields();
                    let form_fields = fields.into_iter().map(|field| {
                        let field_name = field.field.clone();
                        let field_label = field.display_name();
                        let field_type = field.field_type.as_deref().unwrap_or("string").to_string();
                        let fname = field_name.clone();
                        let fname2 = field_name.clone();
                        let input_type = if field_type == "integer" || field_type == "float" || field_type == "decimal" {
                            "number"
                        } else if field_type == "boolean" {
                            "checkbox"
                        } else {
                            "text"
                        };
                        view! {
                            <div class="form-field">
                                <label>{field_label}</label>
                                <input
                                    type=input_type
                                    prop:value=move || {
                                        let e = edits.get();
                                        if let Some(v) = e.get(&fname) {
                                            return match v {
                                                Value::String(s) => s.clone(),
                                                Value::Null => String::new(),
                                                other => other.to_string(),
                                            };
                                        }
                                        item.get()
                                            .and_then(|i| i.get(&fname).cloned())
                                            .map(|v| match v {
                                                Value::String(s) => s,
                                                Value::Null => String::new(),
                                                other => other.to_string(),
                                            })
                                            .unwrap_or_default()
                                    }
                                    on:input=move |ev| {
                                        let val = event_target_value(&ev);
                                        edits.update(|m| {
                                            m.insert(fname2.clone(), Value::String(val));
                                        });
                                    }
                                />
                            </div>
                        }
                    }).collect::<Vec<_>>();

                    view! { <div class="item-form">{form_fields}</div> }.into_any()
                }}
            </div>
        </div>
    }
}

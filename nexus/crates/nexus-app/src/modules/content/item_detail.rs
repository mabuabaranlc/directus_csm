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
                        let interface = field.interface.as_deref().unwrap_or("input").to_string();
                        let readonly = field.readonly;
                        let width_class = field.width_class().to_string();
                        // Create a signal for the current field value
                        let field_value = {
                            let fname = field_name.clone();
                            RwSignal::new(
                                edits.get().get(&fname).cloned().unwrap_or_else(|| {
                                    item.get()
                                        .and_then(|i| i.get(&fname).cloned())
                                        .unwrap_or(Value::Null)
                                })
                            )
                        };

                        // Callback to update edits when value changes
                        let fname_cb = field_name.clone();
                        let on_change = Callback::new(move |val: Value| {
                            edits.update(|m| {
                                m.insert(fname_cb.clone(), val);
                            });
                        });

                        let interface_view = match interface.as_str() {
                            "boolean" | "toggle" => {
                                view! {
                                    <crate::interfaces::boolean::BooleanInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "input-multiline" | "textarea" => {
                                view! {
                                    <crate::interfaces::input_multiline::InputMultilineInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "datetime" => {
                                let dt_type = if field_type == "date" {
                                    "date"
                                } else if field_type == "time" {
                                    "time"
                                } else {
                                    "datetime-local"
                                };
                                view! {
                                    <crate::interfaces::datetime::DatetimeInterface
                                        value=field_value
                                        input_type=dt_type.to_string()
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "select-dropdown" => {
                                view! {
                                    <crate::interfaces::select_dropdown::SelectDropdownInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "input-code" => {
                                view! {
                                    <crate::interfaces::input_code::InputCodeInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "input-rich-text-html" => {
                                view! {
                                    <crate::interfaces::input_rich_text::InputRichTextInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "slider" => {
                                view! {
                                    <crate::interfaces::slider::SliderInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "tags" => {
                                view! {
                                    <crate::interfaces::tags::TagsInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            "select-color" => {
                                view! {
                                    <crate::interfaces::color::ColorInterface
                                        value=field_value
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                            _ => {
                                // Default: use the standard input interface
                                let input_type = if field_type == "integer" || field_type == "float" || field_type == "decimal" {
                                    "number"
                                } else {
                                    "text"
                                };
                                view! {
                                    <crate::interfaces::input::InputInterface
                                        value=field_value
                                        input_type=input_type.to_string()
                                        disabled=readonly
                                        on_change=on_change
                                    />
                                }.into_any()
                            }
                        };

                        view! {
                            <div class=format!("form-field {}", width_class)>
                                <label>{field_label}</label>
                                {interface_view}
                                {field.note.as_ref().map(|note| view! {
                                    <p class="field-note">{note.clone()}</p>
                                })}
                            </div>
                        }
                    }).collect::<Vec<_>>();

                    view! { <div class="item-form">{form_fields}</div> }.into_any()
                }}
            </div>
        </div>
    }
}

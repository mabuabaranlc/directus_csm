use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn MapInterface(
    value: RwSignal<Value>,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    // Map interface stub — full implementation would use a JS map library via wasm-bindgen
    let display_value = move || match value.get() {
        Value::Object(m) => {
            let lat = m.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lng = m.get("lng").and_then(|v| v.as_f64()).unwrap_or(0.0);
            format!("{:.6}, {:.6}", lat, lng)
        }
        Value::String(s) => s,
        _ => String::new(),
    };

    view! {
        <div class="interface-map">
            <div class="map-placeholder">
                <p>"Map view"</p>
            </div>
            <input
                type="text"
                placeholder="lat, lng"
                disabled=disabled.unwrap_or(false)
                prop:value=display_value
                on:input=move |ev| {
                    let val = Value::String(event_target_value(&ev));
                    value.set(val.clone());
                    on_change.run(val);
                }
            />
        </div>
    }
}

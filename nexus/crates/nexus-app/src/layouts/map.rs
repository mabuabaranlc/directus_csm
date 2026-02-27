use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn MapLayout(
    items: Signal<Vec<Value>>,
    #[prop(into)] geometry_field: String,
) -> impl IntoView {
    // Map layout stub — full implementation would use a JS map library via wasm-bindgen
    view! {
        <div class="layout-map">
            <div class="map-container">
                <p class="map-placeholder">"Map View"</p>
                <p>{move || format!("{} items with geometry field '{}'", items.get().len(), geometry_field)}</p>
            </div>
        </div>
    }
}

use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn TimeSeriesPanel(
    data: Vec<Value>,
    #[prop(optional, into)] label: Option<String>,
) -> impl IntoView {
    // Time series stub — full implementation would use a charting library via JS interop
    view! {
        <div class="panel-time-series">
            {label.map(|l| view! { <h3>{l}</h3> })}
            <div class="chart-placeholder">
                <p>{format!("{} data points", data.len())}</p>
                <p>"Chart visualization"</p>
            </div>
        </div>
    }
}

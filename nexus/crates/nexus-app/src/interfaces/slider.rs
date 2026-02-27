use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn SliderInterface(
    value: RwSignal<Value>,
    #[prop(optional, default = 0.0)] min: f64,
    #[prop(optional, default = 100.0)] max: f64,
    #[prop(optional, default = 1.0)] step: f64,
    #[prop(optional)] disabled: Option<bool>,
    on_change: Callback<Value>,
) -> impl IntoView {
    let num_value = move || match value.get() {
        Value::Number(n) => n.as_f64().unwrap_or(min),
        _ => min,
    };

    view! {
        <div class="interface-slider">
            <input
                type="range"
                min=min.to_string()
                max=max.to_string()
                step=step.to_string()
                disabled=disabled.unwrap_or(false)
                prop:value=move || num_value().to_string()
                on:input=move |ev| {
                    let str_val = event_target_value(&ev);
                    let num: f64 = str_val.parse().unwrap_or(min);
                    let val = serde_json::Number::from_f64(num)
                        .map(Value::Number)
                        .unwrap_or(Value::Null);
                    value.set(val.clone());
                    on_change.run(val);
                }
            />
            <span class="slider-value">{move || format!("{}", num_value())}</span>
        </div>
    }
}

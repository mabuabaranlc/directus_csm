use leptos::prelude::*;
use serde_json::Value;

#[component]
pub fn RatingDisplay(
    value: Value,
    #[prop(optional, default = 5)] max: usize,
) -> impl IntoView {
    let rating = match &value {
        Value::Number(n) => n.as_f64().unwrap_or(0.0),
        _ => 0.0,
    };

    let stars: Vec<bool> = (1..=max).map(|i| i as f64 <= rating).collect();

    view! {
        <span class="display-rating">
            {stars.into_iter().map(|filled| {
                if filled {
                    view! { <span class="star filled">"\u{2605}"</span> }
                } else {
                    view! { <span class="star">{"\u{2606}"}</span> }
                }
            }).collect::<Vec<_>>()}
        </span>
    }
}

use leptos::prelude::*;

#[component]
pub fn Loading(
    #[prop(optional, into)] text: Option<String>,
) -> impl IntoView {
    view! {
        <div class="loading">
            <div class="loading-spinner"></div>
            {text.map(|t| view! { <p class="loading-text">{t}</p> })}
        </div>
    }
}

#[component]
pub fn LoadingOverlay(
    #[prop(optional, into)] text: Option<String>,
) -> impl IntoView {
    view! {
        <div class="loading-overlay">
            <Loading text=text.unwrap_or_default()/>
        </div>
    }
}

use leptos::prelude::*;

#[component]
pub fn Badge(
    label: String,
    #[prop(optional)] color: Option<String>,
) -> impl IntoView {
    let style = color.map(|c| format!("background-color: {}", c)).unwrap_or_default();
    view! { <span class="badge" style=style>{label}</span> }
}

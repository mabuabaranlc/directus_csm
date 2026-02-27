use leptos::prelude::*;

#[component]
pub fn Icon(
    name: impl Into<String>,
    #[prop(optional)] class: &'static str,
) -> impl IntoView {
    let name = name.into();
    let class_str = format!("icon icon-{} {}", name, class);

    // Uses Material Symbols / custom icon font
    view! {
        <span class=class_str aria-hidden="true">{name.clone()}</span>
    }
}

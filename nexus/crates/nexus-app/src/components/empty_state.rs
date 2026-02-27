use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn EmptyState(
    #[prop(optional)] icon: Option<String>,
    #[prop(into)] title: String,
    #[prop(optional)] description: Option<String>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    view! {
        <div class="empty-state">
            {icon.map(|name| view! {
                <div class="empty-state-icon"><Icon name=name/></div>
            })}
            <h3 class="empty-state-title">{title}</h3>
            {description.map(|d| view! { <p class="empty-state-desc">{d}</p> })}
            {children.map(|c| view! { <div class="empty-state-actions">{c()}</div> })}
        </div>
    }
}

use leptos::prelude::*;
use crate::components::icon::Icon;

#[derive(Clone)]
pub struct ButtonLink {
    pub label: String,
    pub icon: Option<String>,
    pub url: String,
    pub new_tab: bool,
}

#[component]
pub fn ButtonLinksPanel(
    links: Vec<ButtonLink>,
) -> impl IntoView {
    view! {
        <div class="panel-button-links">
            <For
                each=move || links.clone()
                key=|link| link.url.clone()
                children=|link| {
                    let target = if link.new_tab { "_blank" } else { "_self" };
                    let rel = if link.new_tab { "noopener noreferrer" } else { "" };
                    view! {
                        <a
                            class="button-link"
                            href=link.url.clone()
                            target=target
                            rel=rel
                        >
                            {link.icon.map(|icon| view! { <Icon name=icon/> })}
                            <span>{link.label}</span>
                        </a>
                    }
                }
            />
        </div>
    }
}

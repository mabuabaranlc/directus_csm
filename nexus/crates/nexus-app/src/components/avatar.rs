use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn Avatar(
    #[prop(optional)] src: Option<String>,
    #[prop(optional, into)] name: Option<String>,
    #[prop(optional, into)] size: String,
) -> impl IntoView {
    let size_class = if size.is_empty() { "avatar-md".to_string() } else { size };
    let initials = name
        .as_ref()
        .map(|n| {
            n.split_whitespace()
                .filter_map(|w| w.chars().next())
                .take(2)
                .collect::<String>()
                .to_uppercase()
        })
        .unwrap_or_default();

    view! {
        <div class=format!("avatar {}", size_class)>
            {match src {
                Some(url) => view! { <img class="avatar-img" src=url alt=""/> }.into_any(),
                None if !initials.is_empty() => view! { <span class="avatar-initials">{initials}</span> }.into_any(),
                _ => view! { <Icon name="user"/> }.into_any(),
            }}
        </div>
    }
}

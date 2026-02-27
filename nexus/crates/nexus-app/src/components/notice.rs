use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn Notice(
    #[prop(optional, into, default = "info".into())] kind: String,
    #[prop(optional, into)] icon: Option<String>,
    children: Children,
) -> impl IntoView {
    let icon_name = icon.unwrap_or_else(|| match kind.as_str() {
        "warning" => "warning".into(),
        "danger" => "error".into(),
        "success" => "check_circle".into(),
        _ => "info".into(),
    });

    view! {
        <div class=format!("notice notice-{kind}")>
            <div class="notice-icon">
                <Icon name=icon_name/>
            </div>
            <div class="notice-content">
                {children()}
            </div>
        </div>
    }
}

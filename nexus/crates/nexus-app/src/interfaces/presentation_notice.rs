use leptos::prelude::*;
use crate::components::notice::Notice;

#[component]
pub fn PresentationNoticeInterface(
    #[prop(optional, into)] text: Option<String>,
    #[prop(optional, into, default = "info".into())] kind: String,
    #[prop(optional, into)] icon: Option<String>,
) -> impl IntoView {
    view! {
        <div class="interface-notice">
            <Notice kind=kind icon=icon.unwrap_or_default()>
                {text.unwrap_or_else(|| "Notice".into())}
            </Notice>
        </div>
    }
}

use leptos::prelude::*;

#[component]
pub fn LabelPanel(
    #[prop(into)] text: String,
    #[prop(optional, into)] color: Option<String>,
) -> impl IntoView {
    view! {
        <div class="panel-label" style=color.map(|c| format!("color: {c}")).unwrap_or_default()>
            <h2>{text}</h2>
        </div>
    }
}

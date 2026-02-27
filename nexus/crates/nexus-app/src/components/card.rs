use leptos::prelude::*;

#[component]
pub fn Card(
    #[prop(optional)] title: Option<String>,
    #[prop(optional)] class: &'static str,
    children: Children,
) -> impl IntoView {
    view! {
        <div class=format!("card {}", class)>
            {title.map(|t| view! { <div class="card-header"><h3 class="card-title">{t}</h3></div> })}
            <div class="card-body">
                {children()}
            </div>
        </div>
    }
}

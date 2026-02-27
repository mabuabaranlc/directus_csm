use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn Dialog(
    #[prop(optional)] title: Option<String>,
    open: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    let content = children();
    view! {
        <div class="dialog-overlay" style=move || if open.get() { "display: flex" } else { "display: none" }
            on:click=move |_| open.set(false)>
            <div class="dialog" on:click=|ev| ev.stop_propagation()>
                <div class="dialog-header">
                    {title.map(|t| view! { <h2 class="dialog-title">{t}</h2> })}
                    <button class="dialog-close" on:click=move |_| open.set(false)>
                        <Icon name="x"/>
                    </button>
                </div>
                <div class="dialog-body">
                    {content}
                </div>
            </div>
        </div>
    }
}

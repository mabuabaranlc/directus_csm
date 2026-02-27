use leptos::prelude::*;
use crate::components::icon::Icon;

#[component]
pub fn Drawer(
    #[prop(optional)] title: Option<String>,
    open: RwSignal<bool>,
    children: Children,
) -> impl IntoView {
    let content = children();
    view! {
        <div class="drawer-overlay" style=move || if open.get() { "display: flex" } else { "display: none" }
            on:click=move |_| open.set(false)>
            <div class="drawer" on:click=|ev| ev.stop_propagation()>
                <div class="drawer-header">
                    {title.map(|t| view! { <h2 class="drawer-title">{t}</h2> })}
                    <button class="drawer-close" on:click=move |_| open.set(false)>
                        <Icon name="x"/>
                    </button>
                </div>
                <div class="drawer-body">
                    {content}
                </div>
            </div>
        </div>
    }
}

use leptos::prelude::*;

#[component]
pub fn Tabs(
    tabs: Vec<(String, String)>,
    active: RwSignal<String>,
    children: Children,
) -> impl IntoView {
    view! {
        <div class="tabs">
            <div class="tabs-header">
                <For
                    each=move || tabs.clone()
                    key=|(key, _)| key.clone()
                    children=move |tab| {
                        let key = tab.0.clone();
                        let label = tab.1.clone();
                        let key_click = key.clone();
                        view! {
                            <button
                                class="tab-btn"
                                class:active=move || active.get() == key
                                on:click=move |_| active.set(key_click.clone())
                            >
                                {label}
                            </button>
                        }
                    }
                />
            </div>
            <div class="tabs-content">
                {children()}
            </div>
        </div>
    }
}

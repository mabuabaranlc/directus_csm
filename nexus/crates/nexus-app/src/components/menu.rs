use leptos::prelude::*;
use crate::components::icon::Icon;

#[derive(Clone)]
pub struct MenuItem {
    pub label: String,
    pub icon: Option<String>,
    pub disabled: bool,
}

impl MenuItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            icon: None,
            disabled: false,
        }
    }

    pub fn with_icon(mut self, icon: impl Into<String>) -> Self {
        self.icon = Some(icon.into());
        self
    }
}

#[component]
pub fn Menu(
    items: Vec<MenuItem>,
    on_select: Callback<usize>,
    #[prop(optional)] open: Option<RwSignal<bool>>,
) -> impl IntoView {
    let is_open = open.unwrap_or_else(|| RwSignal::new(false));

    let menu_items = items.into_iter().enumerate().map(|(idx, menu_item)| {
        let icon = menu_item.icon.clone();
        view! {
            <button
                class="menu-item"
                disabled=menu_item.disabled
                on:click=move |_| {
                    on_select.run(idx);
                    is_open.set(false);
                }
            >
                {icon.map(|i| view! { <Icon name=i/> })}
                <span>{menu_item.label}</span>
            </button>
        }
    }).collect::<Vec<_>>();

    view! {
        <div class="menu-wrapper" style=move || if is_open.get() { "display: block" } else { "display: none" }>
            <div class="menu-backdrop" on:click=move |_| is_open.set(false)></div>
            <div class="menu">
                {menu_items}
            </div>
        </div>
    }
}

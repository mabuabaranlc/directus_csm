use leptos::prelude::*;
use crate::components::icon::Icon;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub enum ButtonKind {
    #[default]
    Primary,
    Secondary,
    Warning,
    Danger,
    Info,
}

impl ButtonKind {
    pub fn class(&self) -> &'static str {
        match self {
            Self::Primary => "btn-primary",
            Self::Secondary => "btn-secondary",
            Self::Warning => "btn-warning",
            Self::Danger => "btn-danger",
            Self::Info => "btn-info",
        }
    }
}

impl From<&str> for ButtonKind {
    fn from(s: &str) -> Self {
        match s {
            "primary" => Self::Primary,
            "secondary" => Self::Secondary,
            "warning" => Self::Warning,
            "danger" => Self::Danger,
            "info" => Self::Info,
            _ => Self::Primary,
        }
    }
}

impl From<String> for ButtonKind {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

#[component]
pub fn Button(
    #[prop(optional)] label: Option<String>,
    #[prop(optional)] icon: Option<String>,
    #[prop(optional, into)] kind: ButtonKind,
    #[prop(optional, into)] disabled: Signal<bool>,
    #[prop(optional, into)] loading: Signal<bool>,
    #[prop(optional)] full_width: bool,
    #[prop(optional)] outlined: bool,
    #[prop(optional)] rounded: bool,
    #[prop(optional)] small: bool,
    #[prop(optional)] on_click: Option<Callback<web_sys::MouseEvent>>,
    #[prop(optional)] children: Option<Children>,
) -> impl IntoView {
    let class = move || {
        let mut classes = vec!["btn", kind.class()];
        if full_width { classes.push("btn-full"); }
        if outlined { classes.push("btn-outlined"); }
        if rounded { classes.push("btn-rounded"); }
        if small { classes.push("btn-small"); }
        if loading.get() { classes.push("btn-loading"); }
        classes.join(" ")
    };

    view! {
        <button
            class=class
            disabled=move || disabled.get() || loading.get()
            on:click=move |ev| {
                if let Some(ref cb) = on_click {
                    cb.run(ev);
                }
            }
        >
            <Show when=move || loading.get()>
                <span class="btn-spinner"/>
            </Show>
            {icon.map(|i| view! { <Icon name=i/> })}
            {label.map(|l| view! { <span class="btn-label">{l}</span> })}
            {children.map(|c| c())}
        </button>
    }
}

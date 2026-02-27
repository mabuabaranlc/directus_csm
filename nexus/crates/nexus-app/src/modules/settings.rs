use leptos::prelude::*;
use leptos_router::hooks::use_params_map;
use leptos_router::components::A;
use crate::components::icon::Icon;

#[derive(Clone)]
struct SettingsModule {
    id: &'static str,
    icon: &'static str,
    label: &'static str,
    _description: &'static str,
}

const SETTINGS_MODULES: &[SettingsModule] = &[
    SettingsModule { id: "data-model", icon: "database", label: "Data Model", _description: "Configure collections and fields" },
    SettingsModule { id: "roles", icon: "supervisor_account", label: "Access Control", _description: "Manage roles and permissions" },
    SettingsModule { id: "presets", icon: "bookmark", label: "Presets & Bookmarks", _description: "Manage default collection presets" },
    SettingsModule { id: "translations", icon: "translate", label: "Translation Strings", _description: "Manage custom translations" },
    SettingsModule { id: "flows", icon: "bolt", label: "Flows", _description: "Automation workflows and triggers" },
    SettingsModule { id: "webhooks", icon: "webhook", label: "Webhooks", _description: "HTTP callbacks for events" },
    SettingsModule { id: "extensions", icon: "extension", label: "Extensions", _description: "Manage installed extensions" },
    SettingsModule { id: "project", icon: "public", label: "Project Settings", _description: "General project configuration" },
];

#[component]
pub fn SettingsPage() -> impl IntoView {
    let params = use_params_map();
    let sub_route = move || params.get().get("rest").unwrap_or_default();

    view! {
        <div class="settings-page module-page">
            <div class="page-header">
                <h1>"Settings"</h1>
            </div>
            <div class="page-body settings-body">
                <nav class="settings-nav">
                    {SETTINGS_MODULES.iter().map(|m| {
                        let id = m.id;
                        let icon = m.icon.to_string();
                        let label = m.label;
                        view! {
                            <A href=format!("/settings/{}", id) attr:class="settings-nav-item">
                                <Icon name=icon/>
                                <span>{label}</span>
                            </A>
                        }
                    }).collect::<Vec<_>>()}
                </nav>
                <div class="settings-content">
                    <Show
                        when=move || !sub_route().is_empty()
                        fallback=|| view! {
                            <div class="settings-welcome">
                                <Icon name="settings"/>
                                <h2>"Settings"</h2>
                                <p>"Select a module from the sidebar to get started."</p>
                            </div>
                        }
                    >
                        <div class="settings-module">
                            <p>{move || format!("Settings module: {}", sub_route())}</p>
                        </div>
                    </Show>
                </div>
            </div>
        </div>
    }
}

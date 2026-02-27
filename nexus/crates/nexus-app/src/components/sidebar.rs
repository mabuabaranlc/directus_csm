use leptos::prelude::*;
use crate::components::icon::Icon;
use crate::stores::collections::CollectionsStore;
use crate::stores::settings::SettingsStore;
use crate::stores::user::UserStore;

#[component]
pub fn Sidebar(open: RwSignal<bool>) -> impl IntoView {
    let collections = expect_context::<CollectionsStore>();
    let settings = expect_context::<SettingsStore>();
    let user_store = expect_context::<UserStore>();

    let project_name = move || {
        settings.settings.get().display_name().to_string()
    };

    let project_color = move || {
        settings.settings.get().theme_color().to_string()
    };

    let visible_collections = move || collections.visible_collections();

    let user_name = move || {
        user_store
            .current
            .get()
            .map(|u| u.display_name())
            .unwrap_or_else(|| "User".to_string())
    };

    view! {
        <nav class="sidebar" class:open=move || open.get()>
            <div class="sidebar-header">
                <div class="project-logo" style=move || format!("background-color: {}", project_color())>
                    <span class="project-initial">{move || project_name().chars().next().unwrap_or('N').to_string()}</span>
                </div>
                <span class="project-name">{project_name}</span>
            </div>

            <div class="sidebar-modules">
                <SidebarLink href="/" icon="box" label="Content"/>
                <SidebarLink href="/users" icon="users" label="User Directory"/>
                <SidebarLink href="/files" icon="folder" label="File Library"/>
                <SidebarLink href="/insights" icon="bar-chart-2" label="Insights"/>
                <SidebarLink href="/settings" icon="settings" label="Settings"/>
            </div>

            <div class="sidebar-divider"/>

            <div class="sidebar-collections">
                <div class="sidebar-section-title">"Collections"</div>
                <For
                    each=visible_collections
                    key=|c| c.collection.clone()
                    children=move |col| {
                        let href = format!("/content/{}", col.collection);
                        let icon = col.display_icon().to_string();
                        let label = col.collection.clone();
                        view! {
                            <SidebarLink href=href icon=icon label=label/>
                        }
                    }
                />
            </div>

            <div class="sidebar-footer">
                <div class="sidebar-user">
                    <div class="user-avatar-small">
                        <Icon name="user"/>
                    </div>
                    <span class="user-name">{user_name}</span>
                </div>
                <SidebarLink href="/activity" icon="activity" label="Activity Log"/>
            </div>
        </nav>
    }
}

#[component]
fn SidebarLink(
    href: impl Into<String>,
    icon: impl Into<String>,
    label: impl Into<String>,
) -> impl IntoView {
    let href = href.into();
    let icon = icon.into();
    let label = label.into();

    view! {
        <a class="sidebar-link" href=href.clone()>
            <Icon name=icon/>
            <span class="sidebar-link-label">{label}</span>
        </a>
    }
}

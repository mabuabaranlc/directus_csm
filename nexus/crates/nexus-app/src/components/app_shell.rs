use leptos::prelude::*;
use leptos_router::components::Outlet;
use crate::components::sidebar::Sidebar;
use crate::components::header::Header;
use crate::stores::auth::{AuthState, AuthStatus};
use crate::stores::collections::CollectionsStore;
use crate::stores::fields::FieldsStore;
use crate::stores::user::UserStore;
use crate::stores::settings::SettingsStore;
use crate::stores::notifications::NotificationsStore;

#[component]
pub fn AppShell() -> impl IntoView {
    let auth = expect_context::<AuthState>();

    // Provide stores for child components
    let user_store = UserStore::new();
    let collections_store = CollectionsStore::new();
    let fields_store = FieldsStore::new();
    let settings_store = SettingsStore::new();
    let notifications_store = NotificationsStore::new();

    provide_context(user_store.clone());
    provide_context(collections_store.clone());
    provide_context(fields_store.clone());
    provide_context(settings_store.clone());
    provide_context(notifications_store.clone());

    // Redirect to login if not authenticated
    let navigate = leptos_router::hooks::use_navigate();
    Effect::new(move |_| {
        if auth.status.get() == AuthStatus::Unauthenticated {
            navigate("/login", Default::default());
        }
    });

    // Fetch initial data when authenticated
    let client = auth.client.clone();
    let user_store_c = user_store.clone();
    let collections_store_c = collections_store.clone();
    let fields_store_c = fields_store.clone();
    let settings_store_c = settings_store.clone();
    let notifications_store_c = notifications_store.clone();

    Effect::new(move |_| {
        if auth.status.get() == AuthStatus::Authenticated {
            let client = client.clone();
            let us = user_store_c.clone();
            let cs = collections_store_c.clone();
            let fs = fields_store_c.clone();
            let ss = settings_store_c.clone();
            let ns = notifications_store_c.clone();
            leptos::task::spawn_local(async move {
                us.fetch_current(&client).await;
                cs.fetch(&client).await;
                fs.fetch(&client).await;
                ss.fetch(&client).await;
                ns.fetch(&client).await;
            });
        }
    });

    let sidebar_open = RwSignal::new(true);

    view! {
        <div class="app-shell" class:sidebar-collapsed=move || !sidebar_open.get()>
            <Sidebar open=sidebar_open/>
            <div class="main-area">
                <Header sidebar_open=sidebar_open/>
                <main class="content-area">
                    <Outlet/>
                </main>
            </div>
        </div>
    }
}

pub mod api;
pub mod components;
pub mod displays;
pub mod interfaces;
pub mod layouts;
pub mod modules;
pub mod panels;
pub mod stores;
pub mod router;
pub mod i18n;

use leptos::prelude::*;
use leptos_meta::*;
use crate::stores::auth::AuthState;

/// Root application component
#[component]
pub fn App() -> impl IntoView {
    provide_meta_context();

    // Provide global stores
    let auth = AuthState::new();
    provide_context(auth);

    view! {
        <Title text="Nexus CMS"/>
        <Meta charset="utf-8"/>
        <Meta name="viewport" content="width=device-width, initial-scale=1"/>
        <Stylesheet href="/assets/styles.css"/>
        <router::AppRouter/>
    }
}

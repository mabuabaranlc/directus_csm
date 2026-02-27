use leptos::prelude::*;
use leptos_router::components::*;
use leptos_router::path;

use crate::components::app_shell::AppShell;
use crate::modules::login::LoginPage;
use crate::modules::content::collection_list::CollectionListPage;
use crate::modules::content::item_detail::ItemDetailPage;
use crate::modules::content::content_overview::ContentOverview;
use crate::modules::files::FilesPage;
use crate::modules::users::UsersPage;
use crate::modules::users::user_detail::UserDetailPage;
use crate::modules::settings::SettingsPage;
use crate::modules::activity::ActivityPage;
use crate::modules::insights::InsightsPage;

#[component]
pub fn AppRouter() -> impl IntoView {
    view! {
        <Router>
            <Routes fallback=|| view! { <div class="not-found"><h1>"404 — Not Found"</h1></div> }>
                <Route path=path!("/login") view=LoginPage/>
                <ParentRoute path=path!("/") view=AppShell>
                    <Route path=path!("/") view=ContentOverview/>
                    <Route path=path!("/content/:collection") view=CollectionListPage/>
                    <Route path=path!("/content/:collection/:pk") view=ItemDetailPage/>
                    <Route path=path!("/files") view=FilesPage/>
                    <Route path=path!("/files/:id") view=FilesPage/>
                    <Route path=path!("/users") view=UsersPage/>
                    <Route path=path!("/users/:id") view=UserDetailPage/>
                    <Route path=path!("/insights") view=InsightsPage/>
                    <Route path=path!("/insights/:dashboard") view=InsightsPage/>
                    <Route path=path!("/activity") view=ActivityPage/>
                    <Route path=path!("/settings") view=SettingsPage/>
                    <Route path=path!("/settings/*rest") view=SettingsPage/>
                </ParentRoute>
            </Routes>
        </Router>
    }
}

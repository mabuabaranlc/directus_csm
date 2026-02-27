use leptos::prelude::*;
use crate::components::icon::Icon;
use crate::stores::notifications::NotificationsStore;

#[component]
pub fn Header(sidebar_open: RwSignal<bool>) -> impl IntoView {
    let notifications = expect_context::<NotificationsStore>();

    let unread = move || notifications.unread_count.get();

    view! {
        <header class="app-header">
            <button class="header-btn toggle-sidebar" on:click=move |_| sidebar_open.update(|v| *v = !*v)>
                <Icon name="menu"/>
            </button>

            <div class="header-spacer"></div>

            <div class="header-actions">
                <a class="header-btn" href="/activity" title="Activity">
                    <Icon name="activity"/>
                </a>
                <button class="header-btn notification-btn" title="Notifications">
                    <Icon name="bell"/>
                    {move || {
                        let count = unread();
                        if count > 0 {
                            Some(view! { <span class="notification-badge">{count}</span> })
                        } else {
                            None
                        }
                    }}
                </button>
            </div>
        </header>
    }
}

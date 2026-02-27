use leptos::prelude::*;
use leptos_router::hooks::use_navigate;
use crate::stores::auth::{AuthState, AuthStatus};
use crate::components::button::Button;
use crate::components::icon::Icon;

#[component]
pub fn LoginPage() -> impl IntoView {
    let auth = expect_context::<AuthState>();
    let email = RwSignal::new(String::new());
    let password = RwSignal::new(String::new());
    let error = RwSignal::new(Option::<String>::None);
    let loading = RwSignal::new(false);
    let navigate = use_navigate();

    // Redirect if already authenticated
    Effect::new(move || {
        if matches!(auth.status.get(), AuthStatus::Authenticated) {
            navigate("/", Default::default());
        }
    });

    let on_submit = move |ev: web_sys::SubmitEvent| {
        ev.prevent_default();
        let email_val = email.get();
        let password_val = password.get();
        let navigate = use_navigate();
        let auth = auth.clone();

        loading.set(true);
        error.set(None);

        leptos::task::spawn_local(async move {
            match auth.login(&email_val, &password_val).await {
                Ok(_) => {
                    navigate("/", Default::default());
                }
                Err(e) => {
                    error.set(Some(e));
                    loading.set(false);
                }
            }
        });
    };

    view! {
        <div class="login-page">
            <div class="login-card">
                <div class="login-logo">
                    <Icon name="hub"/>
                    <h1>"Nexus"</h1>
                </div>
                <form class="login-form" on:submit=on_submit>
                    <Show when=move || error.get().is_some()>
                        <div class="login-error">
                            {move || error.get().unwrap_or_default()}
                        </div>
                    </Show>
                    <div class="field">
                        <label for="email">"Email"</label>
                        <input
                            id="email"
                            type="email"
                            placeholder="admin@example.com"
                            required=true
                            prop:value=move || email.get()
                            on:input=move |ev| email.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label for="password">"Password"</label>
                        <input
                            id="password"
                            type="password"
                            placeholder="Password"
                            required=true
                            prop:value=move || password.get()
                            on:input=move |ev| password.set(event_target_value(&ev))
                        />
                    </div>
                    <Button kind="primary" loading=loading>
                        "Sign In"
                    </Button>
                </form>
            </div>
        </div>
    }
}

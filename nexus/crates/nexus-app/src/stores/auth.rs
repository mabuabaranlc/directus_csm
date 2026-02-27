use leptos::prelude::*;
use crate::api::ApiClient;

#[derive(Debug, Clone, PartialEq)]
pub enum AuthStatus {
    Unknown,
    Authenticated,
    Unauthenticated,
}

#[derive(Debug, Clone)]
pub struct AuthState {
    pub status: RwSignal<AuthStatus>,
    pub loading: RwSignal<bool>,
    pub error: RwSignal<Option<String>>,
    pub client: ApiClient,
}

impl AuthState {
    pub fn new() -> Self {
        let has_token = gloo_storage::LocalStorage::get::<String>("nexus_access_token").is_ok();
        Self {
            status: RwSignal::new(if has_token {
                AuthStatus::Authenticated
            } else {
                AuthStatus::Unauthenticated
            }),
            loading: RwSignal::new(false),
            error: RwSignal::new(None),
            client: ApiClient::from_current_origin(),
        }
    }

    pub fn is_authenticated(&self) -> bool {
        self.status.get() == AuthStatus::Authenticated
    }

    pub async fn login(&self, email: &str, password: &str) -> Result<(), String> {
        self.loading.set(true);
        self.error.set(None);

        let result = match self.client.login(email, password).await {
            Ok(_tokens) => {
                self.status.set(AuthStatus::Authenticated);
                Ok(())
            }
            Err(e) => {
                self.status.set(AuthStatus::Unauthenticated);
                self.error.set(Some(e.clone()));
                Err(e)
            }
        };

        self.loading.set(false);
        result
    }

    pub async fn logout(&self) {
        let _ = self.client.logout().await;
        self.status.set(AuthStatus::Unauthenticated);
    }

    pub async fn refresh(&self) -> bool {
        match self.client.refresh_token().await {
            Ok(_) => {
                self.status.set(AuthStatus::Authenticated);
                true
            }
            Err(_) => {
                self.status.set(AuthStatus::Unauthenticated);
                false
            }
        }
    }
}

use gloo_storage::Storage;

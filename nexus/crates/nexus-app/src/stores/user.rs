use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::api::{ApiClient, client::ApiResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct CurrentUser {
    pub id: String,
    #[serde(default)]
    pub first_name: Option<String>,
    #[serde(default)]
    pub last_name: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub avatar: Option<String>,
    #[serde(default)]
    pub language: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub role: Option<String>,
    #[serde(default)]
    pub appearance: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub last_page: Option<String>,
    #[serde(default)]
    pub theme_dark: Option<String>,
    #[serde(default)]
    pub theme_light: Option<String>,
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

impl CurrentUser {
    pub fn display_name(&self) -> String {
        match (&self.first_name, &self.last_name) {
            (Some(first), Some(last)) => format!("{} {}", first, last),
            (Some(first), None) => first.clone(),
            (None, Some(last)) => last.clone(),
            _ => self.email.clone().unwrap_or_else(|| "User".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct UserStore {
    pub current: RwSignal<Option<CurrentUser>>,
    pub loading: RwSignal<bool>,
}

impl UserStore {
    pub fn new() -> Self {
        Self {
            current: RwSignal::new(None),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch_current(&self, client: &ApiClient) {
        self.loading.set(true);
        match client.get::<ApiResponse<CurrentUser>>("/users/me").await {
            Ok(resp) => {
                self.current.set(Some(resp.data));
            }
            Err(_) => {
                self.current.set(None);
            }
        }
        self.loading.set(false);
    }
}

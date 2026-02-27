use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::api::{ApiClient, client::ApiResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub id: Option<i64>,
    #[serde(default)]
    pub project_name: Option<String>,
    #[serde(default)]
    pub project_url: Option<String>,
    #[serde(default)]
    pub project_color: Option<String>,
    #[serde(default)]
    pub project_logo: Option<String>,
    #[serde(default)]
    pub public_foreground: Option<String>,
    #[serde(default)]
    pub public_background: Option<String>,
    #[serde(default)]
    pub public_note: Option<String>,
    #[serde(default)]
    pub default_language: Option<String>,
    #[serde(default)]
    pub default_appearance: Option<String>,
    #[serde(default)]
    pub custom_css: Option<String>,
    #[serde(default)]
    pub module_bar: Option<Value>,
    #[serde(default)]
    pub project_descriptor: Option<String>,
    #[serde(default)]
    pub mapbox_key: Option<String>,
}

impl Settings {
    pub fn display_name(&self) -> &str {
        self.project_name.as_deref().unwrap_or("Nexus")
    }

    pub fn theme_color(&self) -> &str {
        self.project_color.as_deref().unwrap_or("#6644FF")
    }
}

#[derive(Debug, Clone)]
pub struct SettingsStore {
    pub settings: RwSignal<Settings>,
    pub loading: RwSignal<bool>,
}

impl SettingsStore {
    pub fn new() -> Self {
        Self {
            settings: RwSignal::new(Settings::default()),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);
        match client.get::<ApiResponse<Settings>>("/settings").await {
            Ok(resp) => self.settings.set(resp.data),
            Err(_) => {}
        }
        self.loading.set(false);
    }
}

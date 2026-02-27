use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::api::ApiClient;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ServerInfo {
    #[serde(default)]
    pub project: Option<ProjectInfo>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectInfo {
    #[serde(default)]
    pub project_name: Option<String>,
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
    #[serde(flatten)]
    pub extra: std::collections::HashMap<String, Value>,
}

#[derive(Debug, Clone)]
pub struct ServerStore {
    pub info: RwSignal<Option<ServerInfo>>,
    pub loading: RwSignal<bool>,
}

impl ServerStore {
    pub fn new() -> Self {
        Self {
            info: RwSignal::new(None),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);

        #[derive(Deserialize)]
        struct InfoResp {
            data: ServerInfo,
        }

        match client.get::<InfoResp>("/server/info").await {
            Ok(resp) => self.info.set(Some(resp.data)),
            Err(_) => {}
        }
        self.loading.set(false);
    }
}

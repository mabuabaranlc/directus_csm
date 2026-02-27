use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::api::{ApiClient, client::ApiListResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Collection {
    pub collection: String,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub display_template: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub singleton: bool,
    #[serde(default)]
    pub sort: Option<i32>,
    #[serde(default)]
    pub accountability: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub collapse: Option<String>,
    #[serde(default)]
    pub translations: Option<Value>,
    #[serde(default)]
    pub versioning: bool,
    #[serde(default)]
    pub sort_field: Option<String>,
    #[serde(default)]
    pub archive_field: Option<String>,
    #[serde(default)]
    pub archive_value: Option<String>,
    #[serde(default)]
    pub unarchive_value: Option<String>,
}

impl Collection {
    pub fn is_system(&self) -> bool {
        self.collection.starts_with("nexus_")
    }

    pub fn display_icon(&self) -> &str {
        self.icon.as_deref().unwrap_or("database")
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CollectionsStore {
    pub collections: RwSignal<Vec<Collection>>,
    pub loading: RwSignal<bool>,
}

impl CollectionsStore {
    pub fn new() -> Self {
        Self {
            collections: RwSignal::new(Vec::new()),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);
        match client
            .get::<ApiListResponse<Collection>>("/collections")
            .await
        {
            Ok(resp) => {
                self.collections.set(resp.data);
            }
            Err(_) => {
                self.collections.set(Vec::new());
            }
        }
        self.loading.set(false);
    }

    pub fn visible_collections(&self) -> Vec<Collection> {
        self.collections
            .get()
            .into_iter()
            .filter(|c| !c.hidden && !c.is_system())
            .collect()
    }

    pub fn get_collection(&self, name: &str) -> Option<Collection> {
        self.collections
            .get()
            .into_iter()
            .find(|c| c.collection == name)
    }
}

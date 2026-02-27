use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::api::{ApiClient, client::ApiListResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Relation {
    #[serde(default)]
    pub id: Option<i64>,
    pub many_collection: String,
    pub many_field: String,
    #[serde(default)]
    pub one_collection: Option<String>,
    #[serde(default)]
    pub one_field: Option<String>,
    #[serde(default)]
    pub one_collection_field: Option<String>,
    #[serde(default)]
    pub one_allowed_collections: Option<String>,
    #[serde(default)]
    pub junction_field: Option<String>,
    #[serde(default)]
    pub sort_field: Option<String>,
    #[serde(default)]
    pub one_deselect_action: Option<String>,
}

#[derive(Debug, Clone)]
pub struct RelationsStore {
    pub relations: RwSignal<Vec<Relation>>,
    pub loading: RwSignal<bool>,
}

impl RelationsStore {
    pub fn new() -> Self {
        Self {
            relations: RwSignal::new(Vec::new()),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);
        match client.get::<ApiListResponse<Relation>>("/relations").await {
            Ok(resp) => self.relations.set(resp.data),
            Err(_) => self.relations.set(Vec::new()),
        }
        self.loading.set(false);
    }

    pub fn for_collection(&self, collection: &str) -> Vec<Relation> {
        self.relations
            .get()
            .into_iter()
            .filter(|r| {
                r.many_collection == collection
                    || r.one_collection.as_deref() == Some(collection)
            })
            .collect()
    }
}

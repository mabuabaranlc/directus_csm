use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use crate::api::{ApiClient, client::ApiListResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Field {
    #[serde(default)]
    pub id: Option<i64>,
    pub collection: String,
    pub field: String,
    #[serde(default)]
    pub special: Option<String>,
    #[serde(rename = "type", default)]
    pub field_type: Option<String>,
    #[serde(default)]
    pub interface: Option<String>,
    #[serde(default)]
    pub options: Option<Value>,
    #[serde(default)]
    pub display: Option<String>,
    #[serde(default)]
    pub display_options: Option<Value>,
    #[serde(default)]
    pub readonly: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub sort: Option<i32>,
    #[serde(default)]
    pub width: Option<String>,
    #[serde(default)]
    pub translations: Option<Value>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub required: bool,
    #[serde(default)]
    pub group: Option<String>,
    #[serde(default)]
    pub validation: Option<Value>,
    #[serde(default)]
    pub validation_message: Option<String>,
    #[serde(default)]
    pub conditions: Option<Value>,
}

impl Field {
    pub fn display_name(&self) -> String {
        self.field
            .replace('_', " ")
            .split_whitespace()
            .map(|w| {
                let mut c = w.chars();
                match c.next() {
                    None => String::new(),
                    Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub fn width_class(&self) -> &str {
        match self.width.as_deref() {
            Some("half") => "half",
            Some("half-left") => "half-left",
            Some("half-right") => "half-right",
            _ => "full",
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct FieldsStore {
    pub fields: RwSignal<Vec<Field>>,
    pub loading: RwSignal<bool>,
}

impl FieldsStore {
    pub fn new() -> Self {
        Self {
            fields: RwSignal::new(Vec::new()),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);
        match client.get::<ApiListResponse<Field>>("/fields").await {
            Ok(resp) => {
                self.fields.set(resp.data);
            }
            Err(_) => {
                self.fields.set(Vec::new());
            }
        }
        self.loading.set(false);
    }

    pub fn for_collection(&self, collection: &str) -> Vec<Field> {
        self.fields
            .get()
            .into_iter()
            .filter(|f| f.collection == collection && !f.hidden)
            .collect()
    }

    pub fn visible_for_collection(&self, collection: &str) -> Vec<Field> {
        let mut fields = self.for_collection(collection);
        fields.sort_by(|a, b| a.sort.unwrap_or(999).cmp(&b.sort.unwrap_or(999)));
        fields
    }
}

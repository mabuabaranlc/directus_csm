use leptos::prelude::*;
use serde::{Deserialize, Serialize};
use crate::api::{ApiClient, client::ApiListResponse};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Notification {
    pub id: i64,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub subject: Option<String>,
    #[serde(default)]
    pub message: Option<String>,
    #[serde(default)]
    pub timestamp: Option<String>,
    #[serde(default)]
    pub sender: Option<String>,
    #[serde(default)]
    pub collection: Option<String>,
    #[serde(default)]
    pub item: Option<String>,
}

#[derive(Debug, Clone)]
pub struct NotificationsStore {
    pub notifications: RwSignal<Vec<Notification>>,
    pub unread_count: RwSignal<usize>,
    pub loading: RwSignal<bool>,
}

impl NotificationsStore {
    pub fn new() -> Self {
        Self {
            notifications: RwSignal::new(Vec::new()),
            unread_count: RwSignal::new(0),
            loading: RwSignal::new(false),
        }
    }

    pub async fn fetch(&self, client: &ApiClient) {
        self.loading.set(true);
        match client
            .get::<ApiListResponse<Notification>>("/notifications?sort=-timestamp&limit=25")
            .await
        {
            Ok(resp) => {
                let unread = resp.data.iter().filter(|n| n.status.as_deref() == Some("inbox")).count();
                self.unread_count.set(unread);
                self.notifications.set(resp.data);
            }
            Err(_) => {}
        }
        self.loading.set(false);
    }
}

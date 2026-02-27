use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct File {
    pub id: String,
    pub storage: String,
    pub filename_disk: String,
    pub filename_download: String,
    pub title: Option<String>,
    #[serde(rename = "type")]
    pub mime_type: Option<String>,
    pub folder: Option<String>,
    pub created_on: String,
    pub uploaded_by: Option<String>,
    pub uploaded_on: Option<String>,
    pub modified_by: Option<String>,
    pub modified_on: String,
    pub charset: Option<String>,
    pub filesize: i64,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration: Option<i32>,
    pub embed: Option<String>,
    pub description: Option<String>,
    pub location: Option<String>,
    pub tags: Option<String>,
    pub metadata: Option<Value>,
    pub focal_point_x: Option<f64>,
    pub focal_point_y: Option<f64>,
    pub tus_id: Option<String>,
    pub tus_data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Folder {
    pub id: String,
    pub name: String,
    pub parent: Option<String>,
}

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    pub id: i64,
    pub project_name: String,
    pub project_descriptor: Option<String>,
    pub project_url: Option<String>,
    pub default_language: Option<String>,
    pub project_color: Option<String>,
    pub project_logo: Option<String>,
    pub public_foreground: Option<String>,
    pub public_background: Option<Value>,
    pub public_favicon: Option<String>,
    pub public_note: Option<String>,
    pub auth_login_attempts: i32,
    pub auth_password_policy: Option<String>,
    pub storage_asset_transform: String,
    pub storage_asset_presets: Option<Vec<AssetPreset>>,
    pub custom_css: Option<String>,
    pub storage_default_folder: Option<String>,
    pub mapbox_key: Option<String>,
    pub module_bar: Option<Value>,
    pub default_appearance: String,
    pub default_theme_light: Option<String>,
    pub default_theme_dark: Option<String>,
    pub collaborative_editing_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetPreset {
    pub key: Option<String>,
    pub fit: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub quality: Option<i32>,
    #[serde(rename = "withoutEnlargement")]
    pub without_enlargement: Option<bool>,
    pub format: Option<String>,
    pub transforms: Option<Vec<Value>>,
}

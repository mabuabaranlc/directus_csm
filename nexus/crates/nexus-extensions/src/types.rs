use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Extension types supported by Nexus
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionType {
    /// Server-side hook (filter/action events)
    Hook,
    /// Custom API endpoint
    Endpoint,
    /// Custom data display component (frontend)
    Display,
    /// Custom field interface component (frontend)
    Interface,
    /// Custom layout component (frontend)
    Layout,
    /// Custom module (frontend page)
    Module,
    /// Custom panel (dashboard widget)
    Panel,
    /// Custom flow operation
    Operation,
    /// Bundle containing multiple extensions
    Bundle,
}

/// Extension manifest — describes an extension's metadata and configuration
/// Mirrors the directus extension manifest format
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionManifest {
    pub name: String,
    #[serde(rename = "type")]
    pub extension_type: ExtensionType,
    pub version: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub icon: String,
    /// The entry point file (e.g., "index.js", "extension.wasm")
    pub entrypoint: Option<String>,
    /// Language/runtime hint
    pub language: Option<ExtensionLanguage>,
    /// Extension-specific options
    #[serde(default)]
    pub options: Option<Value>,
    /// API path for endpoint extensions
    pub path: Option<String>,
    /// Whether this extension is enabled
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Child extensions (for bundles)
    #[serde(default)]
    pub entries: Vec<ExtensionManifest>,
}

fn default_true() -> bool {
    true
}

/// Supported extension languages
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ExtensionLanguage {
    /// JavaScript (executed via Bun)
    Javascript,
    /// TypeScript (compiled and executed via Bun)
    Typescript,
    /// Rust (compiled to WASM)
    Rust,
    /// Go (compiled to WASM)
    Go,
    /// Python (compiled to WASM via appropriate toolchain)
    Python,
    /// Java (compiled to WASM via TeaVM or similar)
    Java,
}

/// Extension API context passed to hook/endpoint extensions
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExtensionContext {
    pub event: String,
    pub collection: Option<String>,
    pub payload: Value,
    pub accountability: Option<Value>,
}

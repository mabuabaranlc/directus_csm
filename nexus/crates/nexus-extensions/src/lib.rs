use serde_json::Value;
use std::path::PathBuf;
use std::sync::Arc;
use thiserror::Error;

pub mod types;
pub mod loader;
pub mod wasm_runtime;
pub mod bun_runtime;
pub mod sandbox;

#[derive(Debug, Error)]
pub enum ExtensionError {
    #[error("Extension not found: {0}")]
    NotFound(String),
    #[error("Failed to load extension: {0}")]
    LoadFailed(String),
    #[error("Extension execution failed: {0}")]
    ExecutionFailed(String),
    #[error("Invalid extension manifest: {0}")]
    InvalidManifest(String),
    #[error("Sandbox violation: {0}")]
    SandboxViolation(String),
    #[error("Runtime error: {0}")]
    RuntimeError(String),
}

/// Extension manager — loads, validates, and manages all extensions
/// Supports hybrid WASM (Rust/Go/Python/Java) + Bun (JS/TS)
pub struct ExtensionManager {
    extensions: dashmap::DashMap<String, Arc<LoadedExtension>>,
    wasm_runtime: wasm_runtime::WasmRuntime,
    bun_runtime: bun_runtime::BunRuntime,
    extensions_path: PathBuf,
}

impl ExtensionManager {
    pub fn new(extensions_path: PathBuf) -> Self {
        Self {
            extensions: dashmap::DashMap::new(),
            wasm_runtime: wasm_runtime::WasmRuntime::new(),
            bun_runtime: bun_runtime::BunRuntime::new(),
            extensions_path,
        }
    }

    /// Load all extensions from the extensions directory
    pub async fn load_all(&self) -> Result<Vec<String>, ExtensionError> {
        let loaded = loader::discover_extensions(&self.extensions_path).await?;
        let mut names = Vec::new();

        for ext in loaded {
            names.push(ext.manifest.name.clone());
            self.extensions
                .insert(ext.manifest.name.clone(), Arc::new(ext));
        }

        tracing::info!(count = names.len(), "Extensions loaded");
        Ok(names)
    }

    /// Get a loaded extension by name
    pub fn get(&self, name: &str) -> Option<Arc<LoadedExtension>> {
        self.extensions.get(name).map(|e| e.value().clone())
    }

    /// List all loaded extensions
    pub fn list(&self) -> Vec<types::ExtensionManifest> {
        self.extensions
            .iter()
            .map(|e| e.value().manifest.clone())
            .collect()
    }

    /// Execute a hook extension
    pub async fn execute_hook(
        &self,
        name: &str,
        event: &str,
        payload: Value,
    ) -> Result<Value, ExtensionError> {
        let ext = self
            .get(name)
            .ok_or_else(|| ExtensionError::NotFound(name.to_string()))?;

        match &ext.runtime {
            RuntimeType::Wasm => {
                self.wasm_runtime
                    .execute(&ext, event, payload)
                    .await
            }
            RuntimeType::Bun => {
                self.bun_runtime
                    .execute(&ext, event, payload)
                    .await
            }
        }
    }

    /// Get API extensions (endpoints) for route registration
    pub fn get_api_extensions(&self) -> Vec<Arc<LoadedExtension>> {
        self.extensions
            .iter()
            .filter(|e| e.value().manifest.extension_type == types::ExtensionType::Endpoint)
            .map(|e| e.value().clone())
            .collect()
    }
}

/// A loaded extension ready for execution
pub struct LoadedExtension {
    pub manifest: types::ExtensionManifest,
    pub runtime: RuntimeType,
    pub path: PathBuf,
    pub source: Vec<u8>,
}

/// Runtime type for executing the extension
#[derive(Debug, Clone, PartialEq)]
pub enum RuntimeType {
    /// WASM module (compiled from Rust, Go, Python, Java, etc.)
    Wasm,
    /// JavaScript/TypeScript executed via Bun
    Bun,
}

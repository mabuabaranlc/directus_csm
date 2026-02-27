use crate::{ExtensionError, LoadedExtension, RuntimeType};
use crate::types::{ExtensionLanguage, ExtensionManifest};
use std::path::{Path, PathBuf};

/// Discover and load extensions from the extensions directory
pub async fn discover_extensions(
    extensions_dir: &Path,
) -> Result<Vec<LoadedExtension>, ExtensionError> {
    let mut extensions = Vec::new();

    if !extensions_dir.exists() {
        tracing::info!(
            path = %extensions_dir.display(),
            "Extensions directory does not exist, skipping"
        );
        return Ok(extensions);
    }

    let mut entries = tokio::fs::read_dir(extensions_dir)
        .await
        .map_err(|e| ExtensionError::LoadFailed(format!("Failed to read extensions dir: {}", e)))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| ExtensionError::LoadFailed(e.to_string()))?
    {
        let path = entry.path();

        if path.is_dir() {
            match load_extension(&path).await {
                Ok(ext) => {
                    tracing::info!(
                        name = %ext.manifest.name,
                        ext_type = ?ext.manifest.extension_type,
                        runtime = ?ext.runtime,
                        "Loaded extension"
                    );
                    extensions.push(ext);
                }
                Err(e) => {
                    tracing::warn!(
                        path = %path.display(),
                        error = %e,
                        "Failed to load extension, skipping"
                    );
                }
            }
        }
    }

    Ok(extensions)
}

/// Load a single extension from its directory
async fn load_extension(dir: &Path) -> Result<LoadedExtension, ExtensionError> {
    // Look for manifest file (package.json or nexus-extension.json)
    let manifest_path = dir.join("package.json");
    let alt_manifest_path = dir.join("nexus-extension.json");

    let manifest_content = if manifest_path.exists() {
        tokio::fs::read_to_string(&manifest_path)
            .await
            .map_err(|e| ExtensionError::LoadFailed(e.to_string()))?
    } else if alt_manifest_path.exists() {
        tokio::fs::read_to_string(&alt_manifest_path)
            .await
            .map_err(|e| ExtensionError::LoadFailed(e.to_string()))?
    } else {
        return Err(ExtensionError::InvalidManifest(
            "No package.json or nexus-extension.json found".to_string(),
        ));
    };

    let manifest: ExtensionManifest = serde_json::from_str(&manifest_content)
        .map_err(|e| ExtensionError::InvalidManifest(e.to_string()))?;

    // Determine runtime type
    let runtime = determine_runtime(&manifest);

    // Load the entry point file
    let entrypoint = manifest
        .entrypoint
        .as_deref()
        .unwrap_or_else(|| default_entrypoint(&runtime));

    let source_path = dir.join(entrypoint);
    let source = if source_path.exists() {
        tokio::fs::read(&source_path)
            .await
            .map_err(|e| ExtensionError::LoadFailed(e.to_string()))?
    } else {
        Vec::new()
    };

    Ok(LoadedExtension {
        manifest,
        runtime,
        path: dir.to_path_buf(),
        source,
    })
}

/// Determine the runtime type based on the extension's language/entrypoint
fn determine_runtime(manifest: &ExtensionManifest) -> RuntimeType {
    if let Some(lang) = &manifest.language {
        match lang {
            ExtensionLanguage::Rust
            | ExtensionLanguage::Go
            | ExtensionLanguage::Python
            | ExtensionLanguage::Java => RuntimeType::Wasm,
            ExtensionLanguage::Javascript | ExtensionLanguage::Typescript => RuntimeType::Bun,
        }
    } else if let Some(entrypoint) = &manifest.entrypoint {
        if entrypoint.ends_with(".wasm") {
            RuntimeType::Wasm
        } else {
            RuntimeType::Bun
        }
    } else {
        // Default to Bun for JS/TS compatibility
        RuntimeType::Bun
    }
}

fn default_entrypoint(runtime: &RuntimeType) -> &'static str {
    match runtime {
        RuntimeType::Wasm => "extension.wasm",
        RuntimeType::Bun => "dist/index.js",
    }
}

use crate::{ExtensionError, LoadedExtension};
use serde_json::Value;

/// WASM runtime for executing extensions compiled to WebAssembly
/// Uses wasmtime for sandboxed execution of Rust/Go/Python/Java extensions
pub struct WasmRuntime {
    engine: wasmtime::Engine,
}

impl WasmRuntime {
    pub fn new() -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        // Set resource limits for sandboxing
        config.consume_fuel(true);

        let engine = wasmtime::Engine::new(&config).expect("Failed to create WASM engine");

        Self { engine }
    }

    /// Execute a WASM extension
    pub async fn execute(
        &self,
        extension: &LoadedExtension,
        _event: &str,
        payload: Value,
    ) -> Result<Value, ExtensionError> {
        if extension.source.is_empty() {
            return Err(ExtensionError::LoadFailed(
                "Extension has no WASM source".to_string(),
            ));
        }

        // Compile the WASM module
        let module = wasmtime::Module::new(&self.engine, &extension.source)
            .map_err(|e| ExtensionError::LoadFailed(format!("WASM compilation failed: {}", e)))?;

        // Create a store with fuel limits
        let mut store = wasmtime::Store::new(&self.engine, ());
        store.set_fuel(1_000_000).map_err(|e| {
            ExtensionError::RuntimeError(format!("Failed to set fuel: {}", e))
        })?;

        // Create a linker with WASI-like imports
        let linker = wasmtime::Linker::new(&self.engine);

        // Instantiate the module
        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| {
                ExtensionError::ExecutionFailed(format!("WASM instantiation failed: {}", e))
            })?;

        // Call the extension's entry function
        // Convention: extensions export a function called "handle"
        // that takes a JSON string and returns a JSON string
        let handle = instance
            .get_typed_func::<(), ()>(&mut store, "handle")
            .map_err(|e| {
                ExtensionError::ExecutionFailed(format!(
                    "Extension missing 'handle' export: {}",
                    e
                ))
            })?;

        // TODO: Implement proper memory sharing for passing JSON data
        // For now, call the function and return the original payload
        handle
            .call(&mut store, ())
            .map_err(|e| {
                ExtensionError::ExecutionFailed(format!("WASM execution failed: {}", e))
            })?;

        Ok(payload)
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

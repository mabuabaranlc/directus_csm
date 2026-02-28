use crate::{ExtensionError, LoadedExtension};
use serde_json::Value;

/// WASM runtime for executing extensions compiled to WebAssembly.
/// Uses wasmtime for sandboxed execution of Rust/Go/Python/Java extensions.
///
/// Convention: extensions export:
/// - `alloc(size: i32) -> i32` — allocate bytes in WASM memory
/// - `dealloc(ptr: i32, size: i32)` — free allocated memory
/// - `handle(ptr: i32, len: i32) -> i64` — process JSON, returns (ptr << 32 | len)
///
/// The host writes the JSON payload into WASM memory via `alloc`, calls `handle`,
/// then reads the result JSON from the returned pointer.
pub struct WasmRuntime {
    engine: wasmtime::Engine,
}

impl WasmRuntime {
    pub fn new() -> Self {
        let mut config = wasmtime::Config::new();
        config.async_support(true);
        config.consume_fuel(true);

        let engine = wasmtime::Engine::new(&config).expect("Failed to create WASM engine");
        Self { engine }
    }

    /// Execute a WASM extension, passing JSON payload via shared memory.
    pub async fn execute(
        &self,
        extension: &LoadedExtension,
        event: &str,
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
        store.set_fuel(10_000_000).map_err(|e| {
            ExtensionError::RuntimeError(format!("Failed to set fuel: {}", e))
        })?;

        // Build the input JSON: { "event": "...", "payload": ... }
        let input = serde_json::json!({
            "event": event,
            "payload": payload,
        });
        let input_bytes = serde_json::to_vec(&input).map_err(|e| {
            ExtensionError::ExecutionFailed(format!("Failed to serialize input: {}", e))
        })?;

        let linker = wasmtime::Linker::new(&self.engine);

        let instance = linker
            .instantiate(&mut store, &module)
            .map_err(|e| {
                ExtensionError::ExecutionFailed(format!("WASM instantiation failed: {}", e))
            })?;

        // Get the WASM memory export
        let memory = instance
            .get_memory(&mut store, "memory")
            .ok_or_else(|| {
                ExtensionError::ExecutionFailed("Extension missing 'memory' export".to_string())
            })?;

        // Try the typed API: handle(ptr, len) -> i64
        let has_alloc = instance
            .get_typed_func::<i32, i32>(&mut store, "alloc")
            .is_ok();

        if has_alloc {
            // Full memory-sharing protocol
            let alloc_fn = instance
                .get_typed_func::<i32, i32>(&mut store, "alloc")
                .map_err(|e| {
                    ExtensionError::ExecutionFailed(format!("Missing alloc: {}", e))
                })?;

            let handle_fn = instance
                .get_typed_func::<(i32, i32), i64>(&mut store, "handle")
                .map_err(|e| {
                    ExtensionError::ExecutionFailed(format!("Missing handle(i32,i32)->i64: {}", e))
                })?;

            // Allocate memory in WASM for the input
            let input_len = input_bytes.len() as i32;
            let input_ptr = alloc_fn.call(&mut store, input_len).map_err(|e| {
                ExtensionError::ExecutionFailed(format!("alloc failed: {}", e))
            })?;

            // Write input bytes to WASM memory
            let mem_data = memory.data_mut(&mut store);
            let start = input_ptr as usize;
            let end = start + input_bytes.len();
            if end > mem_data.len() {
                return Err(ExtensionError::ExecutionFailed(
                    "WASM memory too small for input".to_string(),
                ));
            }
            mem_data[start..end].copy_from_slice(&input_bytes);

            // Call handle
            let result_packed = handle_fn
                .call(&mut store, (input_ptr, input_len))
                .map_err(|e| {
                    ExtensionError::ExecutionFailed(format!("WASM handle failed: {}", e))
                })?;

            // Unpack result: high 32 bits = ptr, low 32 bits = len
            let result_ptr = (result_packed >> 32) as usize;
            let result_len = (result_packed & 0xFFFF_FFFF) as usize;

            if result_len == 0 {
                // Extension returned empty — return original payload
                return Ok(payload);
            }

            // Read result from WASM memory
            let mem_data = memory.data(&store);
            let result_end = result_ptr + result_len;
            if result_end > mem_data.len() {
                return Err(ExtensionError::ExecutionFailed(
                    "WASM result pointer out of bounds".to_string(),
                ));
            }
            let result_bytes = &mem_data[result_ptr..result_end];

            // Parse JSON result
            let result: Value = serde_json::from_slice(result_bytes).map_err(|e| {
                ExtensionError::ExecutionFailed(format!("Failed to parse WASM output: {}", e))
            })?;

            // If the result has a "payload" key, extract it
            if let Some(inner) = result.get("payload") {
                Ok(inner.clone())
            } else {
                Ok(result)
            }
        } else {
            // Fallback: simple handle() -> () with no data passing
            // Try to get handle with no args
            if let Ok(simple_handle) =
                instance.get_typed_func::<(), ()>(&mut store, "handle")
            {
                simple_handle.call(&mut store, ()).map_err(|e| {
                    ExtensionError::ExecutionFailed(format!("WASM execution failed: {}", e))
                })?;
            }

            // Return original payload since we couldn't pass data
            Ok(payload)
        }
    }
}

impl Default for WasmRuntime {
    fn default() -> Self {
        Self::new()
    }
}

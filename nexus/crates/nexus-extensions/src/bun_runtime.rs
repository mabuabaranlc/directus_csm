use crate::{ExtensionError, LoadedExtension};
use serde_json::Value;

/// Bun runtime for executing JavaScript/TypeScript extensions
/// Uses Bun subprocess for sandboxed execution (NOT Node.js, NOT Deno)
pub struct BunRuntime {
    bun_path: String,
}

impl BunRuntime {
    pub fn new() -> Self {
        let bun_path = nexus_env::env_string_or("EXTENSIONS_BUN_PATH", "bun");
        Self { bun_path }
    }

    /// Execute a JS/TS extension via Bun subprocess
    pub async fn execute(
        &self,
        extension: &LoadedExtension,
        event: &str,
        payload: Value,
    ) -> Result<Value, ExtensionError> {
        let entrypoint = extension
            .manifest
            .entrypoint
            .as_deref()
            .unwrap_or("dist/index.js");

        let script_path = extension.path.join(entrypoint);

        if !script_path.exists() {
            return Err(ExtensionError::LoadFailed(format!(
                "Entrypoint not found: {}",
                script_path.display()
            )));
        }

        // Create the execution context as JSON
        let context = serde_json::json!({
            "event": event,
            "payload": payload,
            "extension": extension.manifest.name,
        });

        let context_json = serde_json::to_string(&context)
            .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))?;

        // Execute via Bun subprocess with the context passed via stdin
        // The wrapper script reads stdin, calls the extension, and writes result to stdout
        let wrapper_script = format!(
            r#"
            const ext = await import("{}");
            const context = JSON.parse(await Bun.stdin.text());
            const handler = ext.default || ext.handler || ext;
            let result;
            if (typeof handler === 'function') {{
                result = await handler(context);
            }} else if (typeof handler.handle === 'function') {{
                result = await handler.handle(context);
            }} else {{
                result = context.payload;
            }}
            console.log(JSON.stringify(result ?? context.payload));
            "#,
            script_path.display()
        );

        let output = tokio::process::Command::new(&self.bun_path)
            .arg("eval")
            .arg(&wrapper_script)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| {
                ExtensionError::RuntimeError(format!(
                    "Failed to start Bun runtime ({}): {}",
                    self.bun_path, e
                ))
            })?;

        let output = output
            .wait_with_output()
            .await
            .map_err(|e| ExtensionError::ExecutionFailed(e.to_string()))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(ExtensionError::ExecutionFailed(format!(
                "Bun execution failed: {}",
                stderr
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);

        // Parse the last line as JSON (the result)
        let result_line = stdout.lines().last().unwrap_or("null");
        let result: Value = serde_json::from_str(result_line).unwrap_or(payload);

        Ok(result)
    }
}

impl Default for BunRuntime {
    fn default() -> Self {
        Self::new()
    }
}

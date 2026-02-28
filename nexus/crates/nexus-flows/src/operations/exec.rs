use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};
use tokio::process::Command;

/// Exec operation — runs inline JavaScript/TypeScript code via Bun subprocess
/// Mirrors api/src/operations/exec/index.ts
pub struct ExecOperation;

#[async_trait]
impl FlowOperation for ExecOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let code = options
            .get("code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| FlowError::InvalidConfig("Missing 'code' option".to_string()))?;

        let timeout_ms = nexus_env::env_number_or("FLOWS_RUN_SCRIPT_TIMEOUT", 10000) as u64;

        // Wrap user code so it receives data and returns output via stdout
        let wrapper = format!(
            r#"const data = {};
const fn = (function() {{
    {}
}});
try {{
    const result = await fn(data);
    if (result !== undefined) {{
        console.log(JSON.stringify(result));
    }} else {{
        console.log(JSON.stringify(null));
    }}
}} catch (e) {{
    console.log(JSON.stringify({{ "__error": e.message || String(e) }}));
}}"#,
            serde_json::to_string(&data).unwrap_or_else(|_| "{}".to_string()),
            code
        );

        // Try bun first, fall back to node, then deno
        let runtime = nexus_env::env_string_or("FLOWS_EXEC_RUNTIME", "auto");

        let (cmd, args): (&str, Vec<&str>) = match runtime.as_str() {
            "bun" => ("bun", vec!["eval"]),
            "node" => ("node", vec!["-e"]),
            "deno" => ("deno", vec!["eval"]),
            _ => {
                // Auto-detect: try bun, then node, then deno
                if Command::new("bun").arg("--version").output().await.is_ok() {
                    ("bun", vec!["eval"])
                } else if Command::new("node").arg("--version").output().await.is_ok() {
                    ("node", vec!["-e"])
                } else if Command::new("deno").arg("--version").output().await.is_ok() {
                    ("deno", vec!["eval"])
                } else {
                    return Err(FlowError::OperationFailed(
                        "No JavaScript runtime found. Install bun, node, or deno.".to_string(),
                    ));
                }
            }
        };

        let output = tokio::time::timeout(
            std::time::Duration::from_millis(timeout_ms),
            Command::new(cmd)
                .args(&args)
                .arg(&wrapper)
                .output(),
        )
        .await
        .map_err(|_| FlowError::OperationFailed(format!(
            "Script execution timed out after {}ms", timeout_ms
        )))?
        .map_err(|e| FlowError::OperationFailed(format!("Failed to execute {}: {}", cmd, e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            tracing::warn!(stderr = %stderr, "Exec operation: script failed");
            return Err(FlowError::OperationFailed(format!(
                "Script exited with code {}: {}",
                output.status.code().unwrap_or(-1),
                stderr.trim()
            )));
        }

        let stdout = String::from_utf8_lossy(&output.stdout);
        let result: Value = serde_json::from_str(stdout.trim())
            .unwrap_or_else(|_| json!(stdout.trim()));

        // Check for error from the wrapper
        if let Some(err) = result.get("__error") {
            return Err(FlowError::OperationFailed(format!(
                "Script error: {}",
                err.as_str().unwrap_or("Unknown error")
            )));
        }

        tracing::info!(code_len = code.len(), "Exec operation: executed successfully");

        Ok(result)
    }

    fn operation_type(&self) -> &str {
        "exec"
    }
}

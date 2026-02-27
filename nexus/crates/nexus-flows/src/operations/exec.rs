use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::{json, Value};

/// Exec operation — runs inline code (JavaScript via Bun or any configured runtime)
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

        // TODO: Execute code via Bun runtime or WASM sandbox
        // For now, return the data as-is with metadata about the code
        tracing::info!(code_len = code.len(), "Exec operation: code execution pending runtime integration");

        Ok(json!({
            "result": data,
            "executed": false,
            "reason": "Runtime not yet connected"
        }))
    }

    fn operation_type(&self) -> &str {
        "exec"
    }
}

use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::Value;

/// Transform operation — transforms data using a JSON template
/// Mirrors api/src/operations/transform/index.ts
pub struct TransformOperation;

#[async_trait]
impl FlowOperation for TransformOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let json_template = options.get("json").cloned().unwrap_or(Value::Null);

        if json_template.is_null() {
            return Ok(data);
        }

        // Resolve template variables like {{$trigger.body}}, {{operation_key.field}}
        let resolved = resolve_template(&json_template, context);

        Ok(resolved)
    }

    fn operation_type(&self) -> &str {
        "transform"
    }
}

/// Resolve {{variable}} references in a JSON template
fn resolve_template(template: &Value, context: &OperationContext) -> Value {
    match template {
        Value::String(s) => {
            // Check for {{variable}} pattern
            if s.starts_with("{{") && s.ends_with("}}") {
                let path = s.trim_start_matches("{{").trim_end_matches("}}").trim();
                resolve_path(path, context)
            } else {
                Value::String(s.clone())
            }
        }
        Value::Object(obj) => {
            let mut result = serde_json::Map::new();
            for (key, value) in obj {
                result.insert(key.clone(), resolve_template(value, context));
            }
            Value::Object(result)
        }
        Value::Array(arr) => {
            Value::Array(arr.iter().map(|v| resolve_template(v, context)).collect())
        }
        other => other.clone(),
    }
}

/// Resolve a dotted path like "$trigger.body.name" from context data
fn resolve_path(path: &str, context: &OperationContext) -> Value {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        return Value::Null;
    }

    // Get the root value from context
    let root_key = parts[0];
    let root_value = context.get(root_key);

    if let Some(mut current) = root_value.cloned() {
        for &part in &parts[1..] {
            current = match current {
                Value::Object(ref obj) => obj.get(part).cloned().unwrap_or(Value::Null),
                Value::Array(ref arr) => {
                    if let Ok(idx) = part.parse::<usize>() {
                        arr.get(idx).cloned().unwrap_or(Value::Null)
                    } else {
                        Value::Null
                    }
                }
                _ => Value::Null,
            };
        }
        current
    } else {
        Value::Null
    }
}

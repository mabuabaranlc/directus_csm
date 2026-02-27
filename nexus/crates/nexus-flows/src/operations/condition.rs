use async_trait::async_trait;
use crate::{FlowError, FlowOperation, OperationContext};
use serde_json::Value;

/// Condition operation — evaluates a rule and returns the data if true
/// Mirrors api/src/operations/condition/index.ts
pub struct ConditionOperation;

#[async_trait]
impl FlowOperation for ConditionOperation {
    async fn execute(
        &self,
        data: Value,
        options: &Value,
        _context: &OperationContext,
    ) -> Result<Value, FlowError> {
        let filter = options.get("filter").cloned().unwrap_or(Value::Null);

        if filter.is_null() {
            // No filter = always passes
            return Ok(data);
        }

        // Evaluate the filter against the data
        let passes = evaluate_condition(&data, &filter);

        if passes {
            Ok(data)
        } else {
            Err(FlowError::OperationFailed(
                "Condition not met".to_string(),
            ))
        }
    }

    fn operation_type(&self) -> &str {
        "condition"
    }
}

/// Simple condition evaluation — checks if data matches filter rules
fn evaluate_condition(data: &Value, filter: &Value) -> bool {
    match filter {
        Value::Object(rules) => {
            for (key, rule) in rules {
                if key == "_and" {
                    if let Value::Array(conditions) = rule {
                        return conditions.iter().all(|c| evaluate_condition(data, c));
                    }
                } else if key == "_or" {
                    if let Value::Array(conditions) = rule {
                        return conditions.iter().any(|c| evaluate_condition(data, c));
                    }
                } else {
                    // Field-level comparison
                    let field_value = data.get(key);
                    if let Value::Object(operators) = rule {
                        for (op, expected) in operators {
                            let passes = match op.as_str() {
                                "_eq" => field_value == Some(expected),
                                "_neq" => field_value != Some(expected),
                                "_null" => {
                                    let is_null = field_value.is_none()
                                        || field_value == Some(&Value::Null);
                                    expected.as_bool().unwrap_or(false) == is_null
                                }
                                "_nnull" => {
                                    let is_not_null = field_value.is_some()
                                        && field_value != Some(&Value::Null);
                                    expected.as_bool().unwrap_or(false) == is_not_null
                                }
                                "_contains" => {
                                    if let (Some(Value::String(v)), Some(s)) =
                                        (field_value, expected.as_str())
                                    {
                                        v.contains(s)
                                    } else {
                                        false
                                    }
                                }
                                "_gt" => compare_values(field_value, Some(expected)) > 0,
                                "_gte" => compare_values(field_value, Some(expected)) >= 0,
                                "_lt" => compare_values(field_value, Some(expected)) < 0,
                                "_lte" => compare_values(field_value, Some(expected)) <= 0,
                                "_in" => {
                                    if let Value::Array(arr) = expected {
                                        field_value
                                            .map(|v| arr.contains(v))
                                            .unwrap_or(false)
                                    } else {
                                        false
                                    }
                                }
                                _ => true,
                            };
                            if !passes {
                                return false;
                            }
                        }
                    }
                }
            }
            true
        }
        _ => true,
    }
}

fn compare_values(a: Option<&Value>, b: Option<&Value>) -> i32 {
    match (a, b) {
        (Some(Value::Number(a)), Some(Value::Number(b))) => {
            let a = a.as_f64().unwrap_or(0.0);
            let b = b.as_f64().unwrap_or(0.0);
            a.partial_cmp(&b)
                .map(|o| o as i32)
                .unwrap_or(0)
        }
        (Some(Value::String(a)), Some(Value::String(b))) => a.cmp(b) as i32,
        _ => 0,
    }
}

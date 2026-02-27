mod defaults;

use once_cell::sync::OnceCell;
use serde_json::Value;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::path::Path;
use tracing::warn;

pub type Env = HashMap<String, Value>;

static ENV_CACHE: OnceCell<Env> = OnceCell::new();

/// Get the global environment configuration (singleton)
/// Mirrors Directus' useEnv() pattern
pub fn use_env() -> &'static Env {
    ENV_CACHE.get_or_init(create_env)
}

/// Reset the environment cache (for testing)
pub fn reset_env() {
    // OnceCell doesn't support reset, so this is a no-op in production.
    // For testing, consider using a RwLock instead.
}

/// Create the environment configuration
fn create_env() -> Env {
    let mut output = Env::new();

    // Step 1: Apply defaults
    for (key, value) in defaults::DEFAULTS.iter() {
        output.insert(key.to_string(), value.clone());
    }

    // Step 2: Read from config file (.env)
    let config_path = env::var("CONFIG_PATH")
        .unwrap_or_else(|_| ".env".to_string());

    if Path::new(&config_path).exists() {
        if let Ok(contents) = fs::read_to_string(&config_path) {
            for line in contents.lines() {
                let line = line.trim();
                if line.is_empty() || line.starts_with('#') {
                    continue;
                }
                if let Some((key, value)) = line.split_once('=') {
                    let key = key.trim().to_string();
                    let value = value.trim().trim_matches('"').trim_matches('\'');
                    output.insert(key, cast_value(value));
                }
            }
        }
    }

    // Step 3: Override with process environment variables
    for (key, value) in env::vars() {
        // Handle _FILE indirection
        if key.len() > 5 && key.ends_with("_FILE") {
            if let Ok(file_content) = fs::read_to_string(&value) {
                let real_key = &key[..key.len() - 5];
                output.insert(real_key.to_string(), cast_value(file_content.trim()));
                continue;
            } else {
                warn!("Failed to read file for env var {}: {}", key, value);
            }
        }

        output.insert(key, cast_value(&value));
    }

    output
}

/// Cast a string value to the appropriate JSON type
/// Mirrors Directus' cast.ts with type guessing
fn cast_value(value: &str) -> Value {
    // Check for explicit cast prefix (e.g., "string:value", "number:42")
    if let Some((prefix, rest)) = value.split_once(':') {
        match prefix {
            "string" => return Value::String(rest.to_string()),
            "number" => {
                if let Ok(n) = rest.parse::<f64>() {
                    return json_number(n);
                }
            }
            "boolean" => {
                return Value::Bool(rest == "true" || rest == "1");
            }
            "array" => {
                let items: Vec<Value> = rest
                    .split(',')
                    .map(|s| Value::String(s.trim().to_string()))
                    .collect();
                return Value::Array(items);
            }
            "json" => {
                if let Ok(parsed) = serde_json::from_str(rest) {
                    return parsed;
                }
                return Value::String(rest.to_string());
            }
            _ => {} // Not a recognized prefix, fall through
        }
    }

    // Heuristic type guessing
    guess_and_cast(value)
}

/// Guess the type and cast accordingly
fn guess_and_cast(value: &str) -> Value {
    // Boolean
    if value == "true" {
        return Value::Bool(true);
    }
    if value == "false" {
        return Value::Bool(false);
    }

    // Number (not zero-padded, not empty, within safe range)
    if !value.is_empty()
        && !value.starts_with('0')
        && value != "0"
        || value == "0"
    {
        if let Ok(n) = value.parse::<f64>() {
            if n >= -(2_f64.powi(53)) && n <= 2_f64.powi(53) {
                if value == "0" || !value.starts_with('0') {
                    return json_number(n);
                }
            }
        }
    }

    // Array (contains commas)
    if value.contains(',') {
        let items: Vec<Value> = value
            .split(',')
            .map(|s| guess_and_cast(s.trim()))
            .filter(|v| v != &Value::String(String::new()))
            .collect();
        return Value::Array(items);
    }

    // Try JSON
    if let Ok(parsed) = serde_json::from_str::<Value>(value) {
        if parsed.is_object() || parsed.is_array() {
            return parsed;
        }
    }

    // Fallback: string
    Value::String(value.to_string())
}

fn json_number(n: f64) -> Value {
    if n.fract() == 0.0 && n.abs() < i64::MAX as f64 {
        Value::Number(serde_json::Number::from(n as i64))
    } else {
        serde_json::Number::from_f64(n)
            .map(Value::Number)
            .unwrap_or(Value::String(n.to_string()))
    }
}

// ── Convenience accessors ──────────────────────────────────────────

/// Get a string value from the environment
pub fn env_string(key: &str) -> Option<String> {
    use_env().get(key).and_then(|v| match v {
        Value::String(s) => Some(s.clone()),
        other => Some(other.to_string().trim_matches('"').to_string()),
    })
}

/// Get a string value with a default
pub fn env_string_or(key: &str, default: &str) -> String {
    env_string(key).unwrap_or_else(|| default.to_string())
}

/// Get a number value from the environment
pub fn env_number(key: &str) -> Option<i64> {
    use_env().get(key).and_then(|v| match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    })
}

/// Get a number value with a default
pub fn env_number_or(key: &str, default: i64) -> i64 {
    env_number(key).unwrap_or(default)
}

/// Get a boolean value from the environment
pub fn env_bool(key: &str) -> Option<bool> {
    use_env().get(key).and_then(|v| match v {
        Value::Bool(b) => Some(*b),
        Value::String(s) => Some(s == "true" || s == "1"),
        _ => None,
    })
}

/// Get a boolean value with a default
pub fn env_bool_or(key: &str, default: bool) -> bool {
    env_bool(key).unwrap_or(default)
}

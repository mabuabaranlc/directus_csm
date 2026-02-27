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

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // Test cast_value with explicit prefixes

    #[test]
    fn test_cast_string_prefix() {
        assert_eq!(cast_value("string:hello"), Value::String("hello".to_string()));
        assert_eq!(cast_value("string:123"), Value::String("123".to_string()));
        assert_eq!(cast_value("string:true"), Value::String("true".to_string()));
    }

    #[test]
    fn test_cast_number_prefix() {
        assert_eq!(cast_value("number:42"), json!(42));
        assert_eq!(cast_value("number:3.14"), json!(3.14));
        assert_eq!(cast_value("number:0"), json!(0));
    }

    #[test]
    fn test_cast_boolean_prefix() {
        assert_eq!(cast_value("boolean:true"), Value::Bool(true));
        assert_eq!(cast_value("boolean:1"), Value::Bool(true));
        assert_eq!(cast_value("boolean:false"), Value::Bool(false));
        assert_eq!(cast_value("boolean:0"), Value::Bool(false));
    }

    #[test]
    fn test_cast_array_prefix() {
        assert_eq!(
            cast_value("array:a,b,c"),
            json!(["a", "b", "c"])
        );
    }

    #[test]
    fn test_cast_json_prefix() {
        assert_eq!(
            cast_value(r#"json:{"key":"value"}"#),
            json!({"key": "value"})
        );
    }

    // Test guess_and_cast heuristics

    #[test]
    fn test_guess_boolean() {
        assert_eq!(guess_and_cast("true"), Value::Bool(true));
        assert_eq!(guess_and_cast("false"), Value::Bool(false));
    }

    #[test]
    fn test_guess_number() {
        assert_eq!(guess_and_cast("42"), json!(42));
        assert_eq!(guess_and_cast("0"), json!(0));
        assert_eq!(guess_and_cast("3.14"), json!(3.14));
        assert_eq!(guess_and_cast("-100"), json!(-100));
    }

    #[test]
    fn test_guess_zero_padded_stays_string() {
        // Zero-padded numbers should remain strings (e.g., zip codes "01234")
        assert_eq!(guess_and_cast("0123"), Value::String("0123".to_string()));
    }

    #[test]
    fn test_guess_comma_separated_as_array() {
        let result = guess_and_cast("a,b,c");
        assert!(result.is_array());
        assert_eq!(result.as_array().unwrap().len(), 3);
    }

    #[test]
    fn test_guess_plain_string() {
        assert_eq!(guess_and_cast("hello"), Value::String("hello".to_string()));
        assert_eq!(guess_and_cast("my-host.com"), Value::String("my-host.com".to_string()));
    }

    #[test]
    fn test_guess_empty_string() {
        assert_eq!(guess_and_cast(""), Value::String(String::new()));
    }

    #[test]
    fn test_guess_json_object() {
        let result = guess_and_cast(r#"{"key":"value"}"#);
        assert!(result.is_object());
        assert_eq!(result["key"], json!("value"));
    }

    // Test json_number helper

    #[test]
    fn test_json_number_integer() {
        assert_eq!(json_number(42.0), json!(42));
        assert_eq!(json_number(0.0), json!(0));
        assert_eq!(json_number(-10.0), json!(-10));
    }

    #[test]
    fn test_json_number_float() {
        assert_eq!(json_number(3.14), json!(3.14));
    }

    // Test defaults are loaded

    #[test]
    fn test_defaults_exist() {
        assert!(defaults::DEFAULTS.contains_key("HOST"));
        assert!(defaults::DEFAULTS.contains_key("PORT"));
        assert_eq!(defaults::DEFAULTS["PORT"], json!(8055));
        assert_eq!(defaults::DEFAULTS["HOST"], json!("0.0.0.0"));
    }

    #[test]
    fn test_defaults_auth_tokens() {
        assert_eq!(defaults::DEFAULTS["ACCESS_TOKEN_TTL"], json!("15m"));
        assert_eq!(defaults::DEFAULTS["REFRESH_TOKEN_TTL"], json!("7d"));
    }

    #[test]
    fn test_defaults_rate_limiter() {
        assert_eq!(defaults::DEFAULTS["RATE_LIMITER_ENABLED"], json!(false));
        assert_eq!(defaults::DEFAULTS["RATE_LIMITER_POINTS"], json!(50));
    }

    #[test]
    fn test_defaults_storage() {
        assert_eq!(defaults::DEFAULTS["STORAGE_LOCATIONS"], json!("local"));
        assert_eq!(defaults::DEFAULTS["STORAGE_LOCAL_ROOT"], json!("./uploads"));
    }

    #[test]
    fn test_defaults_cache() {
        assert_eq!(defaults::DEFAULTS["CACHE_ENABLED"], json!(false));
        assert_eq!(defaults::DEFAULTS["CACHE_STORE"], json!("memory"));
    }

    // Test that unknown prefix falls through to guessing
    #[test]
    fn test_unknown_prefix_falls_through() {
        // "http://localhost" has "http" as prefix, which is unknown → falls through
        let result = cast_value("http://localhost");
        assert_eq!(result, Value::String("http://localhost".to_string()));
    }
}

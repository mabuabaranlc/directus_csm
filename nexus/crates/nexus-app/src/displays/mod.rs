pub mod raw;
pub mod formatted_value;
pub mod boolean;
pub mod datetime;
pub mod image;
pub mod file;
pub mod color;
pub mod rating;
pub mod labels;
pub mod user;
pub mod related_values;
pub mod icon;
pub mod badge;
pub mod translations;

use serde_json::Value;

/// Format a value for display based on the display type
pub fn format_display(value: &Value, display_type: &str, _options: &Value) -> String {
    match display_type {
        "raw" => raw::format_raw(value),
        "formatted-value" => formatted_value::format_value(value),
        "boolean" => boolean::format_boolean(value),
        "datetime" => datetime::format_datetime(value),
        "labels" => labels::format_labels(value),
        _ => raw::format_raw(value),
    }
}

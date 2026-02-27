pub mod input;
pub mod input_multiline;
pub mod select_dropdown;
pub mod select_multiple;
pub mod boolean;
pub mod datetime;
pub mod file;
pub mod file_image;
pub mod input_code;
pub mod input_rich_text;
pub mod color;
pub mod slider;
pub mod tags;
pub mod map;
pub mod translations;
pub mod input_hash;
pub mod presentation_notice;

use leptos::prelude::*;
use serde_json::Value;

/// Props passed to every interface component
#[derive(Clone)]
pub struct InterfaceProps {
    pub value: Value,
    pub field_type: String,
    pub options: Value,
    pub disabled: bool,
    pub on_change: Callback<Value>,
}

/// Registry of available interfaces
pub fn get_interface(interface_type: &str) -> &'static str {
    match interface_type {
        "input" => "input",
        "input-multiline" | "textarea" => "input-multiline",
        "select-dropdown" => "select-dropdown",
        "select-multiple-dropdown" => "select-multiple",
        "boolean" | "toggle" => "boolean",
        "datetime" => "datetime",
        "file" => "file",
        "file-image" => "file-image",
        "input-code" => "input-code",
        "input-rich-text-html" => "input-rich-text",
        "select-color" => "color",
        "slider" => "slider",
        "tags" => "tags",
        "map" => "map",
        "translations" => "translations",
        "input-hash" => "input-hash",
        "presentation-notice" => "presentation-notice",
        _ => "input",
    }
}

pub mod label;
pub mod metric;
pub mod list;
pub mod time_series;
pub mod global_variable;
pub mod button_links;


/// Available panel types
pub fn panel_types() -> Vec<(&'static str, &'static str, &'static str)> {
    vec![
        ("label", "Label", "title"),
        ("metric", "Metric", "speed"),
        ("list", "List", "list"),
        ("time-series", "Time Series", "show_chart"),
        ("global-variable", "Variable", "code"),
        ("button-links", "Button Links", "link"),
    ]
}

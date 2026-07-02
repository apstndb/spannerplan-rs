//! Ensures `schema/render-config.example.json` matches Rust `RenderConfig` serde.

use spannerplan::core::reference::{PrintSection, RenderConfig};

#[test]
fn render_config_example_json_deserializes() {
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../schema/render-config.example.json"
    );
    let text = std::fs::read_to_string(path).unwrap();
    let config: RenderConfig = serde_json::from_str(&text).unwrap();
    assert_eq!(config.wrap_width, 80);
    assert!(config.hanging_indent);
    assert_eq!(
        config.print_sections,
        Some(vec![PrintSection::Predicates, PrintSection::Ordering])
    );
    assert!(!config.show_scalar_vars);
    assert!(config.resolve_scalar_vars);
    assert!(!config.resolve_scalar_vars_recursive);
    assert!(!config.disallow_unknown_stats);
}

#[test]
fn render_config_null_print_sections_means_default() {
    let config: RenderConfig = serde_json::from_str(r#"{"printSections":null}"#).unwrap();
    assert_eq!(config.print_sections, None);
}

#[test]
fn render_config_empty_print_sections_means_none() {
    let config: RenderConfig = serde_json::from_str(r#"{"printSections":[]}"#).unwrap();
    assert_eq!(config.print_sections, Some(vec![]));
}

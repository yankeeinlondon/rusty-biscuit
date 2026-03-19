use crate::artifact::{OutputFormat, RenderRequest};
use crate::cache::FileCache;
use crate::mermaid::{MermaidConfig, MermaidDiagram, MermaidTheme, QuadrantTheme};
use serial_test::serial;

#[test]
#[serial]
fn mermaid_diagram_renders_to_png() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let request = RenderRequest::default();

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert!(artifact.path.exists(), "PNG file should exist");
    assert_eq!(artifact.format, OutputFormat::Png);
    assert!(
        !artifact.cache_hit,
        "First render should not be a cache hit"
    );

    // Verify the file has content
    let metadata = std::fs::metadata(&artifact.path).expect("Failed to read metadata");
    assert!(metadata.len() > 0, "PNG file should not be empty");
}

#[test]
#[serial]
fn mermaid_diagram_cache_hit_on_second_render() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let request = RenderRequest::default();

    // First render
    let artifact1 = diagram.render(&request).expect("Failed to render diagram");
    assert!(
        !artifact1.cache_hit,
        "First render should not be a cache hit"
    );

    // Second render with same inputs
    let artifact2 = diagram.render(&request).expect("Failed to render diagram");
    assert!(artifact2.cache_hit, "Second render should be a cache hit");
    assert_eq!(artifact1.path, artifact2.path, "Paths should be the same");
}

#[test]
fn mermaid_fallback_code_block() {
    let instructions = "graph LR; A-->B";
    let diagram = MermaidDiagram::new(instructions);

    let code_block = diagram.fallback_code_block();

    assert!(code_block.starts_with("```mermaid"));
    assert!(code_block.contains(instructions));
    assert!(code_block.ends_with("```"));
}

#[test]
fn mermaid_builder_methods_are_chainable() {
    let diagram = MermaidDiagram::new("graph LR; A-->B")
        .with_theme(MermaidTheme::Forest)
        .with_title("Test Diagram")
        .with_config(MermaidConfig::new().with_point_label_font_size(14));

    assert_eq!(diagram.instructions(), "graph LR; A-->B");
}

#[test]
#[serial]
fn mermaid_handles_unusual_syntax_gracefully() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    // mermaid-rs-renderer tends to handle invalid syntax gracefully
    // by producing a simple diagram or placeholder, rather than failing.
    // This test verifies that rendering completes without panicking.
    let diagram = MermaidDiagram::new("this is not valid mermaid syntax");
    let request = RenderRequest::default();

    let result = diagram.render(&request);
    // The renderer should complete (either with success or a specific error)
    // but should not panic
    match result {
        Ok(artifact) => {
            // If it renders successfully, verify the artifact exists
            assert!(artifact.path.exists(), "Output file should exist");
        }
        Err(e) => {
            // If it fails, ensure it's a proper error
            assert!(
                e.to_string().contains("render") || e.to_string().contains("parse"),
                "Error message should indicate render/parse failure"
            );
        }
    }
}

#[test]
#[serial]
fn mermaid_renders_to_svg() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let mut request = RenderRequest::default();
    request.format = OutputFormat::Svg;

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert!(artifact.path.exists(), "SVG file should exist");
    assert_eq!(artifact.format, OutputFormat::Svg);

    // Verify the file is valid SVG
    let content = std::fs::read_to_string(&artifact.path).expect("Failed to read SVG");
    assert!(content.contains("<svg"), "Content should be valid SVG");
}

#[test]
fn mermaid_theme_as_str() {
    assert_eq!(MermaidTheme::Dark.as_str(), "dark");
    assert_eq!(MermaidTheme::Default.as_str(), "default");
    assert_eq!(MermaidTheme::Forest.as_str(), "forest");
    assert_eq!(MermaidTheme::Neutral.as_str(), "neutral");
}

#[test]
fn mermaid_theme_for_color_mode() {
    assert_eq!(MermaidTheme::for_color_mode(true), MermaidTheme::Dark);
    assert_eq!(MermaidTheme::for_color_mode(false), MermaidTheme::Default);
}

#[test]
fn mermaid_theme_inverse() {
    assert_eq!(MermaidTheme::Dark.inverse(), MermaidTheme::Default);
    assert_eq!(MermaidTheme::Default.inverse(), MermaidTheme::Dark);
    assert_eq!(MermaidTheme::Forest.inverse(), MermaidTheme::Dark);
    assert_eq!(MermaidTheme::Neutral.inverse(), MermaidTheme::Dark);
}

#[test]
fn quadrant_theme_parse() {
    assert_eq!(
        QuadrantTheme::parse("default"),
        Some(QuadrantTheme::Default)
    );
    assert_eq!(
        QuadrantTheme::parse("magic-quadrangle"),
        Some(QuadrantTheme::MagicQuadrangle)
    );
    assert_eq!(
        QuadrantTheme::parse("MAGIC_QUADRANGLE"),
        Some(QuadrantTheme::MagicQuadrangle)
    );
    assert_eq!(QuadrantTheme::parse("invalid"), None);
}

#[test]
fn mermaid_config_has_options() {
    let config = MermaidConfig::new();
    assert!(!config.has_options(), "Empty config should have no options");

    let config = MermaidConfig::new().with_point_label_font_size(14);
    assert!(
        config.has_options(),
        "Config with options should return true"
    );
}

#[test]
fn mermaid_config_to_json() {
    let config = MermaidConfig::new();
    assert_eq!(config.to_json(), None, "Empty config should return None");

    let config = MermaidConfig::new()
        .with_point_label_font_size(14)
        .with_point_radius(8);

    let json = config
        .to_json()
        .expect("Config with options should return JSON");
    assert!(json.contains("pointLabelFontSize"));
    assert!(json.contains("pointRadius"));
}

#[test]
fn mermaid_config_with_quadrant_fill() {
    let config = MermaidConfig::new()
        .with_quadrant_fill(1, "#ff0000")
        .with_quadrant_fill(2, "#00ff00")
        .with_quadrant_fill(3, "#0000ff")
        .with_quadrant_fill(4, "#ffff00");

    assert_eq!(config.quadrant1_fill, Some("#ff0000".to_string()));
    assert_eq!(config.quadrant2_fill, Some("#00ff00".to_string()));
    assert_eq!(config.quadrant3_fill, Some("#0000ff".to_string()));
    assert_eq!(config.quadrant4_fill, Some("#ffff00".to_string()));

    let json = config.to_json().expect("Config should return JSON");
    assert!(json.contains("quadrant1Fill"));
    assert!(json.contains("quadrant2Fill"));
    assert!(json.contains("quadrant3Fill"));
    assert!(json.contains("quadrant4Fill"));
}

#[test]
fn quadrant_theme_apply_dark_mode() {
    let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), true);

    assert_eq!(config.quadrant1_fill, Some("#1e2a1e".to_string()));
    assert_eq!(config.quadrant3_fill, Some("#2a1e1e".to_string()));
    assert_eq!(config.quadrant2_fill, Some("#1a1a1a".to_string()));
    assert_eq!(config.quadrant4_fill, Some("#1a1a1a".to_string()));
}

#[test]
fn quadrant_theme_apply_light_mode() {
    let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), false);

    assert_eq!(config.quadrant1_fill, Some("#f6faf6".to_string()));
    assert_eq!(config.quadrant3_fill, Some("#faf6f6".to_string()));
    assert_eq!(config.quadrant2_fill, Some("#f8f8f8".to_string()));
    assert_eq!(config.quadrant4_fill, Some("#f8f8f8".to_string()));
}

#[test]
#[serial]
fn mermaid_alt_text_uses_title() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    let diagram = MermaidDiagram::new("graph LR; A-->B").with_title("My Custom Diagram");
    let request = RenderRequest::default();

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert_eq!(artifact.alt_text, "My Custom Diagram");
}

#[test]
#[serial]
fn mermaid_alt_text_defaults_to_mermaid_diagram() {
    // Clear cache to ensure clean state
    let cache = FileCache::new();
    let _ = cache.clear();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let request = RenderRequest::default();

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert_eq!(artifact.alt_text, "Mermaid diagram");
}

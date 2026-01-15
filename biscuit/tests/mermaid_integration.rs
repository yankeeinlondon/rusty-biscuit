//! Integration tests for mermaid module.
//!
//! This test suite validates the complete mermaid rendering pipeline including:
//! - HTML rendering with theme support
//! - Accessibility features (alt text, ARIA attributes)
//! - XSS prevention through proper HTML escaping
//! - Hash computation for caching
//! - Builder pattern APIs
//!
//! Fixtures are located in `tests/fixtures/mermaid/`:
//! - `valid/` contains well-formed mermaid diagrams
//! - `invalid/` contains edge cases (empty, too large, whitespace-only)

mod common;

use common::load_fixture;
use shared::markdown::highlighting::{ColorMode, ThemePair};
use shared::mermaid::{DEFAULT_DARK_THEME, DEFAULT_LIGHT_THEME, Mermaid};

/// Test simple flowchart fixture can be loaded and rendered to HTML.
#[test]
fn test_fixture_simple_flowchart() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    // Verify structure
    assert!(html.head.contains("mermaid"));
    assert!(html.body.contains("flowchart"));
    assert!(html.body.contains(r#"role="img""#));
}

/// Test complex sequence diagram fixture renders correctly.
#[test]
fn test_fixture_complex_sequence() {
    let content = load_fixture("mermaid/valid/complex_sequence.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    // Verify sequence diagram elements
    assert!(html.body.contains("sequenceDiagram"));
    assert!(html.body.contains("participant"));
    assert!(html.body.contains(r#"aria-label=""#));
}

/// Test all diagram types fixture loads successfully.
#[test]
fn test_fixture_all_types() {
    let content = load_fixture("mermaid/valid/all_types.mmd");
    let diagram = Mermaid::from(content.as_str());

    // Verify hash is computed
    let hash = diagram.hash();
    assert_ne!(hash, 0);

    // Verify same content produces same hash
    let diagram2 = Mermaid::from(content.as_str());
    assert_eq!(hash, diagram2.hash());
}

/// Test fixture with frontmatter title.
#[test]
fn test_fixture_with_title() {
    let content = load_fixture("mermaid/valid/with_title.mmd");
    let diagram = Mermaid::from(content.as_str());

    // Note: The Mermaid struct doesn't parse frontmatter automatically,
    // so we test that the raw content is preserved
    assert!(
        diagram
            .instructions()
            .contains("title: User Authentication Flow")
    );
}

/// Test empty fixture produces valid (though useless) diagram.
#[test]
fn test_fixture_empty() {
    let content = load_fixture("mermaid/invalid/empty.mmd");
    let diagram = Mermaid::from(content.as_str());

    assert_eq!(diagram.instructions(), "");

    // Should still render to HTML (mermaid.js will handle the empty diagram)
    let html = diagram.render_for_html();
    assert!(html.head.contains("mermaid"));
}

/// Test whitespace-only fixture.
#[test]
fn test_fixture_whitespace_only() {
    let content = load_fixture("mermaid/invalid/whitespace_only.mmd");
    let diagram = Mermaid::from(content.as_str());

    // Instructions preserve whitespace
    assert!(diagram.instructions().trim().is_empty());
}

/// Test too-large fixture can be loaded (size validation happens at render time).
#[test]
fn test_fixture_too_large() {
    let content = load_fixture("mermaid/invalid/too_large.mmd");
    assert!(content.len() > 2000, "Fixture should exceed 2KB");

    let diagram = Mermaid::from(content.as_str());

    // HTML rendering should work (no size limit for client-side rendering)
    let html = diagram.render_for_html();
    assert!(html.body.contains("flowchart"));
}

// Builder pattern tests with fixtures

/// Test custom theme application with fixture.
#[test]
fn test_custom_theme_with_fixture() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let light = DEFAULT_LIGHT_THEME.clone();
    let dark = DEFAULT_DARK_THEME.clone();

    let diagram = Mermaid::from(content.as_str()).with_theme(light.clone(), dark.clone());

    assert_eq!(diagram.theme(ColorMode::Light), &light);
    assert_eq!(diagram.theme(ColorMode::Dark), &dark);
}

/// Test syntect theme resolution with fixture.
#[test]
fn test_syntect_theme_with_fixture() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str()).use_syntect_theme(ThemePair::Gruvbox);

    // Verify theme is resolved from syntect (may be same as default, so just check it's valid)
    let theme_light = diagram.theme(ColorMode::Light);
    let theme_dark = diagram.theme(ColorMode::Dark);

    // Both themes should have non-empty background colors
    assert!(!theme_light.background.is_empty());
    assert!(!theme_dark.background.is_empty());
}

/// Test title and footer metadata with fixture.
#[test]
fn test_metadata_with_fixture() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str())
        .with_title("Test Flowchart")
        .with_footer("Generated 2026-01-03");

    assert_eq!(diagram.title(), Some("Test Flowchart"));
    assert_eq!(diagram.footer(), Some("Generated 2026-01-03"));
}

/// Test alt text generation with fixture.
#[test]
fn test_alt_text_detection() {
    // Flowchart
    let flowchart = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(flowchart.as_str());
    assert_eq!(diagram.alt_text(), "Flowchart diagram");

    // Sequence
    let sequence = load_fixture("mermaid/valid/complex_sequence.mmd");
    let diagram = Mermaid::from(sequence.as_str());
    assert_eq!(diagram.alt_text(), "Sequence diagram");
}

/// Test alt text with explicit title overrides detection.
#[test]
fn test_alt_text_with_title_override() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str()).with_title("Custom Title");

    assert_eq!(diagram.alt_text(), "Custom Title");
}

// HTML rendering security tests

/// Test HTML escaping prevents XSS in instructions.
#[test]
fn test_html_escaping_in_instructions() {
    let malicious = r#"flowchart LR
    A["<script>alert('xss')</script>"] --> B"#;

    let diagram = Mermaid::from(malicious);
    let html = diagram.render_for_html();

    // Should escape HTML entities
    assert!(html.body.contains("&lt;script&gt;"));
    assert!(html.body.contains("&lt;/script&gt;"));

    // Should NOT contain raw script tags
    assert!(!html.body.contains("<script>alert"));
}

/// Test HTML escaping in title attribute.
#[test]
fn test_html_escaping_in_title() {
    let diagram =
        Mermaid::new("flowchart LR\n    A --> B").with_title("<script>alert('xss')</script>");

    let html = diagram.render_for_html();

    // Should escape the title attribute
    assert!(html.body.contains("&lt;script&gt;"));

    // Should NOT contain raw script tags in title attribute
    assert!(!html.body.contains(r#"title="<script>"#));
}

// HTML rendering structure tests

/// Test HTML head contains mermaid.js ESM import.
#[test]
fn test_html_head_has_mermaid_import() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    assert!(html.head.contains("https://cdn.jsdelivr.net/npm/mermaid"));
    assert!(html.head.contains("type=\"module\""));
    assert!(html.head.contains("import mermaid from"));
}

/// Test HTML body has ARIA attributes for accessibility.
#[test]
fn test_html_body_has_aria_attributes() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    assert!(html.body.contains(r#"role="img""#));
    assert!(html.body.contains(r#"aria-label=""#));
    assert!(html.body.contains(r#"class="mermaid""#));
}

/// Test HTML rendering with title includes title attribute.
#[test]
fn test_html_with_title_includes_attribute() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str()).with_title("Test Diagram");

    let html = diagram.render_for_html();
    assert!(html.body.contains(r#"title="Test Diagram""#));
}

/// Test HTML rendering without title has no title attribute.
#[test]
fn test_html_without_title_no_attribute() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());

    let html = diagram.render_for_html();
    // Should not have a title attribute (only aria-label)
    assert!(!html.body.contains(r#"title="#));
}

// Hash computation tests

/// Test hash is deterministic for same content.
#[test]
fn test_hash_deterministic() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");

    let diagram1 = Mermaid::from(content.as_str());
    let diagram2 = Mermaid::from(content.as_str());

    assert_eq!(diagram1.hash(), diagram2.hash());
}

/// Test hash differs for different content.
#[test]
fn test_hash_differs_for_different_content() {
    let content1 = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let content2 = load_fixture("mermaid/valid/complex_sequence.mmd");

    let diagram1 = Mermaid::from(content1.as_str());
    let diagram2 = Mermaid::from(content2.as_str());

    assert_ne!(diagram1.hash(), diagram2.hash());
}

/// Test hash is non-zero for valid content.
#[test]
fn test_hash_non_zero() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());

    assert_ne!(diagram.hash(), 0);
}

// Clone and Debug tests

/// Test Mermaid implements Clone correctly.
#[test]
fn test_clone_preserves_state() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram1 = Mermaid::from(content.as_str())
        .with_title("Test")
        .with_footer("Footer");

    let diagram2 = diagram1.clone();

    assert_eq!(diagram1.instructions(), diagram2.instructions());
    assert_eq!(diagram1.title(), diagram2.title());
    assert_eq!(diagram1.footer(), diagram2.footer());
    assert_eq!(diagram1.hash(), diagram2.hash());
}

/// Test Mermaid implements Debug.
#[test]
fn test_debug_trait() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());

    let debug_str = format!("{:?}", diagram);
    assert!(debug_str.contains("Mermaid"));
}

// Default trait test

/// Test Default implementation creates valid flowchart.
#[test]
fn test_default_creates_flowchart() {
    let diagram = Mermaid::default();

    assert!(diagram.instructions().contains("flowchart"));
    assert!(diagram.instructions().contains("Start"));
    assert!(diagram.instructions().contains("Decision"));
}

// Icon Pack Integration Tests (Phase 4)

/// Test HTML output includes icon pack registration.
#[test]
fn test_html_has_icon_pack_registration() {
    let content = load_fixture("mermaid/valid/simple_flowchart.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    // Verify registerIconPacks is present
    assert!(
        html.head.contains("registerIconPacks"),
        "HTML head should contain registerIconPacks call"
    );

    // Verify all 4 icon packs are registered
    assert!(
        html.head.contains("@iconify-json/fa7-brands"),
        "Should register fa7-brands icon pack"
    );
    assert!(
        html.head.contains("@iconify-json/lucide"),
        "Should register lucide icon pack"
    );
    assert!(
        html.head.contains("@iconify-json/carbon"),
        "Should register carbon icon pack"
    );
    assert!(
        html.head.contains("@iconify-json/system-uicons"),
        "Should register system-uicons icon pack"
    );

    // Verify registerIconPacks comes BEFORE initialize
    let register_pos = html
        .head
        .find("registerIconPacks")
        .expect("registerIconPacks should exist");
    let initialize_pos = html
        .head
        .find("initialize")
        .expect("initialize should exist");
    assert!(
        register_pos < initialize_pos,
        "registerIconPacks must be called before initialize"
    );
}

/// Test fixture containing icons loads correctly.
#[test]
fn test_fixture_with_icons() {
    let content = load_fixture("mermaid/valid/with_icons.mmd");
    let diagram = Mermaid::from(content.as_str());
    let html = diagram.render_for_html();

    // Verify the icon syntax is preserved in the HTML body
    assert!(
        html.body.contains("fa7-brands"),
        "Should preserve fa7-brands icon reference"
    );
    assert!(
        html.body.contains("lucide"),
        "Should preserve lucide icon reference"
    );
    assert!(
        html.body.contains("carbon"),
        "Should preserve carbon icon reference"
    );
    assert!(
        html.body.contains("system-uicons"),
        "Should preserve system-uicons icon reference"
    );
}

/// Test MmdcNotFound error message contains installation instructions.
#[test]
fn test_mmdc_not_found_error_message() {
    use shared::mermaid::MermaidRenderError;

    let error = MermaidRenderError::MmdcNotFound;
    let message = error.to_string();

    assert!(
        message.contains("npm install -g @mermaid-js/mermaid-cli"),
        "Error message should contain installation instructions: {}",
        message
    );
}

/// Test MmdcExecutionFailed error includes exit code and stderr.
#[test]
fn test_mmdc_execution_failed_error() {
    use shared::mermaid::MermaidRenderError;

    let error = MermaidRenderError::MmdcExecutionFailed {
        exit_code: 1,
        stderr: "Parse error at line 1".to_string(),
    };
    let message = error.to_string();

    assert!(
        message.contains("exit code 1"),
        "Error should contain exit code"
    );
    assert!(
        message.contains("Parse error at line 1"),
        "Error should contain stderr"
    );
}

/// Test render_for_terminal is synchronous (no async).
/// This verifies the API migration from async to sync.
#[test]
fn test_render_for_terminal_is_sync() {
    // This test verifies compilation succeeds with sync signature.
    // The function should NOT require .await or tokio runtime.
    let diagram = Mermaid::new("flowchart LR\n    A --> B");

    // render_for_terminal() returns Result directly, not a Future
    // If this compiles, the sync migration was successful.
    let _result: Result<(), shared::mermaid::MermaidRenderError> = diagram.render_for_terminal();

    // Note: We don't check the actual result because mmdc may not be installed
    // in the test environment. The test verifies the API is sync.
}

//! D41, D55, D63, D70, AC-C6, AC-C6a at the level of the Mermaid cache key
//! (`MermaidDiagram::render`'s pipeline step 4).
//!
//! Each test changes one input and checks whether the key changes. The font
//! inputs themselves (`MERMAID_FONT`, installed and removed faces, fallback
//! order, file metadata) are covered on `FontChain::cache_identity` in
//! `cache_identity_tests`; here one test proves the key carries that identity.

use std::sync::Arc;

use serial_test::serial;

use crate::artifact::{OutputFormat, RenderRequest};
use crate::cache::file_cache::MERMAID_BACKEND;
use crate::cache::{FileCache, VisualizationKind};
use crate::fonts::{FontChain, default_chain};
use crate::mermaid::render::take_merman_calls;
use crate::mermaid::{MermaidConfig, MermaidDiagram, MermaidTarget, MermaidTheme};
use crate::tests::isolated_cache_dir;

fn key(diagram: &MermaidDiagram, request: &RenderRequest) -> String {
    diagram
        .cache_key_with(default_chain(), request)
        .expect("the key is computed")
}

fn flowchart() -> MermaidDiagram {
    MermaidDiagram::new("flowchart LR\n    A --> B")
}

/// An artifact stored under the key `main`'s mermaid-rs-renderer path
/// computed for the same diagram and request is not served (AC-C6).
#[test]
#[serial]
fn an_artifact_cached_before_the_cutover_is_not_served() {
    let _cache = isolated_cache_dir();
    let request = RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    };
    let source = "flowchart LR\n    A --> B";
    // `main`'s key: the source, its options JSON, and the old backend id.
    let old_options = serde_json::to_string(&serde_json::json!({
        "theme": "dark",
        "title": null,
        "config": {},
        "request": request.cache_identity(),
    }))
    .unwrap();
    let old_key = FileCache::cache_key(
        VisualizationKind::Mermaid,
        source,
        &old_options,
        "mermaid-rs-renderer@0.3.x+bv3",
        OutputFormat::Svg,
    );
    let stale = FileCache::new()
        .store(
            VisualizationKind::Mermaid,
            &old_key,
            OutputFormat::Svg,
            b"<svg>mmdr</svg>",
        )
        .unwrap();

    let artifact = MermaidDiagram::new(source).render(&request).unwrap();

    assert_eq!(MERMAID_BACKEND, "merman@6e6b1e6554e8+bv3");
    assert!(!artifact.cache_hit);
    assert_ne!(artifact.path, stale);
    assert!(
        !std::fs::read_to_string(artifact.path)
            .unwrap()
            .contains("mmdr")
    );
}

/// The key covers the source as step 3b rewrote it: a `\n` label and its
/// `<br/>` rewrite are one key, a different source another.
#[test]
fn the_key_follows_the_rewritten_source() {
    let request = RenderRequest::default();
    let with_newline = MermaidDiagram::new("classDiagram\n    class A[\"first\\nsecond\"]");
    let rewritten = MermaidDiagram::new("classDiagram\n    class A[\"first<br/>second\"]");
    let other = MermaidDiagram::new("classDiagram\n    class A[\"first<br/>third\"]");

    assert_eq!(key(&with_newline, &request), key(&rewritten, &request));
    assert_ne!(key(&rewritten, &request), key(&other, &request));
}

/// A gitGraph whose tags collide is keyed on the source with its
/// `commitSpacing` directive.
#[test]
fn the_key_covers_the_written_commit_spacing() {
    let request = RenderRequest::default();
    let text = "gitGraph\n    commit id: \"d1\"\n    commit id: \"d2\" tag: \"main\"\n    commit id: \"d3\" tag: \"origin/main\"";
    let diagram = MermaidDiagram::new(text);
    let spaced = diagram.rewritten_source().unwrap();
    assert_ne!(spaced, text, "the tags collide at the default spacing");

    assert_eq!(
        key(&diagram, &request),
        key(&MermaidDiagram::new(spaced), &request)
    );
}

#[test]
fn the_key_follows_the_output_format() {
    let svg = RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    };

    assert_ne!(
        key(&flowchart(), &svg),
        key(&flowchart(), &RenderRequest::default())
    );
}

/// The HTML static target changes SVG bytes, so it has its own SVG key; a
/// PNG is rasterized from the raster-safe SVG under either target, so its
/// key, like the default target's, is unchanged.
#[test]
fn the_html_static_target_has_its_own_svg_key_only() {
    let svg = RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    };
    let png = RenderRequest::default();
    let html_static = flowchart().with_target(MermaidTarget::HtmlStatic);
    let raster_safe = flowchart().with_target(MermaidTarget::RasterSafe);

    assert_ne!(key(&html_static, &svg), key(&flowchart(), &svg));
    assert_eq!(key(&raster_safe, &svg), key(&flowchart(), &svg));
    assert_eq!(key(&html_static, &png), key(&flowchart(), &png));
}

/// One color of the design handed to Merman, through `MermaidConfig` or
/// the theme, changes the key.
#[test]
fn the_key_follows_one_design_color() {
    let request = RenderRequest::default();
    let quadrant = |fill: &str| {
        MermaidDiagram::new("quadrantChart\n    A: [0.2, 0.8]")
            .with_config(MermaidConfig::new().with_quadrant_fill(1, fill))
    };

    assert_ne!(
        key(&quadrant("#112233"), &request),
        key(&quadrant("#112234"), &request)
    );
    assert_eq!(
        key(&quadrant("#112233"), &request),
        key(&quadrant("#112233"), &request)
    );
    assert_ne!(
        key(&flowchart().with_theme(MermaidTheme::Forest), &request),
        key(&flowchart().with_theme(MermaidTheme::Neutral), &request)
    );
}

/// The post-process version is the backend id's `+bv<n>`; a bump changes
/// every key.
#[test]
fn the_key_follows_the_post_process_version() {
    let key_for = |backend: &str| {
        FileCache::cache_key(
            VisualizationKind::Mermaid,
            "flowchart LR\n    A --> B",
            "{}",
            backend,
            OutputFormat::Png,
        )
    };

    assert_ne!(key_for(MERMAID_BACKEND), key_for("merman@6e6b1e6554e8+bv4"));
}

/// The key carries the font chain's cache identity: two chains resolving
/// different faces give two keys, and the same chain the same key.
#[test]
fn the_key_carries_the_font_cache_identity() {
    let request = RenderRequest::default();
    let inter = Arc::new(FontChain::injected(&[], &[], Some("Inter")));
    let no_mermaid_font = Arc::new(FontChain::injected(&[], &[], None));

    let with_inter = flowchart().cache_key_with(&inter, &request).unwrap();
    let without = flowchart()
        .cache_key_with(&no_mermaid_font, &request)
        .unwrap();

    assert_ne!(
        inter.cache_identity(&theme_stacks()),
        no_mermaid_font.cache_identity(&theme_stacks())
    );
    assert_ne!(with_inter, without);
    assert_eq!(
        with_inter,
        flowchart().cache_key_with(&inter, &request).unwrap()
    );
}

fn theme_stacks() -> Vec<crate::fonts::FontQuery> {
    let stack = r#""trebuchet ms", verdana, arial, sans-serif"#;
    vec![
        crate::fonts::FontQuery::from_css(stack, 400, false),
        crate::fonts::FontQuery::from_css(stack, 700, false),
    ]
}

/// The font contents are hashed once per chain, not once per render.
#[test]
fn font_contents_are_hashed_once_across_two_renders() {
    let chain = Arc::new(FontChain::injected(&[], &[], Some("Inter")));
    let request = RenderRequest::default();

    flowchart().cache_key_with(&chain, &request).unwrap();
    let after_first = chain.contents_hashed();
    MermaidDiagram::new("flowchart LR\n    B --> C")
        .cache_key_with(&chain, &request)
        .unwrap();

    assert!(after_first > 0);
    assert_eq!(chain.contents_hashed(), after_first);
}

/// The key is computed before Merman is asked for an SVG: computing it calls
/// Merman not at all for a flowchart, and a cache hit makes no Merman call.
#[test]
#[serial]
fn the_key_is_checked_before_any_merman_svg_call() {
    let _cache = isolated_cache_dir();
    let diagram = flowchart();
    let request = RenderRequest::default();
    take_merman_calls();

    key(&diagram, &request);
    assert!(take_merman_calls().is_empty());

    assert!(!diagram.render(&request).unwrap().cache_hit);
    assert_eq!(take_merman_calls().len(), 1);
    assert!(diagram.render(&request).unwrap().cache_hit);
    assert!(take_merman_calls().is_empty());
}

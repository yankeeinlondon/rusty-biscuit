use std::time::Duration;

use crate::artifact::{Background, OutputFormat, RenderRequest};
use crate::mermaid::render::take_merman_calls;
use crate::mermaid::{MermaidConfig, MermaidDiagram, MermaidError, MermaidTheme, Palette, QuadrantTheme};
use crate::tests::{capture_warnings, isolated_cache_dir};
use serial_test::serial;

const THEMES: [MermaidTheme; 4] = [
    MermaidTheme::Dark,
    MermaidTheme::Default,
    MermaidTheme::Forest,
    MermaidTheme::Neutral,
];

/// How far past its budget a cancelled Merman call may return: Merman stops
/// at its next checkpoint, not at the deadline itself.
const BUDGET_MARGIN: Duration = Duration::from_millis(250);

fn svg_request(background: Background) -> RenderRequest {
    RenderRequest {
        format: OutputFormat::Svg,
        background,
        ..RenderRequest::default()
    }
}

fn render_svg(diagram: &MermaidDiagram, background: Background) -> String {
    let artifact = diagram.render(&svg_request(background)).unwrap();
    std::fs::read_to_string(artifact.path).unwrap()
}

fn render_failure(result: Result<impl std::fmt::Debug, MermaidError>) -> String {
    match result {
        Err(MermaidError::RenderFailed(reason)) => reason,
        other => panic!("expected RenderFailed, got {other:?}"),
    }
}

/// The root `<svg>` element's opening tag.
fn root_tag(svg: &str) -> &str {
    let start = svg.find("<svg").expect("an <svg> root");
    &svg[start..start + svg[start..].find('>').expect("a closed root tag")]
}

fn root_attribute<'a>(svg: &'a str, name: &str) -> &'a str {
    let tag = root_tag(svg);
    let pattern = format!(" {name}=\"");
    let start = tag.find(&pattern).unwrap_or_else(|| panic!("root {name}")) + pattern.len();
    &tag[start..start + tag[start..].find('"').unwrap()]
}

/// A flowchart large enough that Merman needs well over a few milliseconds.
fn large_flowchart() -> String {
    let mut source = String::from("flowchart LR\n");
    for index in 0..15 {
        source.push_str(&format!("    N{index}[Node number {index}] --> N{}\n", index + 1));
    }
    source
}

/// The Merman time of an uncancelled render of `source` plus a comment line,
/// so its cache key differs from the timed render's. Run first, it also warms
/// the process's font chain: a fresh test process otherwise resolves faces
/// for the measurer's styles inside the first Merman call, and that work,
/// stretched by host load, would be timed as Merman overrunning its budget.
fn warm_merman_time(source: &str, request: impl FnOnce(MermaidDiagram)) -> Duration {
    take_merman_calls();
    request(MermaidDiagram::new(format!("{source}\n%% warm-up twin\n")));
    take_merman_calls().into_iter().sum()
}

/// A gitGraph large enough that Merman needs well over a few milliseconds.
fn large_gitgraph() -> String {
    let mut source = String::from("gitGraph\n");
    for index in 0..400 {
        source.push_str(&format!("    commit id: \"c{index}\"\n"));
    }
    source
}

#[test]
#[serial]
fn mermaid_diagram_renders_to_png() {
    let _cache_dir = isolated_cache_dir();

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
    let _cache_dir = isolated_cache_dir();

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
        .with_config(MermaidConfig::new().with_point_label_font_size(14))
        .with_time_budget(Duration::from_millis(750));

    assert_eq!(diagram.instructions(), "graph LR; A-->B");
    assert_eq!(diagram.time_budget(), Duration::from_millis(750));
}

#[test]
fn mermaid_time_budget_defaults_to_two_seconds() {
    let diagram = MermaidDiagram::new("graph LR; A-->B");

    assert_eq!(diagram.time_budget(), Duration::from_secs(2));
    assert_eq!(MermaidDiagram::DEFAULT_TIME_BUDGET, Duration::from_secs(2));
}

#[test]
#[serial]
fn mermaid_renders_to_svg() {
    let _cache_dir = isolated_cache_dir();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let request = RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    };

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert!(artifact.path.exists(), "SVG file should exist");
    assert_eq!(artifact.format, OutputFormat::Svg);

    // Verify the file is valid SVG
    let content = std::fs::read_to_string(&artifact.path).expect("Failed to read SVG");
    assert!(content.contains("<svg"), "Content should be valid SVG");
}

// ---------------------------------------------------------------------------
// Fallback cases (AC-C9, AC-C13): each is a render failure, which callers
// show as the code block
// ---------------------------------------------------------------------------

/// The two deliberately malformed flowcharts (D53): Mermaid.js and Merman
/// reject both.
#[test]
#[serial]
fn malformed_flowcharts_are_parse_errors() {
    let _cache_dir = isolated_cache_dir();

    for source in ["graph TD; A to B", "graph TD; A --&gt; B"] {
        let diagram = MermaidDiagram::new(source);

        let reason = render_failure(diagram.render(&RenderRequest::default()));

        assert!(reason.contains("parse error"), "{source}: {reason}");
        assert!(diagram.fallback_code_block().contains(source));
    }
}

#[test]
#[serial]
fn an_unknown_diagram_type_is_a_render_failure() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("this is not valid mermaid syntax");

    let reason = render_failure(diagram.render(&RenderRequest::default()));

    assert!(reason.contains("No Mermaid diagram type detected"), "{reason}");
}

/// Merman renders ZenUML outside its primary matrix; D20 keeps today's
/// fallback, so no Merman call is made for it.
#[test]
#[serial]
fn zenuml_is_a_render_failure_without_a_merman_call() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("zenuml\n    title Demo\n    Alice->Bob: Hello");
    take_merman_calls();

    let reason = render_failure(diagram.render(&RenderRequest::default()));

    assert!(reason.contains("ZenUML"), "{reason}");
    assert!(take_merman_calls().is_empty());
}

/// D58: Merman's math support is not built in. Merman refuses a diagram with
/// a `$$…$$` label rather than drawing the label as text, so the diagram is
/// a render failure and its source, the math label included, is what the
/// caller shows.
#[test]
#[serial]
fn a_math_label_shows_as_its_source_text() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("flowchart LR\n    A[\"$$x^2$$\"] --> B");

    let reason = render_failure(diagram.render(&RenderRequest::default()));

    assert!(reason.to_lowercase().contains("math"), "{reason}");
    assert!(diagram.fallback_code_block().contains("$$x^2$$"));
}

// ---------------------------------------------------------------------------
// Time budget (AC-C10, library level)
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn an_svg_render_over_its_time_budget_fails_with_a_warning() {
    let _cache_dir = isolated_cache_dir();
    let budget = Duration::from_millis(2);
    let full = warm_merman_time(&large_flowchart(), |twin| {
        twin.render(&RenderRequest::default()).expect("the twin fits 2 s");
    });
    let diagram = MermaidDiagram::new(large_flowchart()).with_time_budget(budget);

    let (result, warnings) = capture_warnings(|| diagram.render(&RenderRequest::default()));

    let reason = render_failure(result);
    assert!(reason.contains("time budget"), "{reason}");
    assert!(
        warnings.iter().any(|warning| warning.contains("time budget")),
        "{warnings:?}"
    );
    let calls = take_merman_calls();
    let merman = calls.last().expect("one Merman call");
    assert!(
        *merman <= budget + BUDGET_MARGIN && *merman < full,
        "Merman ran {merman:?} on a {budget:?} budget; uncancelled {full:?}"
    );
}

#[test]
fn a_gitgraph_geometry_request_over_its_time_budget_fails_with_a_warning() {
    let budget = Duration::from_millis(2);
    let full = warm_merman_time(&large_gitgraph(), |twin| {
        twin.gitgraph_geometry().expect("the twin fits 2 s");
    });
    let diagram = MermaidDiagram::new(large_gitgraph()).with_time_budget(budget);

    let (result, warnings) = capture_warnings(|| diagram.gitgraph_geometry());

    let reason = render_failure(result);
    assert!(reason.contains("time budget"), "{reason}");
    assert!(
        warnings.iter().any(|warning| warning.contains("time budget")),
        "{warnings:?}"
    );
    let calls = take_merman_calls();
    let merman = calls.last().expect("one Merman call");
    assert!(
        *merman <= budget + BUDGET_MARGIN && *merman < full,
        "Merman ran {merman:?} on a {budget:?} budget; uncancelled {full:?}"
    );
}

/// Control for the budget tests: the same diagrams fit the default budget.
#[test]
#[serial]
fn the_large_diagrams_render_within_the_default_budget() {
    let _cache_dir = isolated_cache_dir();

    take_merman_calls();

    MermaidDiagram::new(large_flowchart())
        .render(&svg_request(Background::Transparent))
        .expect("the flowchart fits 2 s");
    MermaidDiagram::new(large_gitgraph())
        .gitgraph_geometry()
        .expect("the gitGraph fits 2 s")
        .expect("a gitGraph");

    // Each needs several times the budget tests' 2 ms.
    let calls = take_merman_calls();
    assert_eq!(calls.len(), 2, "{calls:?}");
    assert!(calls.iter().all(|call| *call > Duration::from_millis(5)), "{calls:?}");
}

// ---------------------------------------------------------------------------
// Themes (AC-C15) and backgrounds
// ---------------------------------------------------------------------------

/// AC-C15 after the palette phase: no theme maps to a Merman built-in theme.
/// Every theme hands Merman the `base` theme and its own fixed palette,
/// compiled; the four render four different SVGs, each carrying its
/// palette's accent strokes.
#[test]
#[serial]
fn each_theme_renders_through_its_compiled_palette_not_a_built_in_theme() {
    let _cache_dir = isolated_cache_dir();
    let source = "flowchart LR\n    A[Start] --> B{Decide}\n    B --> C[Done]";

    let mut rendered = Vec::new();
    for theme in THEMES {
        let diagram = MermaidDiagram::new(source).with_theme(theme);
        assert_eq!(diagram.prepared_site_config()["theme"], "base", "{theme:?}");
        assert_eq!(diagram.palette(), Palette::fixed(theme));

        let svg = render_svg(&diagram, Background::Transparent);
        let palette = Palette::fixed(theme);
        assert!(
            svg.contains(&format!("stroke:{} !important", palette.accents()[0])),
            "{theme:?}: the first node's accent stroke"
        );
        rendered.push(svg);
    }
    for (index, svg) in rendered.iter().enumerate() {
        for other in &rendered[index + 1..] {
            assert_ne!(svg, other, "two themes render the same SVG");
        }
    }
}

/// `Theme` paints the palette's background, `Solid` its own color,
/// and `Transparent` nothing, in SVG and in PNG.
#[test]
#[serial]
fn backgrounds_paint_the_theme_a_solid_color_or_nothing() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("graph LR; A-->B").with_theme(MermaidTheme::Dark);
    let backdrop = |svg: &str| {
        let after_root = &svg[svg.find("<svg").unwrap() + root_tag(svg).len() + 1..];
        after_root
            .starts_with("<rect ")
            .then(|| after_root[..after_root.find("/>").unwrap()].to_string())
    };

    let theme = render_svg(&diagram, Background::Theme);
    let solid = render_svg(&diagram, Background::Solid { r: 0x12, g: 0x34, b: 0x56 });
    let transparent = render_svg(&diagram, Background::Transparent);

    assert!(backdrop(&theme).unwrap().contains("fill=\"#333333\""), "{theme}");
    assert!(backdrop(&solid).unwrap().contains("fill=\"#123456\""), "{solid}");
    assert_eq!(backdrop(&transparent), None, "{transparent}");

    let corner = |background| {
        let artifact = diagram
            .render(&RenderRequest::default().with_background(background))
            .unwrap();
        let png = image::load_from_memory(&std::fs::read(artifact.path).unwrap()).unwrap();
        png.to_rgba8().get_pixel(0, 0).0
    };
    assert_eq!(corner(Background::Theme), [0x33, 0x33, 0x33, 0xff]);
    assert_eq!(
        corner(Background::Solid { r: 0x12, g: 0x34, b: 0x56 }),
        [0x12, 0x34, 0x56, 0xff]
    );
    assert_eq!(corner(Background::Transparent)[3], 0);
}

#[test]
#[serial]
fn mermaid_theme_changes_svg_output() {
    let _cache_dir = isolated_cache_dir();

    let request = RenderRequest {
        format: OutputFormat::Svg,
        scale: 1,
        target_width: None,
        row_height: None,
        background: Background::Theme,
    };

    let light = MermaidDiagram::new("graph LR; A-->B")
        .with_theme(MermaidTheme::Default)
        .render(&request)
        .unwrap();
    let dark = MermaidDiagram::new("graph LR; A-->B")
        .with_theme(MermaidTheme::Dark)
        .render(&request)
        .unwrap();

    assert_ne!(light.path, dark.path);

    let light_svg = std::fs::read_to_string(light.path).unwrap();
    let dark_svg = std::fs::read_to_string(dark.path).unwrap();
    assert_ne!(light_svg, dark_svg);
    // The Dark palette's background (Mermaid.js 12.2.0's dark theme's).
    assert!(dark_svg.contains("fill=\"#333333\""));
}

#[test]
#[serial]
fn mermaid_transparent_background_changes_svg_output() {
    let _cache_dir = isolated_cache_dir();

    let diagram = MermaidDiagram::new("graph LR; A-->B").with_theme(MermaidTheme::Default);
    let opaque = diagram.render(&svg_request(Background::Theme)).unwrap();
    let transparent = diagram.render(&svg_request(Background::Transparent)).unwrap();

    assert_ne!(opaque.path, transparent.path);

    let opaque_svg = std::fs::read_to_string(opaque.path).unwrap();
    let transparent_svg = std::fs::read_to_string(transparent.path).unwrap();
    assert_ne!(opaque_svg, transparent_svg);
    assert!(opaque_svg.contains("fill=\"#ffffff\" style=\"fill:#ffffff"), "{opaque_svg}");
    assert!(!transparent_svg.contains("fill=\"#ffffff\" style=\"fill:#ffffff"), "{transparent_svg}");
}

// ---------------------------------------------------------------------------
// MermaidConfig and QuadrantTheme
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn mermaid_quadrant_config_changes_svg_output() {
    let _cache_dir = isolated_cache_dir();

    let instructions = "quadrantChart\n    title Sample\n    A: [0.2, 0.8]\n    B: [0.7, 0.3]";
    let diagram = MermaidDiagram::new(instructions)
        .with_config(
            MermaidConfig::new()
                .with_quadrant_fill(1, "#112233")
                .with_point_radius(9)
                .with_point_label_font_size(18),
        )
        .with_theme(MermaidTheme::Neutral);
    let svg = render_svg(&diagram, Background::Theme);

    assert!(svg.contains("#112233"));
    assert!(svg.contains(" r=\"9\""));
    assert!(svg.contains("font-size=\"18\""), "point labels at 18 px");
}

/// Every quadrant fill `QuadrantTheme` sets reaches Merman's SVG.
#[test]
#[serial]
fn magic_quadrangle_fills_reach_the_svg() {
    let _cache_dir = isolated_cache_dir();
    let instructions = "quadrantChart\n    title Sample\n    A: [0.2, 0.8]";
    let config = QuadrantTheme::MagicQuadrangle.apply(MermaidConfig::new(), true);

    let svg = render_svg(
        &MermaidDiagram::new(instructions)
            .with_config(config)
            .with_theme(MermaidTheme::Dark),
        Background::Theme,
    );

    for fill in ["#1e2a1e", "#2a1e1e", "#1a1a1a"] {
        assert!(svg.contains(fill), "{fill}");
    }
}

#[test]
#[serial]
fn mermaid_cache_separates_scale_theme_and_transparency() {
    let _cache_dir = isolated_cache_dir();

    let base = MermaidDiagram::new("graph LR; A-->B");
    let scaled = base
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 1,
            target_width: None,
            row_height: None,
            background: Background::Theme,
        })
        .unwrap();
    let scaled_again = base
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 3,
            target_width: None,
            row_height: None,
            background: Background::Theme,
        })
        .unwrap();
    let light = MermaidDiagram::new("graph LR; A-->B")
        .with_theme(MermaidTheme::Default)
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 1,
            target_width: None,
            row_height: None,
            background: Background::Theme,
        })
        .unwrap();
    let transparent = MermaidDiagram::new("graph LR; A-->B")
        .render(&RenderRequest {
            format: OutputFormat::Png,
            scale: 1,
            target_width: None,
            row_height: None,
            background: Background::Transparent,
        })
        .unwrap();

    assert_ne!(scaled.path, scaled_again.path);
    assert_ne!(scaled.path, light.path);
    assert_ne!(scaled.path, transparent.path);
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
    let _cache_dir = isolated_cache_dir();

    let diagram = MermaidDiagram::new("graph LR; A-->B").with_title("My Custom Diagram");
    let request = RenderRequest::default();

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert_eq!(artifact.alt_text, "My Custom Diagram");
}

/// An author's `%%{init}%%` theme variables reach Merman (D54's precedence
/// check).
#[test]
#[serial]
fn mermaid_pie_chart_init_directive_applies_custom_colors() {
    let _cache_dir = isolated_cache_dir();

    let instructions = "%%{init: {'themeVariables': {'pie1': '#3178C6', 'pie2': '#A72145'}}}%%\npie\n    \"TypeScript\" : 45\n    \"Rust\" : 35\n    \"Python\" : 20";
    let svg = render_svg(&MermaidDiagram::new(instructions), Background::Transparent);

    assert!(
        svg.contains("#3178C6"),
        "SVG should contain custom TypeScript color #3178C6"
    );
    assert!(
        svg.contains("#A72145"),
        "SVG should contain custom Rust color #A72145"
    );
}

#[test]
#[serial]
fn mermaid_alt_text_defaults_to_mermaid_diagram() {
    let _cache_dir = isolated_cache_dir();

    let diagram = MermaidDiagram::new("graph LR; A-->B");
    let request = RenderRequest::default();

    let artifact = diagram.render(&request).expect("Failed to render diagram");

    assert_eq!(artifact.alt_text, "Mermaid diagram");
}

// ---------------------------------------------------------------------------
// Natural size
// ---------------------------------------------------------------------------

#[test]
#[serial]
fn mermaid_natural_size_matches_the_rendered_viewbox() {
    let _cache_dir = isolated_cache_dir();

    let instructions = "gitGraph\n    commit id: \"a1\"\n    branch feat/theme\n    checkout feat/theme\n    commit id: \"b2\"";
    for theme in [MermaidTheme::Default, MermaidTheme::Dark] {
        let diagram = MermaidDiagram::new(instructions).with_theme(theme);
        let size = diagram.natural_size().expect("gitGraph measures");
        let svg = render_svg(&diagram, Background::Transparent);
        let viewbox = root_attribute(&svg, "viewBox");
        let values: Vec<f32> = viewbox
            .split_whitespace()
            .map(|v| v.parse().unwrap())
            .collect();
        assert!(
            (values[2] - size.width).abs() < 0.01,
            "{theme:?}: {viewbox} vs {size:?}"
        );
        assert!(
            (values[3] - size.height).abs() < 0.01,
            "{theme:?}: {viewbox} vs {size:?}"
        );
    }
}

/// The PNG the rasterizer draws at scale 1 has the natural size.
#[test]
#[serial]
fn mermaid_natural_size_is_the_drawn_png_size() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("flowchart LR\n    A[Start] --> B[Done]");

    let size = diagram.natural_size().unwrap();
    let artifact = diagram
        .render(&RenderRequest {
            scale: 1,
            ..RenderRequest::default()
        })
        .unwrap();
    let png = image::load_from_memory(&std::fs::read(artifact.path).unwrap()).unwrap();

    assert_eq!(png.width(), size.width.ceil() as u32, "{size:?}");
    assert_eq!(png.height(), size.height.ceil() as u32, "{size:?}");
}

#[test]
#[serial]
fn mermaid_natural_size_rejects_unparseable_input() {
    let _cache_dir = isolated_cache_dir();
    let diagram = MermaidDiagram::new("not a diagram header\n    A --> B");
    assert!(diagram.natural_size().is_err());
}

#[test]
#[serial]
fn mermaid_natural_size_grows_with_commits() {
    let _cache_dir = isolated_cache_dir();
    let short = MermaidDiagram::new("gitGraph\n    commit id: \"a\"")
        .natural_size()
        .unwrap();
    let long = MermaidDiagram::new(
        "gitGraph\n    commit id: \"a\"\n    commit id: \"b\"\n    commit id: \"c\"",
    )
    .natural_size()
    .unwrap();
    assert!(long.width > short.width, "{long:?} vs {short:?}");
}

/// A solid background paints the theme's background color over, and every
/// background and geometry is its own artifact.
#[test]
#[serial]
fn mermaid_render_separates_backgrounds_and_geometry() {
    let _cache_dir = isolated_cache_dir();

    let diagram = MermaidDiagram::new("graph LR; A-->B").with_theme(MermaidTheme::Default);
    let theme = render_svg(&diagram, Background::Theme);
    let solid = render_svg(&diagram, Background::Solid { r: 0x12, g: 0x34, b: 0x56 });
    assert!(!theme.contains("#123456"));
    assert!(solid.contains("#123456"), "{solid}");

    let png = RenderRequest::default().with_target_width(240);
    let requests = [
        png.clone(),
        png.clone().with_background(Background::Transparent),
        png.clone()
            .with_background(Background::Solid { r: 0x12, g: 0x34, b: 0x56 }),
        png.clone().with_row_height(16),
        png.clone().with_row_height(17),
        png.clone().with_target_width(241),
    ];
    let artifacts: Vec<_> = requests
        .iter()
        .map(|request| diagram.render(request).unwrap())
        .collect();
    for (i, a) in artifacts.iter().enumerate() {
        for b in &artifacts[i + 1..] {
            assert_ne!(a.path, b.path);
        }
    }
    let again = diagram.render(&png).unwrap();
    assert!(again.cache_hit);
    assert_eq!(again.path, artifacts[0].path);

    // Row padding on the real diagram: 240 px wide, whole 16 px rows.
    let padded = image::load_from_memory(&std::fs::read(&artifacts[3].path).unwrap()).unwrap();
    assert_eq!(padded.width(), 240);
    assert_eq!(padded.height() % 16, 0);
}

/// Two different diagrams never share a diagram id, and the id is never
/// Merman's default.
#[test]
#[serial]
fn diagram_ids_are_unique_per_diagram() {
    let _cache_dir = isolated_cache_dir();
    let first = render_svg(&MermaidDiagram::new("graph LR; A-->B"), Background::Transparent);
    let second = render_svg(&MermaidDiagram::new("graph LR; A-->C"), Background::Transparent);
    let again = render_svg(&MermaidDiagram::new("graph LR; A-->B"), Background::Theme);

    let ids = [first.as_str(), second.as_str(), again.as_str()].map(|svg| root_attribute(svg, "id"));

    assert!(ids.iter().all(|id| id.starts_with("bv-")), "{ids:?}");
    assert_ne!(ids[0], ids[1]);
    assert!(first.contains(&format!("#{}", ids[0])), "the stylesheet is scoped by the id");
}

//! Smoke render of every family Merman supports (R-C6, R-C8, AC-C7, the
//! phase's "every family" validation checkpoint).
//!
//! One fixture per family of Merman's family registry at the pinned rev
//! (`fixtures/merman/families/`), each rendered through
//! `mermaid::MermaidDiagram` to SVG and PNG under each fixed-palette theme. The SVG
//! artifact is checked for `foreignObject`: it is the SVG the rasterizer
//! receives for the PNG with the same background (one code path builds both).
//! Two tests per family (SVG, PNG) keep each test fast; a guard test keeps the list
//! equal to the registry.
//!
//! The family renders use the goldens' chain of bundled faces
//! (`render_in`): building the host's chain scans every installed face, and
//! dozens of test processes doing so at once pushed each past the L1 budget.
//! One flowchart test renders through the public `render`, so the host's
//! chain is still exercised end to end.

use serial_test::serial;

use super::golden_tests::golden_chain;
use crate::artifact::{Background, OutputFormat, RenderRequest};
use crate::mermaid::{MermaidDiagram, MermaidTheme};
use crate::tests::isolated_cache_dir;

const THEMES: [MermaidTheme; 4] = [
    MermaidTheme::Dark,
    MermaidTheme::Default,
    MermaidTheme::Forest,
    MermaidTheme::Neutral,
];

/// Families of the registry that are not smoke-rendered, with the reason.
const NOT_RENDERED: [(&str, &str); 2] = [
    ("error", "Mermaid's own error diagram, not an input family"),
    (
        "zenuml",
        "falls back to the code block (D20); see mermaid_tests",
    ),
];

fn svg_request() -> RenderRequest {
    RenderRequest {
        format: OutputFormat::Svg,
        background: Background::Theme,
        ..RenderRequest::default()
    }
}

/// Drawn small: the smoke test needs a PNG, not its detail, and
/// rasterization is most of its time.
fn png_request() -> RenderRequest {
    RenderRequest::default().with_target_width(160)
}

fn assert_svg_without_html_labels(artifact_path: &std::path::Path, context: &str) {
    let svg = std::fs::read_to_string(artifact_path).unwrap();
    assert!(svg.contains("<svg"), "{context}");
    assert!(!svg.contains("foreignObject"), "{context} has HTML labels");
}

fn assert_png(artifact_path: &std::path::Path, context: &str) {
    let png = std::fs::read(artifact_path).unwrap();
    assert!(png.starts_with(b"\x89PNG"), "{context}");
}

/// Every fixed-palette theme renders `source` to SVG without HTML labels.
fn smoke_svg(family: &str, source: &str) {
    let _cache = isolated_cache_dir();
    for theme in THEMES {
        let artifact = MermaidDiagram::new(source)
            .with_theme(theme)
            .render_in(golden_chain(), &svg_request())
            .unwrap_or_else(|error| panic!("{family} {theme:?} SVG: {error}"));
        assert_svg_without_html_labels(&artifact.path, &format!("{family} {theme:?}"));
    }
}

/// Every fixed-palette theme renders `source` to PNG.
fn smoke_png(family: &str, source: &str) {
    let _cache = isolated_cache_dir();
    for theme in THEMES {
        let artifact = MermaidDiagram::new(source)
            .with_theme(theme)
            .render_in(golden_chain(), &png_request())
            .unwrap_or_else(|error| panic!("{family} {theme:?} PNG: {error}"));
        assert_png(&artifact.path, &format!("{family} {theme:?}"));
    }
}

/// The public `render`, over the host's installed fonts, draws a family to
/// SVG without HTML labels and to PNG.
#[test]
#[serial]
fn smoke_renders_on_host_fonts_through_the_public_render() {
    let _cache = isolated_cache_dir();
    let source = include_str!("fixtures/merman/families/flowchart.mmd");
    let diagram = MermaidDiagram::new(source);

    let svg = diagram.render(&svg_request()).expect("flowchart SVG on host fonts");
    assert_svg_without_html_labels(&svg.path, "flowchart on host fonts");
    let png = diagram.render(&png_request()).expect("flowchart PNG on host fonts");
    assert_png(&png.path, "flowchart on host fonts");
}

macro_rules! family_smoke_tests {
    ($($test:ident => ($family:literal, $fixture:literal)),+ $(,)?) => {
        /// The registry ids the smoke tests cover.
        const SMOKE_FAMILIES: &[&str] = &[$($family),+];

        /// The fixture each smoke test renders.
        pub(super) const SMOKE_FIXTURES: &[&str] = &[$($fixture),+];

        $(
            mod $test {
                use super::*;

                const SOURCE: &str = include_str!(concat!("fixtures/merman/families/", $fixture, ".mmd"));

                #[test]
                #[serial]
                fn svg() {
                    smoke_svg($family, SOURCE);
                }

                #[test]
                #[serial]
                fn png() {
                    smoke_png($family, SOURCE);
                }
            }
        )+
    };
}

family_smoke_tests! {
    smoke_renders_flowchart => ("flowchart", "flowchart"),
    smoke_renders_swimlane => ("swimlane", "swimlane"),
    smoke_renders_mindmap => ("mindmap", "mindmap"),
    smoke_renders_architecture => ("architecture", "architecture"),
    smoke_renders_sequence => ("sequence", "sequence"),
    smoke_renders_c4 => ("c4", "c4"),
    smoke_renders_kanban => ("kanban", "kanban"),
    smoke_renders_class => ("class", "class"),
    smoke_renders_er => ("er", "er"),
    smoke_renders_gantt => ("gantt", "gantt"),
    smoke_renders_info => ("info", "info"),
    smoke_renders_pie => ("pie", "pie"),
    smoke_renders_requirement => ("requirement", "requirement"),
    smoke_renders_timeline => ("timeline", "timeline"),
    smoke_renders_gitgraph => ("gitGraph", "gitgraph"),
    smoke_renders_state => ("state", "state"),
    smoke_renders_journey => ("journey", "journey"),
    smoke_renders_quadrant_chart => ("quadrantChart", "quadrantchart"),
    smoke_renders_sankey => ("sankey", "sankey"),
    smoke_renders_packet => ("packet", "packet"),
    smoke_renders_xychart => ("xychart", "xychart"),
    smoke_renders_block => ("block", "block"),
    smoke_renders_event_modeling => ("eventmodeling", "eventmodeling"),
    smoke_renders_tree_view => ("treeView", "treeView"),
    smoke_renders_radar => ("radar", "radar"),
    smoke_renders_ishikawa => ("ishikawa", "ishikawa"),
    smoke_renders_treemap => ("treemap", "treemap"),
    smoke_renders_railroad => ("railroad", "railroad"),
    smoke_renders_venn => ("venn", "venn"),
    smoke_renders_wardley => ("wardley", "wardley"),
    smoke_renders_cynefin => ("cynefin", "cynefin"),
    smoke_renders_usecase => ("usecase", "usecase"),
    smoke_renders_agentflow => ("agentflow", "agentflow"),
}

/// The smoke list plus the families deliberately not rendered is exactly
/// Merman's registry, so a family a Merman bump adds fails here until it has
/// a fixture.
#[test]
fn the_smoke_list_is_mermans_family_registry() {
    let mut covered: Vec<&str> = SMOKE_FAMILIES
        .iter()
        .copied()
        .chain(NOT_RENDERED.iter().map(|(family, _)| *family))
        .collect();
    covered.sort_unstable();
    let mut registry: Vec<&str> = merman::DiagramFamilyId::all()
        .iter()
        .map(|family| family.as_str())
        .collect();
    registry.sort_unstable();

    assert_eq!(covered, registry);
}

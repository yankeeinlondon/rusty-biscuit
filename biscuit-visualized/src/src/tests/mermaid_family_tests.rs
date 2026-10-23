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

use serial_test::serial;

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

/// Every fixed-palette theme renders `source` to SVG without HTML labels.
fn smoke_svg(family: &str, source: &str) {
    let _cache = isolated_cache_dir();
    for theme in THEMES {
        let artifact = MermaidDiagram::new(source)
            .with_theme(theme)
            .render(&RenderRequest {
                format: OutputFormat::Svg,
                background: Background::Theme,
                ..RenderRequest::default()
            })
            .unwrap_or_else(|error| panic!("{family} {theme:?} SVG: {error}"));

        let svg = std::fs::read_to_string(&artifact.path).unwrap();
        assert!(svg.contains("<svg"), "{family} {theme:?}");
        assert!(
            !svg.contains("foreignObject"),
            "{family} {theme:?} has HTML labels"
        );
    }
}

/// Every fixed-palette theme renders `source` to PNG, drawn small: the smoke test
/// needs a PNG, not its detail, and rasterization is most of its time.
fn smoke_png(family: &str, source: &str) {
    let _cache = isolated_cache_dir();
    for theme in THEMES {
        let artifact = MermaidDiagram::new(source)
            .with_theme(theme)
            .render(&RenderRequest::default().with_target_width(160))
            .unwrap_or_else(|error| panic!("{family} {theme:?} PNG: {error}"));

        let png = std::fs::read(&artifact.path).unwrap();
        assert!(png.starts_with(b"\x89PNG"), "{family} {theme:?}");
    }
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

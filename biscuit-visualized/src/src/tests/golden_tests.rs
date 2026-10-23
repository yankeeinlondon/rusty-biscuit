//! The golden corpus (R-V1, R-V2, R-V11, AC-V1, AC-V3, AC-C5, AC-C7).
//!
//! Every family fixture (`fixtures/merman/families/`) is rendered under each
//! fixed palette to an SVG snapshot (`goldens/svg/`, `insta`), and under
//! `Dark` and `Default` to a PNG compared within the cross-OS tolerance
//! (`goldens/png/`). Flowchart, sequence, and gitGraph also get SVG snapshots
//! under two synthetic terminal palettes: a full ANSI set and a background
//! only (D47).
//!
//! Every golden pins its palette, and draws with a chain of the bundled faces
//! with `MERMAID_FONT=Inter` and no installed fonts, so no host font or
//! terminal reaches the output. The SVG's diagram id is derived from the
//! cache key, which hashes the font set (Linux bundles Noto Emoji too), so the
//! id is normalized before comparison.
//!
//! Updating: `INSTA_UPDATE=always` rewrites the SVG snapshots, and
//! `BISCUIT_VISUALIZED_UPDATE_GOLDENS=1` the PNGs. A change to either is a
//! change to what users see; review it in the diff.

use std::path::PathBuf;
use std::sync::{Arc, OnceLock};
use std::time::Duration;

use crate::artifact::{Background, OutputFormat, RenderRequest};
use crate::fonts::FontChain;
use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::mermaid::render::take_rasterized_svgs;
use crate::mermaid::{MermaidDiagram, MermaidTheme, Palette, Rgb, TerminalColors};
use crate::tests::isolated_cache_dir;

/// Most pixels that may differ between a PNG and its golden (AE share, D42).
const MAX_AE_SHARE: f64 = 0.001;
/// Largest normalized root-mean-square channel error against the golden (D42).
const MAX_RMSE: f64 = 0.005;

const UPDATE_PNG_ENV: &str = "BISCUIT_VISUALIZED_UPDATE_GOLDENS";

/// The budget is not under test; a loaded runner must not turn a golden into
/// the fallback.
const BUDGET: Duration = Duration::from_secs(60);

pub(super) fn golden_chain() -> &'static Arc<FontChain> {
    static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
    CHAIN.get_or_init(|| Arc::new(FontChain::injected(&[], &[], Some("Inter"))))
}

fn hex(color: &str) -> Rgb {
    Rgb::from_hex(color).expect("a hex color")
}

/// S0's Tokyo-Night-like terminal: every ANSI slot answered (R-P3). The same
/// colors as the themed gallery's terminal column.
fn tokyo_night() -> Palette {
    let ansi = [
        "#15161e", "#f7768e", "#9ece6a", "#e0af68", "#7aa2f7", "#bb9af7", "#7dcfff", "#a9b1d6",
        "#414868", "#ff899d", "#9fe044", "#faba4a", "#8db0ff", "#c7a9ff", "#a4daff", "#c0caf5",
    ];
    let colors = TerminalColors {
        background: Some(hex("#1a1b26")),
        foreground: Some(hex("#c0caf5")),
        ansi: ansi.map(|color| Some(hex(color))),
    };
    Palette::for_terminal(&colors, true)
}

/// A terminal that answered only its background (R-P4).
fn background_only() -> Palette {
    let colors = TerminalColors { background: Some(hex("#1a1b26")), ..TerminalColors::default() };
    Palette::for_terminal(&colors, true)
}

/// Replaces each content-addressed diagram id (`bv-` and 16 hex digits).
fn normalize_ids(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("bv-") {
        let (before, from) = rest.split_at(at);
        out.push_str(before);
        let digits = &from[3..];
        if digits.len() >= 16 && digits.as_bytes()[..16].iter().all(u8::is_ascii_hexdigit) {
            out.push_str("bv-ID");
            rest = &digits[16..];
        } else {
            out.push_str("bv-");
            rest = digits;
        }
    }
    out.push_str(rest);
    out
}

fn diagram(source: &str, palette: Palette) -> MermaidDiagram {
    MermaidDiagram::new(source).with_palette(palette).with_time_budget(BUDGET)
}

/// The golden SVG for `source` under `palette`, as stored: ids normalized.
pub(super) fn golden_svg_text(source: &str, palette: Palette) -> String {
    let _cache = isolated_cache_dir();
    let request = RenderRequest {
        format: OutputFormat::Svg,
        background: Background::Theme,
        ..RenderRequest::default()
    };
    let artifact = diagram(source, palette)
        .render_in(golden_chain(), &request)
        .unwrap_or_else(|error| panic!("SVG render failed: {error}"));
    normalize_ids(&std::fs::read_to_string(&artifact.path).unwrap())
}

fn assert_svg_golden(name: &str, source: &str, palette: Palette) {
    let svg = golden_svg_text(source, palette);
    assert!(!svg.contains("foreignObject"), "{name} has HTML labels");
    let mut settings = insta::Settings::clone_current();
    settings.set_snapshot_path("goldens/svg");
    settings.set_prepend_module_to_snapshot(false);
    settings.set_omit_expression(true);
    settings.bind(|| insta::assert_snapshot!(name, svg));
}

/// Pixel difference between two same-size RGBA images: the share of pixels
/// with any channel different, and the root-mean-square channel error on a
/// 0–1 scale.
pub(super) fn png_difference(actual: &image::RgbaImage, golden: &image::RgbaImage) -> (f64, f64) {
    let mut differing = 0u64;
    let mut squared = 0f64;
    for (a, g) in actual.pixels().zip(golden.pixels()) {
        if a != g {
            differing += 1;
        }
        for channel in 0..4 {
            let delta = (f64::from(a[channel]) - f64::from(g[channel])) / 255.0;
            squared += delta * delta;
        }
    }
    let pixels = u64::from(actual.width()) * u64::from(actual.height());
    let ae_share = differing as f64 / pixels.max(1) as f64;
    let rmse = (squared / (pixels.max(1) * 4) as f64).sqrt();
    (ae_share, rmse)
}

fn png_golden_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/tests/goldens/png")
}

fn assert_png_golden(name: &str, source: &str, theme: MermaidTheme) {
    let _cache = isolated_cache_dir();
    let request = RenderRequest { scale: 1, ..RenderRequest::default() };
    take_rasterized_svgs();
    let artifact = diagram(source, Palette::fixed(theme))
        .render_in(golden_chain(), &request)
        .unwrap_or_else(|error| panic!("{name} PNG render failed: {error}"));
    // AC-C7: the SVG the rasterizer received has no HTML labels.
    let rasterized = take_rasterized_svgs();
    assert_eq!(rasterized.len(), 1, "{name}: one SVG rasterized");
    assert!(!rasterized[0].contains("foreignObject"), "{name} rasterized HTML labels");

    let bytes = std::fs::read(&artifact.path).unwrap();
    let golden_path = png_golden_dir().join(format!("{name}.png"));
    if std::env::var_os(UPDATE_PNG_ENV).is_some() {
        std::fs::create_dir_all(png_golden_dir()).unwrap();
        std::fs::write(&golden_path, &bytes).unwrap();
        return;
    }
    let golden = std::fs::read(&golden_path).unwrap_or_else(|error| {
        panic!("{name}: no golden PNG at {} ({error}); run with {UPDATE_PNG_ENV}=1", golden_path.display())
    });
    let actual = image::load_from_memory(&bytes).unwrap().to_rgba8();
    let golden = image::load_from_memory(&golden).unwrap().to_rgba8();
    assert_eq!(actual.dimensions(), golden.dimensions(), "{name}: PNG size changed");
    let (ae_share, rmse) = png_difference(&actual, &golden);
    // Printed for the cross-OS record (`--no-capture`).
    eprintln!("png-golden {name}: ae_share={ae_share:.6} rmse={rmse:.6}");
    assert!(
        ae_share <= MAX_AE_SHARE && rmse <= MAX_RMSE,
        "{name}: AE share {ae_share:.6} (max {MAX_AE_SHARE}), RMSE {rmse:.6} (max {MAX_RMSE}); actual PNG at {}",
        artifact.path.display()
    );
}

/// One module per family: an SVG golden under each fixed palette and a PNG
/// golden under `Dark` and `Default`, each its own test.
macro_rules! family_goldens {
    ($($module:ident => $fixture:literal),+ $(,)?) => {
        /// The fixture names with goldens, in registry order.
        pub(super) const GOLDEN_FIXTURES: &[&str] = &[$($fixture),+];

        /// Every golden input, for the corpus-wide checks.
        pub(super) const GOLDEN_SOURCES: &[(&str, &str)] = &[
            $(($fixture, include_str!(concat!("fixtures/merman/families/", $fixture, ".mmd")))),+
        ];

        $(
            mod $module {
                use super::*;

                const SOURCE: &str = include_str!(concat!("fixtures/merman/families/", $fixture, ".mmd"));

                #[test]
                fn svg_dark() {
                    assert_svg_golden(concat!($fixture, "_dark"), SOURCE, Palette::fixed(MermaidTheme::Dark));
                }

                #[test]
                fn svg_default() {
                    assert_svg_golden(concat!($fixture, "_default"), SOURCE, Palette::fixed(MermaidTheme::Default));
                }

                #[test]
                fn svg_forest() {
                    assert_svg_golden(concat!($fixture, "_forest"), SOURCE, Palette::fixed(MermaidTheme::Forest));
                }

                #[test]
                fn svg_neutral() {
                    assert_svg_golden(concat!($fixture, "_neutral"), SOURCE, Palette::fixed(MermaidTheme::Neutral));
                }

                #[test]
                fn png_dark() {
                    assert_png_golden(concat!($fixture, "_dark"), SOURCE, MermaidTheme::Dark);
                }

                #[test]
                fn png_default() {
                    assert_png_golden(concat!($fixture, "_default"), SOURCE, MermaidTheme::Default);
                }
            }
        )+
    };
}

family_goldens! {
    golden_flowchart => "flowchart",
    golden_swimlane => "swimlane",
    golden_mindmap => "mindmap",
    golden_architecture => "architecture",
    golden_sequence => "sequence",
    golden_c4 => "c4",
    golden_kanban => "kanban",
    golden_class => "class",
    golden_er => "er",
    golden_gantt => "gantt",
    golden_info => "info",
    golden_pie => "pie",
    golden_requirement => "requirement",
    golden_timeline => "timeline",
    golden_gitgraph => "gitgraph",
    golden_state => "state",
    golden_journey => "journey",
    golden_quadrant_chart => "quadrantchart",
    golden_sankey => "sankey",
    golden_packet => "packet",
    golden_xychart => "xychart",
    golden_block => "block",
    golden_event_modeling => "eventmodeling",
    golden_tree_view => "treeView",
    golden_radar => "radar",
    golden_ishikawa => "ishikawa",
    golden_treemap => "treemap",
    golden_railroad => "railroad",
    golden_venn => "venn",
    golden_wardley => "wardley",
    golden_cynefin => "cynefin",
    golden_usecase => "usecase",
    golden_agentflow => "agentflow",
}

/// The derived-palette goldens (D47): flowchart, sequence, and gitGraph under
/// the full-ANSI and background-only terminals.
pub(super) fn derived_palettes() -> [(&'static str, Palette); 2] {
    [("terminal_ansi", tokyo_night()), ("terminal_background", background_only())]
}

macro_rules! derived_goldens {
    ($($test:ident => ($fixture:literal, $palette:literal)),+ $(,)?) => {
        /// `(fixture, palette name)` of every derived-palette golden.
        pub(super) const DERIVED_GOLDENS: &[(&str, &str)] = &[$(($fixture, $palette)),+];

        $(
            #[test]
            fn $test() {
                let source = include_str!(concat!("fixtures/merman/families/", $fixture, ".mmd"));
                let palette = derived_palettes()
                    .into_iter()
                    .find(|(name, _)| *name == $palette)
                    .map(|(_, palette)| palette)
                    .unwrap();
                assert_svg_golden(concat!($fixture, "_", $palette), source, palette);
            }
        )+
    };
}

derived_goldens! {
    derived_golden_flowchart_ansi => ("flowchart", "terminal_ansi"),
    derived_golden_flowchart_background => ("flowchart", "terminal_background"),
    derived_golden_sequence_ansi => ("sequence", "terminal_ansi"),
    derived_golden_sequence_background => ("sequence", "terminal_background"),
    derived_golden_gitgraph_ansi => ("gitgraph", "terminal_ansi"),
    derived_golden_gitgraph_background => ("gitgraph", "terminal_background"),
}

/// The golden list is the smoke list: every family Merman renders has
/// goldens (AC-C5), so a family added to the registry fails the smoke guard
/// and then this one until it has goldens.
#[test]
fn every_smoke_family_has_goldens() {
    let mut goldens: Vec<&str> = GOLDEN_FIXTURES.to_vec();
    goldens.sort_unstable();
    let mut smoke: Vec<&str> = super::mermaid_family_tests::SMOKE_FIXTURES.to_vec();
    smoke.sort_unstable();
    assert_eq!(goldens, smoke);
}

/// Every derived-palette golden names a palette that exists, and the six of
/// D47 are all present.
#[test]
fn the_six_derived_goldens_cover_three_families_and_two_palettes() {
    let palettes: Vec<&str> = derived_palettes().iter().map(|(name, _)| *name).collect();
    for family in ["flowchart", "sequence", "gitgraph"] {
        for palette in &palettes {
            assert!(DERIVED_GOLDENS.contains(&(family, palette)), "{family} × {palette}");
        }
    }
    assert_eq!(DERIVED_GOLDENS.len(), 6);
}

/// AC-V1: every code point in a golden input is one Inter maps (both faces),
/// so the goldens never reach a fallback face.
#[test]
fn every_golden_input_code_point_is_mapped_by_inter() {
    let faces = [INTER_REGULAR, INTER_BOLD].map(|bytes| ttf_parser::Face::parse(bytes, 0).unwrap());
    let mut unmapped = Vec::new();
    for (fixture, source) in GOLDEN_SOURCES {
        for c in source.chars().filter(|c| !c.is_control()) {
            for face in &faces {
                if face.glyph_index(c).is_none_or(|glyph| glyph.0 == 0) {
                    unmapped.push(format!("{fixture}: U+{:04X} {c:?}", c as u32));
                }
            }
        }
    }
    unmapped.dedup();
    assert!(unmapped.is_empty(), "code points Inter does not map:\n{}", unmapped.join("\n"));
}

#[test]
fn normalize_ids_replaces_only_sixteen_hex_digits() {
    assert_eq!(
        normalize_ids(r#"<svg id="bv-0123456789abcdef"><g class="bv-x">#bv-fedcba9876543210 .x"#),
        r#"<svg id="bv-ID"><g class="bv-x">#bv-ID .x"#
    );
    assert_eq!(normalize_ids("bv-012"), "bv-012");
}

#[test]
fn png_difference_counts_differing_pixels_and_channel_error() {
    let golden = image::RgbaImage::from_pixel(10, 10, image::Rgba([0, 0, 0, 255]));
    let mut actual = golden.clone();
    actual.put_pixel(0, 0, image::Rgba([255, 0, 0, 255]));
    let (ae_share, rmse) = png_difference(&actual, &golden);
    assert_eq!(ae_share, 0.01);
    assert!((rmse - (1.0f64 / 400.0).sqrt()).abs() < 1e-12);
    assert_eq!(png_difference(&golden, &golden), (0.0, 0.0));
}

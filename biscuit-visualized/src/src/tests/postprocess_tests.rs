//! The Merman SVG post-process (D33, R-M9).
//!
//! - **Divider guard (R-V10, AC-M7):** class and ER dividers in the dark theme
//!   are painted after the post-process, and the rows unpainted before it.
//! - **`sans-serif` default:** text with no font set gets `sans-serif`; text
//!   with a font keeps it.
//! - **No-op and idempotence:** an SVG with neither defect comes back byte for
//!   byte, and a second pass changes nothing.
//!
//! Diagrams render through Merman's resvg-safe pipeline with our measurer over
//! an injected chain of the bundled faces, so the result does not depend on
//! the host's fonts.

use std::borrow::Cow;
use std::sync::{Arc, OnceLock};

use merman::svg::{
    MeasurementProfileId, SvgEnvironment, SvgPipeline, TextMeasurementPolicy,
    TextMeasurementProfile, TextMeasurementProfileIdentity,
};
use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest,
};
use resvg::tiny_skia::Pixmap;
use resvg::usvg::{self, Node};

use crate::fonts::{FontChain, GenericFamily, LabelMeasurer};
use crate::mermaid::postprocess::postprocess;
use crate::raster::usvg_options;

pub(super) const CLASS: &str = "classDiagram\n  class Animal {\n    +String name\n    +eat()\n  }";
pub(super) const ER: &str = "erDiagram\n  CUSTOMER {\n    string name\n    int id\n  }";
pub(super) const FLOWCHART: &str = "flowchart LR\n  A --> B";
/// Wardley is the family whose output sets no font anywhere (S1).
pub(super) const WARDLEY: &str = "wardley-beta\ntitle Tea\nanchor Business [0.95, 0.63]";

/// Rasterizer scale: a 1.3 px divider covers at least one whole pixel row.
const SCALE: f32 = 2.0;

pub(super) fn chain() -> &'static Arc<FontChain> {
    static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
    CHAIN.get_or_init(|| {
        Arc::new(FontChain::injected(
            &[],
            &[(GenericFamily::SansSerif, "Inter")],
            None,
        ))
    })
}

pub(super) fn render(source: &str, theme: &str) -> String {
    let identity = TextMeasurementProfileIdentity::new(
        MeasurementProfileId::new("biscuit-visualized-test").expect("profile id"),
        "1",
    )
    .expect("profile identity");
    let environment = SvgEnvironment::deterministic().with_text_measurement_policy(
        TextMeasurementPolicy::uniform(TextMeasurementProfile::new(
            identity,
            LabelMeasurer::text_measurer(chain()),
        )),
    );
    let mut request = SvgRequest {
        environment,
        ..SvgRequest::default()
    };
    request.options.diagram_id = Some("pp".into());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    let renderer = Renderer::new().with_engine(Engine::new().with_site_config(
        MermaidConfig::from_value(serde_json::json!({ "theme": theme })),
    ));
    match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => svg.svg().to_string(),
        other => panic!("Merman renders the fixture: {other:?}"),
    }
}

pub(super) fn tree(svg: &str) -> usvg::Tree {
    usvg::Tree::from_str(svg, &usvg_options(chain())).expect("usvg parses the SVG")
}

fn rasterize(tree: &usvg::Tree) -> Pixmap {
    let size = tree.size();
    let mut pixmap = Pixmap::new(
        (size.width() * SCALE).ceil() as u32,
        (size.height() * SCALE).ceil() as u32,
    )
    .expect("pixmap");
    resvg::render(
        tree,
        usvg::Transform::from_scale(SCALE, SCALE),
        &mut pixmap.as_mut(),
    );
    pixmap
}

/// A divider: a stroked path whose bounding box has zero height (`Row`) or
/// zero width (`Column`), in canvas units.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Divider {
    Row { left: f32, right: f32, y: f32 },
    Column { top: f32, bottom: f32, x: f32 },
}

/// Every distinct divider in `group`. Merman draws some dividers twice.
fn dividers(group: &usvg::Group, into: &mut Vec<Divider>) {
    for node in group.children() {
        let divider = match node {
            Node::Group(child) => {
                dividers(child, into);
                continue;
            }
            Node::Path(path) if path.stroke().is_some() => {
                let bounds = path.abs_bounding_box();
                if bounds.height() < 0.01 && bounds.width() > 10.0 {
                    Divider::Row {
                        left: bounds.left(),
                        right: bounds.right(),
                        y: bounds.top(),
                    }
                } else if bounds.width() < 0.01 && bounds.height() > 10.0 {
                    Divider::Column {
                        top: bounds.top(),
                        bottom: bounds.bottom(),
                        x: bounds.left(),
                    }
                } else {
                    continue;
                }
            }
            _ => continue,
        };
        if !into.contains(&divider) {
            into.push(divider);
        }
    }
}

/// Whether the pixel at `(x, y)` is at least `#a0` in some channel. The dark
/// theme's divider stroke is `#ccc` over node fills no lighter than `#2d2d2d`.
fn bright(pixmap: &Pixmap, x: i64, y: i64) -> bool {
    let (Ok(x), Ok(y)) = (u32::try_from(x), u32::try_from(y)) else {
        return false;
    };
    pixmap.pixel(x, y).is_some_and(|pixel| {
        let color = pixel.demultiply();
        color.red().max(color.green()).max(color.blue()) >= 0xa0
    })
}

/// The share of points along the middle 60% of `divider` with a bright pixel
/// on the line or the pixel before it (the stroke straddles the line).
fn painted_share(pixmap: &Pixmap, divider: Divider) -> f32 {
    let (start, end, across) = match divider {
        Divider::Row { left, right, y } => (left, right, y),
        Divider::Column { top, bottom, x } => (top, bottom, x),
    };
    let from = ((start + 0.2 * (end - start)) * SCALE) as i64;
    let to = ((end - 0.2 * (end - start)) * SCALE) as i64;
    let line = (across * SCALE).round() as i64;
    let painted = (from..to)
        .filter(|&along| {
            (line - 1..=line).any(|across| match divider {
                Divider::Row { .. } => bright(pixmap, along, across),
                Divider::Column { .. } => bright(pixmap, across, along),
            })
        })
        .count();
    painted as f32 / (to - from) as f32
}

/// Renders `source` in the dark theme and checks every divider: painted after
/// the post-process, and every row mostly unpainted before it. Returns the
/// dividers checked.
fn check_dividers(source: &str) -> Vec<Divider> {
    let raw = render(source, "dark");
    let fixed = postprocess(&raw);
    let fixed_tree = tree(&fixed);
    let mut found = Vec::new();
    dividers(fixed_tree.root(), &mut found);

    let fixed_pixels = rasterize(&fixed_tree);
    let raw_pixels = rasterize(&tree(&raw));
    for &divider in &found {
        let after = painted_share(&fixed_pixels, divider);
        assert!(
            after > 0.9,
            "{divider:?} is painted after the post-process: {after}"
        );
        // Load-bearing check: Merman's own output leaves this row unpainted
        // (class: a zero-height path, nothing drawn; ER: a 0.0001-tall
        // outline whose gradient fades to the background within its first
        // third). If this fails, Merman has fixed the defect upstream and the
        // gradient-stroke fix is dead code to remove (D33). Columns are not
        // checked: the gradient's pad spread paints the left half of a
        // near-zero-width stroke in the first stop, so they half-show anyway.
        if matches!(divider, Divider::Row { .. }) {
            let before = painted_share(&raw_pixels, divider);
            assert!(
                before < 0.5,
                "{divider:?} is unpainted in Merman's output: {before}"
            );
        }
    }
    found
}

#[test]
fn postprocess_paints_class_dividers_in_dark_theme() {
    let found = check_dividers(CLASS);
    // Two rows: under the name, and between attributes and methods.
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(
        found
            .iter()
            .all(|divider| matches!(divider, Divider::Row { .. }))
    );
}

#[test]
fn postprocess_paints_er_dividers_in_dark_theme() {
    let found = check_dividers(ER);
    // One row under the name, one column between attribute type and name.
    assert!(
        found
            .iter()
            .any(|divider| matches!(divider, Divider::Row { .. })),
        "{found:?}"
    );
    assert!(
        found
            .iter()
            .any(|divider| matches!(divider, Divider::Column { .. })),
        "{found:?}"
    );
}

/// The font families usvg resolved for every text span in `group`.
fn span_families(group: &usvg::Group, into: &mut Vec<Vec<usvg::FontFamily>>) {
    for node in group.children() {
        match node {
            Node::Group(child) => span_families(child, into),
            Node::Text(text) => into.extend(
                text.chunks()
                    .iter()
                    .flat_map(|chunk| chunk.spans())
                    .map(|span| span.font().families().to_vec()),
            ),
            _ => {}
        }
    }
}

pub(super) fn families(svg: &str) -> Vec<Vec<usvg::FontFamily>> {
    let mut found = Vec::new();
    span_families(tree(svg).root(), &mut found);
    assert!(!found.is_empty(), "the SVG draws text");
    found
}

#[test]
fn postprocess_gives_unset_merman_text_sans_serif() {
    let raw = render(WARDLEY, "dark");
    let sans = vec![usvg::FontFamily::SansSerif];
    // Load-bearing check: Merman's wardley output sets no font. If this
    // fails, the default may be dead for this family; find another or drop it.
    assert!(families(&raw).iter().all(|family| *family != sans));
    assert!(
        families(&postprocess(&raw))
            .iter()
            .all(|family| *family == sans)
    );
}

#[test]
fn postprocess_defaults_only_text_without_a_font() {
    let svg = concat!(
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="40" height="20">"#,
        r#"<text y="10" font-family="Inter">set</text><text y="18">unset</text></svg>"#,
    );
    let found = families(&postprocess(svg));
    assert_eq!(
        found,
        vec![
            vec![usvg::FontFamily::Named("Inter".into())],
            vec![usvg::FontFamily::SansSerif],
        ]
    );
}

#[test]
fn postprocess_keeps_merman_root_font_stack() {
    let raw = render(CLASS, "dark");
    let fixed = postprocess(&raw);
    assert!(!fixed.contains(r#"font-family="sans-serif""#));
    assert_eq!(families(&fixed), families(&raw));
    assert!(
        families(&fixed)
            .iter()
            .all(|family| family.first() == Some(&usvg::FontFamily::Named("trebuchet ms".into())))
    );
}

#[test]
fn postprocess_leaves_svg_without_defects_unchanged() {
    let raw = render(FLOWCHART, "default");
    let result = postprocess(&raw);
    assert!(
        matches!(result, Cow::Borrowed(_)),
        "nothing to fix, nothing allocated"
    );
    assert_eq!(result, raw);
}

#[test]
fn postprocess_is_idempotent_and_adds_no_overflow_patch() {
    for (source, theme) in [
        (CLASS, "dark"),
        (ER, "forest"),
        (WARDLEY, "dark"),
        (FLOWCHART, "neutral"),
    ] {
        let raw = render(source, theme);
        let once = postprocess(&raw).into_owned();
        assert_eq!(postprocess(&once), once, "{theme} {source:?}");
        // Merman's stylesheet and label viewports carry their own
        // `overflow:visible`; the post-process adds none (D33).
        assert_eq!(
            once.matches("overflow:visible").count(),
            raw.matches("overflow:visible").count(),
            "{theme} {source:?}"
        );
        assert!(!once.contains("stroke:url("), "{theme} {source:?}");
    }
}


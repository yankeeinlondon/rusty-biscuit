//! The "HTML static" target (D37, D60, R-C9a, AC-C8a): every label's
//! `font-family` leads with the face it was measured in, followed by the
//! original stack, and label viewports let text overflow. Raster output, and
//! the default SVG, carry neither change.
//!
//! The post-process tests render through Merman over the injected chain of
//! `postprocess_tests` (only the bundled faces; `sans-serif` answers Inter),
//! so the measured face is Inter on every host. The end-to-end tests go
//! through `MermaidDiagram::render` with the host's chain.

use std::borrow::Cow;

use resvg::usvg;
use serial_test::serial;

use super::postprocess_tests::{CLASS, ER, FLOWCHART, WARDLEY, chain, families, render};
use crate::artifact::{OutputFormat, RenderRequest};
use crate::fonts::{FontQuery, LabelMeasurer, default_chain};
use crate::mermaid::postprocess::{html_static, postprocess};
use crate::mermaid::render::take_rasterized_svgs;
use crate::mermaid::{MermaidDiagram, MermaidTarget};
use crate::raster::usvg_options;
use crate::tests::isolated_cache_dir;

const SEQUENCE: &str = "sequenceDiagram\n  Alice->>Bob: Hi there\n  Note right of Bob: thinks";
const GANTT: &str = "gantt\n  title G\n  section S\n  Task :a1, 2024-01-01, 3d";
const THEME_STACK: &str = r#""trebuchet ms", verdana, arial, sans-serif"#;

/// The raster-safe SVG (Merman's output, post-processed) and its HTML static
/// counterpart.
fn raster_and_html(source: &str) -> (String, String) {
    let raster = postprocess(&render(source, "dark")).into_owned();
    let html = html_static(&raster, &LabelMeasurer::new(chain())).into_owned();
    (raster, html)
}

/// Every `<svg>` start tag after the root's.
fn nested_svg_tags(svg: &str) -> Vec<&str> {
    svg.match_indices("<svg")
        .skip(1)
        .map(|(at, _)| &svg[at..at + svg[at..].find('>').expect("tag ends")])
        .collect()
}

fn root_tag(svg: &str) -> &str {
    &svg[..svg.find('>').expect("root tag ends")]
}

/// Each text span's families in `html` are `face` followed by that span's
/// families in `raster`.
fn assert_led_by(raster_families: &[Vec<usvg::FontFamily>], html_families: &[Vec<usvg::FontFamily>], face: &str) {
    assert_eq!(raster_families.len(), html_families.len());
    for (raster, html) in raster_families.iter().zip(html_families) {
        let mut expected = vec![usvg::FontFamily::Named(face.to_string())];
        expected.extend(raster.iter().cloned());
        assert_eq!(*html, expected);
    }
}

#[test]
fn html_static_leads_every_label_font_with_the_measured_face() {
    // Flowchart and class labels set their font inline, sequence labels
    // inherit the root rule, gantt mixes rules and a `font-family`
    // attribute, and wardley's only font is the post-process's default.
    for source in [FLOWCHART, CLASS, SEQUENCE, GANTT, WARDLEY] {
        let (raster, html) = raster_and_html(source);
        assert_led_by(&families(&raster), &families(&html), "Inter");
    }
}

#[test]
fn html_static_names_the_face_the_measurer_resolves() {
    // The injected chain has no Trebuchet MS, Verdana, or Arial: the theme
    // stack is measured in its `sans-serif` answer.
    let face = chain()
        .resolve(&FontQuery::from_css(THEME_STACK, 400, false))
        .and_then(|id| chain().database().face(id))
        .map(|face| face.families[0].0.clone());
    assert_eq!(face.as_deref(), Some("Inter"));
    let (_, html) = raster_and_html(FLOWCHART);
    assert!(html.contains(r#"#pp{font-family:'Inter',"trebuchet ms",verdana,arial,sans-serif;"#));
}

#[test]
fn html_static_lets_label_viewports_overflow() {
    for source in [CLASS, ER] {
        let (raster, html) = raster_and_html(source);
        // Load-bearing: Merman clips these labels to their measured boxes.
        // If this fails, Merman stopped clipping and the overflow edit is
        // dead for this family.
        assert!(
            nested_svg_tags(&raster).iter().any(|tag| tag.contains("overflow:hidden")),
            "{source:?}"
        );
        let viewports = nested_svg_tags(&html);
        assert!(!viewports.is_empty());
        for tag in viewports {
            assert!(tag.contains("overflow:visible"), "{tag}");
            assert!(!tag.contains("overflow:hidden"), "{tag}");
        }
        assert!(!root_tag(&html).contains("overflow"), "the root is left alone");
    }
}

#[test]
fn html_static_edits_only_font_values_and_text_viewports() {
    let svg = concat!(
        r#"<svg id="r" xmlns="http://www.w3.org/2000/svg">"#,
        r#"<style>#r{font-family:Verdana;}</style>"#,
        r#"<svg width="9" height="9"><text style="font-family:&quot;a b&quot;">font-family:x</text></svg>"#,
        r#"<svg style="overflow:hidden"><rect width="1" height="1"/></svg>"#,
        "</svg>",
    );
    let html = html_static(svg, &LabelMeasurer::new(chain()));
    assert_eq!(
        html,
        concat!(
            r#"<svg id="r" xmlns="http://www.w3.org/2000/svg">"#,
            r#"<style>#r{font-family:'Inter',Verdana;}</style>"#,
            r#"<svg width="9" height="9" overflow="visible"><text style="font-family:'Inter',&quot;a b&quot;">font-family:x</text></svg>"#,
            r#"<svg style="overflow:hidden"><rect width="1" height="1"/></svg>"#,
            "</svg>",
        ),
        "text content and a viewport without text are untouched"
    );
}

#[test]
fn html_static_is_idempotent() {
    for source in [FLOWCHART, CLASS, ER, SEQUENCE, GANTT, WARDLEY] {
        let (_, once) = raster_and_html(source);
        let twice = html_static(&once, &LabelMeasurer::new(chain()));
        assert!(matches!(twice, Cow::Borrowed(_)), "{source:?}");
        assert_eq!(twice, once, "{source:?}");
    }
}

// ---------------------------------------------------------------------------
// Through `MermaidDiagram::render`, with the host's chain
// ---------------------------------------------------------------------------

fn svg_request() -> RenderRequest {
    RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    }
}

fn render_svg(diagram: &MermaidDiagram) -> String {
    let artifact = diagram.render(&svg_request()).expect("the diagram renders");
    std::fs::read_to_string(artifact.path).expect("the SVG is readable")
}

fn host_families(svg: &str) -> Vec<Vec<usvg::FontFamily>> {
    let tree = usvg::Tree::from_str(svg, &usvg_options(default_chain())).expect("usvg parses the SVG");
    let mut found = Vec::new();
    collect_families(tree.root(), &mut found);
    assert!(!found.is_empty(), "the SVG draws text");
    found
}

fn collect_families(group: &usvg::Group, into: &mut Vec<Vec<usvg::FontFamily>>) {
    for node in group.children() {
        match node {
            usvg::Node::Group(child) => collect_families(child, into),
            usvg::Node::Text(text) => into.extend(
                text.chunks()
                    .iter()
                    .flat_map(|chunk| chunk.spans())
                    .map(|span| span.font().families().to_vec()),
            ),
            _ => {}
        }
    }
}

/// The family the host's chain measures the theme stack in.
fn host_measured_face() -> String {
    let chain = default_chain();
    chain
        .resolve(&FontQuery::from_css(THEME_STACK, 400, false))
        .and_then(|id| chain.database().face(id))
        .map(|face| face.families[0].0.clone())
        .expect("the host resolves the theme stack")
}

#[test]
#[serial]
fn html_static_target_svg_leads_label_fonts_with_the_measured_face() {
    let _cache = isolated_cache_dir();
    let face = host_measured_face();
    for source in [FLOWCHART, SEQUENCE] {
        let raster = render_svg(&MermaidDiagram::new(source));
        let html = render_svg(&MermaidDiagram::new(source).with_target(MermaidTarget::HtmlStatic));
        assert_led_by(&host_families(&raster), &host_families(&html), &face);
    }
}

#[test]
#[serial]
fn default_target_svg_keeps_original_fonts_and_clipping() {
    let _cache = isolated_cache_dir();
    let face = host_measured_face();
    let raster = render_svg(&MermaidDiagram::new(CLASS));
    assert_eq!(MermaidDiagram::new(CLASS).target(), MermaidTarget::RasterSafe);
    assert!(!raster.contains(&format!("'{face}',")));
    assert!(
        host_families(&raster)
            .iter()
            .all(|family| family.first() == Some(&usvg::FontFamily::Named("trebuchet ms".into())))
    );
    assert!(nested_svg_tags(&raster).iter().any(|tag| tag.contains("overflow:hidden")));

    let html = render_svg(&MermaidDiagram::new(CLASS).with_target(MermaidTarget::HtmlStatic));
    assert!(html.contains(&format!("'{face}',")));
    assert!(!nested_svg_tags(&html).iter().any(|tag| tag.contains("overflow:hidden")));
}

#[test]
#[serial]
fn png_render_rasterizes_the_raster_safe_svg_under_the_html_static_target() {
    let face = host_measured_face();
    let png = RenderRequest::default();
    assert_eq!(png.format, OutputFormat::Png);
    take_rasterized_svgs();

    // Separate caches: the two renders share a key, so the second would
    // otherwise be a cache hit and rasterize nothing.
    let html_cache = isolated_cache_dir();
    MermaidDiagram::new(CLASS)
        .with_target(MermaidTarget::HtmlStatic)
        .render(&png)
        .expect("the PNG renders");
    drop(html_cache);
    let default_cache = isolated_cache_dir();
    MermaidDiagram::new(CLASS).render(&png).expect("the PNG renders");
    drop(default_cache);

    let rasterized = take_rasterized_svgs();
    assert_eq!(rasterized.len(), 2);
    assert_eq!(rasterized[0], rasterized[1]);
    assert!(!rasterized[0].contains(&format!("'{face}',")));
    assert!(nested_svg_tags(&rasterized[0]).iter().any(|tag| tag.contains("overflow:hidden")));
}


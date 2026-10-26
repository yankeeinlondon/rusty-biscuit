//! The embedded-font opt-in on the render path: the face each family is
//! measured in, the plain-advance measurement, the label `font-family` lead,
//! the cache identity and record, and the page's font built from rendered
//! diagrams.
//!
//! Every render goes through `MermaidDiagram::render_with_font_in` over an
//! injected chain (the bundled faces, plus copies of Inter renamed `Arial`
//! with a patched `fsType`), so results do not depend on the host's fonts.

use std::sync::Arc;

use base64::Engine as _;
use base64::engine::general_purpose::STANDARD as BASE64;
use merman::svg::TextStyle;
use resvg::usvg;
use serial_test::serial;

use super::embed_faces_tests::{arial, chain};
use super::embed_permission_tests::collection;
use super::embed_subset_tests::tables_of;
use super::golden_tests::golden_chain;
use super::postprocess_tests::{CLASS, FLOWCHART, families};
use crate::artifact::{OutputFormat, RenderRequest, RenderedArtifact};
use crate::fonts::bundled::{INTER_BOLD, INTER_REGULAR};
use crate::fonts::{FontChain, FontQuery, LabelMeasurer};
use crate::mermaid::embed::subset::subset;
use crate::mermaid::{DiagramFont, MermaidDiagram, MermaidTarget, page_font};
use crate::tests::{capture_warnings, isolated_cache_dir};

const STACK: &str = r#""trebuchet ms", verdana, arial, sans-serif"#;
/// Wide enough to grow its node past the minimum width, short enough not to
/// wrap, and full of `AV` pairs, which Inter kerns.
const KERNED: &str = "flowchart LR\n  A[AVAVAVAVAVAVAVAV] --> B";
const TAGGED_GITGRAPH: &str = "gitGraph\n  commit tag: \"v1.0.0\"\n  commit tag: \"v1.1.0\"";
const RESTRICTED: u16 = 0x0002;
const INSTALLABLE: u16 = 0x0000;

fn svg_request() -> RenderRequest {
    RenderRequest {
        format: OutputFormat::Svg,
        ..RenderRequest::default()
    }
}

fn html_static(source: &str, embed: bool) -> MermaidDiagram {
    MermaidDiagram::new(source)
        .with_target(MermaidTarget::HtmlStatic)
        .with_embedded_font(embed)
}

/// A chain whose only installed family is `Arial` at `fs_type` (both weights).
fn arial_chain(fs_type: u16, mermaid_font: Option<&str>) -> Arc<FontChain> {
    Arc::new(chain(&arial(fs_type, fs_type), mermaid_font))
}

fn render(
    diagram: &MermaidDiagram,
    chain: &Arc<FontChain>,
    request: &RenderRequest,
) -> (RenderedArtifact, Option<DiagramFont>, Vec<u8>) {
    let (artifact, font) = diagram.render_with_font_in(chain, request).expect("renders");
    let bytes = std::fs::read(&artifact.path).expect("artifact");
    (artifact, font, bytes)
}

fn style(weight: Option<&str>) -> TextStyle {
    TextStyle {
        font_family: Some(STACK.to_string()),
        font_size: 16.0,
        font_weight: weight.map(str::to_string),
        font_style: None,
    }
}

/// The sum of `text`'s advances in `font` at 16 px, read from its `cmap` and
/// `hmtx` alone: what a browser draws from a face without layout tables.
fn hmtx_width(font: &[u8], text: &str) -> f64 {
    let face = ttf_parser::Face::parse(font, 0).expect("font parses");
    let units: u32 = text
        .chars()
        .map(|c| {
            let glyph = face.glyph_index(c).expect("mapped");
            u32::from(face.glyph_hor_advance(glyph).expect("advance"))
        })
        .sum();
    f64::from(units) * 16.0 / f64::from(face.units_per_em())
}

/// The TrueType file of the page's `@font-face` rule for `weight`.
fn embedded_file(css: &str, weight: u16) -> Vec<u8> {
    let rule = css
        .split("@font-face")
        .find(|rule| rule.contains(&format!("font-weight:{weight};")))
        .unwrap_or_else(|| panic!("a weight {weight} rule in {css}"));
    let start = rule.find("base64,").expect("a data URI") + "base64,".len();
    let end = start + rule[start..].find(')').expect("the URI ends");
    BASE64.decode(&rule[start..end]).expect("base64")
}

/// The width of the first node's box in a flowchart SVG.
fn first_node_width(svg: &str) -> f64 {
    let at = svg.find(r#"class="basic label-container""#).expect("a node box");
    let tag = &svg[at..at + svg[at..].find('>').expect("tag ends")];
    let start = tag.find(r#"width=""#).expect("a width") + r#"width=""#.len();
    tag[start..start + tag[start..].find('"').expect("width ends")]
        .parse()
        .expect("a number")
}

fn assert_every_label_leads_with(svg: &str, family: &str) {
    for span in families(svg) {
        assert_eq!(span.first(), Some(&usvg::FontFamily::Named(family.to_string())), "{span:?}");
    }
}

#[test]
#[serial]
fn opt_in_labels_lead_with_the_embedded_family() {
    let _cache = isolated_cache_dir();
    // A permitted resolved face is embedded as itself; a Restricted one as
    // Inter, while the same chain's plain static SVG still leads with it.
    for (fs_type, embedded) in [(INSTALLABLE, "Arial"), (RESTRICTED, "Inter")] {
        let chain = arial_chain(fs_type, None);
        for source in [FLOWCHART, CLASS, TAGGED_GITGRAPH] {
            let (_, font, svg) = render(&html_static(source, true), &chain, &svg_request());
            let svg = String::from_utf8(svg).expect("utf-8");
            assert_every_label_leads_with(&svg, embedded);
            let font = font.expect("an opt-in render returns its font");
            assert_eq!(font.families(), [embedded], "{source}");

            let (_, plain_font, plain) = render(&html_static(source, false), &chain, &svg_request());
            assert!(plain_font.is_none());
            assert_every_label_leads_with(&String::from_utf8(plain).expect("utf-8"), "Arial");
        }
    }
}

#[test]
fn the_measurer_measures_each_family_in_its_embedded_face() {
    let restricted = arial_chain(RESTRICTED, None);
    let forced = [STACK.to_string()];
    let inter = |weight| restricted.bundled_inter(&FontQuery::from_css(STACK, weight, false)).expect("Inter");
    let arial_face = |chain: &FontChain, weight| chain.resolve(&FontQuery::from_css(STACK, weight, false)).expect("Arial");
    for (weight, css_weight) in [(400, None), (700, Some("bold")), (700, Some("600")), (400, Some("500"))] {
        // A family forced to Inter (a render drew its refused face) is
        // measured in Inter; unforced, the refused face is measured as
        // resolved until the render knows whether it draws it.
        let embedding = LabelMeasurer::embedding_forcing(&restricted, &forced);
        let faces = embedding.faces("Label", &style(css_weight));
        assert!(faces.iter().all(|face| *face == inter(weight)), "{css_weight:?}: {faces:?}");
        assert!(!restricted.is_bundled_inter(arial_face(&restricted, weight)));
        let unforced = LabelMeasurer::embedding(&restricted).faces("Label", &style(css_weight));
        assert!(unforced.iter().all(|face| *face == arial_face(&restricted, weight)), "{css_weight:?}");
        let plain = LabelMeasurer::new(&restricted).faces("Label", &style(css_weight));
        assert!(plain.iter().all(|face| *face == arial_face(&restricted, weight)), "{css_weight:?}");

        let permitted = arial_chain(INSTALLABLE, None);
        let faces = LabelMeasurer::embedding(&permitted).faces("Label", &style(css_weight));
        assert!(faces.iter().all(|face| *face == arial_face(&permitted, weight)), "{css_weight:?}");
    }
    // The choice made while measuring is the one returned for the page.
    let embedding = LabelMeasurer::embedding_forcing(&restricted, &forced);
    assert!(embedding.embedded_faces().expect("an opt-in measurer").families.is_empty());
    embedding.width("Label", &style(None));
    let faces = embedding.embedded_faces().expect("an opt-in measurer");
    assert_eq!(faces.families.len(), 1);
    assert_eq!(faces.families[0].family, "Inter");
    assert_eq!(faces.families[0].regular.id, inter(400));
    assert!(LabelMeasurer::new(&restricted).embedded_faces().is_none());
}

#[test]
fn the_measurer_uses_plain_advances_with_the_opt_in() {
    let text = "AVATAR Toyota WAVE";
    for (font, weight) in [(INTER_REGULAR, None), (INTER_BOLD, Some("bold"))] {
        let code_points = text.chars().collect();
        let subset = subset(font, 0, &code_points).expect("subset");
        let embedded = LabelMeasurer::embedding(golden_chain()).width(text, &style(weight));
        assert!((embedded - hmtx_width(&subset, text)).abs() < 1e-3, "{weight:?}: {embedded}");
        // Load-bearing: the plain measurement kerns this text.
        let kerned = LabelMeasurer::new(golden_chain()).width(text, &style(weight));
        assert!(embedded - kerned > 1.0, "{weight:?}: {embedded} vs {kerned}");
    }
}

#[test]
#[serial]
fn an_opt_in_label_is_laid_out_at_the_width_the_embedded_subset_draws() {
    let _cache = isolated_cache_dir();
    let label = "AVAVAVAVAVAVAVAV";
    let (_, font, svg) = render(&html_static(KERNED, true), golden_chain(), &svg_request());
    let css = page_font([&font.expect("font")]).expect("page font").css().to_string();
    let drawn = hmtx_width(&embedded_file(&css, 400), label);

    // The node's padding, from the plain render: its box less its kerned label.
    let (_, _, plain) = render(&html_static(KERNED, false), golden_chain(), &svg_request());
    let kerned = LabelMeasurer::new(golden_chain()).width(label, &style(None));
    let padding = first_node_width(&String::from_utf8(plain).expect("utf-8")) - kerned;

    let width = first_node_width(&String::from_utf8(svg).expect("utf-8"));
    assert!((width - (drawn + padding)).abs() < 0.01, "{width} vs {drawn} + {padding}");
    assert!(drawn - kerned > 1.0, "the label kerns: {drawn} vs {kerned}");
}

#[test]
#[serial]
fn without_html_static_svg_the_flag_changes_nothing() {
    for (target, format) in [
        (MermaidTarget::RasterSafe, OutputFormat::Svg),
        (MermaidTarget::RasterSafe, OutputFormat::Png),
        (MermaidTarget::HtmlStatic, OutputFormat::Png),
    ] {
        let request = RenderRequest {
            format,
            ..RenderRequest::default()
        };
        let diagram = MermaidDiagram::new(FLOWCHART).with_target(target);
        let with_flag = diagram.clone().with_embedded_font(true);
        assert_eq!(
            diagram.cache_key_with(golden_chain(), &request).expect("key"),
            with_flag.cache_key_with(golden_chain(), &request).expect("key"),
            "{target:?} {format:?}"
        );
        let plain = {
            let _cache = isolated_cache_dir();
            render(&diagram, golden_chain(), &request)
        };
        let flagged = {
            let _cache = isolated_cache_dir();
            render(&with_flag, golden_chain(), &request)
        };
        assert!(!flagged.0.cache_hit);
        assert!(flagged.1.is_none() && plain.1.is_none());
        assert_eq!(plain.2, flagged.2, "{target:?} {format:?}");
    }
    // Off is the default.
    assert!(!MermaidDiagram::new(FLOWCHART).embedded_font());
    let off = html_static(FLOWCHART, false);
    assert_eq!(
        off.cache_key_with(golden_chain(), &svg_request()).expect("key"),
        MermaidDiagram::new(FLOWCHART)
            .with_target(MermaidTarget::HtmlStatic)
            .cache_key_with(golden_chain(), &svg_request())
            .expect("key")
    );
}

#[test]
fn the_opt_in_has_its_own_cache_keys() {
    let key = |diagram: &MermaidDiagram, chain: &Arc<FontChain>| {
        diagram.cache_key_with(chain, &svg_request()).expect("key")
    };
    let permitted = arial_chain(INSTALLABLE, None);
    let opt_in = html_static(FLOWCHART, true);
    assert_ne!(key(&opt_in, &permitted), key(&html_static(FLOWCHART, false), &permitted));
    // `MERMAID_FONT` and a Restricted face change the measured face, and so
    // the key.
    assert_ne!(key(&opt_in, &permitted), key(&opt_in, &arial_chain(INSTALLABLE, Some("Inter"))));
    assert_ne!(key(&opt_in, &permitted), key(&opt_in, &arial_chain(RESTRICTED, None)));
    // Control: the same inputs give the same key.
    assert_eq!(key(&opt_in, &permitted), key(&opt_in, &arial_chain(INSTALLABLE, None)));
}

#[test]
#[serial]
fn plain_and_opt_in_renders_never_serve_each_other() {
    let _cache = isolated_cache_dir();
    let chain = arial_chain(RESTRICTED, None);
    let (first, font, opt_in_svg) = render(&html_static(FLOWCHART, true), &chain, &svg_request());
    assert!(!first.cache_hit);
    let (plain, _, plain_svg) = render(&html_static(FLOWCHART, false), &chain, &svg_request());
    assert!(!plain.cache_hit, "a plain render after an opt-in one is a miss");
    assert_ne!(opt_in_svg, plain_svg);

    let (again, cached_font, cached_svg) = render(&html_static(FLOWCHART, true), &chain, &svg_request());
    assert!(again.cache_hit);
    assert_eq!(cached_svg, opt_in_svg);
    let (font, cached_font) = (font.expect("font"), cached_font.expect("a hit returns the font"));
    assert_eq!(cached_font.faces, font.faces);
    assert_eq!(cached_font.glyphs, font.glyphs);
    assert_eq!(
        page_font([&cached_font]).expect("page").markup(),
        page_font([&font]).expect("page").markup()
    );
    assert!(render(&html_static(FLOWCHART, false), &chain, &svg_request()).0.cache_hit);

    // Reverse order, in a fresh cache.
    let _cache = isolated_cache_dir();
    assert!(!render(&html_static(FLOWCHART, false), &chain, &svg_request()).0.cache_hit);
    assert!(!render(&html_static(FLOWCHART, true), &chain, &svg_request()).0.cache_hit);
}

#[test]
#[serial]
fn an_opt_in_svg_without_a_readable_record_is_rendered_again() {
    let _cache = isolated_cache_dir();
    let diagram = html_static(CLASS, true);
    let (first, font, _) = render(&diagram, golden_chain(), &svg_request());
    let font = font.expect("font");
    let record = first.path.with_extension("font.json");
    assert!(record.is_file(), "the record sits beside the SVG");

    for damage in [None, Some("{not json"), Some(r#"{"requested":[]}"#)] {
        match damage {
            None => std::fs::remove_file(&record).expect("remove"),
            Some(text) => std::fs::write(&record, text).expect("write"),
        }
        let (artifact, again, _) = render(&diagram, golden_chain(), &svg_request());
        assert!(!artifact.cache_hit, "{damage:?}");
        assert_eq!(again.expect("font").glyphs, font.glyphs, "{damage:?}");
        // The render restored the record, so the next one is a hit.
        assert!(render(&diagram, golden_chain(), &svg_request()).0.cache_hit, "{damage:?}");
    }

    // A record from before the families forced to Inter were kept lacks the
    // required field.
    let mut old: serde_json::Value = serde_json::from_slice(&std::fs::read(&record).expect("record")).expect("json");
    assert!(old.as_object_mut().expect("an object").remove("forced_inter").is_some());
    std::fs::write(&record, old.to_string()).expect("write");
    assert!(!render(&diagram, golden_chain(), &svg_request()).0.cache_hit, "an old record");
    assert!(render(&diagram, golden_chain(), &svg_request()).0.cache_hit);
}

#[test]
#[serial]
fn a_page_of_rendered_diagrams_embeds_one_rule_per_face_and_weight() {
    let _cache = isolated_cache_dir();
    let fonts: Vec<DiagramFont> = [FLOWCHART, CLASS, KERNED]
        .iter()
        .map(|source| render(&html_static(source, true), golden_chain(), &svg_request()).1.expect("font"))
        .collect();
    let page = page_font(&fonts).expect("page font");
    let css = page.css();
    assert_eq!(css.matches("@font-face").count(), 2, "{css}");
    assert_eq!(css.matches("font-family:'Inter';font-style:normal;font-weight:400;").count(), 1);
    assert_eq!(css.matches("font-family:'Inter';font-style:normal;font-weight:700;").count(), 1);
    assert_eq!(page.comments().len(), 1);
    assert!(page.markup().starts_with(&page.comments()[0]));
    // The regular subset maps every regular character of every diagram.
    let regular = embedded_file(css, 400);
    let face = ttf_parser::Face::parse(&regular, 0).expect("subset parses");
    for font in &fonts {
        for c in font.glyphs.families["Inter"].regular.iter() {
            assert!(face.glyph_index(*c).is_some(), "{c:?}");
        }
    }
    assert!(page_font(std::iter::empty()).expect("empty page").is_empty());
    assert_eq!(page_font(std::iter::empty()).expect("empty page").markup(), "");
}

/// The width of `text` at 16 px in `font`, shaped by rustybuzz with its
/// default features: what a browser draws from the emitted file, kerning
/// included when the file keeps its layout tables.
fn shaped_width(font: &[u8], text: &str) -> f64 {
    let face = rustybuzz::Face::from_slice(font, 0).expect("font parses");
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(text);
    let glyphs = rustybuzz::shape(&face, &[], buffer);
    let units: i32 = glyphs.glyph_positions().iter().map(|position| position.x_advance).sum();
    f64::from(units) * 16.0 / f64::from(face.units_per_em())
}

/// A class whose title (bold) and member (regular) are both `AV` pairs.
const KERNED_CLASS: &str = "classDiagram\n  class AVAVAVAVAVAVAVAV {\n    +AVAVAVAVAVAVAVAV\n  }";

/// For each weight's `fsType`, an opted-in class diagram's labels are
/// measured at the width the emitted file draws: shaped with kerning from a
/// whole face (`fsType` 0x0100 forbids subsetting), plain advances from a
/// subset, whether the face is a file of its own or a collection member.
#[test]
#[serial]
fn the_opt_in_measures_each_weight_as_its_emitted_file_draws() {
    const NO_SUBSETTING: u16 = 0x0100;
    let _cache = isolated_cache_dir();
    let label = "AVAVAVAVAVAVAVAV";
    let combinations = [
        (NO_SUBSETTING, NO_SUBSETTING, false),
        (NO_SUBSETTING, INSTALLABLE, false),
        (INSTALLABLE, NO_SUBSETTING, false),
        (INSTALLABLE, INSTALLABLE, false),
        // Collection members: a whole one is rebuilt with every table.
        (NO_SUBSETTING, NO_SUBSETTING, true),
        (NO_SUBSETTING, INSTALLABLE, true),
    ];
    for (regular_fs_type, bold_fs_type, in_collection) in combinations {
        let faces = arial(regular_fs_type, bold_fs_type);
        let installed = if in_collection {
            vec![collection(&[tables_of(&faces[0]), tables_of(&faces[1])])]
        } else {
            faces.to_vec()
        };
        let chain = Arc::new(chain(&installed, None));
        let (_, font, _) = render(&html_static(KERNED_CLASS, true), &chain, &svg_request());
        let css = page_font([&font.expect("font")]).expect("page font").css().to_string();
        for (weight, css_weight, fs_type) in [(400, None, regular_fs_type), (700, Some("bold"), bold_fs_type)] {
            let file = embedded_file(&css, weight);
            let whole = ttf_parser::Face::parse(&file, 0)
                .expect("font parses")
                .tables()
                .gpos
                .is_some();
            assert_eq!(whole, fs_type == NO_SUBSETTING, "{fs_type:#06x} weight {weight}: only a whole face keeps GPOS");
            let drawn = shaped_width(&file, label);
            let measured = LabelMeasurer::embedding(&chain).width(label, &style(css_weight));
            assert!(
                (measured - drawn).abs() < 1e-3,
                "{regular_fs_type:#06x}/{bold_fs_type:#06x} (collection: {in_collection}) weight {weight}: measured {measured}, drawn {drawn}"
            );
            // Load-bearing: the whole face kerns the label, the subset does not.
            let plain = hmtx_width(&file, label);
            assert_eq!(drawn < plain - 1.0, whole, "weight {weight}: {drawn} vs {plain}");
        }
    }
}

/// A whole face's kerning reaches the layout: the opted-in node box is the
/// emitted file's shaped label plus the node's padding.
#[test]
#[serial]
fn an_opt_in_label_in_a_whole_face_is_laid_out_kerned() {
    let _cache = isolated_cache_dir();
    let label = "AVAVAVAVAVAVAVAV";
    let chain = arial_chain(0x0100, None);
    let (_, font, svg) = render(&html_static(KERNED, true), &chain, &svg_request());
    let css = page_font([&font.expect("font")]).expect("page font").css().to_string();
    let drawn = shaped_width(&embedded_file(&css, 400), label);

    let (_, _, plain) = render(&html_static(KERNED, false), &chain, &svg_request());
    let kerned = LabelMeasurer::new(&chain).width(label, &style(None));
    let padding = first_node_width(&String::from_utf8(plain).expect("utf-8")) - kerned;

    let width = first_node_width(&String::from_utf8(svg).expect("utf-8"));
    assert!((width - (drawn + padding)).abs() < 0.01, "{width} vs {drawn} + {padding}");
}

/// A class whose title (bold) holds characters its member (regular) lacks.
const ZEBRA_CLASS: &str = "classDiagram\n    class Zebra {\n        +String name\n    }";

/// A chain whose only installed face is a permitted regular `Arial`, which
/// `MERMAID_FONT=Arial` selects for both weights.
fn regular_only_arial_chain() -> Arc<FontChain> {
    let [regular, _] = arial(INSTALLABLE, INSTALLABLE);
    Arc::new(chain(&[regular], Some("Arial")))
}

/// The weights of the page's `@font-face` rules, in order.
fn rule_weights(css: &str) -> Vec<u16> {
    css.split("font-weight:")
        .skip(1)
        .map(|rest| rest[..rest.find(';').expect("weight ends")].parse().expect("a weight"))
        .collect()
}

/// The TrueType file of the page's `@font-face` rule for `family` at
/// `weight`.
fn family_file(css: &str, family: &str, weight: u16) -> Vec<u8> {
    let declared = format!("font-family:'{family}';font-style:normal;font-weight:{weight};");
    let rule = css
        .split("@font-face")
        .find(|rule| rule.contains(&declared))
        .unwrap_or_else(|| panic!("a {family} weight {weight} rule in {css}"));
    embedded_file(rule, weight)
}

/// Every character the diagram draws in `family` is mapped by the page's
/// file for that family at the weight it is drawn at.
fn assert_covered(css: &str, font: &DiagramFont, family: &str) {
    let glyphs = &font.glyphs.families[family];
    for (weight, drawn) in [(400, &glyphs.regular), (700, &glyphs.bold)] {
        if drawn.is_empty() {
            continue;
        }
        let file = family_file(css, family, weight);
        let face = ttf_parser::Face::parse(&file, 0).expect("font parses");
        for c in drawn {
            assert!(face.glyph_index(*c).is_some(), "{c:?} at weight {weight}");
        }
    }
}

/// Renders `source` opted in over `chain` twice (a miss, then a hit) and
/// returns each render's page CSS.
fn fresh_and_cached_css(source: &str, chain: &Arc<FontChain>) -> [(String, DiagramFont); 2] {
    let diagram = html_static(source, true);
    [false, true].map(|hit| {
        let (artifact, font, _) = render(&diagram, chain, &svg_request());
        assert_eq!(artifact.cache_hit, hit);
        let font = font.expect("font");
        (page_font([&font]).expect("page font").css().to_string(), font)
    })
}

#[test]
#[serial]
fn one_installed_face_carries_both_weights_rules_fresh_and_cached() {
    let _cache = isolated_cache_dir();
    let chain = regular_only_arial_chain();
    let [(fresh, font), (cached, _)] = fresh_and_cached_css(ZEBRA_CLASS, &chain);
    assert_eq!(rule_weights(&fresh), [400, 700], "{fresh}");
    assert_eq!(cached, fresh, "a cache hit gives the same rules");
    let bold_file = embedded_file(&fresh, 700);
    let bold = ttf_parser::Face::parse(&bold_file, 0).expect("font parses");
    for c in ['Z', 'b'] {
        assert!(bold.glyph_index(c).is_some(), "the bold title's {c:?}");
    }
    assert_covered(&fresh, &font, "Arial");
}

#[test]
#[serial]
fn one_installed_face_drawn_regular_only_gets_one_rule() {
    let _cache = isolated_cache_dir();
    let [(css, font), _] = fresh_and_cached_css(FLOWCHART, &regular_only_arial_chain());
    assert_eq!(rule_weights(&css), [400], "{css}");
    assert_covered(&css, &font, "Arial");
}

#[test]
#[serial]
fn separate_regular_and_bold_faces_get_a_rule_each_fresh_and_cached() {
    let _cache = isolated_cache_dir();
    let [(fresh, font), (cached, _)] = fresh_and_cached_css(ZEBRA_CLASS, &arial_chain(INSTALLABLE, None));
    assert_eq!(rule_weights(&fresh), [400, 700], "{fresh}");
    assert_eq!(cached, fresh);
    assert_covered(&fresh, &font, "Arial");
}

/// A class whose only text is its (bold) title.
const BOLD_ONLY_CLASS: &str = "classDiagram\n    class Zebra";

/// The `(family, weight)` each of the page's `@font-face` rules declares, in
/// order.
fn rule_families(css: &str) -> Vec<(String, u16)> {
    css.split("@font-face{font-family:'")
        .skip(1)
        .map(|rest| {
            let family = rest[..rest.find('\'').expect("family ends")].to_string();
            (family, rule_weights(rest)[0])
        })
        .collect()
}

/// The family decision is made over the weights a diagram draws: a refused
/// weight the diagram never draws keeps the permitted resolved face, and a
/// refused weight it draws makes the whole family Inter. A cache hit
/// reproduces the fresh render's font.
#[test]
#[serial]
fn only_a_drawn_refused_weight_makes_the_family_inter_fresh_and_cached() {
    let resolved = "the face the font chain resolved";
    let refused_bold = r#"resolved face "Arial" is Restricted (fsType 0x0002)"#;
    let cases = [
        // A refused bold, regular drawn only.
        (INSTALLABLE, RESTRICTED, FLOWCHART, "Arial", vec![400], resolved),
        // A refused regular, bold drawn only.
        (RESTRICTED, INSTALLABLE, BOLD_ONLY_CLASS, "Arial", vec![700], resolved),
        // A refused bold that the class title draws.
        (INSTALLABLE, RESTRICTED, ZEBRA_CLASS, "Inter", vec![400, 700], refused_bold),
        // Control: both permitted.
        (INSTALLABLE, INSTALLABLE, FLOWCHART, "Arial", vec![400], resolved),
    ];
    for (regular_fs_type, bold_fs_type, source, family, weights, reason) in cases {
        let _cache = isolated_cache_dir();
        let chain = Arc::new(chain(&arial(regular_fs_type, bold_fs_type), None));
        let label = format!("{regular_fs_type:#06x}/{bold_fs_type:#06x} {source:?}");
        let diagram = html_static(source, true);
        let ((fresh, fresh_font, svg), warnings) = capture_warnings(|| render(&diagram, &chain, &svg_request()));
        let (cached, cached_font, cached_svg) = render(&diagram, &chain, &svg_request());
        assert!(!fresh.cache_hit && cached.cache_hit, "{label}");
        assert_eq!(cached_svg, svg, "{label}");
        let (fresh_font, cached_font) = (fresh_font.expect("font"), cached_font.expect("font"));
        assert_eq!(cached_font.faces, fresh_font.faces, "{label}: a hit chooses the same faces");
        assert_eq!(cached_font.glyphs, fresh_font.glyphs, "{label}");

        assert_every_label_leads_with(&String::from_utf8(svg).expect("utf-8"), family);
        assert_eq!(fresh_font.families(), [family], "{label}");
        let page = page_font([&fresh_font]).expect("page font");
        assert_eq!(page_font([&cached_font]).expect("page font"), page, "{label}");
        let expected: Vec<(String, u16)> = weights.iter().map(|weight| (family.to_string(), *weight)).collect();
        assert_eq!(rule_families(page.css()), expected, "{label}");
        assert_eq!(page.comments().len(), 1, "{label}");
        assert!(page.comments()[0].contains(reason), "{label}: {:?}", page.comments());
        assert_covered(page.css(), &fresh_font, family);
        // Only a family that becomes Inter because of a refusal warns.
        let warned = warnings.iter().filter(|warning| warning.contains("may not be embedded")).count();
        assert_eq!(warned, usize::from(family == "Inter"), "{label}: {warnings:?}");
    }
}

/// Two diagrams over one chain and requested family choose differently (one
/// draws only the permitted regular, one also the refused bold), and the
/// page embeds each choice for the diagram that made it.
#[test]
#[serial]
fn a_page_embeds_each_diagrams_own_choice_for_one_requested_family() {
    let _cache = isolated_cache_dir();
    let chain = Arc::new(chain(&arial(INSTALLABLE, RESTRICTED), None));
    let regular_only = render(&html_static(FLOWCHART, true), &chain, &svg_request()).1.expect("font");
    let with_bold = render(&html_static(ZEBRA_CLASS, true), &chain, &svg_request()).1.expect("font");
    assert_eq!(regular_only.families(), ["Arial"]);
    assert_eq!(with_bold.families(), ["Inter"]);

    let page = page_font([&regular_only, &with_bold]).expect("page font");
    let css = page.css();
    assert_eq!(
        rule_families(css),
        [("Arial".to_string(), 400), ("Inter".to_string(), 400), ("Inter".to_string(), 700)],
        "{css}"
    );
    assert_covered(css, &regular_only, "Arial");
    assert_covered(css, &with_bold, "Inter");
    assert_eq!(page.comments().len(), 2, "{:?}", page.comments());
}


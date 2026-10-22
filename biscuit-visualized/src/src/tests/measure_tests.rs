//! The Merman text measurer against usvg 0.47's own text layout.
//!
//! - **Oracle (R-V8, AC-M1):** a label's measured width equals the width usvg
//!   lays the same `<text>` out at, and its glyphs and faces are the drawn
//!   glyphs and faces.
//! - **Measure vs draw (AC-M3):** for every label Merman draws, the faces the
//!   measurer used for the strings Merman asked about are the faces the
//!   rasterizer's resolver drew.
//! - **Wrapping (AC-M2):** with Merman wrapping on our widths, no label is
//!   clipped or wider than its box.
//! - **Linux emoji (D65, AC-M11):** the bundled Noto Emoji draws emoji when no
//!   emoji face is installed, and loses to one that is.
//!
//! Deterministic cases use injected chains of the bundled faces and patched
//! copies of them (see `font_chain_tests`). Scripts no bundled face covers
//! (Arabic, Hebrew, CJK) are measured on the host's default chain; the
//! comparison holds on any host because both sides use the same faces, and a
//! host without a face for a script must agree on its `.notdef` glyphs too.

use std::collections::BTreeSet;
use std::sync::{Arc, Mutex};

use merman::svg::{
    DeterministicTextMeasurer, MeasurementProfileId, SvgEnvironment, SvgPipeline,
    TextMeasurementPolicy, TextMeasurementProfile, TextMeasurementProfileIdentity, TextMeasurer,
    TextStyle,
};
use merman::{
    Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest,
};
use resvg::usvg::{self, Node, Rect};

use super::font_chain_tests::{MERMAID_STACK, NOTO_EMOJI_FILE, renamed};
use crate::fonts::shaper::ShapeParams;
use crate::fonts::{FontChain, FontQuery, LabelMeasurer, default_chain};
use crate::raster::usvg_options;

/// AC-M1's tolerance. S2 measured exact agreement, so any difference at all
/// is reported, but only one over this fails.
const WIDTH_TOLERANCE_PX: f32 = 1.0;

/// The survey's tolerance for a glyph box past its clip or backing shape.
const CLIP_TOLERANCE_PX: f32 = 0.5;

// ------------------------------------------------------------------ labels

/// One label drawn as a minimal SVG `<text>` and measured with the same
/// style.
struct Label {
    text: &'static str,
    family: &'static str,
    size: f64,
    weight: Option<&'static str>,
    style: Option<&'static str>,
    /// Extra `<text>` attributes Merman's `TextStyle` cannot express
    /// (spacing, small caps, kerning); `params` says the same to the shaper.
    extra: &'static str,
    params: fn(ShapeParams) -> ShapeParams,
}

impl Label {
    fn plain(text: &'static str) -> Self {
        Label {
            text,
            family: MERMAID_STACK,
            size: 16.0,
            weight: None,
            style: None,
            extra: "",
            params: |params| params,
        }
    }

    fn family(mut self, family: &'static str) -> Self {
        self.family = family;
        self
    }

    fn weight(mut self, weight: &'static str) -> Self {
        self.weight = Some(weight);
        self
    }

    fn italic(mut self) -> Self {
        self.style = Some("italic");
        self
    }

    fn size(mut self, size: f64) -> Self {
        self.size = size;
        self
    }

    fn with(mut self, extra: &'static str, params: fn(ShapeParams) -> ShapeParams) -> Self {
        self.extra = extra;
        self.params = params;
        self
    }

    fn text_style(&self) -> TextStyle {
        TextStyle {
            font_family: Some(self.family.to_string()),
            font_size: self.size,
            font_weight: self.weight.map(str::to_string),
            font_style: self.style.map(str::to_string),
        }
    }

    fn svg(&self) -> String {
        let mut attributes = format!(
            r#"font-family="{}" font-size="{}""#,
            escape(self.family),
            self.size
        );
        if let Some(weight) = self.weight {
            attributes.push_str(&format!(r#" font-weight="{weight}""#));
        }
        if let Some(style) = self.style {
            attributes.push_str(&format!(r#" font-style="{style}""#));
        }
        format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1200" height="80"><text id="label" x="10" y="50" {attributes} {}>{}</text></svg>"#,
            self.extra,
            escape(self.text)
        )
    }
}

fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// What usvg drew for one `<text>`: its metric box width and each glyph's
/// id and face, in visual order.
struct Drawn {
    width: f32,
    glyphs: Vec<(u16, usvg::fontdb::ID)>,
}

fn drawn(text: &usvg::Text) -> Drawn {
    Drawn {
        width: text.bounding_box().width(),
        glyphs: text
            .layouted()
            .iter()
            .flat_map(|span| &span.positioned_glyphs)
            .map(|glyph| (glyph.id.0, glyph.font))
            .collect(),
    }
}

fn drawn_label(chain: &Arc<FontChain>, svg: &str) -> Drawn {
    let tree = usvg::Tree::from_str(svg, &usvg_options(chain)).expect("label SVG parses");
    match tree.node_by_id("label") {
        Some(Node::Text(text)) => drawn(text),
        _ => Drawn {
            width: 0.0,
            glyphs: Vec::new(),
        },
    }
}

fn face_name(chain: &FontChain, id: usvg::fontdb::ID) -> String {
    chain
        .database()
        .face(id)
        .map_or_else(|| "?".into(), |face| face.post_script_name.clone())
}

/// Measures and draws every label; returns one line per disagreement and the
/// largest width difference seen.
fn compare_labels(chain: &Arc<FontChain>, labels: &[Label]) -> (Vec<String>, f32) {
    let measurer = LabelMeasurer::new(chain);
    let mut failures = Vec::new();
    let mut worst = 0.0_f32;
    for label in labels {
        let style = label.text_style();
        let query = measurer.query(&style);
        let params = (label.params)(ShapeParams::for_query(&query, label.size as f32));
        let shaped = if label.extra.is_empty() {
            (*measurer.shape(label.text, &style)).clone()
        } else {
            measurer.shape_with(label.text, &query, &params)
        };
        let drawn = drawn_label(chain, &label.svg());
        let measured: Vec<(u16, usvg::fontdb::ID)> = shaped
            .glyphs
            .iter()
            .map(|glyph| (glyph.id, glyph.face))
            .collect();

        let diff = (shaped.bbox_width - drawn.width)
            .abs()
            .max((shaped.advance - drawn.width).abs());
        worst = worst.max(diff);
        if diff > WIDTH_TOLERANCE_PX {
            failures.push(format!(
                "{:?}: measured advance {} / box {}, drawn {}",
                label.text, shaped.advance, shaped.bbox_width, drawn.width
            ));
        }
        if measured != drawn.glyphs {
            let names = |glyphs: &[(u16, usvg::fontdb::ID)]| {
                glyphs
                    .iter()
                    .map(|(id, face)| format!("{id}@{}", face_name(chain, *face)))
                    .collect::<Vec<_>>()
                    .join(" ")
            };
            failures.push(format!(
                "{:?}: measured glyphs [{}], drawn [{}]",
                label.text,
                names(&measured),
                names(&drawn.glyphs)
            ));
        }
    }
    (failures, worst)
}

// ------------------------------------------------------------------ Merman

fn environment(measurer: Arc<dyn TextMeasurer + Send + Sync>) -> SvgEnvironment {
    let identity = TextMeasurementProfileIdentity::new(
        MeasurementProfileId::new("biscuit-visualized-test").expect("profile id"),
        "1",
    )
    .expect("profile identity");
    SvgEnvironment::deterministic().with_text_measurement_policy(TextMeasurementPolicy::uniform(
        TextMeasurementProfile::new(identity, measurer),
    ))
}

fn render(source: &str, measurer: Arc<dyn TextMeasurer + Send + Sync>) -> String {
    let mut request = SvgRequest {
        environment: environment(measurer),
        ..SvgRequest::default()
    };
    request.options.diagram_id = Some("measure".into());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    // An explicit theme, as the render path sets one: with no site config
    // Merman draws its own default family ("Recursive Variable") while
    // measuring Mermaid's ("trebuchet ms"), a mismatch no measurer can fix.
    let renderer = Renderer::new().with_engine(Engine::new().with_site_config(
        MermaidConfig::from_value(serde_json::json!({ "theme": "default" })),
    ));
    match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => svg.svg().to_string(),
        other => panic!("Merman renders the fixture: {other:?}"),
    }
}

/// Every `<text>` in `tree`, depth first.
fn texts(group: &usvg::Group, into: &mut Vec<usvg::Text>) {
    for node in group.children() {
        match node {
            Node::Group(child) => texts(child, into),
            Node::Text(text) => into.push((**text).clone()),
            _ => {}
        }
    }
}

/// The `TextStyle` naming what usvg resolved for `span`.
fn style_of(span: &usvg::TextSpan) -> TextStyle {
    let font = span.font();
    let family = font
        .families()
        .iter()
        .map(|family| match family {
            usvg::FontFamily::Named(name) => format!("\"{name}\""),
            generic => generic.to_string(),
        })
        .collect::<Vec<_>>()
        .join(", ");
    TextStyle {
        font_family: Some(family),
        font_size: f64::from(span.font_size().get()),
        font_weight: Some(font.weight().to_string()),
        font_style: Some(
            match font.style() {
                usvg::FontStyle::Normal => "normal",
                usvg::FontStyle::Italic => "italic",
                usvg::FontStyle::Oblique => "oblique",
            }
            .into(),
        ),
    }
}

/// The shaper parameters usvg read from `span`.
fn params_of(span: &usvg::TextSpan) -> ShapeParams {
    ShapeParams {
        size: span.font_size().get(),
        letter_spacing: span.letter_spacing(),
        word_spacing: span.word_spacing(),
        small_caps: span.small_caps(),
        kerning: span.apply_kerning(),
        variations: span
            .font()
            .variations()
            .iter()
            .map(|variation| rustybuzz::Variation {
                tag: rustybuzz::ttf_parser::Tag::from_bytes(&variation.tag),
                value: variation.value,
            })
            .collect(),
    }
}

/// A drawn text node usvg laid out one chunk per line, each of one span: the
/// oracle compares the widest chunk with the node's box (S2's rule).
fn comparable(text: &usvg::Text) -> bool {
    let chunks = text.chunks();
    let single_spans = chunks.iter().all(|chunk| chunk.spans().len() == 1);
    let mut lines: Vec<i64> = chunks
        .iter()
        .filter_map(|chunk| chunk.y().map(|y| (y * 10.0) as i64))
        .collect();
    lines.sort_unstable();
    lines.dedup();
    single_spans
        && (chunks.len() == 1 || lines.len() == chunks.len())
        && chunks.iter().any(|chunk| !chunk.text().trim().is_empty())
}

/// For each comparable drawn label of `svg`, the measurer's width (widest
/// chunk, shaped with usvg's span values) against usvg's box, and its glyphs
/// and faces against the drawn ones.
fn compare_drawn_labels(
    chain: &Arc<FontChain>,
    svg: &str,
    name: &str,
) -> (Vec<String>, usize, f32) {
    let tree = usvg::Tree::from_str(svg, &usvg_options(chain)).expect("Merman SVG parses");
    let measurer = LabelMeasurer::new(chain);
    let mut nodes = Vec::new();
    texts(tree.root(), &mut nodes);
    let (mut failures, mut compared, mut worst) = (Vec::new(), 0, 0.0_f32);
    for text in nodes.iter().filter(|text| comparable(text)) {
        let drawn = drawn(text);
        let mut width = 0.0_f32;
        let mut glyphs = Vec::new();
        for chunk in text.chunks() {
            let span = &chunk.spans()[0];
            let query = FontQuery::from_usvg(span.font());
            let params = params_of(span);
            assert_eq!(
                ShapeParams::for_query(&query, params.size).variations,
                params.variations,
                "{name}: usvg's variations for {:?}",
                chunk.text()
            );
            let shaped = measurer.shape_with(chunk.text(), &query, &params);
            width = width.max(shaped.bbox_width);
            glyphs.extend(shaped.glyphs.iter().map(|glyph| (glyph.id, glyph.face)));
        }
        compared += 1;
        let diff = (width - drawn.width).abs();
        worst = worst.max(diff);
        let label: String = text.chunks().iter().map(|chunk| chunk.text()).collect();
        if diff > WIDTH_TOLERANCE_PX {
            failures.push(format!(
                "{name} {label:?}: measured {width}, drawn {}",
                drawn.width
            ));
        }
        if glyphs != drawn.glyphs {
            failures.push(format!(
                "{name} {label:?}: measured glyphs differ from drawn"
            ));
        }
    }
    (failures, compared, worst)
}

/// The strings and styles Merman asked a measurer about.
type Calls = Vec<(String, TextStyle)>;

/// A measurer for Merman that records every string and style it is asked
/// about, sharing `measurer`'s widths.
fn recording(
    measurer: &Arc<LabelMeasurer>,
) -> (Arc<dyn TextMeasurer + Send + Sync>, Arc<Mutex<Calls>>) {
    let calls = Arc::new(Mutex::new(Vec::new()));
    let (inner, record) = (Arc::clone(measurer), Arc::clone(&calls));
    let text_measurer =
        DeterministicTextMeasurer::default().with_width_callback(move |text, style| {
            record
                .lock()
                .expect("calls")
                .push((text.to_string(), style.clone()));
            inner.width(text, style)
        });
    (text_measurer, calls)
}

/// AC-M3 over one rendered fixture: for each drawn chunk whose text Merman
/// asked the measurer about, some style it asked with shapes to exactly the
/// drawn glyphs and faces. Returns failures and how many chunks matched.
fn compare_faces(chain: &Arc<FontChain>, source: &str, name: &str) -> (Vec<String>, usize, usize) {
    let measurer = Arc::new(LabelMeasurer::new(chain));
    let (text_measurer, calls) = recording(&measurer);
    let svg = render(source, text_measurer);
    let calls = calls.lock().expect("calls").clone();
    let tree = usvg::Tree::from_str(&svg, &usvg_options(chain)).expect("Merman SVG parses");
    let mut nodes = Vec::new();
    texts(tree.root(), &mut nodes);
    let (mut failures, mut matched, mut unasked) = (Vec::new(), 0, 0);
    for text in &nodes {
        let spans = text.layouted();
        // usvg emits one laid-out span per drawn chunk span, in order.
        let chunk_spans: Vec<&usvg::TextChunk> = text
            .chunks()
            .iter()
            .filter(|chunk| !chunk.text().is_empty())
            .collect();
        if spans.len() != chunk_spans.len() || chunk_spans.iter().any(|c| c.spans().len() != 1) {
            continue;
        }
        for (chunk, span) in chunk_spans.iter().zip(spans) {
            let drawn: Vec<(u16, usvg::fontdb::ID)> = span
                .positioned_glyphs
                .iter()
                .map(|glyph| (glyph.id.0, glyph.font))
                .collect();
            let asked: BTreeSet<String> = calls
                .iter()
                .filter(|(text, _)| text.trim() == chunk.text().trim())
                .map(|(_, style)| format!("{style:?}"))
                .collect();
            let styles: Vec<&TextStyle> = asked
                .iter()
                .filter_map(|key| {
                    calls
                        .iter()
                        .find(|(_, style)| &format!("{style:?}") == key)
                        .map(|(_, style)| style)
                })
                .collect();
            let drawn_faces: Vec<usvg::fontdb::ID> = drawn.iter().map(|(_, face)| *face).collect();
            if styles.is_empty() {
                // Text Merman drew without asking (titles, fixed glyphs):
                // measured in the drawn style, it must still use the drawn
                // faces.
                unasked += 1;
                let style = style_of(&chunk.spans()[0]);
                if measurer.faces(chunk.text(), &style) != drawn_faces {
                    failures.push(format!(
                        "{name} {:?}: drawn-style faces differ",
                        chunk.text()
                    ));
                }
                continue;
            }
            let any_match = styles
                .iter()
                .any(|style| measurer.faces(chunk.text(), style) == drawn_faces);
            if any_match {
                matched += 1;
            } else {
                let drawn_faces: BTreeSet<String> = drawn
                    .iter()
                    .map(|(_, face)| face_name(chain, *face))
                    .collect();
                let measured: Vec<BTreeSet<String>> = styles
                    .iter()
                    .map(|style| {
                        measurer
                            .faces(chunk.text(), style)
                            .into_iter()
                            .map(|face| face_name(chain, face))
                            .collect()
                    })
                    .collect();
                failures.push(format!(
                    "{name} {:?}: drawn in {drawn_faces:?}, measured in {measured:?} (styles {styles:?})",
                    chunk.text()
                ));
            }
        }
    }
    (failures, matched, unasked)
}

// ------------------------------------------------------------------ survey

/// A label whose glyph box overflows an ancestor clip or the smallest filled
/// shape behind it, by more than [`CLIP_TOLERANCE_PX`].
#[derive(Debug, Clone, PartialEq)]
struct Overflow {
    label: String,
    kind: &'static str,
    px: f32,
}

/// The clip and backing parts of S2's defect survey, over usvg's own parse.
struct Survey {
    clips: Vec<Rect>,
    backing: Vec<Vec<Rect>>,
    found: Vec<Overflow>,
}

impl Survey {
    fn run(svg: &str, chain: &Arc<FontChain>) -> Vec<Overflow> {
        let tree = usvg::Tree::from_str(svg, &usvg_options(chain)).expect("Merman SVG parses");
        let mut survey = Survey {
            clips: Vec::new(),
            backing: Vec::new(),
            found: Vec::new(),
        };
        survey.group(tree.root());
        survey.found
    }

    fn group(&mut self, group: &usvg::Group) {
        let clip = group.clip_path().and_then(|clip| clip_rect(group, clip));
        if let Some(rect) = clip {
            self.clips.push(rect);
        }
        let shapes = group
            .children()
            .iter()
            .filter_map(|node| match node {
                Node::Path(path) if is_filled(path) => {
                    let bounds = path.abs_bounding_box();
                    (bounds.width() > 1.0 && bounds.height() > 1.0).then_some(bounds)
                }
                _ => None,
            })
            .collect();
        self.backing.push(shapes);
        for node in group.children() {
            match node {
                Node::Group(child) => self.group(child),
                Node::Text(text) => self.text(text),
                _ => {}
            }
        }
        self.backing.pop();
        if clip.is_some() {
            self.clips.pop();
        }
    }

    fn text(&mut self, text: &usvg::Text) {
        let label: String = text.chunks().iter().map(|chunk| chunk.text()).collect();
        let label = label.trim().to_string();
        if label.is_empty() {
            return;
        }
        let Some(bbox) = text
            .flattened()
            .bounding_box()
            .transform(text.abs_transform())
        else {
            return;
        };
        if bbox.width() <= 0.0 {
            return;
        }
        let clipped = self
            .clips
            .iter()
            .map(|clip| overflow(&bbox, clip))
            .fold(0.0_f32, f32::max);
        if clipped > CLIP_TOLERANCE_PX {
            self.found.push(Overflow {
                label: label.clone(),
                kind: "clipped",
                px: clipped,
            });
        }
        let center = (
            bbox.x() + bbox.width() / 2.0,
            bbox.y() + bbox.height() / 2.0,
        );
        let backing = self
            .backing
            .iter()
            .rev()
            .take(3)
            .flatten()
            .filter(|r| {
                r.x() <= center.0
                    && center.0 <= r.right()
                    && r.y() <= center.1
                    && center.1 <= r.bottom()
            })
            .min_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())));
        if let Some(shape) = backing {
            let past = (shape.x() - bbox.x()).max(0.0) + (bbox.right() - shape.right()).max(0.0);
            if past > CLIP_TOLERANCE_PX {
                self.found.push(Overflow {
                    label,
                    kind: "past its box",
                    px: past,
                });
            }
        }
    }
}

fn overflow(inner: &Rect, outer: &Rect) -> f32 {
    let x = (outer.x() - inner.x()).max(0.0) + (inner.right() - outer.right()).max(0.0);
    let y = (outer.y() - inner.y()).max(0.0) + (inner.bottom() - outer.bottom()).max(0.0);
    x.max(y)
}

/// A clip path's region in canvas coordinates (usvg computes no bounding
/// boxes inside clip paths, so this unions the clip's path data).
fn clip_rect(group: &usvg::Group, clip: &usvg::ClipPath) -> Option<Rect> {
    clip_space_bounds(clip.root())?.transform(group.abs_transform().pre_concat(clip.transform()))
}

fn clip_space_bounds(group: &usvg::Group) -> Option<Rect> {
    group
        .children()
        .iter()
        .filter_map(|node| match node {
            Node::Path(path) => path.data().bounds().transform(path.abs_transform()),
            Node::Group(child) => clip_space_bounds(child),
            _ => None,
        })
        .reduce(|a, b| {
            Rect::from_ltrb(
                a.left().min(b.left()),
                a.top().min(b.top()),
                a.right().max(b.right()),
                a.bottom().max(b.bottom()),
            )
            .unwrap_or(a)
        })
}

fn is_filled(path: &usvg::Path) -> bool {
    path.is_visible() && path.fill().is_some_and(|fill| fill.opacity().get() > 0.05)
}

/// Labels that overflow for a reason S1 and S2 traced to Merman itself, not
/// to measurement: Merman sizes an ER attribute comment containing `<br/>`
/// for one line and draws two, so both lines clip by 5–7 px under every
/// measurer (Merman upstream issue 6, "ER attribute comment with `<br/>`
/// clips").
const MERMAN_OVERFLOWS: [(&str, &str); 2] = [
    ("wrap_er", "when it was created"),
    ("wrap_er", "and stored"),
];

/// Renders each fixture with the production measurer on `chain` and returns
/// the overflows not on the Merman allow-list, after asserting every
/// allow-listed one still overflows.
fn wrap_overflows(chain: &Arc<FontChain>, fixtures: &[(&str, &str)]) -> Vec<String> {
    let mut seen = Vec::new();
    let mut unexpected = Vec::new();
    for (name, source) in fixtures {
        let svg = render(source, LabelMeasurer::text_measurer(chain));
        for found in Survey::run(&svg, chain) {
            if MERMAN_OVERFLOWS.contains(&(*name, found.label.as_str())) {
                seen.push((*name, found.label.clone()));
            } else {
                unexpected.push(format!(
                    "{name}: {:?} {} by {:.1} px",
                    found.label, found.kind, found.px
                ));
            }
        }
    }
    for (name, label) in MERMAN_OVERFLOWS {
        assert!(
            seen.iter().any(|(n, l)| *n == name && l == label),
            "{name} {label:?} no longer overflows: Merman fixed it, remove it from MERMAN_OVERFLOWS"
        );
    }
    unexpected
}

fn inter_chain(installed: &[&[u8]]) -> Arc<FontChain> {
    Arc::new(FontChain::injected(installed, &[], Some("Inter")))
}

// ------------------------------------------------------------ measurer API

/// R-M2: the callback measures each candidate exactly as given; doubled and
/// leading spaces are kept, not collapsed.
#[test]
fn measurer_measures_candidates_without_collapsing_whitespace() {
    let chain = inter_chain(&[]);
    let measurer = LabelMeasurer::new(&chain);
    let style = Label::plain("").text_style();

    let single = measurer.width("a b", &style);
    let double = measurer.width("a  b", &style);
    let leading = measurer.width(" a b", &style);

    assert!(double > single, "{double} > {single}");
    assert!(leading > single, "{leading} > {single}");
}

/// R-M2: Merman's measurer is the width callback: a line's width is the
/// shaped advance.
#[test]
fn merman_measurer_reports_the_shaped_width() {
    let chain = inter_chain(&[]);
    let style = Label::plain("").text_style();
    let expected = LabelMeasurer::new(&chain).width("Hello, World", &style);

    let width =
        LabelMeasurer::text_measurer(&chain).measure_html_line_width_px("Hello, World", &style);

    assert!(expected > 50.0);
    assert_eq!(width, expected);
}

/// usvg draws text with no family in its default family; so does the
/// measurer for a style without one.
#[test]
fn measurer_queries_usvg_default_family_for_a_style_without_one() {
    let chain = inter_chain(&[]);
    let measurer = LabelMeasurer::new(&chain);
    let style = TextStyle {
        font_family: None,
        ..TextStyle::default()
    };

    let query = measurer.query(&style);

    assert_eq!(
        query.families,
        FontQuery::from_css(&usvg_options(&chain).font_family, 400, false).families
    );
}

// ---------------------------------------------------------- oracle (AC-M1)

/// R-V8, AC-M1: on bundled faces only (Inter, and a renamed copy of Noto
/// Emoji as the installed fallback), every label measures to usvg's
/// laid-out width with usvg's glyphs and faces: Latin, bold and italic,
/// kerning on and off, combining marks, letter and word spacing (negative
/// letter spacing empties clusters), small caps, emoji and symbols by
/// fallback, and Arabic, Hebrew, and CJK with no face (`.notdef` on both
/// sides).
#[test]
fn oracle_widths_match_usvg_on_bundled_faces() {
    let coto = renamed(NOTO_EMOJI_FILE, "Noto", "Coto");
    let chain = inter_chain(&[&coto]);
    let labels = [
        Label::plain("Hello, World"),
        Label::plain("Bold label text").weight("bold"),
        Label::plain("Semibold 600").weight("600"),
        Label::plain("Italic text").italic(),
        Label::plain("AVATAR Toyota WAVE"),
        Label::plain("AVATAR Toyota WAVE").with(r#"kerning="0""#, |p| ShapeParams {
            kerning: false,
            ..p
        }),
        Label::plain("Large 28 px").size(28.0),
        Label::plain("Café naïve résumé Zoë Ångström"),
        Label::plain("e\u{301}le\u{300}ve n\u{303}o u\u{308}ber"),
        Label::plain("Spaced out").with(r#"letter-spacing="2""#, |p| ShapeParams {
            letter_spacing: 2.0,
            ..p
        }),
        Label::plain("illicit").with(r#"letter-spacing="-6""#, |p| ShapeParams {
            letter_spacing: -6.0,
            ..p
        }),
        Label::plain("Word spaced label").with(r#"word-spacing="6""#, |p| ShapeParams {
            word_spacing: 6.0,
            ..p
        }),
        Label::plain("Small Caps Label").with(r#"font-variant="small-caps""#, |p| ShapeParams {
            small_caps: true,
            ..p
        }),
        Label::plain("Deploy 🚀 done ✅"),
        Label::plain("🙂 smile 🎉"),
        Label::plain("a → b ✓ ∑x ∞"),
        Label::plain("مرحبا بالعالم"),
        Label::plain("שלום עולם"),
        Label::plain("Hello שלום 123"),
        Label::plain("東京タワー 观光"),
    ];

    let (failures, worst) = compare_labels(&chain, &labels);

    assert!(
        failures.is_empty(),
        "worst {worst} px:\n{}",
        failures.join("\n")
    );
}

/// R-V8, AC-M1: Arabic, Hebrew, CJK, emoji, mixed-direction, and combining
/// labels on the host's own fonts. A host without a face for a script still
/// passes only if the measurer and usvg agree on its `.notdef` glyphs.
#[test]
fn oracle_widths_match_usvg_on_host_fonts() {
    let chain = default_chain();
    let labels = [
        Label::plain("Hello, World"),
        Label::plain("Bold label").weight("bold"),
        Label::plain("مرحبا بالعالم"),
        Label::plain("+int رصيد"),
        Label::plain("שלום עולם"),
        Label::plain("مرحبا → שלום"),
        Label::plain("你好，服务器 🚀"),
        Label::plain("日本語 かな ユーザー"),
        Label::plain("汉字 测试").weight("bold"),
        Label::plain("Deploy 🚀 done ✅"),
        Label::plain("Emoji 🎉 🚀 ✅ 🌍"),
        Label::plain("e\u{301}le\u{300}ve n\u{303}o"),
        Label::plain("Amélie Zoë ñ (combining)"),
        Label::plain("monospace text").family("monospace"),
        Label::plain("serif text").family("serif"),
    ];

    let (failures, worst) = compare_labels(chain, &labels);

    assert!(
        failures.is_empty(),
        "worst {worst} px:\n{}",
        failures.join("\n")
    );
}

/// R-V8, AC-M1: every label Merman draws for the mixed-script corpus, on the
/// host's fonts, measures to usvg's width with usvg's glyphs and faces.
#[test]
fn oracle_widths_match_usvg_for_the_mixed_script_corpus() {
    let fixtures = [
        (
            "mixed_class",
            include_str!("fixtures/merman/mixed/mixed_class.mmd"),
        ),
        (
            "mixed_flowchart",
            include_str!("fixtures/merman/mixed/mixed_flowchart.mmd"),
        ),
        (
            "mixed_mindmap",
            include_str!("fixtures/merman/mixed/mixed_mindmap.mmd"),
        ),
        (
            "mixed_sequence",
            include_str!("fixtures/merman/mixed/mixed_sequence.mmd"),
        ),
    ];
    let chain = default_chain();
    let (mut failures, mut compared, mut worst) = (Vec::new(), 0, 0.0_f32);

    for (name, source) in fixtures {
        let svg = render(source, LabelMeasurer::text_measurer(chain));
        let (found, count, diff) = compare_drawn_labels(chain, &svg, name);
        failures.extend(found);
        compared += count;
        worst = worst.max(diff);
    }

    assert!(compared >= 40, "compared {compared} labels");
    assert!(
        failures.is_empty(),
        "worst {worst} px:\n{}",
        failures.join("\n")
    );
}

// ------------------------------------------------- measure vs draw (AC-M3)

/// AC-M3: with `MERMAID_FONT=Inter` on bundled faces, for every wrap-corpus
/// label Merman measured, the measurer's faces for the strings and styles
/// Merman asked about are the faces usvg's resolver drew.
#[test]
fn measured_faces_are_drawn_faces_for_the_wrap_corpus_with_inter() {
    let fixtures = [
        (
            "wrap_class",
            include_str!("fixtures/merman/wrap/wrap_class.mmd"),
        ),
        ("wrap_er", include_str!("fixtures/merman/wrap/wrap_er.mmd")),
        (
            "wrap_flowchart",
            include_str!("fixtures/merman/wrap/wrap_flowchart.mmd"),
        ),
        (
            "wrap_mindmap",
            include_str!("fixtures/merman/wrap/wrap_mindmap.mmd"),
        ),
        (
            "wrap_sequence",
            include_str!("fixtures/merman/wrap/wrap_sequence.mmd"),
        ),
        (
            "wrap_state",
            include_str!("fixtures/merman/wrap/wrap_state.mmd"),
        ),
    ];
    let chain = inter_chain(&[]);
    let (mut failures, mut matched) = (Vec::new(), 0);

    for (name, source) in fixtures {
        let (found, count, _) = compare_faces(&chain, source, name);
        failures.extend(found);
        matched += count;
    }

    assert!(matched >= 30, "matched {matched} drawn lines");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// AC-M3 on the host's own fonts (no `MERMAID_FONT` override in tests).
#[test]
fn measured_faces_are_drawn_faces_for_the_wrap_corpus_on_host_fonts() {
    let fixtures = [
        (
            "wrap_class",
            include_str!("fixtures/merman/wrap/wrap_class.mmd"),
        ),
        ("wrap_er", include_str!("fixtures/merman/wrap/wrap_er.mmd")),
        (
            "wrap_flowchart",
            include_str!("fixtures/merman/wrap/wrap_flowchart.mmd"),
        ),
        (
            "wrap_mindmap",
            include_str!("fixtures/merman/wrap/wrap_mindmap.mmd"),
        ),
        (
            "wrap_sequence",
            include_str!("fixtures/merman/wrap/wrap_sequence.mmd"),
        ),
        (
            "wrap_state",
            include_str!("fixtures/merman/wrap/wrap_state.mmd"),
        ),
    ];
    let chain = default_chain();
    let (mut failures, mut matched) = (Vec::new(), 0);

    for (name, source) in fixtures {
        let (found, count, _) = compare_faces(chain, source, name);
        failures.extend(found);
        matched += count;
    }

    assert!(matched >= 30, "matched {matched} drawn lines");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

// ---------------------------------------------------------- wrap (AC-M2)

/// AC-M2: Merman wraps on our widths, and no wrap-corpus label is clipped or
/// wider than its box, on bundled Inter; the Merman defects of
/// `MERMAN_OVERFLOWS` must still overflow.
#[test]
fn wrapped_labels_fit_their_boxes_with_inter() {
    let fixtures = [
        (
            "wrap_class",
            include_str!("fixtures/merman/wrap/wrap_class.mmd"),
        ),
        ("wrap_er", include_str!("fixtures/merman/wrap/wrap_er.mmd")),
        (
            "wrap_flowchart",
            include_str!("fixtures/merman/wrap/wrap_flowchart.mmd"),
        ),
        (
            "wrap_mindmap",
            include_str!("fixtures/merman/wrap/wrap_mindmap.mmd"),
        ),
        (
            "wrap_sequence",
            include_str!("fixtures/merman/wrap/wrap_sequence.mmd"),
        ),
        (
            "wrap_state",
            include_str!("fixtures/merman/wrap/wrap_state.mmd"),
        ),
    ];

    let unexpected = wrap_overflows(&inter_chain(&[]), &fixtures);

    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

/// AC-M2 on the host's own fonts.
#[test]
fn wrapped_labels_fit_their_boxes_on_host_fonts() {
    let fixtures = [
        (
            "wrap_class",
            include_str!("fixtures/merman/wrap/wrap_class.mmd"),
        ),
        ("wrap_er", include_str!("fixtures/merman/wrap/wrap_er.mmd")),
        (
            "wrap_flowchart",
            include_str!("fixtures/merman/wrap/wrap_flowchart.mmd"),
        ),
        (
            "wrap_mindmap",
            include_str!("fixtures/merman/wrap/wrap_mindmap.mmd"),
        ),
        (
            "wrap_sequence",
            include_str!("fixtures/merman/wrap/wrap_sequence.mmd"),
        ),
        (
            "wrap_state",
            include_str!("fixtures/merman/wrap/wrap_state.mmd"),
        ),
    ];

    let unexpected = wrap_overflows(default_chain(), &fixtures);

    assert!(unexpected.is_empty(), "{}", unexpected.join("\n"));
}

// ------------------------------------------------- Linux emoji (AC-M11)

/// AC-M11, D65: with no emoji face installed, an emoji label shapes to the
/// bundled Noto Emoji with no `.notdef` glyph, and the rasterizer paints
/// pixels inside its glyph box.
#[cfg(target_os = "linux")]
#[test]
fn linux_emoji_without_an_installed_emoji_face_draws_bundled_noto_emoji() {
    use crate::raster::png::rasterize_svg_to_png_bytes_with;

    let chain = inter_chain(&[]);
    let noto = *chain.bundled_ids().last().expect("bundled Noto Emoji");
    let label = Label::plain("😀 🚀").size(32.0);

    let shaped = LabelMeasurer::new(&chain).shape(label.text, &label.text_style());
    let png = rasterize_svg_to_png_bytes_with(&label.svg(), 1, &chain).expect("rasterizes");

    let emoji: Vec<_> = shaped
        .glyphs
        .iter()
        .filter(|glyph| glyph.text.trim() != "")
        .collect();
    assert_eq!(emoji.len(), 2, "{:?}", shaped.glyphs);
    assert!(
        emoji
            .iter()
            .all(|glyph| glyph.id != 0 && glyph.face == noto),
        "{:?}",
        shaped.glyphs
    );
    let tree = usvg::Tree::from_str(&label.svg(), &usvg_options(&chain)).expect("parses");
    let Some(Node::Text(text)) = tree.node_by_id("label") else {
        panic!("a laid-out label");
    };
    let glyph_box = text.flattened().abs_bounding_box();
    let image = image::load_from_memory(&png)
        .expect("PNG decodes")
        .to_rgba8();
    let painted = image
        .enumerate_pixels()
        .filter(|(x, y, pixel)| {
            pixel.0[3] > 0
                && (*x as f32) >= glyph_box.left().floor()
                && (*x as f32) <= glyph_box.right().ceil()
                && (*y as f32) >= glyph_box.top().floor()
                && (*y as f32) <= glyph_box.bottom().ceil()
        })
        .count();
    assert!(painted > 100, "{painted} painted pixels in {glyph_box:?}");
}

/// AC-M11, D65: an installed emoji face (a renamed copy of Noto Emoji) is
/// chosen over the bundled Noto Emoji.
#[cfg(target_os = "linux")]
#[test]
fn linux_installed_emoji_face_wins_over_bundled_noto_emoji() {
    let installed = renamed(NOTO_EMOJI_FILE, "Noto", "Coto");
    let chain = inter_chain(&[&installed]);
    let noto = *chain.bundled_ids().last().expect("bundled Noto Emoji");
    let label = Label::plain("😀");

    let faces = LabelMeasurer::new(&chain).faces(label.text, &label.text_style());

    assert_eq!(faces.len(), 1);
    assert_ne!(faces[0], noto);
    assert_eq!(
        chain.database().face(faces[0]).expect("face").families[0].0,
        "Coto Emoji"
    );
}

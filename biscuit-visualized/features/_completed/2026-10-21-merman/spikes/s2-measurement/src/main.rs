//! Spike S2: option A (our rustybuzz `TextMeasurer`) against option B
//! (Merman's native measurement) for turning Merman diagrams into images.
//!
//! Subcommands (all write JSON; see `scripts/run.sh`):
//! - `analyze --os L --out DIR [--variant V]`: renders the corpus with A, A
//!   with Merman's wrapping (`a-mw`), and B; measures accuracy, defects,
//!   fallback faces, line breaks, measure calls, and the `\n` experiment.
//! - `cold --variant V --measurer M --input F --out PNG`: one full render in a
//!   fresh process (font discovery, context, render, fix, rasterize, PNG).
//! - `warm --variant V --seed N --out F`: one process, every diagram rendered
//!   once per measurer in a shuffled order; nothing is timed twice.
//!
//! Variants: `unset` (MERMAID_FONT unset), `installed:<family>`, `missing:<name>`.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Instant;

use merman::svg::export::NativeFontContext;
use merman::svg::{
    HostTextMeasurement, HostTextMeasurementRequest, HostTextMeasurer, MeasurementProfileId, SvgEnvironment,
    SvgPipeline, TextMeasurementOperation, TextMeasurementPhase, TextMeasurementPolicy, TextMeasurementProfile,
    TextMeasurementProfileIdentity, TextStyle, WrapMode,
};
use merman::{Engine, MermaidConfig, OperationControl, RenderOutput, RenderRequest, Renderer, SvgRequest};
use resvg::usvg::{self, Node};
use serde::Serialize;

mod fix;
mod fonts;
mod measure_a;
mod measure_b;
mod shaper;
mod survey;

const MERMAN_REV: &str = "482fcde7faffb05a466f198085d615e604075d97";
/// PNG scale and width cap (as S1).
const FULL: (f32, u32) = (2.0, 1000);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
enum Measurer {
    A,
    AMw,
    B,
}

impl Measurer {
    const ALL: [Measurer; 3] = [Measurer::A, Measurer::AMw, Measurer::B];
    fn id(self) -> &'static str {
        match self {
            Measurer::A => "a",
            Measurer::AMw => "a-mw",
            Measurer::B => "b",
        }
    }
    fn parse(s: &str) -> Self {
        match s {
            "a" => Measurer::A,
            "a-mw" => Measurer::AMw,
            "b" => Measurer::B,
            other => panic!("unknown measurer {other}"),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct Input {
    name: String,
    family: String,
    /// `fixture`, `mixed`, `wrap`, or `newline`.
    set: String,
    path: String,
    #[serde(skip)]
    text: String,
}

fn spike_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_corpus() -> Vec<Input> {
    let corpus = spike_dir().join("corpus");
    let rel = |p: &Path| p.strip_prefix(spike_dir()).unwrap_or(p).display().to_string();
    let mut inputs = Vec::new();
    let manifest: Vec<serde_json::Value> =
        serde_json::from_slice(&fs::read(corpus.join("merman/manifest.json")).expect("manifest")).expect("manifest json");
    for e in manifest {
        let family = e["family"].as_str().unwrap().to_string();
        let name = e["name"].as_str().unwrap().to_string();
        let path = corpus.join("merman").join(&family).join(format!("{name}.mmd"));
        inputs.push(Input { text: fs::read_to_string(&path).expect("fixture"), path: rel(&path), set: "fixture".into(), name, family });
    }
    for set in ["mixed", "wrap", "newline"] {
        let mut files: Vec<_> = fs::read_dir(corpus.join(set))
            .expect("corpus set")
            .filter_map(|e| e.ok().map(|e| e.path()))
            .filter(|p| p.extension().is_some_and(|x| x == "mmd"))
            .collect();
        files.sort();
        for path in files {
            let name = path.file_stem().unwrap().to_string_lossy().to_string();
            let family = name.split_once('_').map_or(name.clone(), |(_, f)| f.to_string());
            inputs.push(Input { text: fs::read_to_string(&path).expect("input"), path: rel(&path), set: set.into(), name, family });
        }
    }
    inputs
}

// ------------------------------------------------------------ context

struct Ctx {
    chain: Arc<fonts::FontChain>,
    native: NativeFontContext,
    draw: usvg::Options<'static>,
    renderer: Renderer,
    native_load_ms: f64,
}

fn mermaid_font(variant: &str) -> Option<String> {
    variant.split_once(':').map(|(_, name)| name.to_string())
}

/// Font families Mermaid's stacks request: fixed for Merman's themes, found
/// once by `analyze` from the corpus and cached so `cold` does not render the
/// corpus to learn them.
fn requested_families(inputs: &[Input], renderer: &Renderer) -> BTreeSet<String> {
    let mut requested = BTreeSet::new();
    for input in inputs {
        if let Ok(svg) = render(renderer, &input.text, SvgEnvironment::deterministic()) {
            fonts::requested_families(&svg, &mut requested);
        }
    }
    requested
}

fn build_ctx(variant: &str, requested: &BTreeSet<String>) -> Ctx {
    let discovery = fonts::Discovery::run(mermaid_font(variant));
    let chain = Arc::new(fonts::FontChain::build(&discovery, requested));
    let started = Instant::now();
    let native = measure_b::load_context(&chain.files).expect("native font context");
    let native_load_ms = started.elapsed().as_secs_f64() * 1000.0;
    let draw = chain.usvg_options();
    Ctx { chain, native, draw, renderer: dark_renderer(), native_load_ms }
}

fn dark_renderer() -> Renderer {
    Renderer::new().with_engine(Engine::new().with_site_config(MermaidConfig::from_value(serde_json::json!({ "theme": "dark" }))))
}

fn render(renderer: &Renderer, source: &str, environment: SvgEnvironment) -> Result<String, String> {
    let mut request = SvgRequest { environment, ..SvgRequest::default() };
    request.options.diagram_id = Some("s2".into());
    request.pipeline = Some(SvgPipeline::resvg_safe());
    match renderer.render(RenderRequest::svg(source, OperationControl::new(), request)) {
        Ok(RenderOutput::Svg(Some(svg))) => Ok(fix::fix_merman_svg(svg.svg())),
        Ok(_) => Err("no diagram detected".into()),
        Err(e) => Err(e.to_string()),
    }
}

fn profile_env(measurer: Arc<dyn merman::svg::TextMeasurer + Send + Sync>, id: &str) -> SvgEnvironment {
    let identity = TextMeasurementProfileIdentity::new(MeasurementProfileId::new(id).expect("id"), "1").expect("identity");
    SvgEnvironment::deterministic()
        .with_text_measurement_policy(TextMeasurementPolicy::uniform(TextMeasurementProfile::new(identity, measurer)))
}

#[derive(Debug, Default, Serialize)]
struct Calls {
    total: usize,
    by_method: BTreeMap<String, usize>,
    /// A: lines actually shaped (cache misses).
    shapes: Option<usize>,
    /// B: requests native declined (missing glyph coverage) and Merman
    /// measured with its deterministic fallback instead.
    declines: Option<usize>,
    decline_examples: Vec<String>,
    /// Distinct `font-family` values the measurer was asked for.
    families: BTreeSet<String>,
}

/// Renders with fresh measurer state, instrumented.
fn render_instrumented(ctx: &Ctx, m: Measurer, source: &str) -> (Result<String, String>, Calls) {
    match m {
        Measurer::A => {
            let measurer = Arc::new(measure_a::ShapingMeasurer::new(&ctx.chain));
            let out = render(&ctx.renderer, source, profile_env(Arc::clone(&measurer) as _, "s2-a"));
            let calls = Calls {
                total: measurer.calls.total(),
                by_method: measurer.calls.snapshot(),
                shapes: Some(measurer.shapes.load(std::sync::atomic::Ordering::Relaxed)),
                families: measurer.families.lock().expect("f").clone(),
                ..Calls::default()
            };
            (out, calls)
        }
        Measurer::AMw => {
            let (measurer, calls, inner) = measure_a::a_merman_wrap(&ctx.chain);
            let out = render(&ctx.renderer, source, profile_env(measurer, "s2-a-mw"));
            let calls = Calls {
                total: calls.total(),
                by_method: calls.snapshot(),
                shapes: Some(inner.shapes.load(std::sync::atomic::Ordering::Relaxed)),
                families: inner.families.lock().expect("f").clone(),
                ..Calls::default()
            };
            (out, calls)
        }
        Measurer::B => {
            let control = OperationControl::new();
            let (env, host) = measure_b::counting_environment(&ctx.native, &control);
            let out = render(&ctx.renderer, source, env);
            let calls = Calls {
                total: host.total.load(std::sync::atomic::Ordering::Relaxed),
                by_method: host.calls.lock().expect("calls").clone(),
                declines: Some(host.declines.load(std::sync::atomic::Ordering::Relaxed)),
                decline_examples: host.decline_examples.lock().expect("ex").clone(),
                families: host.families.lock().expect("f").clone(),
                ..Calls::default()
            };
            (out, calls)
        }
    }
}

/// The production render path for timing (B through `with_native_font_context`).
fn render_plain(ctx: &Ctx, m: Measurer, source: &str) -> Result<String, String> {
    match m {
        Measurer::A => render(&ctx.renderer, source, profile_env(Arc::new(measure_a::ShapingMeasurer::new(&ctx.chain)), "s2-a")),
        Measurer::AMw => render(&ctx.renderer, source, profile_env(measure_a::a_merman_wrap(&ctx.chain).0, "s2-a-mw")),
        Measurer::B => render(&ctx.renderer, source, measure_b::environment(&ctx.native)),
    }
}

// ------------------------------------------------------------ analysis

#[derive(Debug, Clone, Serialize)]
struct LabelReport {
    /// The drawn line(s) of one `<text>` node.
    lines: Vec<String>,
    drawn_width: f32,
    /// What the measurer reports for the same line(s), with the drawn style.
    measured_width: f32,
    /// measured - drawn
    diff: f32,
    font: String,
    size: f32,
    /// Characters whose face differs between the measurer and the drawing:
    /// "text: measured-face -> drawn-face".
    fallback_mismatches: Vec<String>,
    /// Non-ASCII glyphs and the face usvg drew them with.
    drawn_faces: Vec<String>,
    /// Text nodes whose chunks share a line are not compared.
    compared: bool,
    #[serde(skip)]
    abs_measured: Option<(f32, f32, f32, f32)>,
}

#[derive(Debug, Default, Serialize)]
struct SvgAnalysis {
    labels: Vec<LabelReport>,
    /// Drawn lines, grouped by parent group, in document order (wrap matching).
    groups: Vec<Vec<String>>,
    defects: Option<survey::Defects>,
    svg_width: f32,
    svg_height: f32,
}

struct Oracle<'a> {
    ctx: &'a Ctx,
    native: NativeFontContext,
    measurer: Measurer,
    b_faces_cache: std::cell::RefCell<BTreeMap<(String, String), Vec<(String, String)>>>,
}

impl Oracle<'_> {
    /// Width the measurer reports for one drawn chunk, plus its per-glyph faces
    /// (as (glyph text, PostScript name)).
    fn measure(&self, text: &str, span: &usvg::TextSpan) -> (f32, Vec<(String, String)>) {
        let query = fonts::FontQuery::from_usvg(span.font());
        let size = span.font_size().get();
        match self.measurer {
            Measurer::A | Measurer::AMw => {
                let Some((face, _)) = self.ctx.chain.resolve(&query) else { return (0.0, vec![]) };
                let params = shaper::ShapeParams {
                    size,
                    letter_spacing: span.letter_spacing(),
                    word_spacing: span.word_spacing(),
                    small_caps: span.small_caps(),
                    kerning: span.apply_kerning(),
                };
                let shaped = shaper::shape(&self.ctx.chain.db, face, text, &params);
                let faces = shaped.glyphs.iter().map(|g| (g.text.clone(), self.ctx.chain.family_name(g.face))).collect();
                (shaped.bbox_width, faces)
            }
            Measurer::B => {
                let style = TextStyle {
                    font_family: Some(query.css_family()),
                    font_size: size as f64,
                    font_weight: Some(query.weight.to_string()),
                    font_style: Some(match query.style {
                        usvg::fontdb::Style::Italic => "italic".into(),
                        usvg::fontdb::Style::Oblique => "oblique".into(),
                        usvg::fontdb::Style::Normal => "normal".into(),
                    }),
                };
                let request = HostTextMeasurementRequest {
                    operation: TextMeasurementOperation::RawBBoxWidth,
                    phase: TextMeasurementPhase::SvgBBox,
                    text,
                    style: &style,
                    max_width: None,
                    wrap_mode: WrapMode::HtmlLike,
                };
                let width = match self.native.measure(request) {
                    Ok(Some(HostTextMeasurement::Length(w))) => w as f32,
                    // Declined: Merman would use its deterministic fallback.
                    _ => f32::NAN,
                };
                (width, self.b_faces(text, &style))
            }
        }
    }

    /// The faces native measurement shapes with: the same probe SVG its
    /// `shape_uncached` builds, parsed with the context's own usvg options.
    fn b_faces(&self, text: &str, style: &TextStyle) -> Vec<(String, String)> {
        let key = (text.to_string(), format!("{:?}", (&style.font_family, style.font_size.to_bits(), &style.font_weight, &style.font_style)));
        if let Some(hit) = self.b_faces_cache.borrow().get(&key) {
            return hit.clone();
        }
        let esc = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;");
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><g font-family="{}" font-size="{}" font-weight="{}" font-style="{}" xml:space="preserve"><text id="start" x="0" y="0">{}</text></g></svg>"#,
            esc(style.font_family.as_deref().unwrap_or("")),
            style.font_size,
            esc(style.font_weight.as_deref().unwrap_or("400")),
            esc(style.font_style.as_deref().unwrap_or("normal")),
            esc(text)
        );
        let mut options = usvg::Options::default();
        self.native.apply_to_usvg_options(&mut options);
        let faces: Vec<(String, String)> = usvg::Tree::from_str(&svg, &options)
            .ok()
            .and_then(|tree| {
                let Some(Node::Text(t)) = tree.node_by_id("start") else { return None };
                Some(
                    t.layouted()
                        .iter()
                        .flat_map(|s| &s.positioned_glyphs)
                        .map(|g| (g.text.clone(), tree.fontdb().face(g.font).map(face_name).unwrap_or_default()))
                        .collect(),
                )
            })
            .unwrap_or_default();
        self.b_faces_cache.borrow_mut().insert(key, faces.clone());
        faces
    }
}

fn face_name(f: &usvg::fontdb::FaceInfo) -> String {
    format!("{}{}", f.post_script_name, if f.index > 0 { format!("#{}", f.index) } else { String::new() })
}

fn analyze_svg(ctx: &Ctx, measurer: Measurer, svg: &str) -> Result<SvgAnalysis, String> {
    let tree = usvg::Tree::from_str(svg, &ctx.draw).map_err(|e| e.to_string())?;
    let oracle = Oracle {
        ctx,
        native: ctx.native.for_operation(&OperationControl::new()),
        measurer,
        b_faces_cache: Default::default(),
    };
    let mut out = SvgAnalysis {
        defects: survey::survey(svg, &ctx.draw).ok(),
        svg_width: tree.size().width(),
        svg_height: tree.size().height(),
        ..Default::default()
    };
    walk(&tree, tree.root(), &oracle, &mut out);
    Ok(out)
}

fn walk(tree: &usvg::Tree, group: &usvg::Group, oracle: &Oracle, out: &mut SvgAnalysis) {
    let mut lines = Vec::new();
    for node in group.children() {
        match node {
            Node::Group(g) => walk(tree, g, oracle, out),
            Node::Text(t) => {
                if let Some(label) = text_label(tree, t, oracle) {
                    lines.extend(label.lines.iter().cloned());
                    out.labels.push(label);
                }
            }
            _ => {}
        }
    }
    if !lines.is_empty() {
        out.groups.push(lines);
    }
}

fn text_label(tree: &usvg::Tree, text: &usvg::Text, oracle: &Oracle) -> Option<LabelReport> {
    let chunks = text.chunks();
    let lines: Vec<String> = chunks.iter().map(|c| c.text().trim().to_string()).filter(|l| !l.is_empty()).collect();
    if lines.is_empty() {
        return None;
    }
    let span = chunks.first()?.spans().first()?;
    let glyphs: Vec<_> = text.layouted().iter().flat_map(|s| s.positioned_glyphs.iter()).collect();
    if glyphs.is_empty() {
        return None;
    }
    // Compare only when every chunk is its own line (a single chunk, or chunks
    // with distinct y) and has a single span.
    let ys: Vec<Option<f32>> = chunks.iter().map(|c| c.y()).collect();
    let distinct = chunks.len() == 1 || (ys.iter().all(Option::is_some) && {
        let mut v: Vec<i64> = ys.iter().map(|y| (y.unwrap() * 10.0) as i64).collect();
        v.sort();
        v.dedup();
        v.len() == ys.len()
    });
    let compared = distinct && chunks.iter().all(|c| c.spans().len() == 1);
    let drawn = text.bounding_box();
    let mut measured = 0.0_f32;
    let mut measured_faces = Vec::new();
    for chunk in chunks {
        let (w, faces) = oracle.measure(chunk.text(), &chunk.spans()[0]);
        measured = if w.is_nan() || measured.is_nan() { f32::NAN } else { measured.max(w) };
        measured_faces.extend(faces);
    }
    let drawn_faces: Vec<(String, String)> =
        glyphs.iter().map(|g| (g.text.clone(), tree.fontdb().face(g.font).map(face_name).unwrap_or_default())).collect();
    let mut mismatches = Vec::new();
    if drawn_faces.len() == measured_faces.len() {
        for ((dt, df), (_, mf)) in drawn_faces.iter().zip(&measured_faces) {
            if df != mf && !dt.trim().is_empty() {
                mismatches.push(format!("{dt}: {mf} -> {df}"));
            }
        }
    } else {
        mismatches.push(format!("glyph count: measured {} vs drawn {}", measured_faces.len(), drawn_faces.len()));
    }
    let non_ascii: BTreeSet<String> =
        drawn_faces.iter().filter(|(t, _)| !t.is_ascii()).map(|(t, f)| format!("{t}:{f}")).collect();
    let abs = text.abs_bounding_box();
    let ts = text.abs_transform();
    let scale = (ts.sx * ts.sx + ts.ky * ts.ky).sqrt();
    let abs_measured = (!measured.is_nan()).then(|| {
        let w = measured * scale;
        let x = match chunks[0].anchor() {
            usvg::TextAnchor::Start => abs.x(),
            usvg::TextAnchor::Middle => abs.x() + abs.width() / 2.0 - w / 2.0,
            usvg::TextAnchor::End => abs.right() - w,
        };
        (x, abs.y(), w, abs.height())
    });
    Some(LabelReport {
        drawn_width: drawn.width(),
        measured_width: measured,
        diff: measured - drawn.width(),
        font: span.font().families().first().map(|f| f.to_string()).unwrap_or_default(),
        size: span.font_size().get(),
        fallback_mismatches: mismatches,
        drawn_faces: non_ascii.into_iter().collect(),
        compared,
        abs_measured,
        lines,
    })
}

// ------------------------------------------------------------ raster

struct Raster {
    png: Vec<u8>,
    width: u32,
    height: u32,
}

fn rasterize(svg: &str, options: &usvg::Options, overlay: &[LabelReport]) -> Result<Raster, String> {
    use resvg::tiny_skia;
    let tree = usvg::Tree::from_str(svg, options).map_err(|e| format!("usvg: {e}"))?;
    let size = tree.size();
    let (factor, cap) = FULL;
    let target = ((size.width() * factor).round() as u32).clamp(1, cap);
    let scale = target as f32 / size.width();
    let width = ((size.width() * scale).round() as u32).max(1);
    let height = ((size.height() * scale).round() as u32).max(1);
    let mut pixmap = tiny_skia::Pixmap::new(width, height).ok_or("pixmap allocation failed")?;
    pixmap.fill(tiny_skia::Color::from_rgba8(0x1f, 0x20, 0x20, 0xff));
    resvg::render(&tree, usvg::Transform::from_scale(scale, scale), &mut pixmap.as_mut());
    // Measured boxes: translucent magenta fill and outline.
    let mut fill = tiny_skia::Paint::default();
    fill.set_color_rgba8(255, 0, 200, 46);
    let mut line = tiny_skia::Paint::default();
    line.set_color_rgba8(255, 60, 220, 200);
    let stroke = tiny_skia::Stroke { width: 1.0, ..Default::default() };
    for label in overlay {
        if let Some((x, y, w, h)) = label.abs_measured
            && let Some(rect) = tiny_skia::Rect::from_xywh(x * scale, y * scale, (w * scale).max(1.0), (h * scale).max(1.0))
        {
            pixmap.fill_rect(rect, &fill, tiny_skia::Transform::identity(), None);
            let path = tiny_skia::PathBuilder::from_rect(rect);
            pixmap.stroke_path(&path, &line, &stroke, tiny_skia::Transform::identity(), None);
        }
    }
    let png = pixmap.encode_png().map_err(|e| e.to_string())?;
    Ok(Raster { png, width, height })
}

// ------------------------------------------------------------ newline

#[derive(Debug, Default, Serialize)]
struct NewlineCase {
    /// The drawn lines of the label that holds "Alpha" (or the first marker).
    lines: Vec<String>,
    literal_backslash_n: bool,
    widest_line: f32,
    /// Width of the label's text (union of its lines).
    text_width: f32,
    /// The smallest filled shape behind the label.
    box_width: Option<f32>,
}

fn newline_case(svg: &str, ctx: &Ctx, marker: &str, end: &str) -> NewlineCase {
    let Ok(tree) = usvg::Tree::from_str(svg, &ctx.draw) else { return NewlineCase::default() };
    // Lines in document order with their absolute boxes.
    let mut texts: Vec<(String, usvg::Rect)> = Vec::new();
    let mut shapes: Vec<usvg::Rect> = Vec::new();
    collect(tree.root(), &mut texts, &mut shapes);
    let Some(start) = texts.iter().position(|(t, _)| t.contains(marker)) else { return NewlineCase::default() };
    let mut picked = Vec::new();
    for (t, r) in &texts[start..] {
        picked.push((t.clone(), *r));
        if t.contains(end) {
            break;
        }
        if picked.len() > 6 {
            break;
        }
    }
    let left = picked.iter().map(|(_, r)| r.left()).fold(f32::INFINITY, f32::min);
    let right = picked.iter().map(|(_, r)| r.right()).fold(f32::NEG_INFINITY, f32::max);
    let top = picked.iter().map(|(_, r)| r.top()).fold(f32::INFINITY, f32::min);
    let bottom = picked.iter().map(|(_, r)| r.bottom()).fold(f32::NEG_INFINITY, f32::max);
    let (cx, cy) = ((left + right) / 2.0, (top + bottom) / 2.0);
    let box_width = shapes
        .iter()
        .filter(|s| s.left() <= cx && cx <= s.right() && s.top() <= cy && cy <= s.bottom() && s.width() >= (right - left) * 0.5)
        .min_by(|a, b| (a.width() * a.height()).total_cmp(&(b.width() * b.height())))
        .map(|s| s.width());
    NewlineCase {
        literal_backslash_n: picked.iter().any(|(t, _)| t.contains("\\n")),
        widest_line: picked.iter().map(|(_, r)| r.width()).fold(0.0, f32::max),
        lines: picked.into_iter().map(|(t, _)| t).collect(),
        text_width: right - left,
        box_width,
    }
}

fn collect(group: &usvg::Group, texts: &mut Vec<(String, usvg::Rect)>, shapes: &mut Vec<usvg::Rect>) {
    for node in group.children() {
        match node {
            Node::Group(g) => collect(g, texts, shapes),
            Node::Text(t) => {
                let content: String = t.chunks().iter().map(|c| c.text()).collect::<Vec<_>>().join("\u{1}");
                for (i, line) in content.split('\u{1}').enumerate() {
                    if !line.trim().is_empty() {
                        // Per-chunk boxes are not exposed; multi-chunk texts share the node box.
                        let _ = i;
                        texts.push((line.trim().to_string(), t.abs_bounding_box()));
                    }
                }
            }
            Node::Path(p) => {
                let filled = p.is_visible() && p.fill().is_some_and(|f| f.opacity().get() > 0.05);
                let b = p.abs_bounding_box();
                if filled && b.width() > 1.0 && b.height() > 1.0 {
                    shapes.push(b);
                }
            }
            _ => {}
        }
    }
}

// ------------------------------------------------------------ analyze

#[derive(Debug, Serialize)]
struct MeasurerResult {
    ok: bool,
    error: Option<String>,
    calls: Calls,
    analysis: Option<SvgAnalysis>,
    png: Option<String>,
    png_width: u32,
    png_height: u32,
    /// Instrumented B equals production B (`with_native_font_context`).
    same_as_production: Option<bool>,
}

#[derive(Debug, Serialize)]
struct NewlineRow {
    measurer: Measurer,
    raw: NewlineCase,
    rewritten: NewlineCase,
    /// `\n` replaced by a space: the one-line box.
    joined: NewlineCase,
    raw_png: Option<String>,
    rewritten_png: Option<String>,
    raw_error: Option<String>,
    rewritten_error: Option<String>,
}

#[derive(Debug, Serialize)]
struct Row {
    input: Input,
    results: BTreeMap<String, MeasurerResult>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    newline: Vec<NewlineRow>,
}

#[derive(Debug, Serialize)]
struct Report {
    os: String,
    variant: String,
    merman_rev: &'static str,
    usvg: &'static str,
    chain: fonts::ChainReport,
    native_context_load_ms: f64,
    native_selected_faces: Vec<String>,
    requested_families: BTreeSet<String>,
    rows: Vec<Row>,
}

fn analyze(os: &str, out: &Path, variant: &str, pngs: bool) {
    fs::create_dir_all(out).expect("out dir");
    let inputs = load_corpus();
    let requested = requested_families(&inputs, &dark_renderer());
    fs::write(out.join("requested-families.json"), serde_json::to_vec_pretty(&requested).unwrap()).unwrap();
    let ctx = build_ctx(variant, &requested);
    eprintln!("{variant}: {} files, fallbacks {:?}", ctx.chain.files.len(), ctx.chain.report.fallbacks);
    let tag = variant.replace([':', ' '], "-");
    let mut rows = Vec::new();
    for input in inputs {
        eprintln!("{} {}", input.set, input.name);
        let mut results = BTreeMap::new();
        let want_png = pngs;
        if input.set != "newline" {
            for m in Measurer::ALL {
                let (svg, calls) = render_instrumented(&ctx, m, &input.text);
                let mut result = MeasurerResult {
                    ok: svg.is_ok(),
                    error: svg.as_ref().err().cloned(),
                    calls,
                    analysis: None,
                    png: None,
                    png_width: 0,
                    png_height: 0,
                    same_as_production: None,
                };
                if let Ok(svg) = &svg {
                    if m == Measurer::B {
                        result.same_as_production = Some(render_plain(&ctx, m, &input.text).as_ref() == Ok(svg));
                    }
                    let analysis = analyze_svg(&ctx, m, svg).ok();
                    if want_png && let Some(a) = &analysis {
                        let name = format!("{}-{}-{tag}.png", input.name, m.id());
                        if let Ok(r) = rasterize(svg, &ctx.draw, &a.labels) {
                            fs::write(out.join(&name), &r.png).unwrap();
                            fs::write(out.join(format!("{}-{}-{tag}.svg", input.name, m.id())), svg).unwrap();
                            result.png = Some(name);
                            result.png_width = r.width;
                            result.png_height = r.height;
                        }
                    }
                    result.analysis = analysis;
                }
                results.insert(m.id().to_string(), result);
            }
        }
        let mut newline = Vec::new();
        if input.set == "newline" {
            let rewritten_source = input.text.replace("\\n", "<br/>");
            // One-line control: the label's box when it has no break at all.
            let joined_source = input.text.replace("\\n", " ");
            for m in [Measurer::B, Measurer::A] {
                let mut row = NewlineRow {
                    measurer: m,
                    raw: NewlineCase::default(),
                    rewritten: NewlineCase::default(),
                    joined: NewlineCase::default(),
                    raw_png: None,
                    rewritten_png: None,
                    raw_error: None,
                    rewritten_error: None,
                };
                for (which, source) in [("raw", &input.text), ("rewritten", &rewritten_source), ("joined", &joined_source)] {
                    match render_plain(&ctx, m, source) {
                        Ok(svg) => {
                            let case = newline_case(&svg, &ctx, "Alpha", "longer");
                            let name = format!("{}-{}-{which}-{tag}.png", input.name, m.id());
                            if which == "joined" {
                                row.joined = case;
                                continue;
                            }
                            if pngs && let Ok(r) = rasterize(&svg, &ctx.draw, &[]) {
                                fs::write(out.join(&name), &r.png).unwrap();
                                if which == "raw" { row.raw_png = Some(name) } else { row.rewritten_png = Some(name) }
                            }
                            if which == "raw" { row.raw = case } else { row.rewritten = case }
                        }
                        Err(_) if which == "joined" => {}
                        Err(e) => {
                            if which == "raw" { row.raw_error = Some(e) } else { row.rewritten_error = Some(e) }
                        }
                    }
                }
                newline.push(row);
            }
        }
        rows.push(Row { input, results, newline });
    }
    let report = Report {
        os: os.into(),
        variant: variant.into(),
        merman_rev: MERMAN_REV,
        usvg: "0.47.0",
        chain: ctx.chain.report.clone(),
        native_context_load_ms: ctx.native_load_ms,
        native_selected_faces: ctx.native.selected_faces(),
        requested_families: requested,
        rows,
    };
    let path = out.join(format!("results-{os}-{tag}.json"));
    fs::write(&path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    eprintln!("wrote {}", path.display());
}

// ------------------------------------------------------------ timing

fn load_requested(out: &Path) -> BTreeSet<String> {
    serde_json::from_slice(&fs::read(out.join("requested-families.json")).expect("run analyze first")).expect("json")
}

/// One full render in this process: discovery, context, render, fix, raster.
fn cold(variant: &str, m: Measurer, input: &Path, out_png: &Path, requested_from: &Path) {
    let requested = load_requested(requested_from);
    let discovery = fonts::Discovery::run(mermaid_font(variant));
    let chain = Arc::new(fonts::FontChain::build(&discovery, &requested));
    let source = fs::read_to_string(input).expect("input");
    let renderer = dark_renderer();
    let env = match m {
        Measurer::A => profile_env(Arc::new(measure_a::ShapingMeasurer::new(&chain)), "s2-a"),
        Measurer::AMw => profile_env(measure_a::a_merman_wrap(&chain).0, "s2-a-mw"),
        Measurer::B => measure_b::environment(&measure_b::load_context(&chain.files).expect("context")),
    };
    let svg = render(&renderer, &source, env).expect("render");
    let raster = rasterize(&svg, &chain.usvg_options(), &[]).expect("raster");
    fs::write(out_png, raster.png).expect("png");
}

#[derive(Debug, Serialize)]
struct WarmSample {
    input: String,
    set: String,
    family: String,
    measurer: Measurer,
    render_ms: f64,
    raster_ms: f64,
}

fn warm(variant: &str, seed: u64, out: &Path, requested_from: &Path) {
    let requested = load_requested(requested_from);
    let ctx = build_ctx(variant, &requested);
    let mut inputs: Vec<Input> = load_corpus().into_iter().filter(|i| i.set != "newline").collect();
    // Deterministic shuffle (xorshift), so each process sees a different order.
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    for i in (1..inputs.len()).rev() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        inputs.swap(i, (state % (i as u64 + 1)) as usize);
    }
    // Warm-up on a diagram outside the corpus, so fonts and code are paged in.
    for m in Measurer::ALL {
        let _ = render_plain(&ctx, m, "flowchart LR\n  warmup1[Warm up the fonts] --> warmup2[and the code paths]\n");
    }
    let mut samples = Vec::new();
    for (i, input) in inputs.iter().enumerate() {
        let mut order = Measurer::ALL.to_vec();
        order.rotate_left((i + seed as usize) % 3);
        for m in order {
            let started = Instant::now();
            let svg = render_plain(&ctx, m, &input.text);
            let render_ms = started.elapsed().as_secs_f64() * 1000.0;
            let raster_ms = match &svg {
                Ok(svg) => {
                    let started = Instant::now();
                    let _ = rasterize(svg, &ctx.draw, &[]);
                    started.elapsed().as_secs_f64() * 1000.0
                }
                Err(_) => f64::NAN,
            };
            samples.push(WarmSample {
                input: input.name.clone(),
                set: input.set.clone(),
                family: input.family.clone(),
                measurer: m,
                render_ms,
                raster_ms,
            });
        }
    }
    fs::write(out, serde_json::to_vec_pretty(&samples).unwrap()).unwrap();
}

fn main() {
    let argv: Vec<String> = std::env::args().collect();
    let flag = |name: &str| argv.iter().position(|a| a == name).and_then(|i| argv.get(i + 1)).cloned();
    let variant = flag("--variant").unwrap_or_else(|| "unset".into());
    match argv.get(1).map(String::as_str) {
        Some("analyze") => analyze(
            &flag("--os").unwrap_or_else(|| std::env::consts::OS.into()),
            Path::new(&flag("--out").expect("--out")),
            &variant,
            !argv.iter().any(|a| a == "--no-png"),
        ),
        Some("cold") => cold(
            &variant,
            Measurer::parse(&flag("--measurer").expect("--measurer")),
            Path::new(&flag("--input").expect("--input")),
            Path::new(&flag("--out").expect("--out")),
            Path::new(&flag("--requested").expect("--requested <analyze out dir>")),
        ),
        Some("warm") => warm(
            &variant,
            flag("--seed").map_or(1, |s| s.parse().expect("seed")),
            Path::new(&flag("--out").expect("--out")),
            Path::new(&flag("--requested").expect("--requested <analyze out dir>")),
        ),
        _ => eprintln!("usage: s2-measurement analyze|cold|warm ..."),
    }
}

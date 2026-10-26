//! Option A: our own Merman `TextMeasurer`, shaping with `shaper.rs` on the
//! chain's database and resolving each `TextStyle` with the same `FontChain`
//! the rasterizer uses. No SVG is parsed.
//!
//! Wrapping is a plain greedy word wrap (split on whitespace, add words while
//! the line fits `max_width`; a word wider than the limit gets its own line).
//! It is deliberately simple: how close it comes to Mermaid.js is one of the
//! things S2 measures. `a_merman_wrap` (below) is the alternative that keeps
//! these widths but lets Merman wrap, through the public
//! `DeterministicTextMeasurer::with_width_callback`.
//!
//! `TextStyle` carries no letter-spacing, word-spacing, small-caps, or kerning,
//! so layout-time measurement always shapes with the defaults; the shaper
//! honors them where the caller has them (the drawn-label oracle).

use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use merman::svg::{TextMeasurer, TextMetrics, TextStyle, WrapMode};
use resvg::usvg::fontdb::ID;

use crate::fonts::{FontChain, FontQuery};
use crate::shaper::{self, ShapeParams, Shaped};

/// Measure calls by trait method.
#[derive(Default)]
pub struct Calls(Mutex<BTreeMap<&'static str, usize>>, AtomicUsize);

impl Calls {
    pub fn hit(&self, name: &'static str) {
        *self.0.lock().expect("calls").entry(name).or_default() += 1;
        self.1.fetch_add(1, Ordering::Relaxed);
    }
    pub fn snapshot(&self) -> BTreeMap<String, usize> {
        self.0.lock().expect("calls").iter().map(|(k, v)| (k.to_string(), *v)).collect()
    }
    pub fn total(&self) -> usize {
        self.1.load(Ordering::Relaxed)
    }
}

pub struct ShapingMeasurer {
    chain: Arc<FontChain>,
    default_family: String,
    resolved: Mutex<HashMap<(Option<String>, Option<String>, Option<String>), Option<ID>>>,
    cache: Mutex<HashMap<(String, ID, u32), Arc<Shaped>>>,
    pub calls: Arc<Calls>,
    pub shapes: AtomicUsize,
    /// Distinct `font-family` values requested during layout.
    pub families: Mutex<std::collections::BTreeSet<String>>,
}

impl ShapingMeasurer {
    pub fn new(chain: &Arc<FontChain>) -> Self {
        ShapingMeasurer {
            chain: Arc::clone(chain),
            default_family: chain.default_family(),
            resolved: Mutex::default(),
            cache: Mutex::default(),
            calls: Arc::default(),
            shapes: AtomicUsize::new(0),
            families: Mutex::default(),
        }
    }

    fn face(&self, style: &TextStyle) -> Option<ID> {
        let key = (style.font_family.clone(), style.font_weight.clone(), style.font_style.clone());
        self.families.lock().expect("families").insert(style.font_family.clone().unwrap_or_else(|| "(none)".into()));
        if let Some(hit) = self.resolved.lock().expect("resolved").get(&key) {
            return *hit;
        }
        let query = FontQuery::from_css(
            style.font_family.as_deref(),
            style.font_weight.as_deref(),
            style.font_style.as_deref(),
            &self.default_family,
        );
        let id = self.chain.resolve(&query).map(|(id, _)| id);
        self.resolved.lock().expect("resolved").insert(key, id);
        id
    }

    /// One line, already whitespace-processed.
    pub fn shape_line(&self, line: &str, style: &TextStyle) -> Arc<Shaped> {
        let Some(face) = self.face(style) else { return Arc::default() };
        let size = style.font_size.max(1.0) as f32;
        let key = (line.to_string(), face, size.to_bits());
        if let Some(hit) = self.cache.lock().expect("cache").get(&key) {
            return Arc::clone(hit);
        }
        self.shapes.fetch_add(1, Ordering::Relaxed);
        let shaped = Arc::new(shaper::shape(&self.chain.db, face, line, &ShapeParams::plain(size)));
        self.cache.lock().expect("cache").insert(key, Arc::clone(&shaped));
        shaped
    }

    fn width(&self, line: &str, style: &TextStyle) -> f64 {
        let line = shaper::collapse_svg_whitespace(line);
        if line.is_empty() { 0.0 } else { self.shape_line(&line, style).advance as f64 }
    }

    /// Explicit breaks: `<br>` variants and newlines.
    fn lines(text: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in text.split('\n') {
            let mut rest = line.to_string();
            while let Some(at) = rest.to_ascii_lowercase().find("<br") {
                let tail = rest[at..].find('>').map_or(String::new(), |e| rest[at + e + 1..].to_string());
                out.push(rest[..at].to_string());
                rest = tail;
            }
            out.push(rest);
        }
        out
    }

    fn greedy(&self, line: &str, style: &TextStyle, limit: f64) -> Vec<f64> {
        let mut widths = Vec::new();
        let mut current = String::new();
        for word in line.split_whitespace() {
            let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
            if !current.is_empty() && self.width(&candidate, style) > limit {
                widths.push(self.width(&current, style));
                current = word.to_string();
            } else {
                current = candidate;
            }
        }
        widths.push(self.width(&current, style));
        widths
    }

    fn metrics(widths: &[f64], style: &TextStyle, mode: WrapMode) -> TextMetrics {
        let factor = if matches!(mode, WrapMode::HtmlLike) { 1.5 } else { 1.1 };
        TextMetrics {
            width: widths.iter().copied().fold(0.0, f64::max),
            height: widths.len() as f64 * style.font_size.max(1.0) * factor,
            line_count: widths.len(),
        }
    }

    fn wrapped(&self, text: &str, style: &TextStyle, max_width: Option<f64>, mode: WrapMode) -> TextMetrics {
        let mut widths = Vec::new();
        for line in Self::lines(text) {
            match max_width.filter(|w| *w > 0.0) {
                Some(limit) => widths.extend(self.greedy(&line, style, limit)),
                None => widths.push(self.width(&line, style)),
            }
        }
        Self::metrics(&widths, style, mode)
    }

    fn joined(text: &str) -> String {
        Self::lines(text).join(" ")
    }
}

impl TextMeasurer for ShapingMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        self.calls.hit("measure");
        let widths: Vec<f64> = Self::lines(text).iter().map(|l| self.width(l, style)).collect();
        Self::metrics(&widths, style, WrapMode::SvgLike)
    }

    fn measure_wrapped(&self, text: &str, style: &TextStyle, max_width: Option<f64>, wrap_mode: WrapMode) -> TextMetrics {
        self.calls.hit("measure_wrapped");
        self.wrapped(text, style, max_width, wrap_mode)
    }

    fn measure_wrapped_with_raw_width(
        &self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f64>,
        wrap_mode: WrapMode,
    ) -> (TextMetrics, Option<f64>) {
        self.calls.hit("measure_wrapped_with_raw_width");
        let raw = Self::lines(text).iter().map(|l| self.width(l, style)).fold(0.0, f64::max);
        (self.wrapped(text, style, max_width, wrap_mode), Some(raw))
    }

    fn measure_html_line_width_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_html_line_width_px");
        self.width(text, style)
    }

    fn measure_svg_text_computed_length_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_svg_text_computed_length_px");
        self.width(text, style)
    }

    fn measure_svg_text_bbox_x_with_ascii_overhang(&self, text: &str, style: &TextStyle) -> (f64, f64) {
        self.calls.hit("measure_svg_text_bbox_x_with_ascii_overhang");
        let line = shaper::collapse_svg_whitespace(text);
        let shaped = self.shape_line(&line, style);
        let half = shaped.advance as f64 / 2.0;
        let (l, r) = shaped.ink.map_or((0.0, shaped.bbox_width as f64), |(l, r)| ((l as f64).min(0.0), (r as f64).max(shaped.bbox_width as f64)));
        ((half - l).max(0.0), (r - half).max(0.0))
    }

    fn measure_svg_simple_text_bbox_width_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_svg_simple_text_bbox_width_px");
        let line = shaper::collapse_svg_whitespace(&Self::joined(text));
        if line.is_empty() { 0.0 } else { self.shape_line(&line, style).bbox_width as f64 }
    }

    fn measure_svg_simple_text_bbox_height_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_svg_simple_text_bbox_height_px");
        let line = shaper::collapse_svg_whitespace(&Self::joined(text));
        if line.is_empty() {
            return 0.0;
        }
        let s = self.shape_line(&line, style);
        (s.ascent - s.descent) as f64
    }

    fn measure_svg_tspan_text_bbox_height_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_svg_tspan_text_bbox_height_px");
        let line = shaper::collapse_svg_whitespace(text);
        if line.is_empty() {
            return 0.0;
        }
        let s = self.shape_line(&line, style);
        (s.ascent - s.descent) as f64
    }

    fn measure_svg_create_text_bbox_y_offset_px(&self, text: &str, style: &TextStyle) -> f64 {
        self.calls.hit("measure_svg_create_text_bbox_y_offset_px");
        // <text y="-10.1"><tspan y="-0.1em" dy="1.1em">: the baseline sits at 1em.
        let line = shaper::collapse_svg_whitespace(text);
        if line.is_empty() {
            return 0.0;
        }
        style.font_size - self.shape_line(&line, style).ascent as f64
    }
}

/// Option A's widths with Merman's own wrapping (the cheap way to close a
/// wrapping gap): `DeterministicTextMeasurer::with_width_callback`, public in
/// the `merman::svg` facade.
pub fn a_merman_wrap(chain: &Arc<FontChain>) -> (Arc<dyn TextMeasurer + Send + Sync>, Arc<Calls>, Arc<ShapingMeasurer>) {
    let inner = Arc::new(ShapingMeasurer::new(chain));
    let calls = Arc::new(Calls::default());
    let (m, c) = (Arc::clone(&inner), Arc::clone(&calls));
    let measurer = merman::svg::DeterministicTextMeasurer::default().with_width_callback(move |text, style| {
        c.hit("width_callback");
        // Merman's contract: measure the complete candidate string as given
        // (whitespace included; U+00AD has no advance in shaping either).
        // Collapsing it here, as the first S2 pass did, broke kanban lines
        // later than Mermaid.js does.
        let line: String = text.chars().filter(|c| !matches!(c, '\n' | '\r')).collect();
        if line.is_empty() { 0.0 } else { m.shape_line(&line, style).advance as f64 }
    });
    (measurer, calls, inner)
}

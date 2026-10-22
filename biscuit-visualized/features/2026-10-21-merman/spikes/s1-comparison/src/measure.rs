//! The "Merman (real fonts)" column's measurer: a Merman `TextMeasurer` on our
//! own resvg 0.45 font database and `FontChain` resolver, installed through
//! Merman's public `TextMeasurementProfile` API.
//!
//! Each label is shaped exactly as the rasterizer will draw it: a one-text SVG
//! parsed by usvg with the same `fontdb` and the same resolution function,
//! width from the text's bounding box (as Merman's native measurer does).
//! Missing glyphs fall back through usvg's `select_fallback` in both, so
//! measuring and drawing agree there too.
//!
//! Limitation: Merman's wrapping helper (`try_measure_wrapped_text_with_line_metrics`)
//! is not exported by the `merman` facade, and `TextMeasurer::measure_wrapped`'s
//! default ignores `max_width`, so this measurer wraps with a plain greedy word
//! wrap. Labels Mermaid wraps may break at different words than Merman would.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use merman::svg::{TextMeasurer, TextMetrics, TextStyle, WrapMode};
use resvg::usvg;

use crate::fonts::FontChain;

pub struct UsvgMeasurer {
    chain: Arc<FontChain>,
    cache: Mutex<HashMap<String, f64>>,
}

impl UsvgMeasurer {
    pub fn new(chain: &Arc<FontChain>) -> Self {
        Self { chain: Arc::clone(chain), cache: Mutex::default() }
    }

    fn line_width(&self, text: &str, style: &TextStyle) -> f64 {
        if text.trim().is_empty() {
            return 0.0;
        }
        let key = format!("{text}\u{0}{:?}{}{:?}{:?}", style.font_family, style.font_size, style.font_weight, style.font_style);
        if let Some(width) = self.cache.lock().expect("cache").get(&key) {
            return *width;
        }
        let escape = |s: &str| s.replace('&', "&amp;").replace('<', "&lt;").replace('"', "&quot;");
        let svg = format!(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="1" height="1"><text id="t" font-family="{}" font-size="{}" font-weight="{}" font-style="{}" xml:space="preserve">{}</text></svg>"#,
            escape(style.font_family.as_deref().unwrap_or("sans-serif")),
            style.font_size,
            escape(style.font_weight.as_deref().unwrap_or("400")),
            escape(style.font_style.as_deref().unwrap_or("normal")),
            escape(text),
        );
        let options = self.chain.usvg_options();
        let width = usvg::Tree::from_str(&svg, &options)
            .ok()
            .and_then(|tree| match tree.node_by_id("t") {
                Some(usvg::Node::Text(t)) => Some(t.bounding_box().width() as f64),
                _ => None,
            })
            .unwrap_or(0.0);
        self.cache.lock().expect("cache").insert(key, width);
        width
    }

    fn lines(text: &str) -> Vec<&str> {
        let mut out = Vec::new();
        for line in text.split('\n') {
            let mut rest = line;
            while let Some(at) = rest.to_ascii_lowercase().find("<br") {
                out.push(&rest[..at]);
                rest = rest[at..].find('>').map_or("", |e| &rest[at + e + 1..]);
            }
            out.push(rest);
        }
        out
    }

    fn metrics(&self, widths: &[f64], style: &TextStyle, mode: WrapMode) -> TextMetrics {
        let factor = if matches!(mode, WrapMode::HtmlLike) { 1.5 } else { 1.1 };
        TextMetrics {
            width: widths.iter().copied().fold(0.0, f64::max),
            height: widths.len() as f64 * style.font_size * factor,
            line_count: widths.len(),
        }
    }
}

impl TextMeasurer for UsvgMeasurer {
    fn measure(&self, text: &str, style: &TextStyle) -> TextMetrics {
        let widths: Vec<f64> = Self::lines(text).iter().map(|l| self.line_width(l, style)).collect();
        self.metrics(&widths, style, WrapMode::SvgLike)
    }

    fn measure_wrapped(&self, text: &str, style: &TextStyle, max_width: Option<f64>, wrap_mode: WrapMode) -> TextMetrics {
        let mut widths = Vec::new();
        for line in Self::lines(text) {
            let Some(limit) = max_width.filter(|w| *w > 0.0) else {
                widths.push(self.line_width(line, style));
                continue;
            };
            let mut current = String::new();
            for word in line.split_whitespace() {
                let candidate = if current.is_empty() { word.to_string() } else { format!("{current} {word}") };
                if !current.is_empty() && self.line_width(&candidate, style) > limit {
                    widths.push(self.line_width(&current, style));
                    current = word.to_string();
                } else {
                    current = candidate;
                }
            }
            widths.push(self.line_width(&current, style));
        }
        self.metrics(&widths, style, wrap_mode)
    }

    fn measure_wrapped_with_raw_width(
        &self,
        text: &str,
        style: &TextStyle,
        max_width: Option<f64>,
        wrap_mode: WrapMode,
    ) -> (TextMetrics, Option<f64>) {
        let raw = Self::lines(text).iter().map(|l| self.line_width(l, style)).fold(0.0, f64::max);
        (self.measure_wrapped(text, style, max_width, wrap_mode), Some(raw))
    }
}

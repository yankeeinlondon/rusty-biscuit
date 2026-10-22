//! Merman's text measurer: label widths shaped in the face the rasterizer
//! draws them in.
//!
//! [`LabelMeasurer`] resolves each Merman [`TextStyle`] through the same
//! [`FontChain`] the rasterizer's usvg `FontResolver` asks, and shapes with
//! [`shaper`](super::shaper), a mirror of usvg's text layout. Merman does all
//! wrapping: [`LabelMeasurer::text_measurer`] hands Merman only a width
//! callback, through `DeterministicTextMeasurer::with_width_callback`.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use merman::svg::{DeterministicTextMeasurer, TextMeasurer, TextStyle};
use resvg::usvg::fontdb::{ID, Style};

use super::chain::FontChain;
use super::query::{FamilyName, FontQuery, parse_font_family_list};
use super::shaper::{self, ShapeParams, Shaped};

/// A style's family, weight, and style strings, as Merman sends them.
type FaceKey = (Option<String>, Option<String>, Option<String>);

/// Measures label text for Merman, over one font chain.
///
/// Every string is shaped exactly as given: no whitespace is collapsed,
/// because Merman's wrapping measures complete candidate strings, spaces
/// included (collapsing them breaks lines later than Mermaid does).
/// Resolved faces and shaped strings are cached for the measurer's lifetime.
pub(crate) struct LabelMeasurer {
    chain: Arc<FontChain>,
    /// usvg's family for text that names none (`usvg::Options::font_family`).
    default_family: String,
    faces: Mutex<HashMap<FaceKey, Option<ID>>>,
    shaped: Mutex<HashMap<(String, FaceKey, u32), Arc<Shaped>>>,
}

impl LabelMeasurer {
    pub(crate) fn new(chain: &Arc<FontChain>) -> Self {
        LabelMeasurer {
            chain: Arc::clone(chain),
            default_family: crate::raster::usvg_options(chain).font_family,
            faces: Mutex::default(),
            shaped: Mutex::default(),
        }
    }

    /// A measurer for Merman's render API, over `chain`.
    pub(crate) fn text_measurer(chain: &Arc<FontChain>) -> Arc<dyn TextMeasurer + Send + Sync> {
        Arc::new(LabelMeasurer::new(chain)).into_text_measurer()
    }

    /// This measurer as Merman's [`TextMeasurer`]: Merman wraps, and asks
    /// [`width`](Self::width) for each candidate string.
    pub(crate) fn into_text_measurer(self: Arc<Self>) -> Arc<dyn TextMeasurer + Send + Sync> {
        DeterministicTextMeasurer::default()
            .with_width_callback(move |text, style| self.width(text, style))
    }

    /// The font request usvg makes for a `<text>` drawn in `style`: the
    /// `font-family` list (usvg's default family when empty), and the weight
    /// and style values usvg's parser recognizes, others reading as normal.
    pub(crate) fn query(&self, style: &TextStyle) -> FontQuery {
        // Merman sometimes passes a CSS declaration's value with its `;`.
        let mut families = style
            .font_family
            .as_deref()
            .map(|list| parse_font_family_list(list.trim().trim_end_matches(';')))
            .unwrap_or_default();
        if families.is_empty() {
            families.push(FamilyName::from_name(&self.default_family));
        }
        // usvg's `resolve_font_weight` for a value on a 400-weight parent.
        let weight = match style.font_weight.as_deref().map(str::trim) {
            Some("bold" | "bolder") => 700,
            Some("lighter") => 200,
            Some(value @ ("100" | "200" | "300" | "500" | "600" | "700" | "800" | "900")) => {
                value.parse().unwrap_or(400)
            }
            _ => 400,
        };
        let font_style = match style.font_style.as_deref().map(str::trim) {
            Some("italic") => Style::Italic,
            Some("oblique") => Style::Oblique,
            _ => Style::Normal,
        };
        FontQuery::new(families)
            .with_weight(weight)
            .with_style(font_style)
    }

    /// The width Merman lays `text` out with: the sum of its shaped advances,
    /// in CSS pixels. Zero when no face resolves (the text is not drawn).
    pub(crate) fn width(&self, text: &str, style: &TextStyle) -> f64 {
        f64::from(self.shape(text, style).advance)
    }

    /// The face of each glyph `text` shapes to in `style`, in visual order:
    /// the faces the rasterizer draws it with.
    pub(crate) fn faces(&self, text: &str, style: &TextStyle) -> Vec<ID> {
        self.shape(text, style)
            .glyphs
            .iter()
            .map(|glyph| glyph.face)
            .collect()
    }

    /// `text` shaped in `style`, cached.
    pub(crate) fn shape(&self, text: &str, style: &TextStyle) -> Arc<Shaped> {
        let size = style.font_size as f32;
        if text.is_empty() || !(size > 0.0 && size.is_finite()) {
            return Arc::default();
        }
        let face_key: FaceKey = (
            style.font_family.clone(),
            style.font_weight.clone(),
            style.font_style.clone(),
        );
        let key = (text.to_string(), face_key, size.to_bits());
        if let Some(hit) = lock(&self.shaped).get(&key) {
            return Arc::clone(hit);
        }
        let query = self.query(style);
        let face = *lock(&self.faces)
            .entry(key.1.clone())
            .or_insert_with(|| self.chain.resolve(&query));
        let shaped = Arc::new(face.map_or_else(Shaped::default, |face| {
            shaper::shape(
                &self.chain,
                face,
                text,
                &ShapeParams::for_query(&query, size),
            )
        }));
        lock(&self.shaped).insert(key, Arc::clone(&shaped));
        shaped
    }

    /// `text` shaped for `query` with explicit `params` (spacing, small caps,
    /// kerning), uncached; Merman's styles carry none of those, so only the
    /// oracle test shapes this way.
    #[cfg(test)]
    pub(crate) fn shape_with(&self, text: &str, query: &FontQuery, params: &ShapeParams) -> Shaped {
        self.chain
            .resolve(query)
            .map_or_else(Shaped::default, |face| {
                shaper::shape(&self.chain, face, text, params)
            })
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

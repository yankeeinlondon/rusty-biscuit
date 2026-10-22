//! Shapes one line of text with rustybuzz exactly as usvg 0.47 lays out a
//! horizontal, single-span `<text>` chunk, so a measured width is the drawn
//! width.
//!
//! Every step mirrors a function in usvg 0.47's `src/text/layout.rs`, named
//! at the step; a usvg bump must re-check them (the oracle test does). Out of
//! scope, because Merman's labels do not use them: vertical writing, text on
//! a path, `textLength`, `dx`/`dy`/`rotate`, and chunks of several spans.

use std::num::NonZeroU16;

use resvg::usvg::fontdb::{Database, ID, Stretch, Style};
use rustybuzz::ttf_parser::{self, Tag};
use unicode_script::UnicodeScript;

use super::chain::FontChain;
use super::query::FontQuery;

/// The style values usvg's layout reads from a text span.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShapeParams {
    pub(crate) size: f32,
    pub(crate) letter_spacing: f32,
    pub(crate) word_spacing: f32,
    pub(crate) small_caps: bool,
    pub(crate) kerning: bool,
    /// Font variations, before the automatic `opsz`.
    pub(crate) variations: Vec<rustybuzz::Variation>,
}

impl ShapeParams {
    /// No spacing, kerning on, no small caps, and the variations usvg's
    /// `convert_font` (`src/parser/text.rs`) derives from `query`'s weight,
    /// stretch, and style when no `font-variation-settings` is given.
    pub(crate) fn for_query(query: &FontQuery, size: f32) -> Self {
        let mut variations = Vec::new();
        let mut push = |tag: &[u8; 4], value: f32| {
            variations.push(rustybuzz::Variation {
                tag: Tag::from_bytes(tag),
                value,
            });
        };
        if query.weight != 400 {
            push(b"wght", f32::from(query.weight));
        }
        let width = match query.stretch {
            Stretch::UltraCondensed => 50.0,
            Stretch::ExtraCondensed => 62.5,
            Stretch::Condensed => 75.0,
            Stretch::SemiCondensed => 87.5,
            Stretch::Normal => 100.0,
            Stretch::SemiExpanded => 112.5,
            Stretch::Expanded => 125.0,
            Stretch::ExtraExpanded => 150.0,
            Stretch::UltraExpanded => 200.0,
        };
        if width != 100.0 {
            push(b"wdth", width);
        }
        match query.style {
            Style::Italic => push(b"ital", 1.0),
            Style::Oblique => push(b"slnt", -12.0),
            Style::Normal => {}
        }
        ShapeParams {
            size,
            letter_spacing: 0.0,
            word_spacing: 0.0,
            small_caps: false,
            kerning: true,
            variations,
        }
    }
}

/// One shaped glyph, in visual order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ShapedGlyph {
    /// The glyph id in `face`; 0 is `.notdef` (no face had the character).
    pub(crate) id: u16,
    pub(crate) face: ID,
    /// The source text of the glyph's cluster.
    pub(crate) text: String,
}

/// A shaped line.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct Shaped {
    /// The sum of the cluster advances: where the next text would start, and
    /// the width Merman lays the label out with.
    pub(crate) advance: f32,
    /// The width of usvg's metric bounding box (`usvg::Text::bounding_box`),
    /// which differs from `advance` only when a cluster's advance is not
    /// positive.
    pub(crate) bbox_width: f32,
    pub(crate) glyphs: Vec<ShapedGlyph>,
}

/// What usvg's `DatabaseExt::load_font` keeps of a face. A face it cannot
/// load is not used, so `None` here means the same.
#[derive(Clone, Copy)]
struct FontMetrics {
    units_per_em: f32,
    ascent: f32,
    descent: f32,
}

fn load_metrics(db: &Database, id: ID) -> Option<FontMetrics> {
    db.with_face_data(id, |data, index| {
        let face = ttf_parser::Face::parse(data, index).ok()?;
        let units_per_em = NonZeroU16::new(face.units_per_em())?;
        let (ascent, descent) = (face.ascender(), face.descender());
        // usvg rejects a face with no usable x-height (`load_font`).
        let x_height = face
            .x_height()
            .and_then(|x| u16::try_from(x).ok())
            .and_then(NonZeroU16::new);
        if x_height.is_none() {
            u16::try_from((f32::from(ascent - descent) * 0.45) as i32)
                .ok()
                .and_then(NonZeroU16::new)?;
        }
        Some(FontMetrics {
            units_per_em: f32::from(units_per_em.get()),
            ascent: f32::from(ascent),
            descent: f32::from(descent),
        })
    })?
}

#[derive(Clone)]
struct Glyph {
    id: u16,
    byte_idx: usize,
    text: String,
    width: i32,
    face: ID,
    metrics: FontMetrics,
}

/// Shapes `text`, as given, starting from the resolved face `base`; missing
/// characters fall back through `chain`, as the rasterizer's
/// `FontResolver::select_fallback` does.
pub(crate) fn shape(chain: &FontChain, base: ID, text: &str, params: &ShapeParams) -> Shaped {
    let db = chain.database();
    // usvg's `layout_text` drops a span whose face does not load.
    let Some(base_metrics) = load_metrics(db, base) else {
        return Shaped::default();
    };
    if text.is_empty() {
        return Shaped::default();
    }
    let glyphs = shape_with_fallback(chain, base, base_metrics, text, params);
    layout(text, glyphs, params)
}

/// usvg's `shape_text`: while a glyph is missing, shape the whole text again
/// with the fallback face for the first missing character; take that result
/// whole when it misses nothing, else copy its glyphs into the missing slots
/// (only when both runs have the same glyph count).
fn shape_with_fallback(
    chain: &FontChain,
    base: ID,
    base_metrics: FontMetrics,
    text: &str,
    params: &ShapeParams,
) -> Vec<Glyph> {
    let db = chain.database();
    let mut glyphs = shape_with_face(db, base, base_metrics, text, params).unwrap_or_default();
    let mut used = vec![base];
    while let Some(missing) = glyphs
        .iter()
        .find(|glyph| glyph.id == 0)
        .and_then(|glyph| text[glyph.byte_idx..].chars().next())
    {
        let Some((fallback, metrics)) = chain
            .fallback(missing, &used)
            .and_then(|id| load_metrics(db, id).map(|metrics| (id, metrics)))
        else {
            break;
        };
        let fallback_glyphs =
            shape_with_face(db, fallback, metrics, text, params).unwrap_or_default();
        if fallback_glyphs.iter().all(|glyph| glyph.id != 0) {
            glyphs = fallback_glyphs;
            break;
        }
        if glyphs.len() != fallback_glyphs.len() {
            break;
        }
        for (glyph, replacement) in glyphs.iter_mut().zip(fallback_glyphs) {
            if glyph.id == 0 && replacement.id != 0 {
                *glyph = replacement;
            }
        }
        used.push(fallback);
    }
    glyphs
}

/// usvg's `shape_text_with_font`: bidi visual runs of an LTR paragraph, each
/// shaped in its direction, with `smcp` for small caps, `kern` off when
/// kerning is off, and `opsz` set to the font size when the face has that
/// axis and no explicit value.
fn shape_with_face(
    db: &Database,
    id: ID,
    metrics: FontMetrics,
    text: &str,
    params: &ShapeParams,
) -> Option<Vec<Glyph>> {
    db.with_face_data(id, |data, index| {
        let mut face = rustybuzz::Face::from_slice(data, index)?;
        let opsz = Tag::from_bytes(b"opsz");
        let mut variations = params.variations.clone();
        if !variations.iter().any(|v| v.tag == opsz)
            && face
                .tables()
                .fvar
                .is_some_and(|fvar| fvar.axes.into_iter().any(|axis| axis.tag == opsz))
        {
            variations.push(rustybuzz::Variation {
                tag: opsz,
                value: params.size,
            });
        }
        if !variations.is_empty() {
            face.set_variations(&variations);
        }

        let mut features = Vec::new();
        if params.small_caps {
            features.push(rustybuzz::Feature::new(Tag::from_bytes(b"smcp"), 1, ..));
        }
        if !params.kerning {
            features.push(rustybuzz::Feature::new(Tag::from_bytes(b"kern"), 0, ..));
        }

        let bidi = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
        let paragraph = &bidi.paragraphs[0];
        let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        let mut glyphs = Vec::new();
        for run in runs {
            let sub_text = &text[run.clone()];
            if sub_text.is_empty() {
                continue;
            }
            let ltr = levels[run.start].is_ltr();
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(sub_text);
            buffer.set_direction(if ltr {
                rustybuzz::Direction::LeftToRight
            } else {
                rustybuzz::Direction::RightToLeft
            });
            let output = rustybuzz::shape(&face, &features, buffer);
            let infos = output.glyph_infos();
            let positions = output.glyph_positions();
            for (i, (info, position)) in infos.iter().zip(positions).enumerate() {
                let start = info.cluster as usize;
                let end = if ltr {
                    i.checked_add(1)
                } else {
                    i.checked_sub(1)
                }
                .and_then(|next| infos.get(next))
                .map_or(sub_text.len(), |next| next.cluster as usize);
                glyphs.push(Glyph {
                    id: info.glyph_id as u16,
                    byte_idx: run.start + start,
                    text: sub_text.get(start..end).unwrap_or_default().to_string(),
                    width: position.x_advance,
                    face: id,
                    metrics,
                });
            }
        }
        Some(glyphs)
    })?
}

struct Cluster {
    codepoint: char,
    advance: f32,
    ascent: f32,
    descent: f32,
    glyphs: Vec<Glyph>,
}

/// usvg's `process_chunk` (clusters from glyphs sharing a byte index),
/// `form_glyph_clusters`, `apply_letter_spacing`, `apply_word_spacing`,
/// `resolve_clusters_positions_horizontal`, and `convert_span`'s metric box.
fn layout(text: &str, glyphs: Vec<Glyph>, params: &ShapeParams) -> Shaped {
    let size = params.size;
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut start = 0;
    while start < glyphs.len() {
        let byte_idx = glyphs[start].byte_idx;
        let end = glyphs[start..]
            .iter()
            .position(|glyph| glyph.byte_idx != byte_idx)
            .map_or(glyphs.len(), |offset| start + offset);
        let group = &glyphs[start..end];
        // `form_glyph_clusters`: the cluster advance is its widest glyph.
        let advance = group
            .iter()
            .map(|glyph| glyph.width as f32 * size / glyph.metrics.units_per_em)
            .fold(0.0_f32, f32::max);
        let first = &group[0].metrics;
        let scale = size / first.units_per_em;
        clusters.push(Cluster {
            codepoint: text[byte_idx..].chars().next().unwrap_or(' '),
            advance,
            ascent: first.ascent * scale,
            descent: first.descent * scale,
            glyphs: group.to_vec(),
        });
        start = end;
    }

    // `apply_letter_spacing`: not after the last cluster, not in cursive
    // scripts; a cluster whose advance is no longer positive is emptied.
    if params.letter_spacing.abs() > f32::EPSILON {
        let last = clusters.len().saturating_sub(1);
        for (index, cluster) in clusters.iter_mut().enumerate() {
            if script_supports_letter_spacing(cluster.codepoint.script()) {
                if index != last {
                    cluster.advance += params.letter_spacing;
                }
                if !(cluster.advance > 0.0 && cluster.advance.is_finite()) {
                    cluster.advance = 0.0;
                    cluster.glyphs.clear();
                }
            }
        }
    }
    // `apply_word_spacing`.
    if params.word_spacing.abs() > f32::EPSILON {
        for cluster in &mut clusters {
            if is_word_separator(cluster.codepoint) {
                cluster.advance += params.word_spacing;
            }
        }
    }

    // `convert_span`: each cluster's metric box is its advance (at least 1)
    // by its font's ascent to descent; a box of no height is skipped.
    let mut x = 0.0_f32;
    let (mut left, mut right) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut shaped = Shaped::default();
    for cluster in &clusters {
        let box_width = if cluster.advance <= 0.0 {
            1.0
        } else {
            cluster.advance
        };
        if cluster.ascent - cluster.descent > 0.0 {
            left = left.min(x);
            right = right.max(x + box_width);
        }
        shaped
            .glyphs
            .extend(cluster.glyphs.iter().map(|glyph| ShapedGlyph {
                id: glyph.id,
                face: glyph.face,
                text: glyph.text.clone(),
            }));
        x += cluster.advance;
    }
    shaped.advance = x;
    shaped.bbox_width = if right > left { right - left } else { 0.0 };
    shaped
}

/// usvg's `script_supports_letter_spacing`.
fn script_supports_letter_spacing(script: unicode_script::Script) -> bool {
    use unicode_script::Script;
    !matches!(
        script,
        Script::Arabic
            | Script::Syriac
            | Script::Nko
            | Script::Manichaean
            | Script::Psalter_Pahlavi
            | Script::Mandaic
            | Script::Mongolian
            | Script::Phags_Pa
            | Script::Devanagari
            | Script::Bengali
            | Script::Gurmukhi
            | Script::Modi
            | Script::Sharada
            | Script::Syloti_Nagri
            | Script::Tirhuta
            | Script::Ogham
    )
}

/// usvg's `is_word_separator_characters`.
fn is_word_separator(c: char) -> bool {
    matches!(
        c as u32,
        0x0020 | 0x00A0 | 0x1361 | 0x010100 | 0x010101 | 0x01039F | 0x01091F
    )
}

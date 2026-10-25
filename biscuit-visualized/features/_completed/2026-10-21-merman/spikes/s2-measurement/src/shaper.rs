//! Option A's shaper: rustybuzz directly, mirroring usvg 0.47's text pipeline
//! (`usvg/src/text/layout.rs`) for one horizontal line in one span:
//!
//! - `shape_text`: shape with the resolved face, then while a glyph is missing
//!   ask the fallback selector for a face (usvg's `default_fallback_selector`,
//!   reimplemented below) and either replace every glyph (all matched) or copy
//!   the missing ones (same glyph count), else stop;
//! - `shape_text_with_font`: BiDi visual runs with an LTR paragraph, `smcp` for
//!   small caps, `kern` off when kerning is disabled, automatic `opsz`;
//! - `form_glyph_clusters`: a cluster's advance is its widest glyph;
//! - `apply_letter_spacing` (not after the last cluster, not for cursive
//!   scripts) and `apply_word_spacing`;
//! - `convert_span`: the metric box spans each cluster's advance, a
//!   non-positive advance counting as 1.
//!
//! No SVG is parsed. Vertical writing, text on a path, `textLength`, and
//! multi-span chunks are out of scope (Merman's labels do not use them).

use std::num::NonZeroU16;

use resvg::usvg::fontdb::{Database, ID};
use rustybuzz::ttf_parser::{self, Tag};
use unicode_script::UnicodeScript;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ShapeParams {
    pub size: f32,
    pub letter_spacing: f32,
    pub word_spacing: f32,
    pub small_caps: bool,
    pub kerning: bool,
}

impl ShapeParams {
    pub fn plain(size: f32) -> Self {
        ShapeParams { size, letter_spacing: 0.0, word_spacing: 0.0, small_caps: false, kerning: true }
    }
}

/// One glyph's source text and face, in visual order (the fallback check).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphFace {
    pub text: String,
    pub face: ID,
}

#[derive(Debug, Clone, Default)]
pub struct Shaped {
    /// Sum of cluster advances (SVG `getComputedTextLength`).
    pub advance: f32,
    /// usvg's metric bounding box width.
    pub bbox_width: f32,
    /// Ink extents (glyph outlines) relative to the start, if any glyph has one.
    pub ink: Option<(f32, f32)>,
    /// Highest ascent and lowest descent over the clusters' fonts (px).
    pub ascent: f32,
    pub descent: f32,
    pub glyphs: Vec<GlyphFace>,
    pub missing: usize,
}

#[derive(Clone)]
struct Glyph {
    id: u16,
    byte_idx: usize,
    text: String,
    dx: i32,
    width: i32,
    font: ID,
    upem: f32,
}

/// Shapes `text` (already whitespace-processed, as usvg's chunk text is).
pub fn shape(db: &Database, base: ID, text: &str, params: &ShapeParams) -> Shaped {
    if text.is_empty() {
        return Shaped::default();
    }
    let mut glyphs = shape_with_font(db, base, text, params).unwrap_or_default();
    let mut used = vec![base];
    loop {
        let Some(missing) = glyphs.iter().find(|g| g.id == 0).map(|g| text[g.byte_idx..].chars().next().unwrap_or(' ')) else {
            break;
        };
        let Some(fallback) = select_fallback(db, missing, &used) else { break };
        let fallback_glyphs = shape_with_font(db, fallback, text, params).unwrap_or_default();
        if fallback_glyphs.iter().all(|g| g.id != 0) {
            glyphs = fallback_glyphs;
            break;
        }
        if glyphs.len() != fallback_glyphs.len() {
            break;
        }
        for (g, f) in glyphs.iter_mut().zip(&fallback_glyphs) {
            if g.id == 0 && f.id != 0 {
                *g = f.clone();
            }
        }
        used.push(fallback);
    }
    layout(db, text, glyphs, params)
}

/// usvg's `FontResolver::default_fallback_selector`.
pub fn select_fallback(db: &Database, c: char, exclude: &[ID]) -> Option<ID> {
    let base = db.face(exclude[0])?;
    for face in db.faces() {
        if exclude.contains(&face.id) {
            continue;
        }
        if base.style != face.style && base.weight != face.weight && base.stretch != face.stretch {
            continue;
        }
        let has = db
            .with_face_data(face.id, |data, index| ttf_parser::Face::parse(data, index).ok().and_then(|f| f.glyph_index(c)).is_some())
            .unwrap_or(false);
        if has {
            return Some(face.id);
        }
    }
    None
}

fn shape_with_font(db: &Database, font: ID, text: &str, params: &ShapeParams) -> Option<Vec<Glyph>> {
    db.with_face_data(font, |data, index| {
        let mut face = rustybuzz::Face::from_slice(data, index)?;
        let upem = NonZeroU16::new(face.units_per_em() as u16)?.get() as f32;
        // font-optical-sizing: auto sets `opsz` to the font size.
        if let Some(fvar) = face.tables().fvar
            && fvar.axes.into_iter().any(|a| a.tag == Tag::from_bytes(b"opsz"))
        {
            face.set_variations(&[rustybuzz::Variation { tag: Tag::from_bytes(b"opsz"), value: params.size }]);
        }
        let bidi = unicode_bidi::BidiInfo::new(text, Some(unicode_bidi::Level::ltr()));
        let paragraph = &bidi.paragraphs[0];
        let (levels, runs) = bidi.visual_runs(paragraph, paragraph.range.clone());
        let mut out = Vec::new();
        for run in runs {
            let sub = &text[run.clone()];
            if sub.is_empty() {
                continue;
            }
            let ltr = levels[run.start].is_ltr();
            let mut buffer = rustybuzz::UnicodeBuffer::new();
            buffer.push_str(sub);
            buffer.set_direction(if ltr { rustybuzz::Direction::LeftToRight } else { rustybuzz::Direction::RightToLeft });
            let mut features = Vec::new();
            if params.small_caps {
                features.push(rustybuzz::Feature::new(Tag::from_bytes(b"smcp"), 1, ..));
            }
            if !params.kerning {
                features.push(rustybuzz::Feature::new(Tag::from_bytes(b"kern"), 0, ..));
            }
            let output = rustybuzz::shape(&face, &features, buffer);
            let infos = output.glyph_infos();
            let positions = output.glyph_positions();
            for i in 0..output.len() {
                let start = infos[i].cluster as usize;
                let next = if ltr { i.checked_add(1) } else { i.checked_sub(1) };
                let end = next.and_then(|n| infos.get(n)).map_or(sub.len(), |info| info.cluster as usize);
                out.push(Glyph {
                    id: infos[i].glyph_id as u16,
                    byte_idx: run.start + start,
                    text: sub.get(start..end).unwrap_or("").to_string(),
                    dx: positions[i].x_offset,
                    width: positions[i].x_advance,
                    font,
                    upem,
                });
            }
        }
        Some(out)
    })?
}

struct Cluster {
    codepoint: char,
    advance: f32,
    font: ID,
    glyphs: Vec<Glyph>,
}

fn layout(db: &Database, text: &str, glyphs: Vec<Glyph>, params: &ShapeParams) -> Shaped {
    let size = params.size;
    // GlyphClusters: consecutive glyphs with the same byte index.
    let mut clusters: Vec<Cluster> = Vec::new();
    let mut i = 0;
    while i < glyphs.len() {
        let start = i;
        while i < glyphs.len() && glyphs[i].byte_idx == glyphs[start].byte_idx {
            i += 1;
        }
        let group = &glyphs[start..i];
        let width = group.iter().map(|g| g.width as f32 * size / g.upem).fold(0.0_f32, |a, w| if w > a { w } else { a });
        clusters.push(Cluster {
            codepoint: text[group[0].byte_idx..].chars().next().unwrap_or(' '),
            advance: width,
            font: group[0].font,
            glyphs: group.to_vec(),
        });
    }
    // apply_letter_spacing
    if params.letter_spacing.abs() > f32::EPSILON {
        let n = clusters.len();
        for (index, cluster) in clusters.iter_mut().enumerate() {
            if script_supports_letter_spacing(cluster.codepoint.script()) {
                if index != n - 1 {
                    cluster.advance += params.letter_spacing;
                }
                if !(cluster.advance.is_finite() && cluster.advance >= 0.0) {
                    cluster.advance = 0.0;
                    cluster.glyphs.clear();
                }
            }
        }
    }
    // apply_word_spacing
    if params.word_spacing.abs() > f32::EPSILON {
        for cluster in &mut clusters {
            if matches!(cluster.codepoint as u32, 0x0020 | 0x00A0 | 0x1361 | 0x010100 | 0x010101 | 0x01039F | 0x01091F) {
                cluster.advance += params.word_spacing;
            }
        }
    }
    // Per-font data, parsing each face once per call.
    let mut fonts: Vec<(ID, Option<(f32, f32)>)> = Vec::new();
    for c in &clusters {
        if !fonts.iter().any(|(id, _)| *id == c.font) {
            fonts.push((c.font, vertical_metrics(db, c.font, size)));
        }
    }
    let mut boxes: std::collections::HashMap<(ID, u16), Option<(f32, f32)>> = std::collections::HashMap::new();
    for (id, _) in &fonts {
        let wanted: Vec<u16> = clusters.iter().flat_map(|c| &c.glyphs).filter(|g| g.font == *id).map(|g| g.id).collect();
        db.with_face_data(*id, |data, index| {
            if let Ok(f) = ttf_parser::Face::parse(data, index) {
                for g in wanted {
                    boxes.entry((*id, g)).or_insert_with(|| {
                        f.glyph_bounding_box(ttf_parser::GlyphId(g)).map(|r| (r.x_min as f32, r.x_max as f32))
                    });
                }
            }
        });
    }
    let mut x = 0.0_f32;
    let (mut left, mut right) = (f32::INFINITY, f32::NEG_INFINITY);
    let mut ink: Option<(f32, f32)> = None;
    let (mut ascent, mut descent) = (0.0_f32, 0.0_f32);
    let mut out = Shaped::default();
    for cluster in &clusters {
        let metric_advance = if cluster.advance <= 0.0 { 1.0 } else { cluster.advance };
        left = left.min(x);
        right = right.max(x + metric_advance);
        if let Some((_, Some((a, d)))) = fonts.iter().find(|(id, _)| *id == cluster.font) {
            ascent = ascent.max(*a);
            descent = descent.min(*d);
        }
        let mut gx = 0.0_f32;
        for g in &cluster.glyphs {
            let scale = size / g.upem;
            if let Some(Some(b)) = boxes.get(&(g.font, g.id)) {
                let x0 = x + (gx + g.dx as f32 + b.0) * scale;
                let x1 = x + (gx + g.dx as f32 + b.1) * scale;
                ink = Some(ink.map_or((x0, x1), |(l, r)| (l.min(x0), r.max(x1))));
            }
            gx += g.width as f32;
            out.glyphs.push(GlyphFace { text: g.text.clone(), face: g.font });
            out.missing += usize::from(g.id == 0);
        }
        x += cluster.advance;
    }
    out.advance = x;
    out.bbox_width = if right > left { right - left } else { 0.0 };
    out.ink = ink;
    out.ascent = ascent;
    out.descent = descent;
    out
}

fn vertical_metrics(db: &Database, id: ID, size: f32) -> Option<(f32, f32)> {
    db.with_face_data(id, |data, index| {
        let f = ttf_parser::Face::parse(data, index).ok()?;
        let scale = size / f.units_per_em() as f32;
        Some((f.ascender() as f32 * scale, f.descender() as f32 * scale))
    })?
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

/// SVG `xml:space="default"` processing for one line: newlines removed, tabs
/// to spaces, runs collapsed, ends trimmed.
pub fn collapse_svg_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut pending = false;
    for c in text.chars() {
        match c {
            '\n' | '\r' => {}
            ' ' | '\t' => pending = !out.is_empty(),
            _ => {
                if pending {
                    out.push(' ');
                    pending = false;
                }
                out.push(c);
            }
        }
    }
    out
}

//! The contrast pass: every text element reaches 4.5:1 against the fill
//! behind it (R-T8, D45), whatever its size.
//!
//! The designs already pair the palette's text color only with fills it reads
//! on. This pass catches the rest: a label whose fill Merman's compiled styles
//! did not reach (it keeps the `base` theme's dark text on a dark palette's
//! surface), palette text on an author's `style`/`classDef` fill, and every
//! label drawn on an author's `%%{init}%%` colors, where no compiled design
//! applies and Merman paints the shapes from its stylesheet.
//!
//! The fill behind a `<text>` is found in two steps:
//!
//! 1. **Inline paint.** The last shape painted with an inline fill in the
//!    text's nearest enclosing group or label viewport (a shape in a closed
//!    sibling group counts, as a node's shape group sits behind its label,
//!    but only one level up: it does not reach the siblings of the node),
//!    mixed with what it covers when its fill is translucent, following
//!    Merman's `<use>` placements of label groups. Text with no shape behind
//!    it is on the palette background, which is what the diagram is drawn on.
//! 2. **Resolved paint.** When a text's own color or a shape it may sit on is
//!    painted by the stylesheet or a presentation attribute, usvg resolves
//!    the document's paint (the same parse resvg rasterizes), and the fill
//!    behind is the color found under most of a grid of points over the
//!    text's box: at each point, the fills and strokes drawn before the text
//!    that cover it (inside their clips), mixed through translucent ones down
//!    to the palette background. A thin line under a label loses to what
//!    surrounds it. This is what reaches a sequence `box`, a rectangle drawn
//!    first whose message text sits far from it in the tree.
//!
//! A text is rewritten only when it fails and its color is not one the author
//! wrote as a text color ([`author_text_colors`]): the palette's text, and
//! every color Merman's `base` theme or a family default picked, are library
//! choices; an author's own text color is kept, whatever it sits on. The new
//! color is [`Palette::text_on`] the fill, written on the `<text>` and on each
//! of its `<tspan>`s (Merman's stylesheet paints some spans directly).

use std::borrow::Cow;
use std::collections::HashMap;
use std::sync::Arc;

use resvg::tiny_skia;
use resvg::usvg;

use super::color::{Rgb, TEXT_CONTRAST};
use super::palette::Palette;
use crate::fonts::FontChain;
use crate::raster::usvg_options;

/// The fill a shape puts behind later text.
#[derive(Clone, Copy)]
enum Behind {
    Known(Rgb),
    Unknown,
}

/// One open `<g>` or `<svg>`: the fill its last painted shape (or a closed
/// child group, or its `<use>` placement) left, and whether a shape of its
/// own painted that fill.
struct Frame {
    group: bool,
    fill: Option<Behind>,
    painted: bool,
}

/// A text's drawn color and the fill behind it, by the byte offset of its
/// `<text>` tag.
type Resolved = HashMap<usize, (Rgb, Rgb)>;

/// What one rewriting walk judges with.
struct Judge<'a> {
    palette: &'a Palette,
    author_text: &'a [Rgb],
    placements: &'a HashMap<String, Behind>,
    resolved: Option<&'a Resolved>,
}

/// `svg` with every failing library-chosen text fill rewritten; borrowed when
/// none fails. `author_text` is what [`author_text_colors`] read from the
/// source; `chain` lays text out when paint has to be resolved.
pub(crate) fn readable_text<'a>(
    svg: &'a str,
    palette: &Palette,
    author_text: &[Rgb],
    chain: &Arc<FontChain>,
) -> Cow<'a, str> {
    // Merman's raster-safe labels are drawn once in a label group and placed
    // with `<use href="#…">`; the first walk records what each placement sits
    // on, so the next can judge the label group's text against it.
    let mut placements = HashMap::new();
    let mut judge = Judge { palette, author_text, placements: &HashMap::new(), resolved: None };
    walk(svg, &judge, Some(&mut placements));
    judge.placements = &placements;
    let (out, undecided) = walk(svg, &judge, None);
    if !undecided {
        return out;
    }
    // Resolving paint costs a usvg parse with text layout, so it runs only
    // when inline paint left a text undecided.
    let resolved = resolved_paint(svg, palette.background(), chain);
    judge.resolved = Some(&resolved);
    walk(svg, &judge, None).0
}

/// One pass over `svg`'s tags. With `record`, notes the fill behind every
/// `<use>` and rewrites nothing; without it, rewrites failing text, starting
/// each group named in `placements` with the fill its placement sits on. The
/// flag reports a text that inline paint left undecided while no resolved
/// paint was given.
fn walk<'a>(svg: &'a str, judge: &Judge<'_>, mut record: Option<&mut HashMap<String, Behind>>) -> (Cow<'a, str>, bool) {
    let palette = judge.palette;
    let mut frames: Vec<Frame> = Vec::new();
    let behind = |frames: &[Frame]| {
        frames.iter().rev().find_map(|frame| frame.fill).unwrap_or(Behind::Known(palette.background()))
    };
    let mut out = String::new();
    let mut copied = 0;
    let mut cursor = 0;
    let mut undecided = false;
    // The color a rewritten `<text>` got, for its `<tspan>`s.
    let mut rewriting: Option<Rgb> = None;
    while let Some(offset) = svg[cursor..].find('<') {
        let start = cursor + offset;
        let Some(length) = svg[start..].find('>') else { break };
        let end = start + length + 1;
        let tag = &svg[start..end];
        cursor = end;
        let self_closing = tag.ends_with("/>");
        match tag_name(tag) {
            "g" | "svg" if !self_closing => {
                let placed = attribute(tag, "id").and_then(|id| judge.placements.get(id)).copied();
                frames.push(Frame { group: tag_name(tag) == "g", fill: placed, painted: false });
            }
            "/g" | "/svg" => {
                // A shape drawn in a closed group (a node's shape group) is
                // still behind that group's later siblings (its label), but
                // not behind the siblings of the group that holds both: a
                // node's fill does not spread to the rest of the diagram. A
                // label viewport's own background stays inside it.
                if let Some(Frame { group: true, fill: Some(fill), painted: true }) = frames.pop()
                    && let Some(parent) = frames.last_mut()
                {
                    parent.fill = Some(fill);
                    parent.painted = false;
                }
            }
            "rect" | "path" | "polygon" | "circle" | "ellipse" => {
                if let Some((fill, alpha)) = shape_fill(tag)
                    && !frames.is_empty()
                {
                    // A translucent fill is seen mixed with what it covers.
                    let fill = match (fill, behind(&frames)) {
                        (Behind::Known(fill), _) if alpha >= 1.0 => Behind::Known(fill),
                        (Behind::Known(fill), Behind::Known(under)) => Behind::Known(fill.over(under, alpha)),
                        _ => Behind::Unknown,
                    };
                    if let Some(frame) = frames.last_mut() {
                        frame.fill = Some(fill);
                        frame.painted = true;
                    }
                }
            }
            "use" => {
                if let Some(record) = record.as_deref_mut()
                    && let Some(id) = attribute(tag, "href").or_else(|| attribute(tag, "xlink:href"))
                {
                    record.insert(id.trim_start_matches('#').to_string(), behind(&frames));
                }
            }
            "text" if record.is_none() && !self_closing => {
                rewriting = None;
                let decided = match (behind(&frames), text_fill(tag)) {
                    (Behind::Known(fill), Some(color)) => Some((color, fill)),
                    _ => judge.resolved.and_then(|resolved| resolved.get(&start)).copied(),
                };
                let Some((color, fill)) = decided else {
                    undecided |= judge.resolved.is_none();
                    continue;
                };
                if color.contrast(fill) >= TEXT_CONTRAST || judge.author_text.contains(&color) {
                    continue;
                }
                let readable = palette.text_on(fill);
                out.push_str(&svg[copied..start]);
                out.push_str(&with_text_fill(tag, readable));
                copied = end;
                rewriting = Some(readable);
            }
            "tspan" if !self_closing => {
                if let Some(readable) = rewriting {
                    out.push_str(&svg[copied..start]);
                    out.push_str(&with_text_fill(tag, readable));
                    copied = end;
                }
            }
            "/text" => rewriting = None,
            _ => {}
        }
    }
    if copied == 0 {
        return (Cow::Borrowed(svg), undecided);
    }
    out.push_str(&svg[copied..]);
    (Cow::Owned(out), undecided)
}

/// Every `<text>` usvg draws, by its tag's offset in `svg`: its drawn color
/// and the paint behind it ([`PaintWalk::fill_behind`]), from the document as
/// usvg resolves it (stylesheet, presentation attributes, `<use>`
/// placements, transforms, clips). A text over a gradient or pattern, or in
/// more than one color, is absent.
fn resolved_paint(svg: &str, background: Rgb, chain: &Arc<FontChain>) -> Resolved {
    // usvg ignores ids inside `<use>` placements, where Merman draws its
    // labels, so each `<text>` is announced by a sibling drawn just before it:
    // an unfilled speck whose stroke color numbers the text.
    let mut offsets = Vec::new();
    let mut marked = String::with_capacity(svg.len() + svg.len() / 4);
    let mut copied = 0;
    let mut cursor = 0;
    while let Some(offset) = svg[cursor..].find("<text") {
        let start = cursor + offset;
        let Some(length) = svg[start..].find('>') else { break };
        let tag = &svg[start..=start + length];
        cursor = start + length + 1;
        if tag_name(tag) != "text" || tag.ends_with("/>") || offsets.len() >= MARKER_LIMIT {
            continue;
        }
        offsets.push(start);
        let number = offsets.len();
        marked.push_str(&svg[copied..start]);
        marked.push_str(&format!(
            r#"<rect width="1" height="1" style="fill:none !important;stroke:#{number:06x} !important;stroke-width:{MARKER_WIDTH}px !important;opacity:1 !important"/>"#
        ));
        copied = start;
    }
    marked.push_str(&svg[copied..]);
    let Ok(tree) = usvg::Tree::from_str(&marked, &usvg_options(chain)) else {
        return Resolved::new();
    };
    let mut walk = PaintWalk {
        offsets: &offsets,
        background,
        layers: Vec::new(),
        clips: Vec::new(),
        pending: None,
        resolved: Resolved::new(),
    };
    walk.visit(tree.root(), 1.0);
    walk.resolved
}

/// More texts than a stroke color can number are left to inline paint.
const MARKER_LIMIT: usize = 0xff_ffff;
/// The stroke width that tells a marker from a drawn shape.
const MARKER_WIDTH: f32 = 0.012_345;

/// What a shape paints over the canvas: its fill or its stroke.
#[derive(Clone, Copy)]
enum Part {
    Fill,
    Stroke,
}

/// One painted part of a shape, in paint order, with the clip paths of the
/// groups around it; `None` paint is a gradient or pattern.
struct Layer<'t> {
    path: &'t usvg::Path,
    part: Part,
    clips: Vec<(&'t usvg::ClipPath, tiny_skia::Transform)>,
    paint: Option<Rgb>,
    alpha: f64,
}

/// usvg's tree in paint order: the shapes drawn so far, the clip paths of
/// the open groups, and the text the last marker announced.
struct PaintWalk<'t, 'o> {
    offsets: &'o [usize],
    background: Rgb,
    layers: Vec<Layer<'t>>,
    clips: Vec<(&'t usvg::ClipPath, tiny_skia::Transform)>,
    pending: Option<usize>,
    resolved: Resolved,
}

impl<'t> PaintWalk<'t, '_> {
    fn visit(&mut self, group: &'t usvg::Group, opacity: f64) {
        for node in group.children() {
            match node {
                usvg::Node::Group(child) => {
                    let clipped = child.clip_path().map(|clip| self.clips.push((clip, child.abs_transform()))).is_some();
                    self.visit(child, opacity * f64::from(child.opacity().get()));
                    if clipped {
                        self.clips.pop();
                    }
                }
                usvg::Node::Path(path) => {
                    if let Some(number) = marker_number(path) {
                        self.pending = number.checked_sub(1).and_then(|index| self.offsets.get(index)).copied();
                        continue;
                    }
                    self.pending = None;
                    if path.is_visible() {
                        self.push(path, opacity);
                    }
                }
                usvg::Node::Text(text) => {
                    let Some(offset) = self.pending.take() else { continue };
                    if self.resolved.contains_key(&offset) {
                        continue;
                    }
                    let Some((color, alpha)) = text_color(text) else { continue };
                    let Some(fill) = self.fill_behind(text.abs_bounding_box()) else { continue };
                    self.resolved.insert(offset, (color.over(fill, opacity * alpha), fill));
                }
                usvg::Node::Image(_) => self.pending = None,
            }
        }
    }

    fn push(&mut self, path: &'t usvg::Path, opacity: f64) {
        let fill = path.fill().map(|fill| (Part::Fill, fill.paint(), fill.opacity()));
        let stroke = path.stroke().map(|stroke| (Part::Stroke, stroke.paint(), stroke.opacity()));
        let parts = match path.paint_order() {
            usvg::PaintOrder::FillAndStroke => [fill, stroke],
            usvg::PaintOrder::StrokeAndFill => [stroke, fill],
        };
        for (part, paint, part_opacity) in parts.into_iter().flatten() {
            let paint = match paint {
                usvg::Paint::Color(color) => Some(Rgb::new(color.red, color.green, color.blue)),
                _ => None,
            };
            let alpha = opacity * f64::from(part_opacity.get());
            self.layers.push(Layer { path, part, clips: self.clips.clone(), paint, alpha });
        }
    }

    /// The most frequent color painted across a grid of points over `bbox`
    /// (so a thin line under a label loses to what surrounds it), `None`
    /// when gradients or patterns cover most of it.
    fn fill_behind(&self, bbox: usvg::Rect) -> Option<Rgb> {
        const COLUMNS: u8 = 5;
        const ROWS: u8 = 3;
        let mut counts: Vec<(Option<Rgb>, u8)> = Vec::new();
        for row in 0..ROWS {
            for column in 0..COLUMNS {
                let x = bbox.left() + bbox.width() * (f32::from(column) + 0.5) / f32::from(COLUMNS);
                let y = bbox.top() + bbox.height() * (f32::from(row) + 0.5) / f32::from(ROWS);
                let color = fill_at(&self.layers, x, y, self.background);
                match counts.iter_mut().find(|(seen, _)| *seen == color) {
                    Some((_, count)) => *count += 1,
                    None => counts.push((color, 1)),
                }
            }
        }
        // The first color seen wins a tie, so the result is stable.
        let (color, _) = counts.iter().fold((None, 0), |best, &(color, count)| if count > best.1 { (color, count) } else { best });
        color
    }
}

/// The number a marker's stroke color carries, `None` for a drawn shape.
fn marker_number(path: &usvg::Path) -> Option<usize> {
    let stroke = path.stroke()?;
    if path.fill().is_some() || (stroke.width().get() - MARKER_WIDTH).abs() > f32::EPSILON {
        return None;
    }
    let usvg::Paint::Color(color) = stroke.paint() else { return None };
    Some(usize::from(color.red) << 16 | usize::from(color.green) << 8 | usize::from(color.blue))
}

/// The one solid color of a text's visible spans, and its fill opacity.
fn text_color(text: &usvg::Text) -> Option<(Rgb, f64)> {
    let mut found: Option<(Rgb, f64)> = None;
    for chunk in text.chunks() {
        for span in chunk.spans() {
            if !span.is_visible() || chunk.text()[span.start()..span.end()].trim().is_empty() {
                continue;
            }
            let fill = span.fill()?;
            let usvg::Paint::Color(color) = fill.paint() else { return None };
            let this = (Rgb::new(color.red, color.green, color.blue), f64::from(fill.opacity().get()));
            match found {
                Some(seen) if seen != this => return None,
                _ => found = Some(this),
            }
        }
    }
    found
}

/// The color painted at `(x, y)` by `layers` over `background`; `None` when
/// a gradient or pattern is part of it.
fn fill_at(layers: &[Layer<'_>], x: f32, y: f32, background: Rgb) -> Option<Rgb> {
    let mut over = Vec::new();
    let mut under = background;
    for layer in layers.iter().rev().filter(|layer| covers(layer, x, y)) {
        let paint = layer.paint?;
        if layer.alpha >= 1.0 {
            under = paint;
            break;
        }
        over.push((paint, layer.alpha));
    }
    Some(over.iter().rev().fold(under, |under, (paint, alpha)| paint.over(under, *alpha)))
}

/// Whether `layer` paints the canvas point `(x, y)`: inside its box, then
/// one pixel of it rasterized at that point, inside every clip around it.
fn covers(layer: &Layer<'_>, x: f32, y: f32) -> bool {
    let path = layer.path;
    let bbox = match layer.part {
        Part::Fill => path.abs_bounding_box(),
        Part::Stroke => path.abs_stroke_bounding_box(),
    };
    if x < bbox.left() || x > bbox.right() || y < bbox.top() || y > bbox.bottom() {
        return false;
    }
    let hit = match layer.part {
        Part::Fill => {
            let rule = fill_rule(path);
            hits(x, y, |pixel, paint, at| pixel.fill_path(path.data(), paint, rule, path.abs_transform().post_concat(at), None))
        }
        Part::Stroke => path.stroke().is_some_and(|stroke| {
            let stroke = stroke.to_tiny_skia();
            hits(x, y, |pixel, paint, at| pixel.stroke_path(path.data(), paint, &stroke, path.abs_transform().post_concat(at), None))
        }),
    };
    hit && layer.clips.iter().all(|(clip, transform)| clip_contains(clip, *transform, x, y))
}

/// Whether a clip path, applied in the space `transform` maps to the canvas,
/// lets the point `(x, y)` through.
fn clip_contains(clip: &usvg::ClipPath, transform: tiny_skia::Transform, x: f32, y: f32) -> bool {
    fn inside(group: &usvg::Group, transform: tiny_skia::Transform, x: f32, y: f32) -> bool {
        group.children().iter().any(|node| match node {
            usvg::Node::Path(path) if path.is_visible() => {
                // usvg stores a clip child's `clip-rule` as its fill rule.
                let rule = fill_rule(path);
                hits(x, y, |pixel, paint, at| pixel.fill_path(path.data(), paint, rule, transform.post_concat(at), None))
            }
            usvg::Node::Group(child) => inside(child, transform.pre_concat(child.transform()), x, y),
            _ => false,
        })
    }
    let transform = transform.pre_concat(clip.transform());
    inside(clip.root(), transform, x, y)
        && clip.clip_path().is_none_or(|outer| clip_contains(outer, transform, x, y))
}

fn fill_rule(path: &usvg::Path) -> tiny_skia::FillRule {
    match path.fill().map(usvg::Fill::rule) {
        Some(usvg::FillRule::EvenOdd) => tiny_skia::FillRule::EvenOdd,
        _ => tiny_skia::FillRule::Winding,
    }
}

/// Draws with `draw` into one pixel standing at the canvas point `(x, y)`
/// (the transform it is given maps the canvas there) and reports whether
/// anything landed.
fn hits(x: f32, y: f32, draw: impl FnOnce(&mut tiny_skia::Pixmap, &tiny_skia::Paint<'_>, tiny_skia::Transform)) -> bool {
    let Some(mut pixel) = tiny_skia::Pixmap::new(1, 1) else { return false };
    let paint = tiny_skia::Paint { anti_alias: false, ..tiny_skia::Paint::default() };
    draw(&mut pixel, &paint, tiny_skia::Transform::from_translate(0.5 - x, 0.5 - y));
    pixel.pixel(0, 0).is_some_and(|pixel| pixel.alpha() > 0)
}

/// The colors the author wrote as text colors in `source`: a CSS `color`
/// (`style`, `classDef`), a C4 `$fontColor`/`$textColor`, and a theme
/// variable or config key naming text, a label, a title, or a font, such as
/// `primaryTextColor`, `noteTextColor`, or `titleColor`. A key naming a
/// surface (`labelBackground`, `sectionBkgColor`, `primaryColor`) is not
/// one. Read from the source as written, so it covers `%%{init}%%`,
/// frontmatter, and statements alike.
pub(crate) fn author_text_colors(source: &str) -> Vec<Rgb> {
    let mut colors = Vec::new();
    for (at, _) in source.match_indices([':', '=']) {
        let key_end = source[..at].trim_end_matches([' ', '\t', '"', '\'']).len();
        let key_start = source[..key_end]
            .rfind(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '_' | '$' | '-')))
            .map_or(0, |index| index + 1);
        if !is_text_key(&source[key_start..key_end]) {
            continue;
        }
        let value = source[at + 1..].trim_start_matches([' ', '\t', '"', '\'']);
        let function = value.trim_start_matches(|c: char| c.is_ascii_alphabetic());
        let length = if function.len() < value.len() && function.starts_with('(') {
            value.find(')').map_or(value.len(), |close| close + 1)
        } else {
            value.find([',', ';', '"', '\'', '\n', '}', ')', '!']).unwrap_or(value.len())
        };
        if let Ok(color) = value[..length].trim().parse::<svgtypes::Color>()
            && color.alpha == u8::MAX
        {
            let color = Rgb::new(color.red, color.green, color.blue);
            if !colors.contains(&color) {
                colors.push(color);
            }
        }
    }
    colors
}

fn is_text_key(key: &str) -> bool {
    let key = key.trim_start_matches('$').to_ascii_lowercase();
    if key.contains("text") {
        return true;
    }
    let surface = ["bkg", "bg", "background", "border", "line", "fill", "stroke"];
    !surface.iter().any(|word| key.contains(word))
        && (key == "color" || ["font", "label", "title"].iter().any(|word| key.contains(word)))
}

fn tag_name(tag: &str) -> &str {
    let body = &tag[1..];
    let end = body
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/' && !body.starts_with('/'))
        .unwrap_or(body.len());
    &body[..end]
}

/// What a shape paints and at what opacity: `None` when it paints no fill
/// (transparent), `Unknown` when its fill is not inline.
fn shape_fill(tag: &str) -> Option<(Behind, f64)> {
    let style = attribute(tag, "style").unwrap_or_default();
    let transparent = |value: &str| matches!(value.trim(), "none" | "transparent");
    let opacity = |property: &str| {
        declaration(style, property)
            .or_else(|| attribute(tag, property))
            .and_then(|value| value.trim().parse::<f64>().ok())
            .map_or(1.0, |opacity| opacity.clamp(0.0, 1.0))
    };
    let alpha = opacity("opacity") * opacity("fill-opacity");
    if alpha == 0.0 {
        return None;
    }
    match declaration(style, "fill") {
        Some(value) if transparent(value) => None,
        Some(value) => Some((parse_css_color(value).map_or(Behind::Unknown, Behind::Known), alpha)),
        None if attribute(tag, "fill").is_some_and(transparent) => None,
        None => Some((Behind::Unknown, alpha)),
    }
}

/// A text's own fill: its inline style, else its `fill` attribute.
fn text_fill(tag: &str) -> Option<Rgb> {
    let style = attribute(tag, "style").unwrap_or_default();
    declaration(style, "fill")
        .or_else(|| attribute(tag, "fill"))
        .and_then(parse_css_color)
}

/// `tag` with its fill set to `color`, in the attribute and the inline style.
/// The style's fill goes first: usvg stops reading a style at a declaration
/// it cannot parse (Merman writes `font-family:&quot;` in some labels), so a
/// fill after one would be lost.
fn with_text_fill(tag: &str, color: Rgb) -> String {
    let mut out = tag.to_string();
    if let Some(value) = attribute(tag, "fill") {
        out = out.replacen(&format!(" fill=\"{value}\""), &format!(" fill=\"{color}\""), 1);
    }
    match attribute(&out, "style") {
        Some(style) => {
            let rest: Vec<&str> = style
                .split(';')
                .filter(|item| item.split_once(':').is_none_or(|(property, _)| property.trim() != "fill"))
                .collect();
            let rest = rest.join(";");
            let rewritten = if rest.is_empty() { format!("fill:{color}") } else { format!("fill:{color};{rest}") };
            out.replacen(&format!(" style=\"{style}\""), &format!(" style=\"{rewritten}\""), 1)
        }
        None => {
            let close = if out.ends_with("/>") { out.len() - 2 } else { out.len() - 1 };
            format!("{} style=\"fill:{color}\"{}", &out[..close], &out[close..])
        }
    }
}

/// The last value of `property` in an inline `style`, without `!important`.
fn declaration<'a>(style: &'a str, property: &str) -> Option<&'a str> {
    style.split(';').rev().find_map(|item| {
        let (name, value) = item.split_once(':')?;
        (name.trim() == property).then(|| value.trim().trim_end_matches("!important").trim())
    })
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

/// A CSS color as Merman writes them: hex, `rgb()`/`rgba()`, `hsl()`/`hsla()`
/// (fractional and negative values included), and the few named colors its
/// themes use. A translucent color is not a known fill.
pub(crate) fn parse_css_color(value: &str) -> Option<Rgb> {
    let value = value.trim();
    if value.starts_with('#') {
        return Rgb::from_hex(value);
    }
    let lower = value.to_ascii_lowercase();
    if let Some(arguments) = function_arguments(&lower, &["rgba", "rgb"]) {
        let channels: Vec<f64> = arguments.iter().filter_map(|text| text.parse().ok()).collect();
        if channels.len() < 3 || channels.get(3).is_some_and(|alpha| *alpha < 1.0) {
            return None;
        }
        let channel = |value: f64| value.round().clamp(0.0, 255.0) as u8;
        return Some(Rgb::new(channel(channels[0]), channel(channels[1]), channel(channels[2])));
    }
    if let Some(arguments) = function_arguments(&lower, &["hsla", "hsl"]) {
        let numbers: Vec<f64> = arguments
            .iter()
            .filter_map(|text| text.trim_end_matches('%').parse().ok())
            .collect();
        if numbers.len() < 3 || numbers.get(3).is_some_and(|alpha| *alpha < 1.0) {
            return None;
        }
        return Some(Rgb::from_hsl(numbers[0], numbers[1] / 100.0, numbers[2] / 100.0));
    }
    match lower.as_str() {
        "white" => Some(Rgb::WHITE),
        "black" => Some(Rgb::BLACK),
        "red" => Some(Rgb::new(255, 0, 0)),
        "lightgray" | "lightgrey" => Some(Rgb::new(211, 211, 211)),
        "gray" | "grey" => Some(Rgb::new(128, 128, 128)),
        _ => None,
    }
}

fn function_arguments<'a>(value: &'a str, names: &[&str]) -> Option<Vec<&'a str>> {
    let name = names.iter().find(|name| value.starts_with(&format!("{name}(")))?;
    let inner = value.get(name.len() + 1..)?.strip_suffix(')')?;
    Some(inner.split([',', ' ', '/']).map(str::trim).filter(|part| !part.is_empty()).collect())
}

#[cfg(test)]
mod tests {
    use std::sync::OnceLock;

    use super::*;
    use crate::mermaid::MermaidTheme;

    fn chain() -> &'static Arc<FontChain> {
        static CHAIN: OnceLock<Arc<FontChain>> = OnceLock::new();
        CHAIN.get_or_init(|| Arc::new(FontChain::injected(&[], &[], Some("Inter"))))
    }

    /// Merman's sequence `box LightGrey`: a rect painted by its `fill`
    /// attribute, drawn first in its own group, with the message text and its
    /// stylesheet-painted spans far from it in the tree. Text over the box is
    /// judged on the box (span included); text beside it on the background.
    #[test]
    fn text_over_a_sequence_box_is_judged_on_the_box() {
        let palette = Palette::fixed(MermaidTheme::Dark);
        let svg = concat!(
            r##"<svg xmlns="http://www.w3.org/2000/svg" id="d" viewBox="-50 -10 600 200">"##,
            r##"<style>#d text.text,#d .messageText,#d .messageText&gt;tspan{fill:#cccccc;font-family:Inter;font-size:16px}</style>"##,
            r##"<g><rect x="-20" y="-5" fill="LightGrey" width="300" height="190" class="rect"/>"##,
            r##"<text x="130" y="14" class="text"><tspan x="130">Alice and Bob</tspan></text></g>"##,
            r##"<g><rect x="0" y="40" width="100" height="40" style="fill:#1f2020"/></g>"##,
            r##"<text x="120" y="135" class="messageText">Hello Bob</text>"##,
            r##"<text x="420" y="135" class="messageText">Outside</text>"##,
            "</svg>"
        );
        let fixed = readable_text(svg, &palette, &[], chain()).into_owned();
        let grey = Rgb::new(211, 211, 211);
        let dark = palette.text_on(grey);
        assert!(dark.contrast(grey) >= TEXT_CONTRAST);
        assert!(
            fixed.contains(&format!(r##"<text x="130" y="14" class="text" style="fill:{dark}"><tspan x="130" style="fill:{dark}">Alice and Bob"##)),
            "{fixed}"
        );
        assert!(
            fixed.contains(&format!(r##"<text x="120" y="135" class="messageText" style="fill:{dark}">Hello Bob"##)),
            "{fixed}"
        );
        assert!(fixed.contains(r##"<text x="420" y="135" class="messageText">Outside"##), "{fixed}");
    }

    #[test]
    fn author_text_colors_are_the_text_keys_the_source_writes() {
        let source = concat!(
            "%%{init: {'themeVariables': {'primaryColor': '#ffffff', 'primaryTextColor': '#123456', 'labelBackground': '#00ff00'}}}%%\n",
            "flowchart LR\n",
            "  classDef warm fill:#ff0000,color:rgb(1, 2, 3),stroke:#333\n",
            "  style A color:blue\n",
            "  UpdateRelStyle(a, b, $textColor=\"red\", $lineColor=\"green\")\n",
        );
        assert_eq!(
            author_text_colors(source),
            [Rgb::from_hex("#123456").unwrap(), Rgb::new(1, 2, 3), Rgb::new(0, 0, 255), Rgb::new(255, 0, 0)]
        );
    }
}

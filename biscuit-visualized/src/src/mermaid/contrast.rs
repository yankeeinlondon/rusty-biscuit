//! The contrast pass: every text element reaches 4.5:1 against the fill
//! behind it (R-T8, D45), whatever its size.
//!
//! The designs already pair the palette's text color only with fills it reads
//! on. This pass catches the rest: a label whose fill Merman's compiled styles
//! did not reach (it keeps the `base` theme's dark text on a dark palette's
//! surface), and palette text on an author's `style`/`classDef` fill.
//!
//! The fill behind a `<text>` is the last shape painted with an **inline**
//! fill in its nearest enclosing group or label viewport (a shape in a closed
//! sibling group counts, as a node's shape group sits behind its label),
//! following Merman's `<use>` placements of label groups; a shape painted only
//! by a stylesheet or a presentation attribute is unknown (the stylesheet may
//! override the attribute), and the text is then left alone. Text with no
//! shape behind it is checked against the palette background, which is what
//! the diagram is drawn on.
//!
//! A text is rewritten only when its color or the fill behind it is a palette
//! color, so an author's own text color on an author's own fill is kept. The
//! new color is [`Palette::text_on`] the fill.

use std::borrow::Cow;
use std::collections::HashMap;

use super::color::{Rgb, TEXT_CONTRAST};
use super::palette::Palette;

/// The fill a shape puts behind later text.
#[derive(Clone, Copy)]
enum Behind {
    Known(Rgb),
    Unknown,
}

/// `svg` with every failing text fill rewritten; borrowed when none fails.
pub(crate) fn readable_text<'a>(svg: &'a str, palette: &Palette) -> Cow<'a, str> {
    // Merman's raster-safe labels are drawn once in a label group and placed
    // with `<use href="#…">`; the first walk records what each placement sits
    // on, so the second can judge the label group's text against it.
    let mut placements = HashMap::new();
    walk(svg, palette, &HashMap::new(), Some(&mut placements));
    walk(svg, palette, &placements, None)
}

/// One pass over `svg`'s tags. With `record`, notes the fill behind every
/// `<use>` and rewrites nothing; without it, rewrites failing text, starting
/// each group named in `placements` with the fill its placement sits on.
fn walk<'a>(
    svg: &'a str,
    palette: &Palette,
    placements: &HashMap<String, Behind>,
    mut record: Option<&mut HashMap<String, Behind>>,
) -> Cow<'a, str> {
    let palette_colors = palette_colors(palette);
    let is_palette = |color: Rgb| palette_colors.contains(&color);
    // One frame per open `<g>`/`<svg>`: whether it is a `<g>`, and the fill
    // its last painted shape left.
    let mut frames: Vec<(bool, Option<Behind>)> = Vec::new();
    let behind = |frames: &[(bool, Option<Behind>)]| {
        frames.iter().rev().find_map(|(_, fill)| *fill).unwrap_or(Behind::Known(palette.background()))
    };
    let mut out = String::new();
    let mut copied = 0;
    let mut cursor = 0;
    while let Some(offset) = svg[cursor..].find('<') {
        let start = cursor + offset;
        let Some(length) = svg[start..].find('>') else { break };
        let end = start + length + 1;
        let tag = &svg[start..end];
        cursor = end;
        let self_closing = tag.ends_with("/>");
        match tag_name(tag) {
            "g" | "svg" if !self_closing => {
                let placed = attribute(tag, "id").and_then(|id| placements.get(id)).copied();
                frames.push((tag_name(tag) == "g", placed));
            }
            "/g" | "/svg" => {
                // A shape drawn in a closed group (a node's shape group) is
                // still behind that group's later siblings (its label). A
                // label viewport's own background stays inside it.
                if let Some((true, Some(fill))) = frames.pop()
                    && let Some(parent) = frames.last_mut()
                {
                    parent.1 = Some(fill);
                }
            }
            "rect" | "path" | "polygon" | "circle" | "ellipse" => {
                if let Some(fill) = shape_fill(tag)
                    && let Some(frame) = frames.last_mut()
                {
                    frame.1 = Some(fill);
                }
            }
            "use" => {
                if let Some(record) = record.as_deref_mut()
                    && let Some(id) = attribute(tag, "href").or_else(|| attribute(tag, "xlink:href"))
                {
                    record.insert(id.trim_start_matches('#').to_string(), behind(&frames));
                }
            }
            "text" if record.is_none() => {
                let (Behind::Known(fill), Some(color)) = (behind(&frames), text_fill(tag)) else {
                    continue;
                };
                if color.contrast(fill) >= TEXT_CONTRAST || !(is_palette(color) || is_palette(fill)) {
                    continue;
                }
                out.push_str(&svg[copied..start]);
                out.push_str(&with_text_fill(tag, palette.text_on(fill)));
                copied = end;
            }
            _ => {}
        }
    }
    if copied == 0 {
        return Cow::Borrowed(svg);
    }
    out.push_str(&svg[copied..]);
    Cow::Owned(out)
}

fn palette_colors(palette: &Palette) -> Vec<Rgb> {
    let mut colors = vec![
        palette.background(),
        palette.text(),
        palette.surface(),
        palette.surface_alt(),
        palette.border(),
        palette.line(),
    ];
    colors.extend(palette.accents());
    colors.extend(palette.tints());
    colors
}

fn tag_name(tag: &str) -> &str {
    let body = &tag[1..];
    let end = body
        .find(|c: char| c.is_whitespace() || c == '>' || c == '/' && !body.starts_with('/'))
        .unwrap_or(body.len());
    &body[..end]
}

/// What a shape leaves behind: `None` when it paints no fill (transparent),
/// `Unknown` when its fill is not inline.
fn shape_fill(tag: &str) -> Option<Behind> {
    let style = attribute(tag, "style").unwrap_or_default();
    let transparent = |value: &str| matches!(value.trim(), "none" | "transparent");
    let hidden = |property: &str| {
        declaration(style, property)
            .or_else(|| attribute(tag, property))
            .is_some_and(|value| value.trim().parse::<f64>().is_ok_and(|opacity| opacity == 0.0))
    };
    if hidden("opacity") || hidden("fill-opacity") {
        return None;
    }
    match declaration(style, "fill") {
        Some(value) if transparent(value) => None,
        Some(value) => Some(parse_css_color(value).map_or(Behind::Unknown, Behind::Known)),
        None if attribute(tag, "fill").is_some_and(transparent) => None,
        None => Some(Behind::Unknown),
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
fn with_text_fill(tag: &str, color: Rgb) -> String {
    let mut out = tag.to_string();
    if let Some(value) = attribute(tag, "fill") {
        out = out.replacen(&format!(" fill=\"{value}\""), &format!(" fill=\"{color}\""), 1);
    }
    match attribute(&out, "style") {
        Some(style) => {
            let rewritten: Vec<String> = style
                .split(';')
                .map(|item| match item.split_once(':') {
                    Some((property, _)) if property.trim() == "fill" => format!("fill:{color}"),
                    _ => item.to_string(),
                })
                .collect();
            let rewritten = rewritten.join(";");
            let rewritten = if declaration(style, "fill").is_some() {
                rewritten
            } else {
                format!("fill:{color};{rewritten}")
            };
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

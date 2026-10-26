//! The accent post-process: what compiled themes cannot express (D6, R-T5,
//! R-T6).
//!
//! Every edit is keyed on colors this crate's palette emitted, never on
//! Merman's class names or ids, so an author's `classDef` or `style` paint,
//! which is not a palette color, is never touched:
//!
//! - **Flowchart-like nodes** get a rotating accent stroke. Merman writes the
//!   ordinal palette inline as `fill:<tint> !important;stroke:<border>
//!   !important`; the stroke of a node whose fill is tint `i` becomes accent
//!   `i`. A node whose author set its stroke keeps it, and one whose author set
//!   its fill has no tint to key on.
//! - **Class diagrams** get one accent per class, in document order. Each
//!   class box is painted with the palette's surface fill, which starts the
//!   next class: its fill becomes that accent's tint, and the border strokes
//!   that follow (the outline and the member dividers) become the accent.
//! - **Sequence diagrams** get one accent per participant. Merman paints
//!   actor boxes from a stylesheet rule rather than inline, so there is no
//!   inline palette color to key on; each actor box is the `<rect>` carrying
//!   the participant's `name`, numbered left to right, and the first
//!   participant keeps the palette's first accent, which the design gave
//!   every actor. Nothing is edited when
//!   that accent is absent from the SVG (an author's `%%{init}%%` theme owns
//!   the colors then).
//!
//! Mindmap accents need no edit: Merman supports mindmap's node ordinal
//! palette, so the compiled theme paints them.

use std::borrow::Cow;

use super::color::Rgb;
use super::palette::Palette;

/// `svg` with the accent edits for `family`; borrowed when none applies.
pub(crate) fn apply_accents<'a>(svg: &'a str, palette: &Palette, family: &str) -> Cow<'a, str> {
    let mut out = node_accent_strokes(svg, palette);
    match family {
        "class" => {
            if let Some(edited) = class_accents(&out, palette) {
                out = Cow::Owned(edited);
            }
        }
        "sequence" => {
            if let Some(edited) = actor_accents(&out, palette) {
                out = Cow::Owned(edited);
            }
        }
        _ => {}
    }
    out
}

fn node_accent_strokes<'a>(svg: &'a str, palette: &Palette) -> Cow<'a, str> {
    let border = palette.border();
    let mut out = Cow::Borrowed(svg);
    for (tint, accent) in palette.tints().into_iter().zip(palette.accents()) {
        if tint == border {
            continue;
        }
        for (from, to) in [
            (
                format!("fill:{tint} !important;stroke:{border} !important"),
                format!("fill:{tint} !important;stroke:{accent} !important"),
            ),
            (
                format!("stroke:{border} !important;fill:{tint} !important"),
                format!("stroke:{accent} !important;fill:{tint} !important"),
            ),
        ] {
            if out.contains(&from) {
                out = Cow::Owned(out.replace(&from, &to));
            }
        }
    }
    out
}

/// Rotates accents over class boxes in document order (see the module docs),
/// or `None` when the SVG has no class box painted with the surface fill.
fn class_accents(svg: &str, palette: &Palette) -> Option<String> {
    let surface = palette.surface();
    let border = palette.border();
    // The keys must be unambiguous: a border equal to the surface or to the
    // edge color would recolor fills or relationship lines too. (Text is
    // painted with `fill`, which the stroke rewrite never touches.)
    if surface == border || palette.line() == border {
        return None;
    }
    let fill_keys = [format!("fill:{surface}"), format!("fill=\"{surface}\"")];
    if !fill_keys.iter().any(|key| svg.contains(key.as_str())) {
        return None;
    }
    let accents = palette.accents();
    let tints = palette.tints();
    let mut out = String::with_capacity(svg.len());
    let mut class: Option<usize> = None;
    let mut rest = svg;
    while let Some(start) = rest.find('<') {
        out.push_str(&rest[..start]);
        let end = rest[start..].find('>').map_or(rest.len(), |end| start + end + 1);
        let tag = &rest[start..end];
        let mut edited = Cow::Borrowed(tag);
        if fill_keys.iter().any(|key| tag.contains(key.as_str())) {
            let index = class.map_or(0, |index| index + 1);
            class = Some(index);
            let tint = tints[index % tints.len()];
            edited = Cow::Owned(replace_color(tag, surface, tint, &["fill:", "fill=\""]));
        }
        if let Some(index) = class {
            let accent = accents[index % accents.len()];
            let stroked = replace_color(&edited, border, accent, &["stroke:", "stroke=\""]);
            if stroked != *edited {
                edited = Cow::Owned(stroked);
            }
        }
        out.push_str(&edited);
        rest = &rest[end..];
    }
    out.push_str(rest);
    Some(out)
}

/// `tag` with `from` replaced by `to` wherever it directly follows one of
/// `prefixes`.
fn replace_color(tag: &str, from: Rgb, to: Rgb, prefixes: &[&str]) -> String {
    let mut out = tag.to_string();
    for prefix in prefixes {
        out = out.replace(&format!("{prefix}{from}"), &format!("{prefix}{to}"));
    }
    out
}

/// One accent per participant (see the module docs), or `None` when the
/// design's actor accent is absent or there are no actor boxes. Participants
/// are numbered left to right by their boxes' `x`, since Merman writes the
/// bottom row of boxes before the top one.
fn actor_accents(svg: &str, palette: &Palette) -> Option<String> {
    let accents = palette.accents();
    if !svg.contains(&accents[0].to_string()) {
        return None;
    }
    let boxes = || {
        svg.match_indices("<rect").filter_map(|(start, _)| {
            let end = svg[start..].find('>').map_or(svg.len(), |end| start + end + 1);
            let tag = &svg[start..end];
            attribute(tag, "name").map(|name| (start, end, tag, name))
        })
    };
    let mut participants: Vec<(f64, &str)> = Vec::new();
    for (_, _, tag, name) in boxes() {
        let x = attribute(tag, "x").and_then(|x| x.parse::<f64>().ok()).unwrap_or(f64::MAX);
        match participants.iter_mut().find(|(_, seen)| *seen == name) {
            Some(participant) => participant.0 = participant.0.min(x),
            None => participants.push((x, name)),
        }
    }
    if participants.is_empty() {
        return None;
    }
    participants.sort_by(|a, b| a.0.total_cmp(&b.0));
    let mut out = String::with_capacity(svg.len());
    let mut copied = 0;
    for (start, end, tag, name) in boxes() {
        let index = participants.iter().position(|(_, seen)| *seen == name).unwrap_or(0);
        out.push_str(&svg[copied..start]);
        out.push_str(&with_inline_stroke(tag, accents[index % accents.len()]));
        copied = end;
    }
    out.push_str(&svg[copied..]);
    Some(out)
}

/// `tag` with `stroke:<accent> !important` in its inline style, which beats
/// the stylesheet rule that paints actors.
fn with_inline_stroke(tag: &str, accent: Rgb) -> String {
    let declaration = format!("stroke:{accent} !important");
    if let Some(at) = tag.find("style=\"") {
        let value_start = at + "style=\"".len();
        let mut out = tag.to_string();
        out.insert_str(value_start, &format!("{declaration};"));
        return out;
    }
    let close = if tag.ends_with("/>") { tag.len() - 2 } else { tag.len() - 1 };
    format!("{} style=\"{declaration}\"{}", &tag[..close], &tag[close..])
}

/// The value of attribute `name` in `tag`.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let needle = format!(" {name}=\"");
    let start = tag.find(&needle)? + needle.len();
    let len = tag[start..].find('"')?;
    Some(&tag[start..start + len])
}

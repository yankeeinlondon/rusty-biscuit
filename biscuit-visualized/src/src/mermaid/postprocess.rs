//! The two corrections applied to every SVG Merman renders (D33):
//!
//! - **Gradient strokes become solid.** Every non-default theme's neo look
//!   strokes node shapes with `stroke:url(#…)` pointing at an
//!   `objectBoundingBox` `<linearGradient>`. Per the SVG specification such a
//!   paint draws nothing on an element whose bounding box has zero width or
//!   height, so the straight class and ER dividers vanish (resvg and Chrome
//!   alike).
//! - **Text with no font set gets `sans-serif`.** Merman sets the theme's font
//!   stack in a root stylesheet rule (`#<id>{font-family:…}`) for most
//!   families; a family that writes neither that rule nor a root font (wardley)
//!   would otherwise reach usvg's serif default.
//!
//! The HTML static target ([`MermaidTarget::HtmlStatic`]) adds two more,
//! [`html_static`], which never reach the rasterizer (D37, D60): every
//! `font-family` starts with the face the label was measured in, and label
//! viewports let text overflow.
//!
//! All edits are string-level and key on the shape of Merman's output at the
//! pinned rev; a Merman bump must re-check them (the guard tests in
//! `src/tests/postprocess_tests.rs` fail when one becomes dead).
//!
//! [`MermaidTarget::HtmlStatic`]: super::MermaidTarget::HtmlStatic

use std::borrow::Cow;

use merman::svg::TextStyle;

use crate::fonts::LabelMeasurer;

/// Applies the gradient-stroke fix and the `sans-serif` default. An SVG with
/// neither defect is returned borrowed, byte for byte; applying it twice
/// equals applying it once.
pub(crate) fn postprocess(svg: &str) -> Cow<'_, str> {
    let solid = solid_gradient_strokes(svg);
    match default_sans_serif(&solid) {
        Some(with_font) => Cow::Owned(with_font),
        None => solid,
    }
}

/// Rewrites every `stroke:url(#<id>)` whose `<id>` is an `objectBoundingBox`
/// linear gradient to the theme's solid node stroke.
fn solid_gradient_strokes(svg: &str) -> Cow<'_, str> {
    let css = stylesheet(svg).unwrap_or_default();
    let mut out = Cow::Borrowed(svg);
    for gradient in bounding_box_gradients(svg) {
        let target = format!("stroke:url(#{})", gradient.id);
        if !out.contains(&target) {
            continue;
        }
        // Every reference is replaced, not only those on zero-height paths:
        // CSS cannot select an element by its bounding box, and the solid
        // stroke is what the classic look draws. This also ends the neo
        // outlines' fade into the background. The stylesheet's own node stroke
        // is preferred because the gradient's first stop is a lighter tint in
        // the forest, neutral, and base themes.
        let Some(color) = node_stroke_color(css).or(gradient.first_stop) else {
            continue;
        };
        out = Cow::Owned(out.replace(&target, &format!("stroke:{color}")));
    }
    out
}

struct Gradient<'a> {
    id: &'a str,
    first_stop: Option<&'a str>,
}

/// The `<linearGradient>`s painted in `objectBoundingBox` units, which is also
/// the value when `gradientUnits` is absent.
fn bounding_box_gradients(svg: &str) -> Vec<Gradient<'_>> {
    let mut gradients = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find("<linearGradient") {
        rest = &rest[at..];
        let Some(tag_end) = rest.find('>') else { break };
        let tag = &rest[..tag_end];
        let units = attribute(tag, "gradientUnits").unwrap_or("objectBoundingBox");
        if let Some(id) = attribute(tag, "id")
            && units == "objectBoundingBox"
        {
            let body_end = if tag.ends_with('/') {
                tag_end
            } else {
                rest.find("</linearGradient>").unwrap_or(tag_end)
            };
            let body = &rest[tag_end..body_end];
            let first_stop = body.find("<stop").and_then(|stop| {
                attribute(&body[stop..stop + body[stop..].find('>')?], "stop-color")
            });
            gradients.push(Gradient { id, first_stop });
        }
        rest = &rest[tag_end..];
    }
    gradients
}

/// The theme's solid node stroke: the first stylesheet rule outside the neo
/// look (`[data-look=…]`) whose selector styles node shapes (`.node path`,
/// `.node rect`, or `.node .neo-node`; families differ in which they list)
/// and whose `stroke` is not a paint server.
fn node_stroke_color(css: &str) -> Option<&str> {
    css.split('}').find_map(|rule| {
        let (selector, body) = rule.split_once('{')?;
        let styles_nodes = [".node path", ".node rect", ".node .neo-node"]
            .iter()
            .any(|shape| selector.contains(shape));
        if !styles_nodes || selector.contains("[data-look") {
            return None;
        }
        let value = body
            .split(';')
            .find_map(|declaration| declaration.trim().strip_prefix("stroke:"))?
            .trim();
        (!value.is_empty() && !value.starts_with("url(")).then_some(value)
    })
}

/// `svg` with `font-family="sans-serif"` on its root `<svg>`, or `None` when
/// the root already has a font. As a presentation attribute it loses to every
/// stylesheet rule and inline style, and descendants inherit it only where
/// they set no font of their own.
fn default_sans_serif(svg: &str) -> Option<String> {
    let start = svg.find("<svg")?;
    let tag = &svg[start..start + svg[start..].find('>')?];
    if tag.contains("font-family") {
        return None;
    }
    let root_rule_sets_font = attribute(tag, "id").is_some_and(|id| {
        let root = format!("#{id}");
        stylesheet(svg).unwrap_or_default().split('}').any(|rule| {
            rule.split_once('{').is_some_and(|(selector, body)| {
                selector.split(',').any(|item| item.trim() == root) && body.contains("font-family")
            })
        })
    });
    if root_rule_sets_font {
        return None;
    }
    let insert_at = start + "<svg".len();
    Some(format!(
        r#"{} font-family="sans-serif"{}"#,
        &svg[..insert_at],
        &svg[insert_at..]
    ))
}

/// The contents of the first `<style>` element.
fn stylesheet(svg: &str) -> Option<&str> {
    let open = svg.find("<style")?;
    let content = open + svg[open..].find('>')? + 1;
    Some(&svg[content..content + svg[content..].find("</style>")?])
}

fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = tag.find(&key)? + key.len();
    Some(&tag[start..start + tag[start..].find('"')?])
}

/// The HTML static target's edits, applied after [`postprocess`]:
///
/// - **Measured face first.** Every `font-family` value (stylesheet rules,
///   inline `style` declarations, and `font-family` attributes) gets the
///   family of the face `measurer` resolves for that stack prepended, so a
///   browser that has the face draws the label in what it was measured in,
///   and one that lacks it falls back to the original stack. Every label's
///   effective font comes from one of those values or inherits it.
/// - **Labels may overflow.** A nested `<svg>` viewport holding text (Merman
///   wraps HTML labels in one, sized to the measured label) gets
///   `overflow:visible`: a reader's fonts are unknowable, so a wider face
///   must spill out of its box rather than be cut. The root `<svg>` is left
///   alone.
///
/// Text content is never edited. An SVG needing neither edit is returned
/// borrowed; applying it twice equals applying it once.
pub(crate) fn html_static<'a>(svg: &'a str, measurer: &LabelMeasurer) -> Cow<'a, str> {
    let mut out = String::new();
    let mut copied = 0;
    let mut at = 0;
    let mut seen_root = false;
    while let Some(offset) = svg[at..].find('<') {
        let start = at + offset;
        let rest = &svg[start..];
        let skip_to = |end_marker: &str| {
            rest.find(end_marker)
                .map_or(svg.len(), |end| start + end + end_marker.len())
        };
        if rest.starts_with("<!--") {
            at = skip_to("-->");
            continue;
        }
        if rest.starts_with("<![CDATA[") {
            at = skip_to("]]>");
            continue;
        }
        if rest.starts_with("</") || rest.starts_with("<?") || rest.starts_with("<!") {
            at = skip_to(">");
            continue;
        }
        let Some(tag_end) = tag_end(rest) else { break };
        let tag = &rest[..tag_end];
        let name = tag[1..]
            .split(|c: char| c.is_whitespace() || c == '/' || c == '>')
            .next()
            .unwrap_or_default();
        let viewport = name == "svg" && std::mem::replace(&mut seen_root, true);
        let label_viewport = viewport && !tag.ends_with("/>") && viewport_has_text(&rest[tag_end..]);
        if let Some(rewritten) = rewrite_tag(tag, measurer, label_viewport) {
            out.push_str(&svg[copied..start]);
            out.push_str(&rewritten);
            copied = start + tag_end;
        }
        at = start + tag_end;
        if name == "style" && !tag.ends_with("/>") {
            let css_end = svg[at..].find("</style>").map_or(svg.len(), |end| at + end);
            if let Some(css) = rewrite_stylesheet(&svg[at..css_end], measurer) {
                out.push_str(&svg[copied..at]);
                out.push_str(&css);
                copied = css_end;
            }
            at = css_end;
        }
    }
    if copied == 0 {
        return Cow::Borrowed(svg);
    }
    out.push_str(&svg[copied..]);
    Cow::Owned(out)
}

/// The length of the start tag `rest` begins with, through its `>`; quoted
/// attribute values may hold `>`.
fn tag_end(rest: &str) -> Option<usize> {
    let mut quote = None;
    for (index, c) in rest.char_indices() {
        match (quote, c) {
            (None, '"' | '\'') => quote = Some(c),
            (Some(open), _) if c == open => quote = None,
            (None, '>') => return Some(index + 1),
            _ => {}
        }
    }
    None
}

/// Whether the content of a nested `<svg>` (from just after its start tag)
/// holds a `<text>` before the viewport closes.
fn viewport_has_text(content: &str) -> bool {
    let mut depth = 0usize;
    let mut rest = content;
    while let Some(offset) = rest.find('<') {
        rest = &rest[offset..];
        if rest.starts_with("<text") {
            return true;
        }
        if rest.starts_with("</svg") {
            if depth == 0 {
                return false;
            }
            depth -= 1;
        } else if rest.starts_with("<svg") && !rest[..tag_end(rest).unwrap_or(rest.len())].ends_with("/>") {
            depth += 1;
        }
        rest = &rest[1..];
    }
    false
}

/// One attribute of a start tag: its name and the byte range of its value
/// within the tag (quotes excluded).
struct Attribute<'a> {
    name: &'a str,
    value: std::ops::Range<usize>,
}

fn attributes(tag: &str) -> Vec<Attribute<'_>> {
    let mut found = Vec::new();
    let bytes = tag.as_bytes();
    let mut index = tag
        .find(|c: char| c.is_whitespace())
        .unwrap_or(tag.len());
    while index < tag.len() {
        while index < tag.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let name_start = index;
        while index < tag.len() && !matches!(bytes[index], b'=' | b'>' | b'/') && !bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let name = &tag[name_start..index];
        while index < tag.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        if name.is_empty() || bytes.get(index) != Some(&b'=') {
            index += usize::from(name.is_empty());
            continue;
        }
        index += 1;
        while index < tag.len() && bytes[index].is_ascii_whitespace() {
            index += 1;
        }
        let Some(&quote @ (b'"' | b'\'')) = bytes.get(index) else {
            break;
        };
        let value_start = index + 1;
        let Some(length) = tag[value_start..].find(char::from(quote)) else {
            break;
        };
        found.push(Attribute {
            name,
            value: value_start..value_start + length,
        });
        index = value_start + length + 1;
    }
    found
}

/// `tag` with its fonts led by the measured face and, for a label viewport,
/// its overflow made visible; `None` when nothing changes.
fn rewrite_tag(tag: &str, measurer: &LabelMeasurer, label_viewport: bool) -> Option<String> {
    let attributes = attributes(tag);
    let value_of = |name: &str| {
        attributes
            .iter()
            .find(|attribute| attribute.name == name)
            .map(|attribute| decode_entities(&tag[attribute.value.clone()]))
    };
    let context = FontContext {
        weight: value_of("font-weight"),
        style: value_of("font-style"),
    };
    let mut edits: Vec<(std::ops::Range<usize>, String)> = Vec::new();
    let mut overflow_set = false;
    for attribute in &attributes {
        // Decoded first: Merman writes `&quot;` in attribute values, and its
        // `;` must not split a declaration.
        let value = decode_entities(&tag[attribute.value.clone()]);
        let edited = match attribute.name {
            "font-family" => led_by_measured_face(&value, &context, measurer),
            "style" => {
                overflow_set |= label_viewport && has_property(&value, "overflow");
                rewrite_declarations(&value, Some(&context), measurer, label_viewport)
            }
            "overflow" if label_viewport => {
                overflow_set = true;
                (value.trim() != "visible").then(|| "visible".to_string())
            }
            _ => None,
        };
        if let Some(edited) = edited {
            let quote = char::from(tag.as_bytes()[attribute.value.end]);
            edits.push((attribute.value.clone(), encode_entities(&edited, Some(quote))));
        }
    }
    let add_overflow = label_viewport && !overflow_set;
    if edits.is_empty() && !add_overflow {
        return None;
    }
    let mut out = String::with_capacity(tag.len() + 32);
    let mut copied = 0;
    for (range, edited) in edits {
        out.push_str(&tag[copied..range.start]);
        out.push_str(&edited);
        copied = range.end;
    }
    out.push_str(&tag[copied..]);
    if add_overflow {
        // As a presentation attribute it beats the user agent's
        // `svg:not(:root){overflow:hidden}`.
        let close = if out.ends_with("/>") { out.len() - 2 } else { out.len() - 1 };
        out.insert_str(close, r#" overflow="visible""#);
    }
    Some(out)
}

/// The `font-weight` and `font-style` declared beside a `font-family`.
#[derive(Default)]
struct FontContext {
    weight: Option<String>,
    style: Option<String>,
}

/// A stylesheet (the raw content of a `<style>` element) with every rule's
/// `font-family` led by the measured face; `None` when nothing changes.
fn rewrite_stylesheet(raw: &str, measurer: &LabelMeasurer) -> Option<String> {
    // Character data outside a CDATA section is entity-escaped.
    let cdata = raw.contains("<![CDATA[");
    let css = if cdata { raw.to_string() } else { decode_entities(raw) };
    let mut out = String::new();
    let mut changed = false;
    for (index, rule) in css.split('}').enumerate() {
        if index > 0 {
            out.push('}');
        }
        match rule.split_once('{') {
            Some((selector, body)) => {
                out.push_str(selector);
                out.push('{');
                match rewrite_declarations(body, None, measurer, false) {
                    Some(body) => {
                        changed = true;
                        out.push_str(&body);
                    }
                    None => out.push_str(body),
                }
            }
            None => out.push_str(rule),
        }
    }
    changed.then(|| if cdata { out } else { encode_entities(&out, None) })
}

/// A decoded `;`-separated declaration list with its `font-family` led by
/// the measured face and, when `visible_overflow`, its `overflow` made
/// visible; `None` when nothing changes. The weight and style come from the
/// list itself, else from `outer` (an element's attributes).
fn rewrite_declarations(
    declarations: &str,
    outer: Option<&FontContext>,
    measurer: &LabelMeasurer,
    visible_overflow: bool,
) -> Option<String> {
    let declared = |property: &str| {
        declarations.split(';').find_map(|declaration| {
            let (name, value) = declaration.split_once(':')?;
            (name.trim() == property).then(|| value.trim().to_string())
        })
    };
    let context = FontContext {
        weight: declared("font-weight").or_else(|| outer.and_then(|outer| outer.weight.clone())),
        style: declared("font-style").or_else(|| outer.and_then(|outer| outer.style.clone())),
    };
    let mut changed = false;
    let rewritten: Vec<String> = declarations
        .split(';')
        .map(|declaration| {
            let edited = declaration.split_once(':').and_then(|(name, value)| {
                let edited = match name.trim() {
                    "font-family" => led_by_measured_face(value, &context, measurer),
                    "overflow" if visible_overflow => (value.trim() != "visible").then(|| "visible".to_string()),
                    _ => None,
                }?;
                Some(format!("{name}:{edited}"))
            });
            changed |= edited.is_some();
            edited.unwrap_or_else(|| declaration.to_string())
        })
        .collect();
    changed.then(|| rewritten.join(";"))
}

fn has_property(declarations: &str, property: &str) -> bool {
    declarations
        .split(';')
        .any(|declaration| declaration.split_once(':').is_some_and(|(name, _)| name.trim() == property))
}

/// `stack` (a decoded `font-family` value) with the
/// family of its measured face in front, or `None` when no face resolves or
/// the stack already leads with that family as this pass writes it.
///
/// The face is prepended even when the stack's first name is the same family
/// in other casing or quoting (Trebuchet MS on a host that has it), so the
/// output is "measured face, then the original stack" on every host; only
/// this pass's exact spelling counts as done, which keeps it idempotent.
fn led_by_measured_face(stack: &str, context: &FontContext, measurer: &LabelMeasurer) -> Option<String> {
    let style = TextStyle {
        font_family: Some(stack.trim().trim_end_matches("!important").trim().to_string()),
        font_size: 16.0,
        font_weight: context.weight.clone(),
        font_style: context.style.clone(),
    };
    let leading = css_quoted(&measurer.measured_family(&style)?) + ",";
    if stack.trim_start().starts_with(&leading) {
        return None;
    }
    let indent = stack.len() - stack.trim_start().len();
    Some(format!("{}{leading}{}", &stack[..indent], &stack[indent..]))
}

/// `family` as a single-quoted CSS string that is safe unescaped in a
/// stylesheet, a CDATA section, and a double-quoted XML attribute alike:
/// quotes, backslashes, markup characters, and controls become CSS hex
/// escapes.
pub(crate) fn css_quoted(family: &str) -> String {
    let mut quoted = String::from("'");
    for c in family.chars() {
        if matches!(c, '\'' | '"' | '\\' | '<' | '>' | '&') || c.is_control() {
            quoted.push_str(&format!("\\{:x} ", u32::from(c)));
        } else {
            quoted.push(c);
        }
    }
    quoted.push('\'');
    quoted
}

/// `text` escaped for XML character data, or for an attribute value
/// delimited by `quote`; the inverse of [`decode_entities`] for what Merman
/// writes.
fn encode_entities(text: &str, quote: Option<char>) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '"' if quote == Some('"') => out.push_str("&quot;"),
            '\'' if quote == Some('\'') => out.push_str("&apos;"),
            _ => out.push(c),
        }
    }
    out
}

/// `value` with the XML entities Merman writes decoded.
fn decode_entities(value: &str) -> String {
    if !value.contains('&') {
        return value.to_string();
    }
    value
        .replace("&quot;", "\"")
        .replace("&apos;", "'")
        .replace("&#39;", "'")
        .replace("&#x27;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&amp;", "&")
}

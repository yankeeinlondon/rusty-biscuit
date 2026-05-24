//! Bracketed-tag parser that builds the target-neutral [`ProseNode`] tree.
//!
//! The atomic-token grammar (`{{token}}`) has been removed — `{{…}}` is now
//! ordinary literal text. Only bracketed tags (`<tag>…</tag>`) and the
//! Markdown subset (pre-processed into bracketed tags) are recognized.
//!
//! Parsing is target-neutral: no terminal capability decisions are made
//! here. The parser resolves tag names to [`ProseStyle`]/[`ProseNode`]
//! intent only; degradation happens in each target emitter.

use std::iter::Peekable;
use std::str::Chars;

use renderable::color::{BasicColor, Color, RgbColor};
use renderable::style::UnderlineStyle;

use super::ir::{ProseNode, ProseStyle};
use super::markdown::{CODE_BLOCK_PLACEHOLDER_MARK, FencedCode};
use super::styles::{parse_rgb, tailwind_by_name, web_color_by_name};

/// Parse an opening tag into its name and attributes.
pub(super) fn parse_opening_tag(tag_content: &str) -> Option<(String, Vec<(String, String)>)> {
    let tag_content = tag_content.trim();
    if tag_content.is_empty() {
        return None;
    }

    let parts: Vec<&str> = tag_content.splitn(2, |c: char| c.is_whitespace()).collect();
    let tag_name = parts[0].to_lowercase();

    let mut attrs = Vec::new();
    if parts.len() > 1 {
        let attr_str = parts[1];

        // No '=' in the attribute string: treat the whole remainder as a
        // positional value (empty key) — used by `<rgb 1,2,3>`.
        if !attr_str.contains('=') {
            let value = attr_str.trim();
            if !value.is_empty() {
                attrs.push((value.to_string(), String::new()));
            }
        } else {
            let mut current_attr = String::new();
            let mut current_value = String::new();
            let mut in_value = false;
            let mut quote_char: Option<char> = None;

            let mut chars = attr_str.chars().peekable();
            while let Some(c) = chars.next() {
                if in_value {
                    // Resolve the `\<`, `\>`, `\\`, `\"`, `\'` escapes that
                    // `Prose::quoted_attr` applies so the value boundary
                    // survives parsing; the backslash itself is dropped.
                    if c == '\\'
                        && chars
                            .peek()
                            .is_some_and(|&n| matches!(n, '<' | '>' | '\\' | '"' | '\''))
                    {
                        current_value.push(chars.next().unwrap());
                        continue;
                    }
                    if let Some(qc) = quote_char {
                        if c == qc {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                            in_value = false;
                            quote_char = None;
                        } else {
                            current_value.push(c);
                        }
                    } else if c == '"' || c == '\'' {
                        quote_char = Some(c);
                    } else if c.is_whitespace() {
                        if !current_value.is_empty() {
                            attrs.push((current_attr.clone(), current_value.clone()));
                            current_attr.clear();
                            current_value.clear();
                        }
                        in_value = false;
                    } else {
                        current_value.push(c);
                    }
                } else if c == '=' {
                    in_value = true;
                } else if !c.is_whitespace() {
                    current_attr.push(c);
                }
            }

            if !current_attr.is_empty() || !current_value.is_empty() {
                attrs.push((current_attr, current_value));
            }
        }
    }

    Some((tag_name, attrs))
}

/// How a recognized opening tag maps to the IR.
enum TagResolution {
    /// A styled span.
    Styled(ProseStyle),
    /// A hyperlink carrying the raw href.
    Link(String),
    /// A fenced code block with an optional language hint.
    Code(Option<String>),
    /// A tag whose styling is handled outside the rendering pipeline
    /// (`<clipboard>`); only its inner content survives.
    Transparent,
    /// Not a recognized tag — the declaration becomes literal text.
    Unknown,
}

/// Build a foreground-color resolution from `<rgb …>` / `<bg-rgb …>` attrs.
fn rgb_color(attrs: &[(String, String)]) -> Option<Color> {
    let rgb_str = attrs
        .iter()
        .find(|(_, v)| v.is_empty())
        .map(|(k, _)| k.as_str())
        .unwrap_or("");
    parse_rgb(rgb_str).map(|(r, g, b)| Color::Rgb(RgbColor::new(r, g, b, BasicColor::White)))
}

/// Resolve a bracketed-tag name + attributes to a [`TagResolution`].
///
/// Target-neutral: no terminal context, no capability decisions.
fn resolve_tag(tag_name: &str, attrs: &[(String, String)]) -> TagResolution {
    use TagResolution::{Code, Link, Styled, Transparent, Unknown};

    match tag_name {
        "bold" | "b" => Styled(ProseStyle::bold()),
        "dim" => Styled(ProseStyle::dim()),
        "italic" | "i" => Styled(ProseStyle::italic()),
        "underline" | "u" => Styled(ProseStyle::underline(UnderlineStyle::Straight)),
        "double-underline" | "uu" => Styled(ProseStyle::underline(UnderlineStyle::Double)),
        "curly-underline" => Styled(ProseStyle::underline(UnderlineStyle::Curly)),
        "dotted-underline" => Styled(ProseStyle::underline(UnderlineStyle::Dotted)),
        "dashed-underline" => Styled(ProseStyle::underline(UnderlineStyle::Dashed)),
        "blink" => Styled(ProseStyle::blink()),
        "inverse" | "reverse" => Styled(ProseStyle::inverse()),
        "hidden" => Styled(ProseStyle::hidden()),
        "strikethrough" | "~" => Styled(ProseStyle::strikethrough()),

        "a" => {
            let href = attrs
                .iter()
                .find(|(k, _)| k == "href")
                .map(|(_, v)| v.clone())
                .unwrap_or_default();
            Link(href)
        }
        "clipboard" => Transparent,
        "code-block" => {
            let lang = attrs
                .iter()
                .find(|(k, _)| k == "lang")
                .map(|(_, v)| v.clone())
                .filter(|s| !s.is_empty());
            Code(lang)
        }

        "rgb" => rgb_color(attrs)
            .map(|c| Styled(ProseStyle::fg(c)))
            .unwrap_or(Unknown),
        "bg-rgb" => rgb_color(attrs)
            .map(|c| Styled(ProseStyle::bg(c)))
            .unwrap_or(Unknown),

        "black" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Black))),
        "red" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Red))),
        "green" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Green))),
        "yellow" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Yellow))),
        "blue" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Blue))),
        "magenta" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Magenta))),
        "cyan" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::Cyan))),
        "white" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::White))),
        "bright-black" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightBlack))),
        "bright-red" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightRed))),
        "bright-green" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightGreen))),
        "bright-yellow" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightYellow))),
        "bright-blue" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightBlue))),
        "bright-magenta" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightMagenta))),
        "bright-cyan" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightCyan))),
        "bright-white" => Styled(ProseStyle::fg(Color::BasicColor(BasicColor::BrightWhite))),

        _ => {
            if let Some(rest) = tag_name.strip_prefix("bg-") {
                if let Some(wc) = web_color_by_name(rest) {
                    return Styled(ProseStyle::bg(Color::Web(wc)));
                }
                if let Some(tw) = tailwind_by_name(rest) {
                    return Styled(ProseStyle::bg(Color::Tailwind(tw)));
                }
            }
            if let Some(wc) = web_color_by_name(tag_name) {
                return Styled(ProseStyle::fg(Color::Web(wc)));
            }
            if let Some(tw) = tailwind_by_name(tag_name) {
                return Styled(ProseStyle::fg(Color::Tailwind(tw)));
            }
            Unknown
        }
    }
}

/// Consume characters from `chars` up to the matching `</tag_name>` at depth
/// zero, returning the inner content (without the closing tag).
///
/// Tracks nesting depth so `<b>x <b>y</b> z</b>` resolves correctly.
fn scan_inner(chars: &mut Peekable<Chars<'_>>, tag_name: &str) -> String {
    let closing_tag = format!("</{}>", tag_name);
    let opening_tag_full = format!("<{}>", tag_name);
    let opening_tag_with_space = format!("<{} ", tag_name);

    let mut inner = String::new();
    let mut depth = 1;

    while depth > 0 {
        let Some(c) = chars.next() else { break };
        inner.push(c);
        let len = inner.len();

        if c == '>' {
            if len >= opening_tag_full.len() {
                let start = len - opening_tag_full.len();
                if inner
                    .get(start..)
                    .is_some_and(|slice| slice.eq_ignore_ascii_case(&opening_tag_full))
                {
                    depth += 1;
                }
            }
            if len >= closing_tag.len() {
                let start = len - closing_tag.len();
                if inner
                    .get(start..)
                    .is_some_and(|slice| slice.eq_ignore_ascii_case(&closing_tag))
                {
                    depth -= 1;
                    if depth == 0 {
                        inner.truncate(start);
                    }
                }
            }
        } else if c == ' ' && len >= opening_tag_with_space.len() {
            let start = len - opening_tag_with_space.len();
            if inner
                .get(start..)
                .is_some_and(|slice| slice.eq_ignore_ascii_case(&opening_tag_with_space))
            {
                depth += 1;
            }
        }
    }

    inner
}

/// Consume a `CODE<n>\u{0002}` placeholder body from `chars`, assuming the
/// opening `\u{0002}` sentinel has already been taken.
///
/// Returns the parsed block index, or `None` when the following
/// characters are not a well-formed placeholder (in which case the caller
/// treats the sentinel as literal text).
fn take_code_placeholder(chars: &mut Peekable<Chars<'_>>) -> Option<usize> {
    for expected in ['C', 'O', 'D', 'E'] {
        chars.next_if_eq(&expected)?;
    }

    let mut digits = String::new();
    while let Some(&d) = chars.peek() {
        if d.is_ascii_digit() {
            digits.push(d);
            chars.next();
        } else {
            break;
        }
    }
    if digits.is_empty() {
        return None;
    }

    chars.next_if_eq(&CODE_BLOCK_PLACEHOLDER_MARK)?;
    digits.parse().ok()
}

/// Push the accumulated literal text (if any) as a [`ProseNode::Text`].
fn flush_text(text: &mut String, nodes: &mut Vec<ProseNode>) {
    if !text.is_empty() {
        nodes.push(ProseNode::Text(std::mem::take(text)));
    }
}

/// Parse pre-processed Prose `content` into a flat list of [`ProseNode`]s.
///
/// Recurses into recognized bracketed tags. Unknown tags and former
/// atomic-token syntax become literal [`ProseNode::Text`]. Fenced
/// code-block placeholders are resolved against `code_blocks` and emitted
/// as opaque [`ProseNode::CodeBlock`]s — their bodies are never scanned
/// for markup, so a body containing `</code-block>`, `<red>`, or any
/// other delimiter stays inert.
pub(super) fn parse_nodes(content: &str, code_blocks: &[FencedCode]) -> Vec<ProseNode> {
    let mut nodes: Vec<ProseNode> = Vec::new();
    let mut text = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        // ── Fenced-code placeholders: \u{0002}CODE<n>\u{0002} ────────────
        if ch == CODE_BLOCK_PLACEHOLDER_MARK {
            if let Some(block) =
                take_code_placeholder(&mut chars).and_then(|index| code_blocks.get(index))
            {
                flush_text(&mut text, &mut nodes);
                nodes.push(ProseNode::CodeBlock {
                    lang: Some(block.lang.clone()).filter(|s| !s.is_empty()),
                    value: block.body.clone(),
                });
            }
            // A malformed placeholder leaves the sentinel dropped; it is a
            // control character with no visible rendering either way.
            continue;
        }

        // ── Backslash escapes: \< \> \{ \\ \* \_ \[ \] \( \) ─────────────
        if ch == '\\' {
            match chars.peek() {
                Some(&'<') | Some(&'>') | Some(&'{') | Some(&'\\') | Some(&'*') | Some(&'_')
                | Some(&'[') | Some(&']') | Some(&'(') | Some(&')') => {
                    text.push(chars.next().unwrap());
                }
                _ => text.push(ch),
            }
            continue;
        }

        // ── Block tags: <tag>content</tag> ────────────────────────────────
        if ch == '<' {
            // Scan the tag declaration up to its terminating `>`. The scan
            // is quote-aware (a `>` inside a quoted attribute value is not
            // the terminator) and escape-aware (the `\<`, `\>`, `\\`, `\"`,
            // `\'` sequences produced by `Prose::quoted_attr` are preserved
            // verbatim, so an escaped delimiter never ends the tag).
            // `parse_opening_tag` resolves the escapes inside attribute
            // values; keeping `tag_content` raw here lets an unrecognized
            // declaration fall back to literal text intact.
            let mut tag_content = String::new();
            let mut found_close = false;
            let mut quote: Option<char> = None;
            while let Some(c) = chars.next() {
                match c {
                    '\\' => {
                        tag_content.push('\\');
                        if let Some(&next) = chars.peek()
                            && matches!(next, '<' | '>' | '\\' | '"' | '\'')
                        {
                            tag_content.push(next);
                            chars.next();
                        }
                    }
                    '"' | '\'' => {
                        match quote {
                            Some(q) if q == c => quote = None,
                            None => quote = Some(c),
                            Some(_) => {}
                        }
                        tag_content.push(c);
                    }
                    '>' if quote.is_none() => {
                        found_close = true;
                        break;
                    }
                    _ => tag_content.push(c),
                }
            }

            if found_close
                && !tag_content.starts_with('/')
                && let Some((tag_name, attrs)) = parse_opening_tag(&tag_content)
            {
                let resolution = resolve_tag(&tag_name, &attrs);
                if !matches!(resolution, TagResolution::Unknown) {
                    let inner = scan_inner(&mut chars, &tag_name);
                    match resolution {
                        TagResolution::Styled(style) => {
                            flush_text(&mut text, &mut nodes);
                            nodes.push(ProseNode::Span {
                                style,
                                children: parse_nodes(&inner, code_blocks),
                            });
                        }
                        TagResolution::Link(href) => {
                            flush_text(&mut text, &mut nodes);
                            nodes.push(ProseNode::Link {
                                href,
                                children: parse_nodes(&inner, code_blocks),
                            });
                        }
                        TagResolution::Transparent => {
                            flush_text(&mut text, &mut nodes);
                            nodes.extend(parse_nodes(&inner, code_blocks));
                        }
                        TagResolution::Code(lang) => {
                            flush_text(&mut text, &mut nodes);
                            nodes.push(ProseNode::CodeBlock { lang, value: inner });
                        }
                        TagResolution::Unknown => unreachable!("guarded above"),
                    }
                    continue;
                }
            }

            // Not a valid block tag — emit as literal text.
            text.push('<');
            text.push_str(&tag_content);
            if found_close {
                text.push('>');
            }
            continue;
        }

        text.push(ch);
    }

    flush_text(&mut text, &mut nodes);
    nodes
}

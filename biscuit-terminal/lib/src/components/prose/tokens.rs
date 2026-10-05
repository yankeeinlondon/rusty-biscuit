//! Bracketed-tag parser that builds shared [`RenderNode`] values directly.
//!
//! The atomic-token grammar (`{{token}}`) has been removed — `{{…}}` is now
//! ordinary literal text. Only bracketed tags (`<tag>…</tag>`) and the
//! Markdown subset (pre-processed into bracketed tags) are recognized.
//!
//! Parsing is target-neutral: no terminal capability decisions are made
//! here. The parser resolves tag names to [`ProseStyle`] intent only;
//! degradation happens in each target renderer.

use std::iter::Peekable;
use std::str::Chars;

use renderable::color::{BasicColor, Color, RgbColor};
use renderable::style::UnderlineStyle;
use renderable::tree::RenderNode;

use super::LineBreaks;
use super::markdown::{
    LIFT_MARK, Lifted, is_escapable, is_sentinel, newline_run, parse_lift_placeholder, tag_declaration_end,
};
use super::styles::{ProseStyle, parse_rgb, tailwind_by_name, web_color_by_name};
use super::tree::{project_span, split_around_blocks, trim_soft_break_whitespace};
use crate::render_tree::link::file_reference_link;

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
                            .is_some_and(|&n| matches!(n, '<' | '>' | '\\' | '"' | '\'') || is_sentinel(n))
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

/// Whether the raw declaration content (between `<` and `>`) opens a tag the
/// parser recognizes, and will therefore consume up to its matching close.
pub(super) fn is_recognized_opening_tag(tag_content: &str) -> bool {
    !tag_content.starts_with('/')
        && parse_opening_tag(tag_content).is_some_and(|(name, attrs)| {
            !matches!(resolve_tag(&name, &attrs), TagResolution::Unknown)
        })
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
/// Tracks nesting depth so `<b>x <b>y</b> z</b>` resolves correctly. Reads
/// the body under the same rules as the other pre-processing scanners: an
/// escape pair is literal, and a recognized tag declaration is skipped whole
/// ([`tag_declaration_end`]), so tag text inside its quoted attributes never
/// opens or closes this scope. Any other `<` is a single literal character,
/// as [`parse_nodes`] treats it.
fn scan_inner(chars: &mut Peekable<Chars<'_>>, tag_name: &str) -> String {
    let rest: Vec<char> = chars.clone().collect();
    let mut depth = 1;
    let (mut inner_end, mut resume) = (rest.len(), rest.len());
    let mut i = 0;

    while i < rest.len() {
        if rest[i] == '\\' && rest.get(i + 1).is_some_and(|&next| is_escapable(next)) {
            i += 2;
            continue;
        }
        if rest[i] == '<'
            && let Some(end) = tag_declaration_end(&rest, i)
        {
            let content: String = rest[i + 1..end - 1].iter().collect();
            if content
                .strip_prefix('/')
                .is_some_and(|name| name.trim().eq_ignore_ascii_case(tag_name))
            {
                depth -= 1;
                if depth == 0 {
                    (inner_end, resume) = (i, end);
                    break;
                }
                i = end;
                continue;
            }
            if is_recognized_opening_tag(&content) {
                if parse_opening_tag(&content).is_some_and(|(name, _)| name == tag_name) {
                    depth += 1;
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    for _ in 0..resume {
        chars.next();
    }
    rest[..inner_end].iter().collect()
}

/// Scan a bracketed-tag declaration, assuming the opening `<` was already
/// consumed.
///
/// The scan is quote-aware (a `>` inside a quoted attribute value is not the
/// terminator) and escape-aware (the `\<`, `\>`, `\\`, `\"`, `\'` sequences
/// produced by [`Prose::quoted_attr`](super::Prose::quoted_attr) are preserved
/// verbatim, so an escaped delimiter never ends the tag). The returned content
/// is raw — [`parse_opening_tag`] resolves the escapes inside attribute values.
///
/// ## Returns
///
/// The raw tag content (without the surrounding `<` / `>`) and whether a
/// closing `>` was found before input ran out.
fn scan_tag_declaration(chars: &mut Peekable<Chars<'_>>) -> (String, bool) {
    let mut tag_content = String::new();
    let mut found_close = false;
    let mut quote: Option<char> = None;
    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                tag_content.push('\\');
                if let Some(&next) = chars.peek()
                    && (matches!(next, '<' | '>' | '\\' | '"' | '\'') || is_sentinel(next))
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
    (tag_content, found_close)
}

/// Push the accumulated literal text (if any) as a [`NodeKind::Text`] node.
///
/// [`NodeKind::Text`]: renderable::tree::NodeKind::Text
fn flush_render_text(text: &mut String, nodes: &mut Vec<RenderNode>) {
    if !text.is_empty() {
        nodes.push(RenderNode::text(std::mem::take(text)));
    }
}

/// How [`parse_render_nodes`] treats breaks and code.
#[derive(Debug, Clone, Copy)]
pub(super) struct InlineOptions {
    /// What a single newline means.
    pub line_breaks: LineBreaks,
    /// `true` for [`InlineProse`](super::InlineProse): fenced code becomes
    /// `InlineCode` instead of a block-level `Code` node.
    pub inline_only: bool,
}

/// Parse pre-processed Prose `content` directly into shared
/// [`RenderNode`](renderable::tree::RenderNode) values.
///
/// Styled spans lower through [`project_span`] to
/// semantic `Strong`/`Emphasis`/`Delete` wrappers or a styled `Span`; links
/// and literal text map to `Link` and `Text`. Lifted-content placeholders
/// resolve against `lifted`: a code span becomes `InlineCode`, a fenced block
/// or `<code-block>` body becomes `Code` (or `InlineCode` when
/// [`InlineOptions::inline_only`]), and an HTML comment becomes nothing. A style or link wrapping a block `Code`
/// is split around it ([`split_around_blocks`]), so the code is returned as a
/// sibling with the wrapper resumed on each side. A `<` that opens no
/// recognized tag is a literal `<`, and the input after it is parsed as
/// usual.
///
/// Newlines follow the break rules: an unescaped `\` before a newline is a
/// hard break unless the newline starts a blank-line run (then the backslash
/// is literal); otherwise a run of newlines is one soft break, or one hard
/// break per newline in [`LineBreaks::Hard`]. The spaces and tabs around a
/// soft break are discarded even across wrapper boundaries
/// ([`trim_soft_break_whitespace`]); other whitespace is kept.
///
/// Two Prose-only tag policies:
///
/// - `<inverse>` / `<reverse>` carry `TextEmphasis::inverse` through the
///   styled `Span`, lowered per target by the shared renderers.
/// - `<hidden>` is not recognized — it renders as inert literal text like any
///   unknown tag.
pub(super) fn parse_render_nodes(content: &str, lifted: &[Lifted], opts: InlineOptions) -> Vec<RenderNode> {
    let mut nodes = parse_nodes(content, lifted, opts);
    trim_soft_break_whitespace(&mut nodes);
    nodes
}

/// [`parse_render_nodes`] before soft-break whitespace is trimmed, which
/// runs once over the whole sequence so it can cross wrapper boundaries.
fn parse_nodes(content: &str, lifted: &[Lifted], opts: InlineOptions) -> Vec<RenderNode> {
    let mut nodes: Vec<RenderNode> = Vec::new();
    let mut text = String::new();
    let mut chars = content.chars().peekable();

    while let Some(ch) = chars.next() {
        // ── Lifted content: \u{0002}<n>\u{0002} ──────────────────────────
        // Literal sentinels were escaped by `lift_fences`, so a bare one is
        // always a placeholder this parser's pre-processor issued.
        if ch == LIFT_MARK {
            let rest: Vec<char> = std::iter::once(ch).chain(chars.clone()).collect();
            if let Some((index, next)) = parse_lift_placeholder(&rest, 0)
                && let Some(entry) = lifted.get(index)
            {
                for _ in 1..next {
                    chars.next();
                }
                // A comment leaves the text around it one run.
                if matches!(entry, Lifted::Comment) {
                    continue;
                }
                flush_render_text(&mut text, &mut nodes);
                match entry {
                    Lifted::Span(value) => nodes.push(RenderNode::inline_code(value.clone())),
                    Lifted::Fence(block) => push_code(
                        Some(block.lang.clone()).filter(|s| !s.is_empty()),
                        &block.body,
                        opts,
                        &mut nodes,
                    ),
                    Lifted::CodeBody(body) => push_code(None, body, opts, &mut nodes),
                    Lifted::Comment => {}
                }
            }
            continue;
        }

        // ── Backslash: escapes and the hard-break marker ─────────────────
        if ch == '\\' {
            match chars.peek() {
                Some(&'\n') => {
                    let mut look = chars.clone();
                    look.next();
                    if starts_blank_run(look) {
                        // A paragraph boundary beats the hard-break marker.
                        text.push('\\');
                    } else {
                        chars.next();
                        flush_render_text(&mut text, &mut nodes);
                        nodes.push(RenderNode::hard_break());
                    }
                }
                Some(&next) if is_escapable(next) => {
                    text.push(next);
                    chars.next();
                }
                _ => text.push(ch),
            }
            continue;
        }

        // ── Newlines: soft or hard breaks ────────────────────────────────
        if ch == '\n' {
            let rest: Vec<char> = std::iter::once(ch).chain(chars.clone()).collect();
            let (count, end) = newline_run(&rest, 0);
            for _ in 1..end {
                chars.next();
            }
            match opts.line_breaks {
                LineBreaks::Soft => {
                    flush_render_text(&mut text, &mut nodes);
                    nodes.push(RenderNode::soft_break());
                }
                LineBreaks::Hard => {
                    flush_render_text(&mut text, &mut nodes);
                    for _ in 0..count {
                        nodes.push(RenderNode::hard_break());
                    }
                }
            }
            continue;
        }

        // ── Block tags: <tag>content</tag> ────────────────────────────────
        if ch == '<' {
            let after_bracket = chars.clone();
            let (tag_content, found_close) = scan_tag_declaration(&mut chars);

            if found_close
                && !tag_content.starts_with('/')
                && let Some((tag_name, attrs)) = parse_opening_tag(&tag_content)
            {
                let resolution = resolve_tag(&tag_name, &attrs);
                if !matches!(resolution, TagResolution::Unknown) {
                    let inner = scan_inner(&mut chars, &tag_name);
                    match resolution {
                        TagResolution::Styled(style) => {
                            flush_render_text(&mut text, &mut nodes);
                            let children = parse_nodes(&inner, lifted, opts);
                            split_around_blocks(children, &mut nodes, |run| project_span(&style, run));
                        }
                        TagResolution::Link(href) => {
                            flush_render_text(&mut text, &mut nodes);
                            let children = parse_nodes(&inner, lifted, opts);
                            split_around_blocks(children, &mut nodes, |run| {
                                file_reference_link(href.clone(), run)
                            });
                        }
                        TagResolution::Transparent => {
                            flush_render_text(&mut text, &mut nodes);
                            nodes.extend(parse_nodes(&inner, lifted, opts));
                        }
                        TagResolution::Code(lang) => {
                            flush_render_text(&mut text, &mut nodes);
                            push_code(lang, code_body(&inner, lifted), opts, &mut nodes);
                        }
                        TagResolution::Unknown => unreachable!("guarded above"),
                    }
                    continue;
                }
            }

            // Not a recognized tag: only the `<` is literal. What follows is
            // parsed as ordinary input, so a placeholder inside a would-be
            // declaration still resolves instead of leaking as text.
            chars = after_bracket;
            text.push('<');
            continue;
        }

        text.push(ch);
    }

    flush_render_text(&mut text, &mut nodes);
    nodes
}

/// Whether the characters after a newline continue it into a blank-line run
/// (only spaces or tabs, then another newline).
fn starts_blank_run(mut rest: Peekable<Chars<'_>>) -> bool {
    while rest.next_if(|c| matches!(c, ' ' | '\t')).is_some() {}
    rest.peek() == Some(&'\n')
}

/// The verbatim body of an explicit `<code-block>`: [`lift_opaque`] left its
/// placeholder as the tag's only content.
///
/// [`lift_opaque`]: super::markdown::lift_opaque
fn code_body<'a>(inner: &'a str, lifted: &'a [Lifted]) -> &'a str {
    let chars: Vec<char> = inner.chars().collect();
    if chars.first() == Some(&LIFT_MARK)
        && let Some((index, next)) = parse_lift_placeholder(&chars, 0)
        && next == chars.len()
        && let Some(Lifted::CodeBody(body)) = lifted.get(index)
    {
        return body;
    }
    inner
}

/// Push a code block's body: a block-level `Code` node, or in
/// [`InlineOptions::inline_only`] mode one `InlineCode` value with the
/// language hint dropped and each line ending replaced by a space (an empty
/// body adds nothing).
fn push_code(lang: Option<String>, body: &str, opts: InlineOptions, nodes: &mut Vec<RenderNode>) {
    if !opts.inline_only {
        nodes.push(RenderNode::code(lang, None, body));
    } else if !body.is_empty() {
        nodes.push(RenderNode::inline_code(body.replace('\n', " ")));
    }
}

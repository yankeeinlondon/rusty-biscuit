//! Markdown cleanup implementation using pulldown-cmark event stream manipulation.
//!
//! This module provides functionality to normalize markdown content by:
//! - Collapsing incidental single newlines inside prose
//! - Ensuring proper blank lines between block elements (via cmark Options)
//! - Aligning table columns for visual consistency
//! - Preserving original list markers (*, -, +)
//!
//! The cleanup leverages `pulldown-cmark-to-cmark`'s built-in newline handling
//! through its `Options` struct, which automatically inserts appropriate spacing.
//!
//! ## Examples
//!
//! ```
//! use darkmatter::markdown::Markdown;
//!
//! let content = "# Header\nSome text\n## Another Header";
//! let mut md: Markdown = content.into();
//! md.cleanup();
//! // Content now has blank lines between headers
//! ```

use biscuit_terminal::utils::UnicodeWidthStr;
use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag, TagEnd};
use pulldown_cmark_to_cmark::Options as CmarkOptions;
use std::ops::Range;

/// Returns parser options suitable for cleanup operations.
///
/// Enables all extensions except:
/// - `ENABLE_SMART_PUNCTUATION` - preserves original quote characters (`"` and `'`)
///   without converting them to typographic "smart quotes" (`\u{201c}`, `\u{201d}`, `\u{2018}`, `\u{2019}`).
/// - `ENABLE_DEFINITION_LIST` - prevents `::file`/`::code`/`::url` transclusion
///   directives from being parsed as definition list items (which would mangle
///   the `::` prefix into `: :`).
fn cleanup_parser_options() -> Options {
    Options::all() - Options::ENABLE_SMART_PUNCTUATION - Options::ENABLE_DEFINITION_LIST
}

/// Emphasis style used for italics in markdown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EmphasisStyle {
    /// Use asterisk for emphasis: `*text*` for italics, `**text**` for bold
    Asterisk,
    /// Use underscore for emphasis: `_text_` for italics, `__text__` for bold
    Underscore,
}

/// Controls whether cleanup collapses fixed-width prose wrapping.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IncidentalNewlineMode {
    /// Collapse incidental single newlines inside prose while preserving block structure.
    Strip,
    /// Leave single newlines untouched.
    Preserve,
}

impl EmphasisStyle {
    /// Returns the emphasis token character for this style.
    pub fn token(&self) -> char {
        match self {
            EmphasisStyle::Asterisk => '*',
            EmphasisStyle::Underscore => '_',
        }
    }

    /// Returns the strong (bold) token string for this style.
    pub fn strong_token(&self) -> &'static str {
        match self {
            EmphasisStyle::Asterisk => "**",
            EmphasisStyle::Underscore => "__",
        }
    }
}

/// Gets the preferred emphasis style from the `PREFER_ITALICS` environment variable.
///
/// Valid values are `*` (asterisk) or `_` / `__` (underscore). Returns `None` if
/// the variable is not set or has an invalid value.
fn get_preferred_emphasis_style() -> Option<EmphasisStyle> {
    std::env::var("PREFER_ITALICS")
        .ok()
        .and_then(|v| match v.trim() {
            "*" => Some(EmphasisStyle::Asterisk),
            "_" | "__" => Some(EmphasisStyle::Underscore),
            _ => None,
        })
}

// Placeholder characters for emphasis markers (private use area - won't be escaped by cmark)
const UNDERSCORE_EMPHASIS_PLACEHOLDER: char = '\u{E000}';
const UNDERSCORE_STRONG_PLACEHOLDER: &str = "\u{E000}\u{E000}";
const ASTERISK_EMPHASIS_PLACEHOLDER: char = '\u{E001}';
const ASTERISK_STRONG_PLACEHOLDER: &str = "\u{E001}\u{E001}";

/// Transforms emphasis/strong events into literal text events to preserve original markers.
///
/// Instead of letting cmark normalize all emphasis to a single style, this function
/// replaces Start/End emphasis events with Text events containing placeholder characters.
/// After cmark renders, these placeholders are replaced with the actual markers.
///
/// We use Unicode private use area characters as placeholders because cmark won't escape them.
///
/// ## Parameters
/// - `standardize_emphasis`: If Some, all emphasis (italics) will use this style.
///   Strong (bold) is NEVER standardized - it always preserves the original style.
fn preserve_original_emphasis<'a>(
    content: &str,
    events_with_ranges: &[(Event<'a>, Range<usize>)],
    standardize_emphasis: Option<EmphasisStyle>,
) -> Vec<Event<'a>> {
    let mut result = Vec::with_capacity(events_with_ranges.len());

    // Stack to track the markers for matching end tags
    // true = underscore style, false = asterisk style
    let mut style_stack: Vec<bool> = Vec::new();

    for (event, range) in events_with_ranges {
        match event {
            Event::Start(Tag::Emphasis) => {
                // Determine emphasis style: use standardized if specified, otherwise original
                let is_underscore = if let Some(style) = standardize_emphasis {
                    style == EmphasisStyle::Underscore
                } else {
                    range.start < content.len() && content[range.start..].starts_with('_')
                };
                style_stack.push(is_underscore);

                let placeholder = if is_underscore {
                    UNDERSCORE_EMPHASIS_PLACEHOLDER
                } else {
                    ASTERISK_EMPHASIS_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder.to_string())));
            }
            Event::Start(Tag::Strong) => {
                // Strong (bold) ALWAYS preserves original style - never standardized
                let is_underscore =
                    range.start < content.len() && content[range.start..].starts_with('_');
                style_stack.push(is_underscore);

                let placeholder = if is_underscore {
                    UNDERSCORE_STRONG_PLACEHOLDER
                } else {
                    ASTERISK_STRONG_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder)));
            }
            Event::End(TagEnd::Emphasis) => {
                let is_underscore = style_stack.pop().unwrap_or(false);
                let placeholder = if is_underscore {
                    UNDERSCORE_EMPHASIS_PLACEHOLDER
                } else {
                    ASTERISK_EMPHASIS_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder.to_string())));
            }
            Event::End(TagEnd::Strong) => {
                let is_underscore = style_stack.pop().unwrap_or(false);
                let placeholder = if is_underscore {
                    UNDERSCORE_STRONG_PLACEHOLDER
                } else {
                    ASTERISK_STRONG_PLACEHOLDER
                };
                result.push(Event::Text(CowStr::from(placeholder)));
            }
            _ => {
                // Pass through all other events unchanged
                result.push(event.clone());
            }
        }
    }

    result
}

/// Replaces emphasis placeholders with actual markers.
fn restore_emphasis_placeholders(output: &mut String) {
    // Replace double placeholders first (strong) to avoid partial matches
    *output = output.replace(UNDERSCORE_STRONG_PLACEHOLDER, "__");
    *output = output.replace(ASTERISK_STRONG_PLACEHOLDER, "**");
    // Then replace single placeholders (emphasis)
    *output = output.replace(UNDERSCORE_EMPHASIS_PLACEHOLDER, "_");
    *output = output.replace(ASTERISK_EMPHASIS_PLACEHOLDER, "*");
}

/// Unescapes underscores and asterisks that cmark escaped in plain text.
///
/// cmark escapes `_` to `\_` and `*` to `\*` when they appear in text that could
/// potentially be interpreted as emphasis markers. However, since we handle all
/// emphasis via placeholders, these escapes are unnecessary.
///
/// This function removes the escape backslashes, but only outside of code blocks/spans.
fn unescape_emphasis_chars(output: &mut String) {
    // Only process if there are escaped characters
    if !output.contains("\\_") && !output.contains("\\*") {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();
    let mut in_code_block = false;
    let mut in_inline_code = false;

    while let Some(c) = chars.next() {
        // Track code blocks
        if c == '`' {
            let mut backtick_count = 1;
            while chars.peek() == Some(&'`') {
                chars.next();
                backtick_count += 1;
            }

            if backtick_count >= 3 {
                in_code_block = !in_code_block;
            } else if !in_code_block {
                // Toggle inline code (simplified - doesn't handle all edge cases)
                in_inline_code = !in_inline_code;
            }

            for _ in 0..backtick_count {
                result.push('`');
            }
            continue;
        }

        // Don't unescape inside code
        if in_code_block || in_inline_code {
            result.push(c);
            continue;
        }

        // Unescape \_ and \*
        if c == '\\' {
            match chars.peek() {
                Some('_') | Some('*') => {
                    // Skip the backslash, push the unescaped character
                    result.push(chars.next().unwrap());
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}

/// Cleans up markdown content by normalizing formatting.
///
/// This preserves the source document's nested list indentation style.
///
/// ## Returns
///
/// The cleaned markdown content as a String.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content;
///
/// let content = "# Title\nParagraph";
/// let cleaned = cleanup_content(content);
/// assert!(cleaned.contains("\n\n"));
/// ```
/// Default indentation width (spaces per nesting level) for list cleanup.
pub const DEFAULT_INDENT: usize = 4;

pub fn cleanup_content(content: &str) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(DEFAULT_INDENT), ListSpacingMode::Normal)
}

/// Cleans up markdown content in compact mode.
///
/// Compact mode removes all blank lines between list items, producing
/// the tightest possible list output.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content_compact;
///
/// let content = "1. First\n\n2. Second\n";
/// let cleaned = cleanup_content_compact(content);
/// assert!(cleaned.contains("1. First\n2. Second"));
/// ```
pub fn cleanup_content_compact(content: &str) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(DEFAULT_INDENT), ListSpacingMode::Compact)
}

/// Cleans up markdown content in loose mode.
///
/// Loose mode adds blank lines between all list items regardless of
/// whether there are level changes.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content_loose;
///
/// let content = "1. First\n2. Second\n";
/// let cleaned = cleanup_content_loose(content);
/// assert!(cleaned.contains("1. First\n\n2. Second"));
/// ```
pub fn cleanup_content_loose(content: &str) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(DEFAULT_INDENT), ListSpacingMode::Loose)
}

/// Cleans up markdown content and enforces a consistent list indentation width.
///
/// When `indent_size` is provided, every nested list level is normalized to that
/// number of spaces.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::cleanup::cleanup_content_with_indent;
///
/// let content = "- Parent\n  - Child";
/// let cleaned = cleanup_content_with_indent(content, 4);
/// assert!(cleaned.contains("\n    - Child"));
/// ```
pub fn cleanup_content_with_indent(content: &str, indent_size: usize) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(indent_size.max(1)), ListSpacingMode::Normal)
}

/// Cleans markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(content, Some(indent_size.max(1)), ListSpacingMode::Normal)
}

/// Cleans up markdown content with forced indentation in compact mode.
pub fn cleanup_content_with_indent_compact(content: &str, indent_size: usize) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(indent_size.max(1)), ListSpacingMode::Compact)
}

/// Cleans compact markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_compact_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(content, Some(indent_size.max(1)), ListSpacingMode::Compact)
}

/// Cleans up markdown content with forced indentation in loose mode.
pub fn cleanup_content_with_indent_loose(content: &str, indent_size: usize) -> String {
    let content = strip_incidental_newlines(content);
    cleanup_content_internal(&content, Some(indent_size.max(1)), ListSpacingMode::Loose)
}

/// Cleans loose markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_loose_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(content, Some(indent_size.max(1)), ListSpacingMode::Loose)
}

/// Cleans Markdown prose by stripping incidental newlines and wrapping it to a fixed width.
///
/// # Panics
///
/// Panics when `width` is `0`.
pub fn cleanup_to_fixed_width(content: &str, width: usize) -> String {
    assert!(width > 0, "fixed-width cleanup requires a width greater than 0");
    let content = strip_incidental_newlines(content);
    reflow_to_width(&content, width)
}

/// Collapses incidental single newlines inside prose without crossing Markdown block boundaries.
pub fn strip_incidental_newlines(content: &str) -> String {
    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return normalized;
    }

    let lines: Vec<&str> = normalized.split('\n').collect();
    let line_count = if lines.last() == Some(&"") {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    };
    let trailing_newline = lines.last() == Some(&"");
    let metadata = LineMetadata::scan(&lines[..line_count]);

    let mut result = String::with_capacity(normalized.len());
    let mut skip_prefix_len = 0;

    for idx in 0..line_count {
        let line = lines[idx];
        if skip_prefix_len > 0 && line.len() >= skip_prefix_len {
            result.push_str(&line[skip_prefix_len..]);
            skip_prefix_len = 0;
        } else {
            result.push_str(line);
        }

        if idx + 1 < line_count {
            let boundary = newline_boundary(&metadata[idx], &metadata[idx + 1]);
            match boundary {
                NewlineBoundary::Preserve => result.push('\n'),
                NewlineBoundary::Collapse { skip_next_prefix } => {
                    if !line.ends_with(char::is_whitespace) {
                        result.push(' ');
                    }
                    skip_prefix_len = skip_next_prefix;
                }
            }
        } else if trailing_newline {
            result.push('\n');
        }
    }

    result
}

/// Reflows Markdown prose lines to `width` display columns.
///
/// Protected Markdown blocks such as fences, indented code, tables, HTML
/// blocks, and transclusion directive lines are emitted unchanged. List,
/// task-list, and blockquote prefixes are preserved while their body text is
/// wrapped inside the remaining width.
///
/// # Panics
///
/// Panics when `width` is `0`.
pub fn reflow_to_width(content: &str, width: usize) -> String {
    assert!(width > 0, "fixed-width reflow requires a width greater than 0");

    let normalized = content.replace("\r\n", "\n").replace('\r', "\n");
    if normalized.is_empty() {
        return normalized;
    }

    let lines: Vec<&str> = normalized.split('\n').collect();
    let line_count = if lines.last() == Some(&"") {
        lines.len().saturating_sub(1)
    } else {
        lines.len()
    };
    let trailing_newline = lines.last() == Some(&"");
    let metadata = LineMetadata::scan(&lines[..line_count]);
    let mut result = String::with_capacity(normalized.len());

    for idx in 0..line_count {
        let line = lines[idx];
        if metadata[idx].blank || metadata[idx].protected {
            result.push_str(line);
        } else {
            let wrapped = reflow_line(line, width);
            result.push_str(&wrapped);
        }

        if idx + 1 < line_count || trailing_newline {
            result.push('\n');
        }
    }

    result
}

#[derive(Debug, Clone)]
struct LineMetadata {
    blank: bool,
    protected: bool,
    list_item: bool,
    inline_code_open_after: bool,
    blockquote_prefix_len: Option<usize>,
}

impl LineMetadata {
    fn scan(lines: &[&str]) -> Vec<Self> {
        let mut metadata = Vec::with_capacity(lines.len());
        let mut fence: Option<String> = None;
        let mut html_block: Option<HtmlBlockEnd> = None;
        let mut inline_code_ticks: Option<usize> = None;
        let mut shell_block_depth = 0usize;

        for line in lines {
            let trimmed = line.trim_start();
            let directive_trimmed = directive_trimmed(line);
            let blank = trimmed.is_empty();
            let was_in_fence = fence.is_some();
            let was_in_html = html_block.is_some();
            let was_in_shell_block = shell_block_depth > 0;
            let starts_fence = fence.is_none().then(|| fence_marker(trimmed)).flatten();
            let starts_html = html_block.is_none().then(|| html_block_end(trimmed)).flatten();
            let starts_shell_block = directive_trimmed.starts_with("::shell-block");
            let ends_shell_block = directive_trimmed.starts_with("::end-block");
            let list_item = is_list_item_start(trimmed);
            let protected = was_in_fence
                || was_in_html
                || was_in_shell_block
                || starts_fence.is_some()
                || starts_html.is_some()
                || is_indented_code_line(line)
                || starts_shell_block
                || is_structural_line(trimmed)
                || is_structural_line(directive_trimmed);

            if !protected {
                update_inline_code_state(line, &mut inline_code_ticks);
            }

            metadata.push(Self {
                blank,
                protected,
                list_item,
                inline_code_open_after: inline_code_ticks.is_some(),
                blockquote_prefix_len: blockquote_prefix_len(line),
            });

            if let Some(marker) = starts_fence {
                fence = Some(marker);
            } else if let Some(marker) = &fence
                && trimmed.starts_with(marker)
            {
                fence = None;
            }

            if let Some(end) = starts_html {
                html_block = Some(end);
            }
            if let Some(end) = html_block
                && end.closes_on(line, blank)
            {
                html_block = None;
            }

            if starts_shell_block {
                shell_block_depth += 1;
            } else if was_in_shell_block && ends_shell_block {
                shell_block_depth = shell_block_depth.saturating_sub(1);
            }
        }

        metadata
    }
}

#[derive(Debug, Clone, Copy)]
enum HtmlBlockEnd {
    BlankLine,
    Contains(&'static str),
}

impl HtmlBlockEnd {
    fn closes_on(self, line: &str, blank: bool) -> bool {
        match self {
            Self::BlankLine => blank,
            Self::Contains(needle) => line.contains(needle),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum NewlineBoundary {
    Preserve,
    Collapse { skip_next_prefix: usize },
}

fn newline_boundary(current: &LineMetadata, next: &LineMetadata) -> NewlineBoundary {
    if current.list_item || next.list_item {
        return if current.list_item && !next.list_item && !next.blank && !next.protected {
            NewlineBoundary::Collapse {
                skip_next_prefix: 0,
            }
        } else {
            NewlineBoundary::Preserve
        };
    }

    if current.blank
        || next.blank
        || current.protected
        || next.protected
        || current.inline_code_open_after
    {
        return NewlineBoundary::Preserve;
    }

    match (current.blockquote_prefix_len, next.blockquote_prefix_len) {
        (Some(current_prefix), Some(next_prefix)) if current_prefix == next_prefix => {
            NewlineBoundary::Collapse {
                skip_next_prefix: next_prefix,
            }
        }
        (Some(_), _) | (_, Some(_)) => NewlineBoundary::Preserve,
        _ => NewlineBoundary::Collapse {
            skip_next_prefix: 0,
        },
    }
}

fn fence_marker(trimmed: &str) -> Option<String> {
    let marker = trimmed.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }
    let count = trimmed.chars().take_while(|&c| c == marker).count();
    (count >= 3).then(|| marker.to_string().repeat(count))
}

fn is_indented_code_line(line: &str) -> bool {
    line.starts_with('\t') || line.starts_with("    ")
}

fn is_structural_line(trimmed: &str) -> bool {
    trimmed.starts_with('#')
        || trimmed.starts_with("::")
        || is_table_line(trimmed)
        || trimmed.starts_with("---")
        || trimmed.starts_with("***")
        || trimmed.starts_with("___")
}

fn directive_trimmed(line: &str) -> &str {
    if let Some(prefix_len) = blockquote_prefix_len(line) {
        return line[prefix_len..].trim_start();
    }

    line.trim_start()
}

fn is_table_line(trimmed: &str) -> bool {
    trimmed.starts_with('|') && trimmed[1..].contains('|')
}

fn html_block_end(trimmed: &str) -> Option<HtmlBlockEnd> {
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("<!--") {
        return Some(HtmlBlockEnd::Contains("-->"));
    }
    if lower.starts_with("<?") {
        return Some(HtmlBlockEnd::Contains("?>"));
    }
    if lower.starts_with("<![cdata[") {
        return Some(HtmlBlockEnd::Contains("]]>"));
    }
    if lower.starts_with("<!") {
        return Some(HtmlBlockEnd::Contains(">"));
    }
    if lower.starts_with("</script")
        || lower.starts_with("</pre")
        || lower.starts_with("</style")
        || lower.starts_with("<script")
        || lower.starts_with("<pre")
        || lower.starts_with("<style")
    {
        return Some(HtmlBlockEnd::BlankLine);
    }

    const BLOCK_TAGS: &[&str] = &[
        "address",
        "article",
        "aside",
        "base",
        "basefont",
        "blockquote",
        "body",
        "caption",
        "center",
        "col",
        "colgroup",
        "dd",
        "details",
        "dialog",
        "dir",
        "div",
        "dl",
        "dt",
        "fieldset",
        "figcaption",
        "figure",
        "footer",
        "form",
        "frame",
        "frameset",
        "h1",
        "h2",
        "h3",
        "h4",
        "h5",
        "h6",
        "head",
        "header",
        "hr",
        "html",
        "iframe",
        "legend",
        "li",
        "link",
        "main",
        "menu",
        "menuitem",
        "nav",
        "noframes",
        "ol",
        "optgroup",
        "option",
        "p",
        "param",
        "search",
        "section",
        "summary",
        "table",
        "tbody",
        "td",
        "tfoot",
        "th",
        "thead",
        "title",
        "tr",
        "track",
        "ul",
    ];

    BLOCK_TAGS
        .iter()
        .any(|tag| html_line_starts_with_tag(&lower, tag))
        .then_some(HtmlBlockEnd::BlankLine)
}

fn html_line_starts_with_tag(line: &str, tag: &str) -> bool {
    let Some(rest) = line.strip_prefix("</").or_else(|| line.strip_prefix('<')) else {
        return false;
    };
    let Some(rest) = rest.strip_prefix(tag) else {
        return false;
    };
    matches!(rest.as_bytes().first(), Some(b' ' | b'\t' | b'>' | b'/'))
}

fn update_inline_code_state(line: &str, open_ticks: &mut Option<usize>) {
    let mut chars = line.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch != '`' {
            continue;
        }

        let mut count = 1;
        while chars.peek() == Some(&'`') {
            chars.next();
            count += 1;
        }

        if count >= 3 {
            continue;
        }

        match open_ticks {
            Some(open) if *open == count => *open_ticks = None,
            None => *open_ticks = Some(count),
            _ => {}
        }
    }
}

fn blockquote_prefix_len(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut idx = 0;
    let mut saw_marker = false;

    while idx < bytes.len() {
        while idx < bytes.len() && bytes[idx] == b' ' {
            idx += 1;
        }
        if bytes.get(idx) != Some(&b'>') {
            break;
        }
        saw_marker = true;
        idx += 1;
        if bytes.get(idx) == Some(&b' ') {
            idx += 1;
        }
    }

    saw_marker.then_some(idx)
}

fn reflow_line(line: &str, width: usize) -> String {
    let prefix = line_reflow_prefix(line);
    let body = line[prefix.first_len..].trim();
    if body.is_empty() {
        return line.to_string();
    }

    wrap_text(body, &prefix.first, &prefix.continuation, width)
}

#[derive(Debug, Clone)]
struct ReflowPrefix {
    first: String,
    continuation: String,
    first_len: usize,
}

fn line_reflow_prefix(line: &str) -> ReflowPrefix {
    if let Some(prefix_len) = blockquote_prefix_len(line) {
        let prefix = line[..prefix_len].to_string();
        return ReflowPrefix {
            first: prefix.clone(),
            continuation: prefix,
            first_len: prefix_len,
        };
    }

    let leading_len = line.len() - line.trim_start().len();
    let trimmed = &line[leading_len..];
    if let Some(marker_len) = list_marker_prefix_len(trimmed) {
        let first_len = leading_len + marker_len;
        let first = line[..first_len].to_string();
        return ReflowPrefix {
            continuation: " ".repeat(UnicodeWidthStr::width(first.as_str())),
            first,
            first_len,
        };
    }

    ReflowPrefix {
        first: String::new(),
        continuation: " ".repeat(leading_len),
        first_len: leading_len,
    }
}

fn list_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let marker_len = unordered_marker_prefix_len(trimmed).or_else(|| ordered_marker_prefix_len(trimmed))?;
    let rest = &trimmed[marker_len..];
    task_marker_prefix_len(rest).map_or(Some(marker_len), |task_len| Some(marker_len + task_len))
}

fn unordered_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let mut chars = trimmed.char_indices();
    let (_, marker) = chars.next()?;
    if !matches!(marker, '*' | '-' | '+') {
        return None;
    }
    let (space_idx, space) = chars.next()?;
    (space == ' ').then_some(space_idx + space.len_utf8())
}

fn ordered_marker_prefix_len(trimmed: &str) -> Option<usize> {
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return None;
    }

    for (idx, &byte) in bytes.iter().enumerate().skip(1) {
        if byte == b'.' || byte == b')' {
            return (bytes.get(idx + 1) == Some(&b' ')).then_some(idx + 2);
        }
        if !byte.is_ascii_digit() {
            return None;
        }
    }

    None
}

fn task_marker_prefix_len(rest: &str) -> Option<usize> {
    let bytes = rest.as_bytes();
    if bytes.len() >= 4
        && bytes[0] == b'['
        && matches!(bytes[1], b' ' | b'x' | b'X')
        && bytes[2] == b']'
        && bytes[3] == b' '
    {
        Some(4)
    } else {
        None
    }
}

fn wrap_text(text: &str, first_prefix: &str, continuation_prefix: &str, width: usize) -> String {
    let tokens = reflow_tokens(text);
    if tokens.is_empty() {
        return first_prefix.to_string();
    }

    let mut lines = Vec::new();
    let mut current = String::new();
    let mut current_width = UnicodeWidthStr::width(first_prefix);
    let mut prefix = first_prefix;

    for token in tokens {
        let token_width = UnicodeWidthStr::width(token.as_str());
        let separator_width = usize::from(!current.is_empty());
        if !current.is_empty() && current_width + separator_width + token_width > width {
            lines.push(format!("{prefix}{current}"));
            current.clear();
            prefix = continuation_prefix;
            current_width = UnicodeWidthStr::width(prefix);
        }

        if !current.is_empty() {
            current.push(' ');
            current_width += 1;
        }
        current.push_str(&token);
        current_width += token_width;
    }

    if !current.is_empty() {
        lines.push(format!("{prefix}{current}"));
    }

    lines.join("\n")
}

fn reflow_tokens(text: &str) -> Vec<String> {
    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = text.chars().peekable();

    while let Some(ch) = chars.next() {
        if ch.is_whitespace() {
            if !current.is_empty() {
                tokens.push(std::mem::take(&mut current));
            }
            continue;
        }

        if ch == '`' {
            current.push(ch);
            let ticks = consume_matching_chars(&mut chars, '`', &mut current) + 1;
            consume_code_span(&mut chars, ticks, &mut current);
            continue;
        }

        current.push(ch);
    }

    if !current.is_empty() {
        tokens.push(current);
    }

    tokens
}

fn consume_matching_chars<I>(chars: &mut std::iter::Peekable<I>, target: char, output: &mut String) -> usize
where
    I: Iterator<Item = char>,
{
    let mut count = 0;
    while chars.peek() == Some(&target) {
        output.push(chars.next().unwrap());
        count += 1;
    }
    count
}

fn consume_code_span<I>(chars: &mut std::iter::Peekable<I>, ticks: usize, output: &mut String)
where
    I: Iterator<Item = char>,
{
    let mut run = 0;
    for ch in chars.by_ref() {
        output.push(ch);
        if ch == '`' {
            run += 1;
            if run == ticks {
                break;
            }
        } else {
            run = 0;
        }
    }
}

/// Controls how blank lines between list items are handled during cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListSpacingMode {
    /// Blank lines only at level transitions and before prose after lists.
    Normal,
    /// No blank lines between list items (tightest output).
    Compact,
    /// Blank lines between all list items (loosest output).
    Loose,
}

fn cleanup_content_internal(
    content: &str,
    forced_indent: Option<usize>,
    list_spacing: ListSpacingMode,
) -> String {
    // Parse with source ranges to preserve list markers and emphasis styles
    // Use custom options that exclude ENABLE_SMART_PUNCTUATION to preserve original quotes
    let parser = Parser::new_ext(content, cleanup_parser_options());
    let events_with_ranges: Vec<(Event, Range<usize>)> = parser.into_offset_iter().collect();

    // Extract list markers from source for each list
    let list_markers = extract_list_markers(content, &events_with_ranges);

    // Determine emphasis style:
    // 1. If PREFER_ITALICS env var is set, use that style (standardize all emphasis)
    // 2. Otherwise, preserve original markers (no standardization)
    // Note: PREFER_ITALICS only affects emphasis (italics), NOT strong (bold)
    let preferred_style = get_preferred_emphasis_style();

    // Transform events: replace emphasis/strong events with placeholder characters.
    // This prevents cmark from normalizing them or escaping literal underscores/asterisks.
    // If preferred_style is set, emphasis will be standardized; strong always preserves original.
    let events: Vec<Event> =
        preserve_original_emphasis(content, &events_with_ranges, preferred_style);

    // Add "text" language to empty fenced code blocks
    let with_text_lang = add_text_language_to_empty_code_blocks(events);

    // Align tables in the event stream
    let processed = align_tables_in_stream(with_text_lang);

    // Convert events back to markdown with proper spacing options
    let mut output = String::new();

    // cmark handles blank line insertion via its Options - defaults are correct:
    // newlines_after_headline: 2, newlines_after_paragraph: 2, etc.
    // Override code_block_token_count: default is 4, but standard markdown uses 3
    // Note: emphasis_token/strong_token don't matter since we use placeholders
    let options = CmarkOptions {
        code_block_token_count: 3,
        increment_ordered_list_bullets: true,
        ..Default::default()
    };

    // cmark expects borrowed events
    let borrowed: Vec<_> = processed.iter().map(std::borrow::Cow::Borrowed).collect();
    if pulldown_cmark_to_cmark::cmark_with_options(borrowed.into_iter(), &mut output, options)
        .is_err()
    {
        // If rendering fails, return original content
        return content.to_string();
    }

    // Restore emphasis/strong markers (replace placeholders with actual characters)
    restore_emphasis_placeholders(&mut output);

    // Unescape underscores/asterisks that cmark escaped in plain text
    // (e.g., '_' becomes '\_' which should be '_')
    unescape_emphasis_chars(&mut output);

    // Normalize list item spacing according to the chosen mode.
    // pulldown-cmark-to-cmark doesn't reliably handle blank lines between
    // list items, so we normalize uniformly:
    //   Normal:  blank lines at level transitions, none between same-level items
    //   Compact: no blank lines between any list items
    //   Loose:   blank lines between all list items
    normalize_list_spacing(&mut output, list_spacing);

    // Post-process to fix blockquote formatting issues from pulldown-cmark-to-cmark
    fix_blockquote_formatting(&mut output);

    // Restore original list markers (the library normalizes to '*')
    restore_list_markers(&mut output, &list_markers);

    // Normalize nested list indentation.
    // When forced indentation is provided, use it for consistent nesting.
    // Otherwise preserve the source style when it differs from cmark's 2-space output.
    if let Some(indent_size) = forced_indent {
        if indent_size != 2 {
            fix_list_indentation(&mut output, indent_size);
        }
    } else {
        let original_indent = detect_list_indentation(content);
        if original_indent > 2 {
            fix_list_indentation(&mut output, original_indent);
        }
    }

    // Unescape unnecessarily escaped brackets (e.g., \[0%\] -> [0%])
    unescape_brackets(&mut output);

    // Trim leading blank lines, then normalize to exactly one trailing newline
    // for non-empty documents.
    let mut normalized = output.trim_start_matches('\n').to_string();
    if normalized.is_empty() {
        return normalized;
    }
    normalized.truncate(normalized.trim_end_matches('\n').len());
    normalized.push('\n');
    normalized
}

/// Extracts the list marker character for each unordered list item from the source.
///
/// Returns a vector of marker characters in document order, one per unordered
/// list item. This ensures a 1:1 correspondence with `* ` lines in the cmark
/// output, which is critical for loose lists where items are separated by
/// blank lines.
fn extract_list_markers(content: &str, events: &[(Event, Range<usize>)]) -> Vec<char> {
    let mut markers = Vec::new();
    // Track whether the innermost list is unordered (true) or ordered (false)
    let mut list_type_stack: Vec<bool> = Vec::new();

    for (event, range) in events {
        match event {
            Event::Start(Tag::List(None)) => {
                list_type_stack.push(true); // unordered
            }
            Event::Start(Tag::List(Some(_))) => {
                list_type_stack.push(false); // ordered
            }
            Event::End(TagEnd::List(_)) => {
                list_type_stack.pop();
            }
            Event::Start(Tag::Item) if list_type_stack.last() == Some(&true) => {
                // Item inside an unordered list - extract its marker from source
                let source_slice = &content[range.start..];
                if let Some(marker) = find_list_marker(source_slice) {
                    markers.push(marker);
                } else {
                    markers.push('*');
                }
            }
            _ => {}
        }
    }

    markers
}

/// Finds the first list marker character (*, -, or +) in a source slice.
fn find_list_marker(source: &str) -> Option<char> {
    // Skip leading whitespace and look for the marker
    for c in source.chars() {
        match c {
            '*' | '-' | '+' => return Some(c),
            ' ' | '\t' | '\n' => continue,
            _ => break,
        }
    }
    None
}

/// Restores original list markers in the output.
///
/// The pulldown-cmark-to-cmark library normalizes all unordered list markers to '*'.
/// This function replaces them with the original markers from the source.
///
/// Since `extract_list_markers` produces one marker per unordered list item (in
/// document order), and cmark produces exactly one `* ` line per item, this is a
/// simple sequential 1:1 replacement.
fn restore_list_markers(output: &mut String, markers: &[char]) {
    if markers.is_empty() {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut marker_idx = 0;
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        // Track code blocks to avoid modifying markers inside them
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        if trimmed.starts_with("* ") {
            let indent = line.len() - trimmed.len();
            let marker = markers.get(marker_idx).copied().unwrap_or('*');
            marker_idx += 1;

            result.push_str(&" ".repeat(indent));
            result.push(marker);
            result.push_str(&trimmed[1..]); // Skip the '*', keep the rest (including space)
        } else {
            result.push_str(line);
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Fixes blockquote formatting issues introduced by pulldown-cmark-to-cmark v18.
///
/// The library adds:
/// 1. A leading space before `>` (e.g., ` > ` instead of `> `)
/// 2. An empty blockquote line at the start of each blockquote
/// 3. Extra spaces in nested blockquotes (e.g., `>  > ` instead of `> > `)
///
/// This function corrects these issues to produce standard markdown.
fn fix_blockquote_formatting(output: &mut String) {
    // Process line by line for clarity
    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;
    let mut prev_was_blockquote = false;

    while let Some(line) = lines.next() {
        // Track code blocks to avoid modifying content inside them
        if line.trim_start().starts_with("```") {
            in_code_block = !in_code_block;
            prev_was_blockquote = false;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process lines inside code blocks
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Only fix blockquote lines (those starting with optional space + ">")
        let is_blockquote_line =
            line.starts_with('>') || (line.starts_with(' ') && line.trim_start().starts_with('>'));

        let fixed_line = if is_blockquote_line {
            fix_blockquote_line(line)
        } else {
            line.to_string()
        };

        // Check if this is an empty blockquote line (just "> " or nested like "> > ")
        let trimmed = fixed_line.trim_end();
        let is_empty_blockquote = trimmed.chars().all(|c| c == '>' || c == ' ')
            && trimmed.contains('>')
            && !trimmed.is_empty();

        // Only strip empty blockquote lines at the START of a blockquote (pulldown-cmark
        // artifact). Preserve them mid-blockquote where they represent intentional
        // paragraph breaks (e.g., blank line before an attribution).
        if is_empty_blockquote
            && !prev_was_blockquote
            && let Some(next_line) = lines.peek()
            && next_line.trim_start().starts_with('>')
        {
            // Skip this empty blockquote line at the start of the blockquote
            continue;
        }

        prev_was_blockquote = is_blockquote_line;

        result.push_str(&fixed_line);
        // Add newline unless this is the last line
        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline if original had one
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Fixes a single blockquote line's prefix formatting.
///
/// Handles:
/// - Leading space: " > text" -> "> text"
/// - Multiple spaces after >: ">  > text" -> "> > text"
fn fix_blockquote_line(line: &str) -> String {
    let mut result = String::with_capacity(line.len());
    let mut chars = line.chars().peekable();
    let mut in_prefix = true;

    // Skip leading space if followed by >
    if chars.peek() == Some(&' ') {
        let mut lookahead = chars.clone();
        lookahead.next(); // consume space
        if lookahead.peek() == Some(&'>') {
            chars.next(); // skip the leading space
        }
    }

    while let Some(c) = chars.next() {
        if in_prefix {
            if c == '>' {
                result.push(c);
                // After >, we expect exactly one space before content or next >
                // Skip any extra spaces, but keep one
                let mut space_count = 0;
                while chars.peek() == Some(&' ') {
                    chars.next();
                    space_count += 1;
                }
                // Add exactly one space after >
                if space_count > 0 || chars.peek().is_some() {
                    result.push(' ');
                }
                // Check if next char is another > (nested blockquote)
                if chars.peek() != Some(&'>') {
                    in_prefix = false;
                }
            } else if c == ' ' {
                // Skip spaces in prefix area (between > markers)
                continue;
            } else {
                in_prefix = false;
                result.push(c);
            }
        } else {
            result.push(c);
        }
    }

    result
}

/// Detects the list indentation style used in the source content.
///
/// Scans for nested list items and returns the number of spaces used for indentation.
/// Returns 2 if no nested lists are found or indentation can't be determined.
fn detect_list_indentation(content: &str) -> usize {
    let mut in_code_block = false;

    for line in content.lines() {
        let trimmed = line.trim_start();

        // Skip code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            continue;
        }
        if in_code_block {
            continue;
        }

        // Look for indented list items
        let indent = line.len() - trimmed.len();
        if indent > 0
            && (trimmed.starts_with("- ") || trimmed.starts_with("* ") || trimmed.starts_with("+ "))
        {
            // Found a nested list item - return its indentation
            return indent;
        }

        // Also check for numbered lists
        if indent > 0 {
            let mut chars = trimmed.chars().peekable();
            let mut is_numbered = false;
            while let Some(c) = chars.next() {
                if c.is_ascii_digit() {
                    continue;
                } else if (c == '.' || c == ')') && chars.peek() == Some(&' ') {
                    is_numbered = true;
                    break;
                } else {
                    break;
                }
            }
            if is_numbered {
                return indent;
            }
        }
    }

    // Default to 2 if no nested lists found
    2
}

/// Fixes list indentation in the output to match the original style.
///
/// `pulldown-cmark-to-cmark` uses 2-space indentation by default. This function
/// converts it to the specified indentation size (e.g., 4 spaces).
fn fix_list_indentation(output: &mut String, target_indent: usize) {
    if target_indent == 2 {
        return; // Already correct
    }

    let mut result = String::with_capacity(output.len());
    let mut lines = output.lines().peekable();
    let mut in_code_block = false;

    while let Some(line) = lines.next() {
        let trimmed = line.trim_start();

        // Track code blocks
        if trimmed.starts_with("```") {
            in_code_block = !in_code_block;
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Don't process code block content
        if in_code_block {
            result.push_str(line);
            if lines.peek().is_some() {
                result.push('\n');
            }
            continue;
        }

        // Check if this is a list item
        let is_list_item = trimmed.starts_with("- ")
            || trimmed.starts_with("* ")
            || trimmed.starts_with("+ ")
            || is_ordered_list_start(trimmed);

        if is_list_item {
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 {
                // Calculate nesting level (assuming 2-space input)
                let nesting_level = current_indent / 2;
                // Apply target indentation
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
        } else {
            // For non-list content that's indented (like continuation text),
            // apply the same scaling
            let current_indent = line.len() - trimmed.len();
            if current_indent > 0 && current_indent % 2 == 0 {
                let nesting_level = current_indent / 2;
                let new_indent = nesting_level * target_indent;
                result.push_str(&" ".repeat(new_indent));
                result.push_str(trimmed);
            } else {
                result.push_str(line);
            }
        }

        if lines.peek().is_some() {
            result.push('\n');
        }
    }

    // Preserve trailing newline
    if output.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }

    *output = result;
}

/// Checks if a line starts with an ordered list marker (e.g., "1. " or "1) ").
fn is_ordered_list_start(line: &str) -> bool {
    let mut chars = line.chars().peekable();
    let mut has_digit = false;

    while let Some(c) = chars.next() {
        if c.is_ascii_digit() {
            has_digit = true;
        } else if has_digit && (c == '.' || c == ')') {
            return chars.peek() == Some(&' ');
        } else {
            return false;
        }
    }
    false
}

/// Unescapes unnecessarily escaped brackets in the output.
///
/// `pulldown-cmark-to-cmark` escapes `[` and `]` characters that could potentially
/// be interpreted as link syntax. This function unescapes patterns like `\[0%\]`
/// that are clearly not links (no `](` following them).
fn unescape_brackets(output: &mut String) {
    // Only process if there are escaped brackets
    if !output.contains("\\[") {
        return;
    }

    let mut result = String::with_capacity(output.len());
    let mut chars = output.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            match chars.peek() {
                Some('[') => {
                    // Look ahead to see if this could be a link
                    // Pattern: \[...\] or \[...](...)
                    // We want to unescape standalone \[...\] that aren't links
                    chars.next(); // consume '['

                    // Collect until we find \] or ]( or end
                    let mut bracket_content = String::new();
                    let mut found_close = false;

                    while let Some(&next) = chars.peek() {
                        if next == '\\' {
                            chars.next();
                            if chars.peek() == Some(&']') {
                                chars.next();
                                found_close = true;
                                break;
                            } else {
                                bracket_content.push('\\');
                            }
                        } else if next == ']' {
                            chars.next();
                            found_close = true;
                            // Check if followed by ( - would make this a link
                            if chars.peek() == Some(&'(') {
                                // This is actually a link, restore and keep escape
                                result.push_str("\\[");
                                result.push_str(&bracket_content);
                                result.push(']');
                                break;
                            }
                            break;
                        } else if next == '\n' {
                            // Line break - not a link, but stop searching
                            break;
                        } else {
                            bracket_content.push(chars.next().unwrap());
                        }
                    }

                    if found_close {
                        // Unescape: output [content] instead of \[content\]
                        result.push('[');
                        result.push_str(&bracket_content);
                        result.push(']');
                    } else {
                        // Didn't find proper close, restore original
                        result.push_str("\\[");
                        result.push_str(&bracket_content);
                        // Restore chars iterator - actually we can't easily do this
                        // Just continue from where we are
                    }
                }
                Some(']') => {
                    // Standalone escaped ] - keep as is (shouldn't happen often)
                    result.push('\\');
                    result.push(chars.next().unwrap());
                }
                _ => {
                    result.push(c);
                }
            }
        } else {
            result.push(c);
        }
    }

    *output = result;
}

/// Adds "text" language to fenced code blocks with no language specified.
///
/// This ensures all code blocks have an explicit language identifier,
/// improving rendering consistency across different markdown viewers.
fn add_text_language_to_empty_code_blocks(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    events
        .into_iter()
        .map(|event| {
            if let Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(ref info))) = event
                && info.is_empty()
            {
                return Event::Start(Tag::CodeBlock(CodeBlockKind::Fenced(CowStr::from("text"))));
            }
            event
        })
        .collect()
}

/// Aligns tables in the event stream for visual consistency.
///
/// This function identifies table events and processes them to ensure
/// column alignment across all rows.
fn align_tables_in_stream(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    let mut result = Vec::with_capacity(events.len());
    let mut table_buffer = Vec::new();
    let mut in_table = false;

    for event in events {
        match &event {
            Event::Start(Tag::Table(_)) => {
                in_table = true;
                table_buffer.clear();
                table_buffer.push(event);
            }
            Event::End(TagEnd::Table) => {
                table_buffer.push(event);
                in_table = false;

                // Process the buffered table
                let aligned = process_single_table(table_buffer.clone());
                result.extend(aligned);
                table_buffer.clear();
            }
            _ => {
                if in_table {
                    table_buffer.push(event);
                } else {
                    result.push(event);
                }
            }
        }
    }

    result
}

/// Calculates the rendered width of table cell content.
///
/// This struct tracks nested formatting elements (links, emphasis, strong) and
/// calculates the actual visual width of the rendered markdown output.
struct CellWidthCalculator {
    /// Current accumulated width
    width: usize,
    /// Stack of active formatting contexts with their overhead
    /// (start_overhead, end_overhead, url_width for links)
    format_stack: Vec<FormatContext>,
}

/// Represents an active formatting context within a table cell.
#[derive(Debug, Clone)]
enum FormatContext {
    /// Emphasis: *text* or _text_ - adds 2 chars total
    Emphasis,
    /// Strong: **text** or __text__ - adds 4 chars total
    Strong,
    /// Link: [text](url) - adds 4 chars + url length
    Link { url_width: usize },
}

impl CellWidthCalculator {
    fn new() -> Self {
        Self {
            width: 0,
            format_stack: Vec::new(),
        }
    }

    fn reset(&mut self) {
        self.width = 0;
        self.format_stack.clear();
    }

    fn add_text(&mut self, text: &str) {
        self.width += UnicodeWidthStr::width(text);
    }

    fn add_code(&mut self, code: &str) {
        // Code spans render with backticks: `code`
        self.width += UnicodeWidthStr::width(code) + 2;
    }

    fn start_emphasis(&mut self) {
        // Opening marker: * or _
        self.width += 1;
        self.format_stack.push(FormatContext::Emphasis);
    }

    fn end_emphasis(&mut self) {
        // Closing marker: * or _
        self.width += 1;
        self.format_stack.pop();
    }

    fn start_strong(&mut self) {
        // Opening marker: ** or __
        self.width += 2;
        self.format_stack.push(FormatContext::Strong);
    }

    fn end_strong(&mut self) {
        // Closing marker: ** or __
        self.width += 2;
        self.format_stack.pop();
    }

    fn start_link(&mut self, url: &str) {
        // Opening bracket: [
        self.width += 1;
        self.format_stack.push(FormatContext::Link {
            url_width: UnicodeWidthStr::width(url),
        });
    }

    fn end_link(&mut self) {
        // Closing: ](url)
        if let Some(FormatContext::Link { url_width }) = self.format_stack.pop() {
            // "](" + url + ")"
            self.width += 2 + url_width + 1;
        }
    }

    fn total_width(&self) -> usize {
        self.width
    }
}

/// Processes a single table's events to align columns.
///
/// This function analyzes all table cells to determine the maximum width
/// for each column and then pads cells accordingly. It preserves the original
/// event structure (keeping Code events as Code, not merging into Text) and
/// adds spacing around cell content for readability: `| content |`
///
/// Width calculation accounts for:
/// - Unicode display width (not just character count)
/// - Link syntax: `[text](url)` adds brackets and URL
/// - Emphasis: `*text*` or `_text_` adds 2 characters
/// - Strong: `**text**` or `__text__` adds 4 characters
/// - Code spans: `` `code` `` adds 2 backticks
fn process_single_table(events: Vec<Event<'_>>) -> Vec<Event<'_>> {
    // Pass 1: Measure column widths (visual width of rendered content)
    let mut col_widths: Vec<usize> = Vec::new();
    let mut current_col = 0;
    let mut in_cell = false;
    let mut calc = CellWidthCalculator::new();

    for ev in &events {
        match ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                calc.reset();
            }
            Event::End(TagEnd::TableCell) => {
                let cell_width = calc.total_width();
                if col_widths.len() <= current_col {
                    col_widths.push(cell_width);
                } else {
                    col_widths[current_col] = col_widths[current_col].max(cell_width);
                }
                current_col += 1;
                in_cell = false;
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
            }
            Event::Text(t) if in_cell => {
                calc.add_text(t);
            }
            Event::Code(t) if in_cell => {
                calc.add_code(t);
            }
            Event::Start(Tag::Emphasis) if in_cell => {
                calc.start_emphasis();
            }
            Event::End(TagEnd::Emphasis) if in_cell => {
                calc.end_emphasis();
            }
            Event::Start(Tag::Strong) if in_cell => {
                calc.start_strong();
            }
            Event::End(TagEnd::Strong) if in_cell => {
                calc.end_strong();
            }
            Event::Start(Tag::Link { dest_url, .. }) if in_cell => {
                calc.start_link(dest_url);
            }
            Event::End(TagEnd::Link) if in_cell => {
                calc.end_link();
            }
            _ => {}
        }
    }

    // Pass 2: Preserve original events, add leading space and trailing padding
    let mut result = Vec::with_capacity(events.len() + col_widths.len() * 2);
    let mut current_col = 0;
    let mut in_cell = false;
    let mut calc = CellWidthCalculator::new();

    for ev in events {
        match &ev {
            Event::Start(Tag::TableCell) => {
                in_cell = true;
                calc.reset();
                result.push(ev);
                // Add leading space for readability: "|content" -> "| content"
                result.push(Event::Text(CowStr::from(" ")));
            }
            Event::End(TagEnd::TableCell) => {
                // Add trailing padding to align columns, plus one space before |
                let target_width = col_widths.get(current_col).copied().unwrap_or(0);
                let cell_width = calc.total_width();
                let padding_needed = target_width.saturating_sub(cell_width);
                // Add padding + trailing space: "content|" -> "content |"
                let padding = " ".repeat(padding_needed + 1);
                result.push(Event::Text(CowStr::from(padding)));
                current_col += 1;
                in_cell = false;
                result.push(ev);
            }
            Event::End(TagEnd::TableRow) | Event::End(TagEnd::TableHead) => {
                current_col = 0;
                result.push(ev);
            }
            Event::Text(t) if in_cell => {
                calc.add_text(t);
                result.push(ev);
            }
            Event::Code(t) if in_cell => {
                calc.add_code(t);
                result.push(ev);
            }
            Event::Start(Tag::Emphasis) if in_cell => {
                calc.start_emphasis();
                result.push(ev);
            }
            Event::End(TagEnd::Emphasis) if in_cell => {
                calc.end_emphasis();
                result.push(ev);
            }
            Event::Start(Tag::Strong) if in_cell => {
                calc.start_strong();
                result.push(ev);
            }
            Event::End(TagEnd::Strong) if in_cell => {
                calc.end_strong();
                result.push(ev);
            }
            Event::Start(Tag::Link { dest_url, .. }) if in_cell => {
                calc.start_link(dest_url);
                result.push(ev);
            }
            Event::End(TagEnd::Link) if in_cell => {
                calc.end_link();
                result.push(ev);
            }
            _ => {
                result.push(ev);
            }
        }
    }

    result
}

/// Normalizes blank lines between list items according to the spacing mode.
///
/// First strips all blank lines between list items (resetting to compact),
/// then inserts blank lines based on the mode:
///
/// - **Compact**: no blank lines between list items
/// - **Normal**: blank lines when returning from a sub-list (shallower
///   transition) or for loose list items, and before prose that follows a list
/// - **Loose**: blank lines between all list items
fn normalize_list_spacing(output: &mut String, mode: ListSpacingMode) {
    let lines: Vec<&str> = output.lines().collect();
    if lines.len() < 2 {
        return;
    }

    // Phase 1: strip blank lines between list items to get a clean baseline.
    // Only strip when both the previous non-blank line and the next non-blank
    // line are list item starts. Blank lines before continuation prose or
    // between a list item and non-item content are preserved.
    let mut stripped: Vec<&str> = Vec::with_capacity(lines.len());
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim().is_empty() && i > 0 {
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }

            // Find the last non-blank line before the gap
            let prev_is_item =
                is_list_item_start(lines[i - 1].trim_start()) || is_list_continuation(lines[i - 1]);
            let next_is_item = j < lines.len() && is_list_item_start(lines[j].trim_start());

            // Only strip if both sides are list items (not continuation prose)
            if prev_is_item && next_is_item {
                i = j;
                continue;
            }
        }
        stripped.push(lines[i]);
        i += 1;
    }

    // Phase 2: insert blank lines based on mode.
    // Track indentation level of list items to detect transitions.
    let mut result = String::with_capacity(output.len() + 64);
    let mut prev_item_indent: Option<usize> = None;
    let mut in_list_run = false; // true when previous line(s) were list items
    let mut prev_was_blank = false;
    let mut had_continuation = false; // true when continuation content seen since last item

    for (idx, line) in stripped.iter().enumerate() {
        let trimmed = line.trim_start();
        let indent = line.len() - trimmed.len();
        let is_item = is_list_item_start(trimmed);
        let is_cont = is_list_continuation(line);

        if !prev_was_blank && idx > 0 {
            if is_item {
                // List item: check if we need a blank line before it
                let need_blank = match mode {
                    ListSpacingMode::Loose => true,
                    ListSpacingMode::Normal => {
                        if let Some(prev) = prev_item_indent {
                            // Descents and same-level siblings stay tight; loose items
                            // and shallower returns keep their separating blank.
                            indent < prev || had_continuation
                        } else {
                            false
                        }
                    }
                    ListSpacingMode::Compact => false,
                };
                if need_blank {
                    result.push('\n');
                }
            } else if is_cont && !trimmed.is_empty() && in_list_run {
                // Non-item continuation after a run of list items =
                // prose following a (sub-)list. Needs a blank line.
                if let Some(prev) = prev_item_indent
                    && indent <= prev
                {
                    result.push('\n');
                }
            }
        }

        if is_item {
            prev_item_indent = Some(indent);
            in_list_run = true;
            had_continuation = false;
        } else if !trimmed.is_empty() {
            if is_cont {
                had_continuation = true;
            }
            if !is_cont {
                prev_item_indent = None;
            }
            in_list_run = false;
        }

        prev_was_blank = trimmed.is_empty();
        result.push_str(line);
        result.push('\n');
    }

    if !output.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    *output = result;
}

/// Returns `true` if the line starts a list item (ordered or unordered).
fn is_list_item_start(trimmed: &str) -> bool {
    // Unordered: *, -, or + followed by space
    if trimmed.starts_with("* ") || trimmed.starts_with("- ") || trimmed.starts_with("+ ") {
        return true;
    }

    // Ordered: digits followed by . or ) and space
    let bytes = trimmed.as_bytes();
    if bytes.is_empty() || !bytes[0].is_ascii_digit() {
        return false;
    }
    for (i, &b) in bytes.iter().enumerate().skip(1) {
        if b == b'.' || b == b')' {
            return bytes.get(i + 1) == Some(&b' ');
        }
        if !b.is_ascii_digit() {
            return false;
        }
    }
    false
}

/// Returns `true` if the line is indented continuation content within a list.
fn is_list_continuation(line: &str) -> bool {
    let trimmed = line.trim_start();
    !trimmed.is_empty() && line.len() > trimmed.len() && !is_list_item_start(trimmed)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Helper to count occurrences of a pattern in a string
    fn count_occurrences(haystack: &str, needle: &str) -> usize {
        haystack.matches(needle).count()
    }

    // ==================== Incidental Newline Tests ====================

    #[test]
    fn strip_incidental_newlines_drops_after_trailing_whitespace() {
        let content = "Wrapped line \ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_replaces_with_space_without_trailing_whitespace() {
        let content = "Wrapped line\ncontinues here";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "Wrapped line continues here");
    }

    #[test]
    fn strip_incidental_newlines_preserves_blank_lines() {
        let content = "First paragraph\n\nSecond paragraph\n\n\nThird paragraph";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_fenced_code_verbatim() {
        let content = "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "Before\n```rust\nlet value = 1;\nlet other = 2;\n```\nAfter"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_indented_code_verbatim() {
        let content = "Before\n\n    let value = 1;\n    let other = 2;\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_newline_inside_inline_code_span() {
        let content = "Before `code\nspan` after";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_html_block_verbatim() {
        let content = "<div>\n<span>One</span>\n<span>Two</span>\n</div>\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquote_prefix() {
        let content = "> Wrapped quote\n> continues here\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "> Wrapped quote continues here\n\nAfter");
    }

    #[test]
    fn strip_incidental_newlines_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_list_marker() {
        let content = "- Wrapped item\ncontinues here\n- Second item";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "- Wrapped item continues here\n- Second item");
    }

    #[test]
    fn strip_incidental_newlines_preserves_transclusion_directives() {
        let content = "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped\ntext\n::end-disclosure\n";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(
            stripped,
            "::file ./README.md\n::code ./src/main.rs\n::shell echo hi\n::disclosure Details\n::details\nWrapped text\n::end-disclosure\n"
        );
    }

    #[test]
    fn strip_incidental_newlines_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_preserves_blockquoted_shell_block_bodies() {
        let content = "> ::shell-block\n> echo a\n> echo b\n> ::end-block\n\nAfter";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, content);
    }

    #[test]
    fn strip_incidental_newlines_treats_crlf_and_lf_as_newlines() {
        let content = "One\r\ntwo\nthree\r\n\nFour";
        let stripped = strip_incidental_newlines(content);
        assert_eq!(stripped, "One two three\n\nFour");
    }

    // ==================== Fixed-Width Reflow Tests ====================

    #[test]
    fn reflow_to_width_wraps_ascii_paragraphs_at_common_widths() {
        let content = "This paragraph has enough words to wrap cleanly at several common editor widths without needing to split any individual word.";

        for width in [20, 40, 80, 100] {
            let reflowed = cleanup_to_fixed_width(content, width);
            assert!(
                reflowed
                    .lines()
                    .all(|line| UnicodeWidthStr::width(line) <= width),
                "line exceeded width {width}:\n{reflowed}"
            );
        }
    }

    #[test]
    fn reflow_to_width_uses_unicode_display_columns() {
        let content = "café naïve résumé words";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, "café naïve\nrésumé words");
        assert!(reflowed.lines().all(|line| UnicodeWidthStr::width(line) <= 12));
    }

    #[test]
    fn reflow_to_width_keeps_long_words_on_one_line() {
        let content = "supercalifragilisticexpialidocious";
        let reflowed = cleanup_to_fixed_width(content, 10);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_handles_empty_document() {
        assert_eq!(cleanup_to_fixed_width("", 80), "");
    }

    #[test]
    fn reflow_to_width_preserves_code_only_document() {
        let content = "```rust\nlet value = \"a very long line that stays untouched\";\n```\n";
        let reflowed = cleanup_to_fixed_width(content, 20);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_mixed_list_marker_alignment() {
        let content = "- short marker has enough text to wrap around the target width\n1. ordered marker has enough text to wrap around the target width\n- [ ] task marker has enough text to wrap around the target width";
        let reflowed = cleanup_to_fixed_width(content, 28);

        assert_eq!(
            reflowed,
            "- short marker has enough\n  text to wrap around the\n  target width\n1. ordered marker has enough\n   text to wrap around the\n   target width\n- [ ] task marker has enough\n      text to wrap around\n      the target width"
        );
    }

    #[test]
    fn reflow_to_width_preserves_table_rows() {
        let content = "| A | B |\n|---|---|\n| a very long cell | another long cell |\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_keeps_blockquote_prefixes() {
        let content = "> This quote has enough text to wrap while preserving the quote prefix";
        let reflowed = cleanup_to_fixed_width(content, 24);

        assert_eq!(
            reflowed,
            "> This quote has enough\n> text to wrap while\n> preserving the quote\n> prefix"
        );
    }

    #[test]
    fn reflow_to_width_preserves_transclusion_directives() {
        let content = "::file ./a/path/that/should/not/be/wrapped.md\n::code ./src/main.rs\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_shell_block_bodies() {
        let content = "::shell-block\necho a\necho b\n::end-block\n";
        let reflowed = cleanup_to_fixed_width(content, 8);

        assert_eq!(reflowed, content);
    }

    #[test]
    fn reflow_to_width_preserves_html_blocks() {
        let content = "<div class=\"options\">\n<span>Very long HTML content stays untouched.</span>\n</div>\n";
        let reflowed = cleanup_to_fixed_width(content, 12);

        assert_eq!(reflowed, content);
    }

    #[test]
    #[should_panic(expected = "fixed-width cleanup requires a width greater than 0")]
    fn cleanup_to_fixed_width_rejects_zero_width() {
        let _ = cleanup_to_fixed_width("content", 0);
    }

    // ==================== Blank Line Tests ====================

    #[test]
    fn test_cleanup_adds_blank_line_between_header_and_paragraph() {
        // Input has no blank line between header and paragraph
        let content = "# Title\nParagraph text";
        let cleaned = cleanup_content(content);

        // Should have exactly one blank line (two newlines) between header and paragraph
        assert!(
            cleaned.contains("# Title\n\nParagraph"),
            "Expected blank line between header and paragraph, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_between_consecutive_headers() {
        let content = "# Header 1\n## Header 2";
        let cleaned = cleanup_content(content);

        // Should have blank line between headers
        assert!(
            cleaned.contains("# Header 1\n\n## Header 2"),
            "Expected blank line between headers, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_code_block() {
        let content = "```rust\nfn main() {}\n```\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after code block
        assert!(
            cleaned.contains("```\n\nParagraph"),
            "Expected blank line after code block, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_list() {
        // Note: In CommonMark, "* Item\nParagraph" creates a "lazy" paragraph inside the list
        // We need an explicit blank line in input to separate list from paragraph
        let content = "* Item 1\n* Item 2\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after list
        assert!(
            cleaned.contains("Item 2\n\nParagraph"),
            "Expected blank line after list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_adds_blank_line_after_blockquote() {
        // Note: In CommonMark, "> Quote\nParagraph" creates a "lazy" paragraph inside blockquote
        // We need an explicit blank line in input to separate blockquote from paragraph
        let content = "> Quote\n\nParagraph after";
        let cleaned = cleanup_content(content);

        // Should have blank line after blockquote
        assert!(
            cleaned.contains("Quote\n\nParagraph"),
            "Expected blank line after blockquote, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_existing_blank_lines() {
        let content = "# Header\n\nSome text\n\n## Subheader";
        let cleaned = cleanup_content(content);

        // Should preserve single blank lines (not double them up)
        assert_eq!(
            count_occurrences(&cleaned, "\n\n\n"),
            0,
            "Should not have triple newlines, got:\n{}",
            cleaned
        );
        assert!(cleaned.contains("# Header\n\nSome text"));
        assert!(cleaned.contains("Some text\n\n## Subheader"));
    }

    #[test]
    fn test_cleanup_does_not_add_excessive_blank_lines() {
        let content = "# Title\nParagraph 1\n\nParagraph 2";
        let cleaned = cleanup_content(content);

        // Count blank lines (consecutive \n\n)
        let blank_line_count = count_occurrences(&cleaned, "\n\n");

        // Should have exactly 2 blank lines: after title and between paragraphs
        assert_eq!(
            blank_line_count, 2,
            "Expected 2 blank lines, got {} in:\n{}",
            blank_line_count, cleaned
        );
    }

    // ==================== Table Alignment Tests ====================

    #[test]
    fn test_table_columns_are_aligned() {
        let content = "|Short|VeryLongHeader|\n|---|---|\n|A|B|";
        let cleaned = cleanup_content(content);

        // The table should be rendered with aligned columns
        // Note: exact format depends on cmark rendering, but cells should be padded
        assert!(cleaned.contains("Short"), "Content should be preserved");
        assert!(
            cleaned.contains("VeryLongHeader"),
            "Content should be preserved"
        );
    }

    #[test]
    fn test_table_structure_preserved() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let cleaned = cleanup_content(content);

        // Should still have pipe characters for table structure
        assert!(cleaned.contains("|"), "Table structure should be preserved");
        assert!(cleaned.contains("Col1"));
        assert!(cleaned.contains("Col2"));
    }

    #[test]
    fn test_align_tables_preserves_non_table_content() {
        let parser = Parser::new_ext("# Title\nParagraph", Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events.clone());

        // Should preserve all events when no table present
        assert_eq!(processed.len(), events.len());
    }

    #[test]
    fn test_align_tables_handles_simple_table() {
        let content = "| Col1 | Col2 |\n|------|------|\n| A | B |";
        let parser = Parser::new_ext(content, Options::all());
        let events: Vec<Event> = parser.collect();
        let processed = align_tables_in_stream(events);

        // Should preserve table structure
        let has_table_start = processed
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        let has_table_end = processed
            .iter()
            .any(|e| matches!(e, Event::End(TagEnd::Table)));
        assert!(has_table_start);
        assert!(has_table_end);
    }

    // ==================== Edge Case Tests ====================

    #[test]
    fn test_cleanup_handles_empty_content() {
        let content = "";
        let cleaned = cleanup_content(content);
        assert_eq!(cleaned, "");
    }

    #[test]
    fn test_cleanup_handles_plain_text() {
        let content = "Just plain text";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Just plain text"));
    }

    #[test]
    fn test_cleanup_ensures_single_trailing_newline() {
        let content = "# Title\n\nParagraph";
        let cleaned = cleanup_content(content);
        let trimmed = cleaned.trim_end_matches('\n');
        let trailing_newlines = cleaned.len() - trimmed.len();

        assert_eq!(
            trailing_newlines, 1,
            "Expected exactly one trailing newline, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_collapses_multiple_trailing_newlines_to_one() {
        let content = "# Title\n\nParagraph\n\n\n";
        let cleaned = cleanup_content(content);
        let trimmed = cleaned.trim_end_matches('\n');
        let trailing_newlines = cleaned.len() - trimmed.len();

        assert_eq!(
            trailing_newlines, 1,
            "Expected trailing newlines to collapse to one, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_handles_multiple_paragraphs() {
        let content = "Para 1\n\nPara 2\n\nPara 3";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("Para 1"));
        assert!(cleaned.contains("Para 2"));
        assert!(cleaned.contains("Para 3"));
    }

    #[test]
    fn test_cleanup_preserves_code_block_content() {
        let content = "```rust\nfn main() {\n    println!(\"Hello\");\n}\n```";
        let cleaned = cleanup_content(content);
        assert!(cleaned.contains("fn main()"));
        assert!(cleaned.contains("println!"));
    }

    #[test]
    fn test_cleanup_preserves_code_block_indentation() {
        // Regression test: indentation inside code blocks must be preserved
        let content =
            "```ts title=\"Greet Function\"\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        // The 4-space indentation before console.log must be preserved
        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside code block should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_code_block_indentation_simple() {
        // Test without attributes - just language
        let content = "```ts\nfunction greet() {\n    console.log(\"hi\")\n}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("    console.log"),
            "Indentation inside simple code block should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Regression Tests ====================

    #[test]
    fn test_no_hardbreak_in_output() {
        // This is the main regression test for the bug
        let content = "# Header\n\nParagraph\n\n## Another Header";
        let cleaned = cleanup_content(content);

        // HardBreak would render as `\` or `<br>` - neither should appear
        assert!(
            !cleaned.contains("\\"),
            "Should not contain backslash (HardBreak), got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("<br"),
            "Should not contain <br> (HardBreak), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_after_cleanup_still_parses() {
        let content = "| A | B |\n|---|---|\n| 1 | 2 |";
        let cleaned = cleanup_content(content);

        // Re-parse the cleaned content - should still be a valid table
        let parser = Parser::new_ext(&cleaned, Options::all());
        let events: Vec<Event> = parser.collect();

        let has_table = events
            .iter()
            .any(|e| matches!(e, Event::Start(Tag::Table(_))));
        assert!(
            has_table,
            "Cleaned table should still parse as table, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_cells_have_spacing() {
        // Regression test: table cells should have space after | and before |
        let content = "|A|B|\n|---|---|\n|1|2|";
        let cleaned = cleanup_content(content);

        // Should have "| A " pattern (space after pipe, content, space before next pipe)
        assert!(
            cleaned.contains("| A "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("| B "),
            "Table cells should have leading space after |, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_table_code_spans_not_escaped() {
        // Regression test: backticks in code spans should not be escaped
        let content = "| Name | Value |\n|---|---|\n| `foo` | bar |";
        let cleaned = cleanup_content(content);

        // Should preserve backticks without escaping
        assert!(
            cleaned.contains("`foo`"),
            "Code spans should preserve backticks, got:\n{}",
            cleaned
        );
        // Should NOT have escaped backticks
        assert!(
            !cleaned.contains("\\`"),
            "Code spans should not have escaped backticks, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with links must be properly aligned.
    ///
    /// Previously, the table alignment code measured link text width (e.g., "no-color.org")
    /// but the rendered markdown output includes the full link syntax `[text](url)`,
    /// causing misaligned columns.
    #[test]
    fn test_table_alignment_with_links() {
        let content = "| Variable | Description |\n|----------|-------------|\n| `THEME` | Default theme |\n| `NO_COLOR` | Disable colors ([no-color.org](https://no-color.org)) |";
        let cleaned = cleanup_content(content);

        // All rows should have the same number of pipe characters
        let lines: Vec<&str> = cleaned.lines().collect();
        assert!(
            lines.len() >= 3,
            "Should have at least 3 lines (header, separator, data)"
        );

        // Each data row should end with " |" (space before closing pipe)
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // The link should be preserved
        assert!(
            cleaned.contains("[no-color.org](https://no-color.org)"),
            "Link should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with emphasis must be properly aligned.
    ///
    /// Previously, emphasis markers (*text* or _text_) weren't counted in the width
    /// calculation, causing misaligned columns.
    #[test]
    fn test_table_alignment_with_emphasis() {
        let content = "| Option | Description |\n|--------|-------------|\n| `--mermaid` | *Terminal only*. Renders diagrams |\n| `--html` | Output HTML |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Emphasis should be preserved
        assert!(
            cleaned.contains("*Terminal only*"),
            "Emphasis should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with bold text must be properly aligned.
    #[test]
    fn test_table_alignment_with_strong() {
        let content = "| Name | Description |\n|------|-------------|\n| Test | **Bold text** here |\n| A | Short |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Bold should be preserved
        assert!(
            cleaned.contains("**Bold text**"),
            "Bold should be preserved in table, got:\n{}",
            cleaned
        );
    }

    /// Regression test: Table columns with mixed formatting must be properly aligned.
    #[test]
    fn test_table_alignment_with_mixed_formatting() {
        let content = "| Column | Content |\n|--------|-------------|\n| A | `code` with *emphasis* and [link](url) |\n| B | Plain text |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // All formatting should be preserved
        assert!(cleaned.contains("`code`"), "Code should be preserved");
        assert!(
            cleaned.contains("*emphasis*"),
            "Emphasis should be preserved"
        );
        assert!(cleaned.contains("[link](url)"), "Link should be preserved");
    }

    /// Regression test: Verify table alignment uses unicode width, not char count.
    #[test]
    fn test_table_alignment_unicode_width() {
        // East Asian characters have display width of 2
        let content = "| Name | Greeting |\n|------|----------|\n| 你好 | Hello |\n| A | World |";
        let cleaned = cleanup_content(content);

        // Each data row should end with " |"
        let lines: Vec<&str> = cleaned.lines().collect();
        for line in &lines[2..] {
            assert!(
                line.ends_with(" |"),
                "Table row should end with ' |' for proper alignment, got:\n{}",
                line
            );
        }

        // Content should be preserved
        assert!(
            cleaned.contains("你好"),
            "CJK characters should be preserved"
        );
    }

    #[test]
    fn test_code_block_uses_three_backticks() {
        // Regression test: code blocks should use 3 backticks, not 4
        // pulldown-cmark-to-cmark defaults to 4 backticks (code_block_token_count)
        // which is non-standard and causes rendering issues
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        // Should use exactly 3 backticks for fence
        assert!(
            cleaned.contains("```rust"),
            "Code blocks should start with 3 backticks, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks (the buggy default)
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_without_language_gets_text_language() {
        // Regression test: code blocks without language should get "text" added
        let content = "```\nsome code\n```";
        let cleaned = cleanup_content(content);

        // Should have "text" as language
        assert!(
            cleaned.starts_with("```text\n") || cleaned.contains("\n```text\n"),
            "Code blocks without language should get 'text' as language, got:\n{}",
            cleaned
        );
        // Should NOT have 4 backticks
        assert!(
            !cleaned.contains("````"),
            "Code blocks should not use 4 backticks, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_code_block_preserves_existing_language() {
        // Ensure code blocks with language are not affected
        let content = "```rust\nfn main() {}\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("```rust\n"),
            "Existing language should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_code_blocks_without_language() {
        // Multiple empty code blocks should all get "text" language
        let content = "```\nfirst\n```\n\n```\nsecond\n```";
        let cleaned = cleanup_content(content);

        // Count occurrences of "```text"
        let text_count = cleaned.matches("```text").count();
        assert_eq!(
            text_count, 2,
            "Both code blocks should get 'text' language, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_indented_code_blocks_unchanged() {
        // Indented code blocks should remain unchanged (they don't have language specifiers)
        let content = "    indented code\n    more code";
        let cleaned = cleanup_content(content);

        // Should preserve indented code block
        assert!(
            cleaned.contains("indented code"),
            "Indented code should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Blockquote Formatting Tests ====================

    #[test]
    fn test_blockquote_no_leading_space() {
        // Regression test: blockquotes should not have leading space before >
        let content = "> Simple quote";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> "),
            "Blockquote should start with '> ', got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.starts_with(" >"),
            "Blockquote should not have leading space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_no_empty_first_line() {
        // Regression test: blockquotes should not have empty first line
        let content = "> Quote content";
        let cleaned = cleanup_content(content);

        // Should NOT have "> \n> " pattern (empty blockquote line)
        assert!(
            !cleaned.contains("> \n>"),
            "Blockquote should not have empty first line, got:\n{:?}",
            cleaned
        );
        // Should start directly with content
        assert!(
            cleaned.starts_with("> Quote"),
            "Blockquote should start with content, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_multiline() {
        // Incidental wrapping inside a blockquote should keep the quote prefix.
        let content = "> Line 1\n> Line 2\n> Line 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Line 1 Line 2 Line 3"),
            "Blockquote prefix should be preserved after prose collapse, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_nested() {
        // Nested blockquotes should have single space between > markers
        let content = "> > Nested quote";
        let cleaned = cleanup_content(content);

        // Should be "> > " not ">  > " or " >  > "
        assert!(
            cleaned.starts_with("> > Nested"),
            "Nested blockquote should have single space between >, got:\n{:?}",
            cleaned
        );
        assert!(
            !cleaned.contains(">  >"),
            "Nested blockquote should not have double space, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_deeply_nested() {
        // Deeply nested blockquotes
        let content = "> > > Triple nested";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> > > Triple"),
            "Triple nested blockquote should have single spaces, got:\n{:?}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_after_header() {
        // Blockquotes following headers should be formatted correctly
        let content = "# Header\n\n> Quote after header";
        let cleaned = cleanup_content(content);

        // Should have blank line between header and quote, and proper formatting
        assert!(
            cleaned.contains("# Header\n\n> Quote"),
            "Blockquote after header should have blank line and proper format, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_long_content() {
        // Long blockquotes should not be mangled
        let content = "> Ut faucibus mauris mauris, sed tincidunt augue hendrerit eu. In ultrices ultrices commodo.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("> Ut faucibus"),
            "Long blockquote should start correctly, got:\n{:?}",
            cleaned
        );
        assert!(
            cleaned.contains("commodo."),
            "Long blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_preserves_content_spaces() {
        // Spaces in blockquote content should be preserved (not just prefix)
        let content = "> Code:   let x = 1";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("> Code:   let"),
            "Spaces in blockquote content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_blockquote_preserves_blank_separator_before_attribution() {
        // A blank `>` line mid-blockquote is an intentional paragraph break
        // (e.g., separating content from an attribution line) and must survive cleanup.
        let content = "> Some quoted content\n>\n> — Wikipedia";
        let cleaned = cleanup_content(content);

        // The blank `>` line may gain a trailing space from fix_blockquote_line,
        // so check for both forms.
        let has_separator =
            cleaned.contains(">\n> — Wikipedia") || cleaned.contains("> \n> — Wikipedia");
        assert!(
            has_separator,
            "Blank blockquote separator before attribution should be preserved, got:\n{:?}",
            cleaned
        );
    }

    // ==================== List Marker Preservation Tests ====================

    #[test]
    fn test_list_marker_dash_preserved() {
        let content = "- Item 1\n- Item 2\n- Item 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Item 1"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Item 2"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_plus_preserved() {
        let content = "+ Alpha\n+ Beta\n+ Gamma";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("+ Alpha"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Beta"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_asterisk_preserved() {
        let content = "* One\n* Two\n* Three";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* One"),
            "Asterisk list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_multiple_lists_different_markers() {
        let content = "- Dash item\n\n+ Plus item\n\n* Asterisk item";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "First list should use dash, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Second list should use plus, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Asterisk"),
            "Third list should use asterisk, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_with_header_and_text() {
        // Regression test: List markers preserved in real document context
        let content = "# Title\n\nSome text.\n\n- First\n- Second\n\nMore text.\n\n+ Alpha\n+ Beta";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- First"),
            "Dash markers should be preserved in document, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Alpha"),
            "Plus markers should be preserved in document, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_inside_blockquote() {
        // List markers inside blockquotes should be preserved
        let content = "> - Quoted item 1\n> - Quoted item 2";
        let cleaned = cleanup_content(content);

        // The structure should be preserved
        assert!(
            cleaned.contains("Quoted item 1"),
            "Blockquote list content should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_not_changed_in_code_block() {
        // List markers inside code blocks should not be modified
        let content = "```\n* This is code\n- Also code\n+ More code\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* This is code"),
            "Asterisk in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("- Also code"),
            "Dash in code should not change, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ More code"),
            "Plus in code should not change, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_mixed_list_types_separated_by_text() {
        // Multiple lists with different markers separated by paragraph text
        let content = "- Dash list\n\nParagraph text\n\n+ Plus list\n\nMore text\n\n* Star list";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- Dash"),
            "Dash list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ Plus"),
            "Plus list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* Star"),
            "Star list marker should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_list_marker_consistency_in_same_list() {
        // All items in the same list should use the same marker
        let content = "- Item 1\n- Item 2\n- Item 3\n- Item 4\n- Item 5";
        let cleaned = cleanup_content(content);

        // All items should have dash markers
        let dash_count =
            cleaned.matches("\n- ").count() + if cleaned.starts_with("- ") { 1 } else { 0 };
        assert_eq!(
            dash_count, 5,
            "All 5 items should use dash marker, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_ordered_list_numbers_are_incremented() {
        let content = "1. First\n2. Second\n3. Third";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("1. First"),
            "First item should be numbered 1, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("2. Second"),
            "Second item should be numbered 2, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("3. Third"),
            "Third item should be numbered 3, got:\n{}",
            cleaned
        );
    }

    // ==================== List Indentation Tests ====================

    #[test]
    fn test_nested_list_preserves_4_space_indentation() {
        // Regression test: 4-space indentation should be preserved
        let content = "- Level 1\n    - Level 2\n        - Level 3";
        let cleaned = cleanup_content(content);

        // Should have 4-space indentation for nested items
        assert!(
            cleaned.contains("\n    - Level 2"),
            "4-space indentation should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n        - Level 3"),
            "8-space indentation should be preserved, got:\n{}",
            cleaned
        );
        // Tight nested lists must not gain a spurious blank line before children.
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n        - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_normalizes_to_4_space_indentation() {
        // Default cleanup normalizes to 4-space indentation
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\n    - Level 2"),
            "Default indentation should be 4 spaces, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_preserves_2_space_with_explicit_indent() {
        // Explicit 2-space indent via cleanup_content_with_indent
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content_with_indent(content, 2);

        assert!(
            cleaned.contains("\n  - Level 2"),
            "2-space indentation should be preserved when explicitly requested, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - Level 3"),
            "4-space indentation should be preserved at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n  - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_nested_list_in_todo_list() {
        // Regression test for nested TODO list items
        let content = "- [x] Task 1\n- Progress\n    - [ ] Sub-task 1\n    - [ ] Sub-task 2";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\n    - [ ] Sub-task 1"),
            "Nested TODO items should preserve 4-space indentation, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_with_indent_forces_4_space_indentation() {
        let content = "- Level 1\n  - Level 2\n    - Level 3";
        let cleaned = cleanup_content_with_indent(content, 4);

        assert!(
            cleaned.contains("\n    - Level 2"),
            "Nested list should use 4-space indentation, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n        - Level 3"),
            "Nested list should use 8-space indentation at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n        - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_with_indent_forces_2_space_indentation() {
        let content = "- Level 1\n    - Level 2\n        - Level 3";
        let cleaned = cleanup_content_with_indent(content, 2);

        assert!(
            cleaned.contains("\n  - Level 2"),
            "Nested list should use 2-space indentation, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - Level 3"),
            "Nested list should use 4-space indentation at level 2, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n  - Level 2"),
            "no blank line before a tight child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - Level 3"),
            "no blank line before a tight grandchild, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_loose_list_markers_preserved() {
        // Regression test: loose lists (items separated by blank lines with paragraph
        // content) should preserve all markers. Previously, only the first item kept
        // its original marker; subsequent items reverted to '*'.
        let content = "\
- **First**

    Paragraph under first item.

- **Second**

    Paragraph under second item.

- **Third**

    Paragraph under third item.";
        let cleaned = cleanup_content(content);

        let dash_count =
            cleaned.matches("\n- ").count() + if cleaned.starts_with("- ") { 1 } else { 0 };
        assert_eq!(
            dash_count, 3,
            "All 3 loose list items should use dash marker, got:\n{}",
            cleaned
        );
    }

    // ==================== Bracket Escaping Tests ====================

    #[test]
    fn test_bracket_not_escaped_progress_indicator() {
        // Regression test: progress indicators should not be escaped
        let content =
            "- [0%] started\n- [25%] progress\n- [50%] halfway\n- [75%] almost\n- [100%] done";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[0%]"),
            "Progress indicator [0%] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[0%\\]"),
            "Progress indicator should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[25%]"),
            "Progress indicator [25%] should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_not_escaped_bang_indicator() {
        // Regression test: [!] blocked indicator should not be escaped
        let content = "- [!] blocked task";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[!]"),
            "Blocked indicator [!] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\\[!\\]"),
            "Blocked indicator should not be escaped, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_bracket_preserved_in_actual_links() {
        // Actual links should still work
        let content = "Check out [this link](https://example.com)";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[this link](https://example.com)"),
            "Links should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_task_list_checkboxes_not_escaped() {
        // Task list checkboxes should never be escaped
        let content = "- [x] done\n- [ ] pending";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("[x]"),
            "Checkbox [x] should not be escaped, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("[ ]"),
            "Checkbox [ ] should not be escaped, got:\n{}",
            cleaned
        );
    }

    // ==================== Smart Quotes Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_straight_double_quotes() {
        // Regression test: Normal quotes should NOT be converted to smart quotes
        let content = r#"He said "hello" and "goodbye"."#;
        let cleaned = cleanup_content(content);

        // Should contain straight quotes (ASCII 0x22)
        assert!(
            cleaned.contains('"'),
            "Straight double quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart quotes (U+201C and U+201D - left and right double quotes)
        assert!(
            !cleaned.contains('\u{201C}') && !cleaned.contains('\u{201D}'),
            "Should not contain smart/curly quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_straight_single_quotes() {
        // Regression test: Single quotes/apostrophes should NOT be converted
        let content = "It's a test and 'quoted text' here.";
        let cleaned = cleanup_content(content);

        // Should contain straight single quote (ASCII 0x27)
        assert!(
            cleaned.contains("'"),
            "Straight single quotes should be preserved, got:\n{}",
            cleaned
        );
        // Should NOT contain smart single quotes (U+2018 and U+2019)
        assert!(
            !cleaned.contains('\u{2018}') && !cleaned.contains('\u{2019}'),
            "Should not contain smart/curly single quotes, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_quotes_in_code() {
        // Quotes inside code blocks should definitely be preserved
        let content = "```\nlet s = \"hello\";\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("\"hello\""),
            "Quotes in code should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== Emphasis/Italics Tests (Regression) ====================

    #[test]
    fn test_cleanup_preserves_asterisk_emphasis() {
        // Regression test: *asterisk* emphasis should NOT be converted to _underscore_
        let content = "This has *asterisk italics* here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk italics*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_emphasis() {
        // Regression test: _underscore_ emphasis should NOT be converted to *asterisk*
        let content = "This has _underscore italics_ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_underscore italics_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_emphasis_styles() {
        // Both asterisk and underscore styles should be preserved in the same document
        let content = "Mix of *asterisk* and _underscore_ emphasis.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*asterisk*"),
            "Asterisk emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_underscore_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_asterisk_strong() {
        // **bold** should be preserved
        let content = "This has **asterisk bold** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_underscore_strong() {
        // __bold__ should be preserved
        let content = "This has __underscore bold__ here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_mixed_strong_styles() {
        // Both asterisk and underscore bold should be preserved
        let content = "Mix of **asterisk bold** and __underscore bold__ text.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**asterisk bold**"),
            "Asterisk strong should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("__underscore bold__"),
            "Underscore strong should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_preserves_nested_emphasis() {
        // Nested emphasis should preserve styles
        let content = "This has **bold with _nested italics_** here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**"),
            "Bold markers should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_nested italics_"),
            "Nested underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_cleanup_emphasis_in_code_blocks_unchanged() {
        // Emphasis markers inside code blocks should NOT be modified
        let content = "```\n*not* _emphasis_ **here**\n```";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("*not*"),
            "Emphasis markers in code should be unchanged, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore in code should be unchanged, got:\n{}",
            cleaned
        );
    }

    // ==================== Regression Tests for Emphasis Restoration ====================

    #[test]
    fn test_regression_nested_emphasis_preserves_original_styles() {
        // Regression test: nested emphasis like **_text_** should preserve both styles
        // Previously, ***text*** was incorrectly output as ***text___ or similar
        let content = "**_nested_** test";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve both styles (** and _), got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_literal_underscores_not_converted() {
        // Regression test: literal underscores in words like word_with_underscores
        // should NOT be converted to asterisks or modified in any way
        let content = "Testing word_with_underscores here.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores in words should not be modified, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_emphasis_and_literal_underscores_combined() {
        // Regression test: combination of emphasis and literal underscores
        // The underscore emphasis should be preserved AND the literal underscores
        // in word_with_underscores should remain unchanged
        let content = "Testing _emphasis_ inside a word_with_underscores and **_nested_** styles.";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_emphasis_"),
            "Underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("word_with_underscores"),
            "Literal underscores should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("**_nested_**"),
            "Nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_triple_emphasis_markers() {
        // Regression test: ***text*** (strong + emphasis) should correctly restore styles
        let content = "***both bold and italic***";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("***both bold and italic***"),
            "Triple emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_mixed_nested_emphasis() {
        // Regression test: mixed nested styles like __*text*__
        let content = "__*mixed nesting*__";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("__*mixed nesting*__"),
            "Mixed nested emphasis should preserve styles, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_multiple_emphasis_with_underscores_in_text() {
        // Regression test: multiple emphasis with underscores in regular text
        let content = "_first_ and snake_case_name and _second_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("_first_"),
            "First underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("snake_case_name"),
            "Literal underscores in snake_case should not be modified, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_second_"),
            "Second underscore emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_list_marker_not_confused_with_emphasis() {
        // Regression test: asterisk list markers should NOT be treated as emphasis markers
        // Previously, `* list item with _emphasis_` was incorrectly converted to
        // `_ list item with _emphasis*` because the list marker `*` consumed an emphasis marker
        let content = "* list item with _emphasis_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.starts_with("* "),
            "List marker should remain as asterisk, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasis_"),
            "Emphasis should be preserved as underscore, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_nested_list_markers_not_confused_with_emphasis() {
        // Regression test: nested list markers should not be confused with emphasis
        let content = "* outer _emphasized_\n  * inner _also emphasized_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("* outer"),
            "Outer list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* inner") || cleaned.contains("  * inner"),
            "Inner list marker should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_emphasized_"),
            "Outer emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("_also emphasized_"),
            "Inner emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn test_regression_multiple_list_types_with_emphasis() {
        // Regression test: different list markers with emphasis in same document
        let content = "- dash with _em1_\n* asterisk with _em2_\n+ plus with _em3_";
        let cleaned = cleanup_content(content);

        assert!(
            cleaned.contains("- dash with _em1_"),
            "Dash list with emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("* asterisk with _em2_"),
            "Asterisk list with emphasis should be preserved, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("+ plus with _em3_"),
            "Plus list with emphasis should be preserved, got:\n{}",
            cleaned
        );
    }

    // ==================== List Spacing Mode Tests ====================

    #[test]
    fn normal_no_blank_lines_between_same_level_items() {
        let input = "1. First\n\n2. Second\n\n3. Third\n";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Normal mode: no blank lines between same-level items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_descent_into_sublist_is_tight() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content(input);
        // Descending into a tight sub-list must not insert a blank line.
        assert!(
            cleaned.contains("lessons:\n    - @docs"),
            "Normal: tight descent into sub-list stays tight, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_return_from_sublist_inserts_blank() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content(input);
        // Returning to the shallower parent level still separates with a blank line.
        assert!(
            cleaned.contains("commits.md\n\n2."),
            "Normal: blank line when returning from sub-list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_no_blank_between_same_level_without_transition() {
        let input = "\
0. If no files are staged then exit.
1. read the lessons
2. evaluate the files
3. organize the work
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("exit.\n1.") || cleaned.contains("exit.\n0."),
            "Normal: no blank lines between same-level items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_blank_line_after_list_before_prose() {
        let input = "\
1. First
   - sub item
2. Second

Some prose after the list.
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("2. Second\n\nSome prose"),
            "Normal: blank line between list end and prose, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_removes_all_blank_lines_between_items() {
        let input = "1. First\n\n2. Second\n\n3. Third\n";
        let cleaned = cleanup_content_compact(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Compact: no blank lines between items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_with_nested_list() {
        let input = "\
1. read the lessons:

   - @docs/knowledge/commits.md

2. evaluate all the _staged_ files
";
        let cleaned = cleanup_content_compact(input);
        assert!(
            !cleaned.contains("\n\n2."),
            "Compact: no blank line before item 2, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn compact_preserves_blank_before_non_list() {
        let input = "Some paragraph.\n\n1. First\n\n2. Second\n";
        let cleaned = cleanup_content_compact(input);
        assert!(
            cleaned.contains("paragraph.\n\n1."),
            "Compact: preserve blank between paragraph and list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_adds_blank_lines_between_all_items() {
        let input = "1. First\n2. Second\n3. Third\n";
        let cleaned = cleanup_content_loose(input);
        assert!(
            cleaned.contains("1. First\n\n2. Second\n\n3. Third"),
            "Loose: blank lines between all items, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_with_nested_list() {
        let input = "\
1. read the lessons:
   - @docs/knowledge/commits.md
2. evaluate
3. organize
";
        let cleaned = cleanup_content_loose(input);
        assert!(
            cleaned.contains("\n\n2."),
            "Loose: blank line before item 2, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n\n3."),
            "Loose: blank line before item 3, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_loose_list_preserves_blank_lines_between_items() {
        // Loose lists (items with continuation paragraphs) should have blank
        // lines between items in Normal mode, not just in Loose mode.
        let input = "\
- **First**

    Paragraph under first item.

- **Second**

    Paragraph under second item.

- **Third**

    Paragraph under third item.
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("first item.\n\n- **Second**"),
            "Normal: blank line before second item in loose list, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("second item.\n\n- **Third**"),
            "Normal: blank line before third item in loose list, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_list_stays_tight_in_normal_mode() {
        let input = "1. First\n2. Second\n3. Third\n";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("1. First\n2. Second\n3. Third"),
            "Normal: tight list stays tight, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn normal_blank_line_between_sublist_and_prose() {
        let input = "\
6. review lessons:
   1. important
   2. not represented

   If both criteria are met then save.
";
        let cleaned = cleanup_content(input);
        assert!(
            cleaned.contains("represented\n\n   If both"),
            "Normal: blank line between sub-list and prose continuation, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_nested_list_stays_tight_after_cleanup() {
        // Regression test modeled on the incident's `## Closure` section.
        let input = "\
## Closure

- Save your review suggestions to \"path.md\"
- Save the following frontmatter properties on \"path.md\":
    - based on your review suggestions indicate whether you think this feature is **ready for production**
    - set the `agent` frontmatter property to \"codex\"
    - set the `model` frontmatter property as \"gpt-4o\"
    - set the `created` frontmatter property to \"2026-06-18\"

**Next steps:**

- verify the file
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("properties on \"path.md\":\n    - based on"),
            "tight nested list: parent must be directly followed by child, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("properties on \"path.md\":\n\n    - based on"),
            "tight nested list: no blank line before the first child, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("\n    - set the `agent` frontmatter property"),
            "nested children must keep the configured 4-space indent, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn tight_siblings_stay_tight() {
        let input = "\
- alpha
- beta
    - gamma
    - delta
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("- alpha\n- beta\n    - gamma\n    - delta"),
            "Normal: same-level siblings and descents stay tight, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n- beta"),
            "Normal: no blank line between same-level siblings, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("\n\n    - gamma"),
            "Normal: no blank line before a tight child, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn closing_a_sublist_inserts_blank() {
        let input = "\
- parent:
    - child

- sibling
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("parent:\n    - child"),
            "Normal: descent into sub-list stays tight, got:\n{}",
            cleaned
        );
        assert!(
            !cleaned.contains("parent:\n\n    - child"),
            "Normal: no blank line between parent and child, got:\n{}",
            cleaned
        );
        assert!(
            cleaned.contains("child\n\n- sibling"),
            "Normal: returning to a shallower level inserts a blank line, got:\n{}",
            cleaned
        );
    }

    #[test]
    fn loose_list_keeps_blank_lines_in_normal_mode() {
        // Focused guard: loose (continuation) items must still separate with blanks.
        let input = "\
- **First**

    Paragraph under first item.

- **Second**
";
        let cleaned = cleanup_content(input);

        assert!(
            cleaned.contains("Paragraph under first item.\n\n- **Second**"),
            "Normal: loose list items keep their blank-line separator, got:\n{}",
            cleaned
        );
    }
}

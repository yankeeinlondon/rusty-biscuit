//! Markdown cleanup implementation using an explicit two-stage pass pipeline.
//!
//! Cleanup first transforms the pulldown-cmark event stream, then applies ordered
//! string passes to the serialized Markdown. Incidental-newline stripping and
//! fixed-width reflow are kept in the reflow module but remain public here.

mod blockquote;
mod brackets;
mod emphasis;
mod lists;
mod opaque;
mod reflow;
mod tables;

#[cfg(test)]
mod perf_profile;
#[cfg(test)]
mod tests;

use pulldown_cmark::{CodeBlockKind, CowStr, Event, Options, Parser, Tag};
use pulldown_cmark_to_cmark::Options as CmarkOptions;
use std::ops::Range;

use blockquote::fix_blockquote_formatting;
use brackets::unescape_brackets;
use emphasis::{
    get_preferred_emphasis_style, preserve_original_emphasis, restore_emphasis_placeholders,
    unescape_emphasis_chars,
};
use lists::{
    detect_list_indentation, extract_additional_paragraph_contexts,
    extract_indented_code_markers, extract_list_item_contexts, extract_list_markers,
    fix_list_indentation, normalize_list_spacing, protect_opaque_directive_bodies,
    restore_list_markers, restore_opaque_directive_bodies,
};
pub use reflow::{reflow_to_width, strip_incidental_newlines};
use tables::align_tables_in_stream;

/// Returns parser options suitable for cleanup operations.
fn cleanup_parser_options() -> Options {
    Options::all() - Options::ENABLE_SMART_PUNCTUATION - Options::ENABLE_DEFINITION_LIST
}

/// Constructs the Markdown parser for a cleanup-path pass.
///
/// Every cleanup-path parse is funneled through this one constructor so the
/// parse count per public entry point is a measurable property rather than an
/// assumption. Spec acceptance criterion 15 caps that count; `tests::parse_count`
/// asserts it. Adding a `Parser::new_ext` call to the cleanup path without
/// routing it here would make that assertion silently under-count.
fn cleanup_parser(content: &str) -> Parser<'_> {
    #[cfg(test)]
    parse_count::record();
    Parser::new_ext(content, cleanup_parser_options())
}

/// Thread-local tally of cleanup-path Markdown parses, for acceptance-criterion
/// 15's structural budget.
///
/// Thread-local rather than global so the count is unaffected by tests running
/// concurrently in one process.
#[cfg(test)]
pub(super) mod parse_count {
    use std::cell::Cell;

    thread_local! {
        static COUNT: Cell<usize> = const { Cell::new(0) };
    }

    /// Records one cleanup-path parse.
    pub(crate) fn record() {
        COUNT.with(|count| count.set(count.get() + 1));
    }

    /// Runs `operation` and returns its value alongside the number of
    /// cleanup-path Markdown parses it performed.
    pub(crate) fn measure<T>(operation: impl FnOnce() -> T) -> (T, usize) {
        let before = COUNT.with(Cell::get);
        let value = operation();
        (value, COUNT.with(Cell::get) - before)
    }
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
    cleanup_content_internal(
        content,
        Some(DEFAULT_INDENT),
        ListSpacingMode::Normal,
        IncidentalNewlineMode::Strip,
    )
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
    cleanup_content_internal(
        content,
        Some(DEFAULT_INDENT),
        ListSpacingMode::Compact,
        IncidentalNewlineMode::Strip,
    )
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
    cleanup_content_internal(
        content,
        Some(DEFAULT_INDENT),
        ListSpacingMode::Loose,
        IncidentalNewlineMode::Strip,
    )
}

/// Cleans up markdown content and enforces a consistent list indentation width.
///
/// `indent_size` is the configured column step between nested levels. An
/// eight-column step uses legal marker padding on parent items when needed so
/// narrow unordered, ordered, and task markers retain their parsed structure.
/// Programmatic values outside the CLI-supported `2`, `4`, and `8` remain
/// constrained to columns that CommonMark can parse as the same list tree.
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
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Normal,
        IncidentalNewlineMode::Strip,
    )
}

/// Cleans markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Normal,
        IncidentalNewlineMode::Preserve,
    )
}

/// Cleans up markdown content with forced indentation in compact mode.
pub fn cleanup_content_with_indent_compact(content: &str, indent_size: usize) -> String {
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Compact,
        IncidentalNewlineMode::Strip,
    )
}

/// Cleans compact markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_compact_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Compact,
        IncidentalNewlineMode::Preserve,
    )
}

/// Cleans up markdown content with forced indentation in loose mode.
pub fn cleanup_content_with_indent_loose(content: &str, indent_size: usize) -> String {
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Loose,
        IncidentalNewlineMode::Strip,
    )
}

/// Cleans loose markdown content with forced indentation without stripping incidental newlines.
pub fn cleanup_content_with_indent_loose_preserving_incidental(
    content: &str,
    indent_size: usize,
) -> String {
    cleanup_content_internal(
        content,
        Some(indent_size.max(1)),
        ListSpacingMode::Loose,
        IncidentalNewlineMode::Preserve,
    )
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
    incidental_newlines: IncidentalNewlineMode,
) -> String {
    // Parse with source ranges to preserve list markers and emphasis styles
    // Use custom options that exclude ENABLE_SMART_PUNCTUATION to preserve original quotes
    let opaque_bodies = protect_opaque_directive_bodies(content);
    if opaque_bodies.requires_source_fallback() {
        return content.to_string();
    }
    let parser = cleanup_parser(opaque_bodies.masked_content());
    let events_with_ranges: Vec<(Event, Range<usize>)> = parser.into_offset_iter().collect();

    // Extract list markers from source for each list
    let opaque_body_lines = opaque_bodies.body_lines();
    let list_markers = extract_list_markers(content, &events_with_ranges, opaque_body_lines);
    let list_item_contexts = extract_list_item_contexts(&events_with_ranges, opaque_body_lines);
    let additional_paragraph_contexts =
        extract_additional_paragraph_contexts(content, &events_with_ranges, opaque_body_lines);
    let protected_markers =
        extract_indented_code_markers(&events_with_ranges, opaque_body_lines);

    // Determine emphasis style:
    // 1. If PREFER_ITALICS env var is set, use that style (standardize all emphasis)
    // 2. Otherwise, preserve original markers (no standardization)
    // Note: PREFER_ITALICS only affects emphasis (italics), NOT strong (bold)
    let preferred_style = get_preferred_emphasis_style();

    // Transform events: replace emphasis/strong events with placeholder characters.
    // This prevents cmark from normalizing them or escaping literal underscores/asterisks.
    // If preferred_style is set, emphasis will be standardized; strong always preserves original.
    let collapsed_events = (incidental_newlines == IncidentalNewlineMode::Strip)
        .then(|| reflow::collapse_incidental_soft_break_events(content, &events_with_ranges));
    let cleanup_events = collapsed_events.as_deref().unwrap_or(&events_with_ranges);
    let events: Vec<Event> = preserve_original_emphasis(content, cleanup_events, preferred_style);

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
    normalize_list_spacing(
        &mut output,
        list_spacing,
        &protected_markers,
    );

    // Post-process to fix blockquote formatting issues from pulldown-cmark-to-cmark
    fix_blockquote_formatting(&mut output, &protected_markers);

    // Restore original list markers (the library normalizes to '*')
    restore_list_markers(
        &mut output,
        &list_markers,
        &protected_markers,
    );

    // Normalize nested list indentation.
    // When forced indentation is provided, use it for consistent nesting.
    // Otherwise preserve the source style when it differs from cmark's 2-space output.
    if let Some(indent_size) = forced_indent {
        fix_list_indentation(
            &mut output,
            indent_size,
            &list_item_contexts,
            &additional_paragraph_contexts,
            &protected_markers,
        );
    } else {
        let original_indent = detect_list_indentation(content, &events_with_ranges);
        fix_list_indentation(
            &mut output,
            original_indent,
            &list_item_contexts,
            &additional_paragraph_contexts,
            &protected_markers,
        );
    }

    // Unescape unnecessarily escaped brackets (e.g., \[0%\] -> [0%])
    unescape_brackets(&mut output);

    restore_opaque_directive_bodies(&mut output, opaque_bodies.payloads());

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

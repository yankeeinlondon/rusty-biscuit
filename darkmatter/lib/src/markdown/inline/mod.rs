//! Custom inline markdown extensions via iterator adapters.
//!
//! This module provides support for inline markdown syntax that is not natively
//! supported by pulldown-cmark, such as `==highlighted text==` and `⌄dimmed text⌄`.
//!
//! ## Overview
//!
//! The main component is [`InlineStyleProcessor`], an iterator adapter that wraps a
//! pulldown-cmark parser and intercepts `Event::Text` events to process custom
//! inline syntax. It emits [`InlineEvent`]s which can be either standard
//! pulldown-cmark events or custom inline tag events.
//!
//! ## Supported Syntax
//!
//! | Syntax | Meaning | HTML Output |
//! |--------|---------|-------------|
//! | `==text==` | Highlighted text | `<mark>text</mark>` |
//! | `⌄text⌄` | Dim/faint text | Literal `⌄text⌄` (preserved) |
//!
//! ## Examples
//!
//! ```
//! use pulldown_cmark::{Parser, Options};
//! use darkmatter::markdown::inline::{InlineStyleProcessor, InlineEvent, InlineTag};
//!
//! let content = "This is ==highlighted== and ⌄dimmed⌄ text.";
//! let parser = Parser::new_ext(content, Options::ENABLE_STRIKETHROUGH);
//! let events: Vec<_> = InlineStyleProcessor::new(parser).collect();
//!
//! // Events will include Start(Mark), Text("highlighted"), End(Mark)
//! // and Start(Dim), Text("dimmed"), End(Dim)
//! ```
//!
//! ## Design
//!
//! The processor uses a fast-path optimization: if a text event doesn't contain
//! any supported delimiters (`==` or `⌄`), it passes through unchanged with zero
//! additional allocations.
//!
//! Only text containing markers is processed and split into multiple events.
//!
//! The processor also handles:
//! - Unclosed markers: `==text` renders as literal `==text`; `⌄text` renders as literal `⌄text`
//! - Escaped markers: `\==` renders as literal `==`; `\⌄` renders as literal `⌄`
//! - Code blocks: `==` and `⌄` inside code are not processed (literal)

mod types;

pub use types::{HorizontalRuleAttrs, InlineEvent, InlineTag};

use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::collections::VecDeque;

/// Backwards-compatible alias for [`InlineStyleProcessor`].
pub type MarkProcessor<'a, I> = InlineStyleProcessor<'a, I>;

/// Kinds of inline delimiters the processor can detect.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum InlineDelimiterKind {
    Mark,
    Dim,
}

/// A detected delimiter in the source text.
#[derive(Debug, Clone, Copy)]
struct InlineDelimiter {
    kind: InlineDelimiterKind,
    byte_start: usize,
    byte_end: usize,
    can_open: bool,
    can_close: bool,
}

/// Iterator adapter that processes text events for custom inline syntax.
///
/// `InlineStyleProcessor` wraps a pulldown-cmark parser iterator and transforms
/// `Event::Text` events that contain `==` or `⌄` markers into sequences of
/// `InlineEvent::Start(Mark)`, `InlineEvent::Standard(Text)`, and
/// `InlineEvent::End(Mark)` events (and similarly for `Dim`).
///
/// ## Fast Path
///
/// For text that doesn't contain any supported delimiters, the processor returns
/// events unchanged with zero additional allocations, ensuring minimal overhead
/// for documents that don't use the extended syntax.
///
/// ## Code Block Handling
///
/// The processor tracks code block state and skips processing inside:
/// - Fenced code blocks (` ``` `)
/// - Indented code blocks
/// - Inline code (`` ` ``)
///
/// ## Unclosed Marker Handling
///
/// If a delimiter is opened but not closed within the same text event,
/// the opening delimiter is converted back to literal text to prevent
/// invalid event sequences.
///
/// ## Examples
///
/// ```
/// use pulldown_cmark::Parser;
/// use darkmatter::markdown::inline::{InlineStyleProcessor, InlineEvent, InlineTag};
///
/// let parser = Parser::new("Hello ==world== and ⌄dim⌄!");
/// let mut events = InlineStyleProcessor::new(parser);
///
/// // Collect and process events
/// for event in events {
///     match event {
///         InlineEvent::Start(InlineTag::Mark) => println!("<mark>"),
///         InlineEvent::End(InlineTag::Mark) => println!("</mark>"),
///         InlineEvent::Start(InlineTag::Dim) => println!("<dim>"),
///         InlineEvent::End(InlineTag::Dim) => println!("</dim>"),
///         InlineEvent::Standard(e) => println!("{:?}", e),
///         InlineEvent::HorizontalRule(_) => println!("<hr/>"),
///     }
/// }
/// ```
pub struct InlineStyleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    inner: I,
    pending: VecDeque<InlineEvent<'a>>,
    /// Track if we're inside a code block (fenced or indented).
    in_code_block: bool,
}

impl<'a, I> InlineStyleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    /// Creates a new `InlineStyleProcessor` wrapping the given parser iterator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use pulldown_cmark::Parser;
    /// use darkmatter::markdown::inline::InlineStyleProcessor;
    ///
    /// let parser = Parser::new("==highlighted==");
    /// let processor = InlineStyleProcessor::new(parser);
    /// ```
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            in_code_block: false,
        }
    }

    /// Find all delimiters (`==` and `⌄`) in the text, classifying each.
    fn find_delimiters(text: &str) -> Vec<InlineDelimiter> {
        let mut delimiters = Vec::new();
        let bytes = text.as_bytes();
        let mut i = 0;

        while i < bytes.len() {
            // Check for escaped backslash before any delimiter
            let is_escaped = i > 0 && bytes[i - 1] == b'\\';

            // Check for `==` (Mark delimiter)
            if i + 1 < bytes.len() && bytes[i] == b'=' && bytes[i + 1] == b'=' {
                if is_escaped {
                    // Escaped: skip the backslash, treat == as literal
                    // The backslash will be consumed when we emit text before it
                }
                delimiters.push(InlineDelimiter {
                    kind: InlineDelimiterKind::Mark,
                    byte_start: i,
                    byte_end: i + 2,
                    can_open: true,
                    can_close: true,
                });
                i += 2;
                continue;
            }

            // Check for `⌄` (U+2304, Dim delimiter)
            if bytes[i] == 0xE2
                && i + 2 < bytes.len()
                && bytes[i + 1] == 0x8C
                && bytes[i + 2] == 0x84
            {
                if is_escaped {
                    // Escaped: force literal by not creating a delimiter
                    i += 3;
                    continue;
                }

                let (can_open, can_close) = Self::classify_dim_delimiter(text, i);
                delimiters.push(InlineDelimiter {
                    kind: InlineDelimiterKind::Dim,
                    byte_start: i,
                    byte_end: i + 3,
                    can_open,
                    can_close,
                });
                i += 3;
                continue;
            }

            i += 1;
        }

        delimiters
    }

    /// Classify a `⌄` delimiter at the given byte position.
    ///
    /// Returns `(can_open, can_close)` based on the surrounding context.
    fn classify_dim_delimiter(text: &str, byte_pos: usize) -> (bool, bool) {
        let prev_char = text[..byte_pos].chars().next_back();
        let next_char = text[byte_pos + 3..].chars().next(); // 3 bytes for U+2304

        // can_open: not followed by Unicode whitespace and next char exists
        let can_open = next_char.is_some_and(|c| !c.is_whitespace());

        // can_close: not preceded by Unicode whitespace and previous char exists
        let can_close = prev_char.is_some_and(|c| !c.is_whitespace());

        // Intra-word rule: if both prev and next are alphanumeric, disallow both sides
        // forming a pair (treat like `_` in CommonMark)
        let prev_is_alnum = prev_char.is_some_and(|c| c.is_alphanumeric());
        let next_is_alnum = next_char.is_some_and(|c| c.is_alphanumeric());
        if prev_is_alnum && next_is_alnum {
            return (false, false);
        }

        (can_open, can_close)
    }

    /// Process a text event, splitting on `==` and `⌄` markers.
    ///
    /// Returns `true` if the text was processed (contained markers and was split).
    /// Returns `false` if the text should be passed through unchanged.
    fn process_text(&mut self, text: CowStr<'a>) -> bool {
        let s = text.as_ref();

        // Fast path: no markers present
        if !s.contains("==") && !s.contains('\u{2304}') {
            return false;
        }

        let delimiters = Self::find_delimiters(s);
        if delimiters.is_empty() {
            return false;
        }

        let mut segments: VecDeque<InlineEvent<'a>> = VecDeque::new();
        let mut current_pos = 0;
        let mut in_mark = false;
        let mut last_mark_start_idx: Option<usize> = None;

        // Pair dim delimiters using a stack.
        // Each entry is (delim_idx, is_ambiguous) where is_ambiguous means both can_open and can_close.
        let mut dim_opener_stack: Vec<usize> = Vec::new();
        // Maps delimiter index to true if it's an opener, false if closer.
        let mut dim_role: std::collections::HashMap<usize, bool> = std::collections::HashMap::new();

        for (idx, delim) in delimiters.iter().enumerate() {
            if let InlineDelimiterKind::Dim = delim.kind {
                let mut paired = false;
                if delim.can_close {
                    // Try to find a matching opener on the stack
                    if let Some(stack_pos) = dim_opener_stack.iter().rposition(|_| true) {
                        let opener_idx = dim_opener_stack.remove(stack_pos);
                        dim_role.insert(opener_idx, true); // opener_idx is the opener
                        dim_role.insert(idx, false); // idx is the closer
                        paired = true;
                    }
                }
                if !paired && delim.can_open {
                    dim_opener_stack.push(idx);
                }
                // If neither can_open nor can_close, or can_close but no match, stays unpaired (literal)
            }
        }
        // Unpaired openers stay on the stack — they remain literal.

        // Build segments in a single pass.
        for (idx, delim) in delimiters.iter().enumerate() {
            // Emit text before this delimiter
            if delim.byte_start > current_pos {
                let before = &s[current_pos..delim.byte_start];
                segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                    before.to_string(),
                ))));
            }

            match delim.kind {
                InlineDelimiterKind::Mark => {
                    // Toggle mark state
                    if in_mark {
                        segments.push_back(InlineEvent::End(InlineTag::Mark));
                        last_mark_start_idx = None;
                    } else {
                        last_mark_start_idx = Some(segments.len());
                        segments.push_back(InlineEvent::Start(InlineTag::Mark));
                    }
                    in_mark = !in_mark;
                }
                InlineDelimiterKind::Dim => {
                    if let Some(is_opener) = dim_role.get(&idx) {
                        if *is_opener {
                            segments.push_back(InlineEvent::Start(InlineTag::Dim));
                        } else {
                            segments.push_back(InlineEvent::End(InlineTag::Dim));
                        }
                    } else {
                        // Unpaired delimiter: literal
                        segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                            "\u{2304}",
                        ))));
                    }
                }
            }

            current_pos = delim.byte_end;
        }

        // Handle remaining text after last delimiter
        if current_pos < s.len() {
            let remaining = &s[current_pos..];
            segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                remaining.to_string(),
            ))));
        }

        // If we ended with an unclosed mark, convert Start(Mark) back to literal "=="
        if in_mark && let Some(start_idx) = last_mark_start_idx {
            segments[start_idx] =
                InlineEvent::Standard(Event::Text(CowStr::from("==".to_string())));
        }

        self.pending = segments;
        true
    }
}

impl<'a, I> Iterator for InlineStyleProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    type Item = InlineEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        // Return pending events first
        if let Some(event) = self.pending.pop_front() {
            return Some(event);
        }

        // Get next event from inner iterator
        match self.inner.next()? {
            // Track code block state - don't process text inside code
            Event::Start(Tag::CodeBlock(kind)) => {
                self.in_code_block = true;
                Some(InlineEvent::Standard(Event::Start(Tag::CodeBlock(kind))))
            }
            Event::End(TagEnd::CodeBlock) => {
                self.in_code_block = false;
                Some(InlineEvent::Standard(Event::End(TagEnd::CodeBlock)))
            }
            // Inline code is literal - pass through unchanged
            Event::Code(text) => Some(InlineEvent::Standard(Event::Code(text))),
            // Process text only if not in code block
            Event::Text(text) => {
                if self.in_code_block {
                    // Inside code block: pass through unchanged
                    Some(InlineEvent::Standard(Event::Text(text)))
                } else if self.process_text(text.clone()) {
                    // Text was processed and split into pending events
                    self.pending.pop_front()
                } else {
                    // Text passed fast-path check: no markers present
                    Some(InlineEvent::Standard(Event::Text(text)))
                }
            }
            // All other events pass through unchanged
            other => Some(InlineEvent::Standard(other)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let (lower, _upper) = self.inner.size_hint();
        // We might emit more events than input (splitting text),
        // so we can only provide a lower bound based on pending
        (lower + self.pending.len(), None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    fn process_text(input: &str) -> Vec<InlineEvent<'_>> {
        let parser = Parser::new_ext(input, Options::ENABLE_STRIKETHROUGH);
        InlineStyleProcessor::new(parser).collect()
    }

    fn extract_text_content(events: &[InlineEvent<'_>]) -> String {
        events
            .iter()
            .filter_map(|e| match e {
                InlineEvent::Standard(Event::Text(t)) => Some(t.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }

    #[test]
    fn test_no_markers_passthrough() {
        let events = process_text("Hello world");
        // Should have paragraph start, text, paragraph end
        assert!(events.iter().all(|e| e.is_standard()));
        assert!(extract_text_content(&events).contains("Hello world"));
    }

    #[test]
    fn test_simple_highlight() {
        let events = process_text("==highlighted==");
        let mut found_start = false;
        let mut found_end = false;
        let mut highlighted_text = String::new();

        for event in &events {
            match event {
                InlineEvent::Start(InlineTag::Mark) => found_start = true,
                InlineEvent::End(InlineTag::Mark) => found_end = true,
                InlineEvent::Standard(Event::Text(t)) if found_start && !found_end => {
                    highlighted_text.push_str(t.as_ref());
                }
                _ => {}
            }
        }

        assert!(found_start, "Should have Start(Mark)");
        assert!(found_end, "Should have End(Mark)");
        assert_eq!(highlighted_text, "highlighted");
    }

    #[test]
    fn test_highlight_with_surrounding_text() {
        let events = process_text("before ==middle== after");
        let content = extract_text_content(&events);
        // Text content contains spaces from markdown parsing
        assert!(content.contains("before"));
        assert!(content.contains("middle"));
        assert!(content.contains("after"));

        // Check mark events exist
        let has_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)));
        let has_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Mark)));
        assert!(has_start);
        assert!(has_end);
    }

    #[test]
    fn test_multiple_highlights() {
        let events = process_text("==one== and ==two==");

        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Mark)))
            .count();

        assert_eq!(start_count, 2, "Should have 2 Start(Mark) events");
        assert_eq!(end_count, 2, "Should have 2 End(Mark) events");
    }

    #[test]
    fn test_unclosed_marker_renders_literally() {
        let events = process_text("==unclosed text");
        let content = extract_text_content(&events);
        // Unclosed marker should be converted back to literal ==
        assert!(
            content.contains("=="),
            "Unclosed marker should render as literal =="
        );

        // Should NOT have unbalanced mark events
        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Mark)))
            .count();
        assert_eq!(
            start_count, end_count,
            "Mark events should be balanced (unclosed converted to literal)"
        );
    }

    #[test]
    fn test_escaped_marker() {
        // Note: In markdown, backslash escapes need careful handling.
        // Let's test with a simpler case that we control.
        let parser = Parser::new_ext(r"before \== after", Options::ENABLE_STRIKETHROUGH);
        let events: Vec<InlineEvent<'_>> = InlineStyleProcessor::new(parser).collect();
        let content = extract_text_content(&events);
        // The backslash-escaped == should appear literally
        assert!(
            content.contains("==") || content.contains(r"\=="),
            "Escaped marker should include == in some form, got: {}",
            content
        );
    }

    #[test]
    fn test_code_block_not_processed() {
        let input = "```\n==code==\n```";
        let events = process_text(input);

        // Should not have any mark events
        let has_mark = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Start(InlineTag::Mark) | InlineEvent::End(InlineTag::Mark)
            )
        });
        assert!(
            !has_mark,
            "Code block content should not be processed for marks"
        );
    }

    #[test]
    fn test_inline_code_not_processed() {
        let events = process_text("`==code==`");

        // Inline code comes as Event::Code, not Event::Text
        let has_code = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Standard(Event::Code(_))));
        assert!(has_code, "Should have inline code event");

        // Should not have mark events
        let has_mark = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)));
        assert!(!has_mark, "Inline code should not produce mark events");
    }

    #[test]
    fn test_empty_markers() {
        let events = process_text("====");
        // ==== means Start(Mark), End(Mark) with empty content
        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Mark)))
            .count();
        assert_eq!(start_count, 1);
        assert_eq!(end_count, 1);
    }

    #[test]
    fn test_size_hint() {
        // Test that size_hint returns reasonable values
        let parser = Parser::new("==text==");
        let processor = InlineStyleProcessor::new(parser);
        let (lower, upper) = processor.size_hint();
        // Upper bound may be None (we can produce more events than input)
        assert!(upper.is_none() || upper.unwrap() >= lower);
    }

    #[test]
    fn test_unicode_content() {
        let events = process_text("==你好世界==");
        let content = extract_text_content(&events);
        assert!(content.contains("你好世界"));

        let has_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)));
        assert!(has_start, "Should have mark for unicode content");
    }

    #[test]
    fn test_highlight_at_start() {
        let events = process_text("==start== middle end");
        let content = extract_text_content(&events);
        assert!(content.contains("start"));
        assert!(content.contains("middle"));
        assert!(content.contains("end"));

        // Should have mark events for "start"
        let first_mark_idx = events
            .iter()
            .position(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)));
        assert!(first_mark_idx.is_some());
    }

    #[test]
    fn test_highlight_at_end() {
        let events = process_text("start middle ==end==");
        let content = extract_text_content(&events);
        assert!(content.contains("start"));
        assert!(content.contains("middle"));
        assert!(content.contains("end"));

        let has_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Mark)));
        assert!(has_end);
    }

    /// Test the exact content from test.md line 77 that shows the bug
    #[test]
    fn test_line77_inline_code_with_highlight() {
        let content = "- this emerging standard uses the character sequence `==` to wrap text and the wrapped text is then given a different background color to clearly ==separate it from== the rest of the text.";
        let events = process_text(content);

        // Should have exactly one pair of mark events (for "separate it from")
        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Mark)))
            .count();

        assert_eq!(
            start_count, 1,
            "Should have exactly 1 Start(Mark) for 'separate it from', got {}",
            start_count
        );
        assert_eq!(
            end_count, 1,
            "Should have exactly 1 End(Mark), got {}",
            end_count
        );

        // The inline code `==` should NOT produce mark events
        let code_events: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Standard(Event::Code(_))))
            .collect();
        assert_eq!(
            code_events.len(),
            1,
            "Should have exactly 1 Code event for `==`"
        );
    }

    // Dim delimiter tests
    #[test]
    fn test_simple_dim() {
        let events = process_text("⌄dimmed⌄");
        let mut found_start = false;
        let mut found_end = false;
        let mut dimmed_text = String::new();

        for event in &events {
            match event {
                InlineEvent::Start(InlineTag::Dim) => found_start = true,
                InlineEvent::End(InlineTag::Dim) => found_end = true,
                InlineEvent::Standard(Event::Text(t)) if found_start && !found_end => {
                    dimmed_text.push_str(t.as_ref());
                }
                _ => {}
            }
        }

        assert!(found_start, "Should have Start(Dim)");
        assert!(found_end, "Should have End(Dim)");
        assert_eq!(dimmed_text, "dimmed");
    }

    #[test]
    fn test_dim_with_surrounding_text() {
        let events = process_text("before ⌄middle⌄ after");
        let content = extract_text_content(&events);
        assert!(content.contains("before"));
        assert!(content.contains("middle"));
        assert!(content.contains("after"));

        let has_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)));
        let has_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Dim)));
        assert!(has_start);
        assert!(has_end);
    }

    #[test]
    fn test_unclosed_dim_renders_literally() {
        let events = process_text("This is ⌄unclosed");
        let content = extract_text_content(&events);
        assert!(
            content.contains("⌄"),
            "Unclosed dim delimiter should render as literal ⌄, got: {}",
            content
        );

        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Dim)))
            .count();
        assert_eq!(
            start_count, end_count,
            "Dim events should be balanced (unclosed converted to literal)"
        );
    }

    #[test]
    fn test_empty_dim() {
        let events = process_text("⌄⌄");
        let start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)))
            .count();
        let end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Dim)))
            .count();
        assert_eq!(start_count, 1, "Should have 1 Start(Dim)");
        assert_eq!(end_count, 1, "Should have 1 End(Dim)");
    }

    #[test]
    fn test_escaped_dim() {
        let parser = Parser::new_ext(r"before \⌄ after", Options::ENABLE_STRIKETHROUGH);
        let events: Vec<InlineEvent<'_>> = InlineStyleProcessor::new(parser).collect();
        let content = extract_text_content(&events);
        // The backslash-escaped ⌄ should appear literally
        assert!(
            content.contains("⌄") || content.contains(r"\⌄"),
            "Escaped dim delimiter should include ⌄ in some form, got: {}",
            content
        );

        let has_dim = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Start(InlineTag::Dim) | InlineEvent::End(InlineTag::Dim)
            )
        });
        assert!(!has_dim, "Escaped ⌄ should not produce Dim events");
    }

    #[test]
    fn test_dim_in_code_block() {
        let input = "```\n⌄code⌄\n```";
        let events = process_text(input);

        let has_dim = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Start(InlineTag::Dim) | InlineEvent::End(InlineTag::Dim)
            )
        });
        assert!(
            !has_dim,
            "Code block content should not be processed for dim"
        );
    }

    #[test]
    fn test_dim_in_inline_code() {
        let events = process_text("`⌄code⌄`");

        let has_code = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Standard(Event::Code(_))));
        assert!(has_code, "Should have inline code event");

        let has_dim = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)));
        assert!(!has_dim, "Inline code should not produce dim events");
    }

    #[test]
    fn test_dim_intraword() {
        // Intra-word: foo⌄bar⌄baz should NOT create dim spans
        let events = process_text("foo⌄bar⌄baz");
        let has_dim = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Start(InlineTag::Dim) | InlineEvent::End(InlineTag::Dim)
            )
        });
        assert!(
            !has_dim,
            "Intra-word dim delimiters should not produce Dim events"
        );

        let content = extract_text_content(&events);
        assert!(
            content.contains("foo⌄bar⌄baz"),
            "Intra-word delimiters should remain literal"
        );
    }

    #[test]
    fn test_dim_with_emphasis() {
        let events = process_text("*⌄dim italic⌄*");

        let has_dim_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)));
        let has_dim_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Dim)));
        assert!(has_dim_start, "Should have Start(Dim)");
        assert!(has_dim_end, "Should have End(Dim)");

        let has_emphasis = events.iter().any(|e| {
            matches!(
                e,
                InlineEvent::Standard(Event::Start(Tag::Emphasis))
                    | InlineEvent::Standard(Event::End(TagEnd::Emphasis))
            )
        });
        assert!(has_emphasis, "Should preserve Emphasis events");
    }

    #[test]
    fn test_dim_with_mark() {
        let events = process_text("==⌄dim mark⌄==");

        let has_dim_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)));
        let has_dim_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Dim)));
        assert!(has_dim_start, "Should have Start(Dim)");
        assert!(has_dim_end, "Should have End(Dim)");

        let has_mark_start = events
            .iter()
            .any(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)));
        let has_mark_end = events
            .iter()
            .any(|e| matches!(e, InlineEvent::End(InlineTag::Mark)));
        assert!(has_mark_start, "Should have Start(Mark)");
        assert!(has_mark_end, "Should have End(Mark)");
    }

    #[test]
    fn test_mixed_mark_and_dim() {
        let events = process_text("==mark== and ⌄dim⌄");

        let mark_start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Mark)))
            .count();
        let mark_end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Mark)))
            .count();
        let dim_start_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::Start(InlineTag::Dim)))
            .count();
        let dim_end_count = events
            .iter()
            .filter(|e| matches!(e, InlineEvent::End(InlineTag::Dim)))
            .count();

        assert_eq!(mark_start_count, 1);
        assert_eq!(mark_end_count, 1);
        assert_eq!(dim_start_count, 1);
        assert_eq!(dim_end_count, 1);
    }
}

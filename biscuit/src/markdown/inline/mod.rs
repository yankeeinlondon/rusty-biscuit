//! Custom inline markdown extensions via iterator adapters.
//!
//! This module provides support for inline markdown syntax that is not natively
//! supported by pulldown-cmark, such as `==highlighted text==`.
//!
//! ## Overview
//!
//! The main component is [`MarkProcessor`], an iterator adapter that wraps a
//! pulldown-cmark parser and intercepts `Event::Text` events to process custom
//! inline syntax. It emits [`InlineEvent`]s which can be either standard
//! pulldown-cmark events or custom inline tag events.
//!
//! ## Supported Syntax
//!
//! | Syntax | Meaning | HTML Output |
//! |--------|---------|-------------|
//! | `==text==` | Highlighted text | `<mark>text</mark>` |
//!
//! ## Examples
//!
//! ```
//! use pulldown_cmark::{Parser, Options};
//! use shared::markdown::inline::{MarkProcessor, InlineEvent, InlineTag};
//!
//! let content = "This is ==highlighted== text.";
//! let parser = Parser::new_ext(content, Options::ENABLE_STRIKETHROUGH);
//! let events: Vec<_> = MarkProcessor::new(parser).collect();
//!
//! // Events will include Start(Mark), Text("highlighted"), End(Mark)
//! ```
//!
//! ## Design
//!
//! The processor uses a fast-path optimization: if a text event doesn't contain
//! the `==` delimiter, it passes through unchanged with zero additional allocations.
//! Only text containing markers is processed and split into multiple events.
//!
//! The processor also handles:
//! - Unclosed markers: `==text` renders as literal `==text`
//! - Escaped markers: `\==` renders as literal `==`
//! - Code blocks: `==` inside code is not processed (literal)

mod types;

pub use types::{InlineEvent, InlineTag};

use pulldown_cmark::{CowStr, Event, Tag, TagEnd};
use std::collections::VecDeque;

/// Iterator adapter that processes text events for custom inline syntax.
///
/// `MarkProcessor` wraps a pulldown-cmark parser iterator and transforms
/// `Event::Text` events that contain `==` markers into sequences of
/// `InlineEvent::Start(Mark)`, `InlineEvent::Standard(Text)`, and
/// `InlineEvent::End(Mark)` events.
///
/// ## Fast Path
///
/// For text that doesn't contain `==`, the processor returns events unchanged
/// with zero additional allocations, ensuring minimal overhead for documents
/// that don't use the highlight syntax.
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
/// If a `==` marker is opened but not closed within the same text event,
/// the opening marker is converted back to literal text to prevent
/// invalid event sequences.
///
/// ## Examples
///
/// ```
/// use pulldown_cmark::Parser;
/// use shared::markdown::inline::{MarkProcessor, InlineEvent, InlineTag};
///
/// let parser = Parser::new("Hello ==world==!");
/// let mut events = MarkProcessor::new(parser);
///
/// // Collect and process events
/// for event in events {
///     match event {
///         InlineEvent::Start(InlineTag::Mark) => println!("<mark>"),
///         InlineEvent::End(InlineTag::Mark) => println!("</mark>"),
///         InlineEvent::Standard(e) => println!("{:?}", e),
///     }
/// }
/// ```
pub struct MarkProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    inner: I,
    pending: VecDeque<InlineEvent<'a>>,
    /// Track if we're inside a code block (fenced or indented).
    in_code_block: bool,
}

impl<'a, I> MarkProcessor<'a, I>
where
    I: Iterator<Item = Event<'a>>,
{
    /// Creates a new `MarkProcessor` wrapping the given parser iterator.
    ///
    /// ## Examples
    ///
    /// ```
    /// use pulldown_cmark::Parser;
    /// use shared::markdown::inline::MarkProcessor;
    ///
    /// let parser = Parser::new("==highlighted==");
    /// let processor = MarkProcessor::new(parser);
    /// ```
    pub fn new(inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            in_code_block: false,
        }
    }

    /// Process a text event, splitting on `==` markers.
    ///
    /// Returns `true` if the text was processed (contained markers and was split).
    /// Returns `false` if the text should be passed through unchanged.
    fn process_text(&mut self, text: CowStr<'a>) -> bool {
        let s = text.as_ref();

        // Fast path: no markers present
        if !s.contains("==") {
            return false;
        }

        let mut segments: VecDeque<InlineEvent<'a>> = VecDeque::new();
        let mut current_pos = 0;
        let mut in_mark = false;
        let mut last_start_idx: Option<usize> = None;

        while let Some(marker_pos) = s[current_pos..].find("==") {
            let abs_pos = current_pos + marker_pos;

            // Check for escape sequence (safely handle multi-byte chars)
            let is_escaped = abs_pos > 0 && s[..abs_pos].ends_with('\\');

            if is_escaped {
                // Escaped marker: emit text before backslash, then "==" literally
                let before_backslash = abs_pos - 1; // Safe: checked abs_pos > 0
                if before_backslash > current_pos {
                    let before = &s[current_pos..before_backslash];
                    segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                        before.to_string(),
                    ))));
                }
                segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                    "==".to_string(),
                ))));
                current_pos = abs_pos + 2;
                continue;
            }

            // Emit text before marker
            if abs_pos > current_pos {
                let before = &s[current_pos..abs_pos];
                segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                    before.to_string(),
                ))));
            }

            // Toggle mark state
            if in_mark {
                segments.push_back(InlineEvent::End(InlineTag::Mark));
                last_start_idx = None; // Paired, clear tracking
            } else {
                last_start_idx = Some(segments.len()); // Track this Start position
                segments.push_back(InlineEvent::Start(InlineTag::Mark));
            }
            in_mark = !in_mark;
            current_pos = abs_pos + 2;
        }

        // Handle remaining text after last marker
        if current_pos < s.len() {
            let remaining = &s[current_pos..];
            segments.push_back(InlineEvent::Standard(Event::Text(CowStr::from(
                remaining.to_string(),
            ))));
        }

        // If we ended with an unclosed mark, convert Start(Mark) back to literal "=="
        if in_mark && let Some(start_idx) = last_start_idx {
            segments[start_idx] =
                InlineEvent::Standard(Event::Text(CowStr::from("==".to_string())));
        }

        self.pending = segments;
        true
    }
}

impl<'a, I> Iterator for MarkProcessor<'a, I>
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
        MarkProcessor::new(parser).collect()
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
        let events: Vec<InlineEvent<'_>> = MarkProcessor::new(parser).collect();
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
        let processor = MarkProcessor::new(parser);
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

        // Debug: print all events
        eprintln!("Events:");
        for (i, e) in events.iter().enumerate() {
            eprintln!("[{}] {:?}", i, e);
        }

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
}

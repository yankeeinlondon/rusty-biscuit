//! Offset-aware block-extension processor for the render-tree fold.
//!
//! This module sits between `pulldown_cmark::Parser::into_offset_iter()` and
//! the inline-span dispatcher. It recognises darkmatter's block-level
//! extensions and emits a unified [`BlockExtensionEvent`] stream that the
//! fold can consume.
//!
//! Today the only block extension implemented here is the **horizontal-rule
//! attribute paragraph** — a simple paragraph whose body matches
//! `---|***|___ { ... }` is rewritten into a single
//! [`BlockExtensionEvent::HorizontalRule`] over the original text-event byte
//! range. Future block extensions will require their own spec; see
//! `renderable/features/2026-05-26-block-extension/spec.md`.
//!
//! ## Why a separate processor
//!
//! HR-attribute syntax is a whole-paragraph construct, but the previous
//! implementation lived inside the span-aware inline transport
//! (`render_tree::span::SpannedRuleProcessor`). That coupled the HR path to
//! `SpannedInlineEvent`, blocking the inline-span replacement. Lifting the
//! processor up to the offset-event layer lets HR-attribute handling run
//! before inline extension logic and removes the dependency.
//!
//! ## Range Policy
//!
//! - **Standard events** keep `pulldown-cmark`'s offset range verbatim.
//! - **Generated HR events** point their `body_range` at the buffered
//!   `Event::Text` byte range — *not* the `End(Paragraph)` range, which
//!   `pulldown-cmark` may extend to include a trailing newline. This matches
//!   the legacy `render_tree::span::SpannedRuleProcessor` policy (now
//!   retired) so generated provenance stays byte-identical.

use std::collections::VecDeque;
use std::ops::Range;

use pulldown_cmark::{Event, Tag, TagEnd};

use crate::markdown::block::{matches_horizontal_rule_pattern, parse_hr_attribute_block};
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::style::warning::StyleWarning;

/// Single item produced by [`BlockExtensionProcessor`].
///
/// `Standard` wraps a parser event verbatim along with its byte range.
/// `HorizontalRule` is the synthetic event emitted when a simple paragraph
/// matched the HR-attribute pattern; `body_range` covers the original
/// `Event::Text` bytes (the paragraph body), excluding any trailing newline
/// `pulldown-cmark` may include in the `End(Paragraph)` range.
#[derive(Debug, Clone)]
pub(crate) enum BlockExtensionEvent<'a> {
    /// Pass-through event with its parser byte range.
    Standard(Event<'a>, Range<usize>),
    /// Synthetic horizontal rule lifted from a simple HR-attribute paragraph.
    HorizontalRule {
        /// Attributes parsed from the `{ ... }` block.
        attrs: HorizontalRuleAttrs,
        /// Byte range of the paragraph body that produced this event.
        body_range: Range<usize>,
    },
}

/// State of the [`BlockExtensionProcessor`].
enum State<'a> {
    /// Outside any paragraph; events stream through unchanged.
    Idle,
    /// Inside an open paragraph. Events accumulate in `buffer` until the
    /// matching `End(Paragraph)` arrives. `paragraph_start` stores the byte
    /// range of the original `Start(Paragraph)` so the processor can replay
    /// it verbatim if the paragraph does not match the HR pattern.
    BufferingParagraph {
        /// Range of the `Start(Paragraph)` event that opened this state.
        paragraph_start: Range<usize>,
        /// Buffered `(Event, Range)` pairs from inside the paragraph.
        buffer: Vec<(Event<'a>, Range<usize>)>,
        /// `true` while every buffered event is `Event::Text`. Any other
        /// inline event flips this to `false`, mirroring the legacy
        /// `paragraph_is_simple` flag from
        /// [`crate::markdown::block::RuleProcessor`].
        simple: bool,
    },
}

/// Iterator adapter that consumes `pulldown_cmark`'s offset-event stream and
/// emits [`BlockExtensionEvent`]s.
///
/// The processor is a small state machine that buffers `Paragraph` regions,
/// runs the simple-paragraph HR-attribute matcher on close, and either emits
/// a synthetic [`BlockExtensionEvent::HorizontalRule`] or flushes the
/// buffered paragraph verbatim.
pub(crate) struct BlockExtensionProcessor<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    inner: I,
    pending: VecDeque<BlockExtensionEvent<'a>>,
    state: State<'a>,
    /// `StyleWarning`s accumulated while parsing matched HR-attribute blocks.
    /// Phase 3 will surface these through the fold's diagnostic channel; for
    /// now they are stored so callers can opt in to a parity check.
    warnings: Vec<StyleWarning>,
}

impl<'a, I> BlockExtensionProcessor<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    /// Wraps `inner` (typically the result of
    /// [`pulldown_cmark::Parser::into_offset_iter`]).
    pub(crate) fn new(inner: I) -> Self {
        Self {
            inner,
            pending: VecDeque::new(),
            state: State::Idle,
            warnings: Vec::new(),
        }
    }

    /// Returns the deprecation/style warnings produced by every matched HR
    /// attribute block this processor has emitted so far.
    #[allow(dead_code)]
    pub(crate) fn warnings(&self) -> &[StyleWarning] {
        &self.warnings
    }

    /// Closes the open paragraph: either fires the HR rewrite or replays the
    /// buffered paragraph verbatim. `end_range` is the `End(Paragraph)` byte
    /// range from `pulldown-cmark`.
    fn close_paragraph(&mut self, end_range: Range<usize>) {
        let State::BufferingParagraph {
            paragraph_start,
            buffer,
            simple,
        } = std::mem::replace(&mut self.state, State::Idle)
        else {
            // Defensive: only invoked while buffering.
            return;
        };

        // HR rewrite only fires for simple single-text paragraphs that match
        // the HR-attribute pattern. Anything else falls through to a verbatim
        // flush so paragraphs with emphasis, links, code, or inline darkmatter
        // tokens are never rewritten.
        if simple
            && buffer.len() == 1
            && let (Event::Text(text), text_range) = &buffer[0]
            && let Some((_, attribute_str)) = matches_horizontal_rule_pattern(text.as_ref())
        {
            let body_range = text_range.clone();
            let result = parse_hr_attribute_block(&attribute_str);
            self.warnings.extend(result.warnings);
            self.pending.push_back(BlockExtensionEvent::HorizontalRule {
                attrs: result.attrs,
                body_range,
            });
            return;
        }

        // Unmatched: replay the paragraph wrapper plus its buffered children.
        self.pending.push_back(BlockExtensionEvent::Standard(
            Event::Start(Tag::Paragraph),
            paragraph_start,
        ));
        for (event, range) in buffer {
            self.pending
                .push_back(BlockExtensionEvent::Standard(event, range));
        }
        self.pending.push_back(BlockExtensionEvent::Standard(
            Event::End(TagEnd::Paragraph),
            end_range,
        ));
    }
}

impl<'a, I> Iterator for BlockExtensionProcessor<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    type Item = BlockExtensionEvent<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(event);
            }

            let Some((event, range)) = self.inner.next() else {
                // Underlying stream is exhausted. If we are still buffering a
                // paragraph (unbalanced parser output), flush it verbatim so
                // no events are lost.
                if let State::BufferingParagraph {
                    paragraph_start,
                    buffer,
                    ..
                } = std::mem::replace(&mut self.state, State::Idle)
                {
                    self.pending.push_back(BlockExtensionEvent::Standard(
                        Event::Start(Tag::Paragraph),
                        paragraph_start,
                    ));
                    for (event, range) in buffer {
                        self.pending
                            .push_back(BlockExtensionEvent::Standard(event, range));
                    }
                    continue;
                }
                return None;
            };

            match (&mut self.state, &event) {
                (State::Idle, Event::Start(Tag::Paragraph)) => {
                    self.state = State::BufferingParagraph {
                        paragraph_start: range,
                        buffer: Vec::new(),
                        simple: true,
                    };
                }
                (State::BufferingParagraph { .. }, Event::End(TagEnd::Paragraph)) => {
                    self.close_paragraph(range);
                }
                (
                    State::BufferingParagraph { buffer, simple, .. },
                    _,
                ) => {
                    if !matches!(event, Event::Text(_)) {
                        *simple = false;
                    }
                    buffer.push((event, range));
                }
                (State::Idle, _) => {
                    return Some(BlockExtensionEvent::Standard(event, range));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    /// Drains the processor over `markdown` into a flat vector.
    fn run(markdown: &str) -> Vec<BlockExtensionEvent<'_>> {
        let parser = Parser::new_ext(
            markdown,
            Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
        )
        .into_offset_iter();
        BlockExtensionProcessor::new(parser).collect()
    }

    fn count_hr(events: &[BlockExtensionEvent<'_>]) -> usize {
        events
            .iter()
            .filter(|e| matches!(e, BlockExtensionEvent::HorizontalRule { .. }))
            .count()
    }

    fn first_hr<'a>(
        events: &'a [BlockExtensionEvent<'a>],
    ) -> Option<(&'a HorizontalRuleAttrs, &'a Range<usize>)> {
        events.iter().find_map(|e| match e {
            BlockExtensionEvent::HorizontalRule { attrs, body_range } => Some((attrs, body_range)),
            _ => None,
        })
    }

    #[test]
    fn matched_kind_emits_horizontal_rule() {
        let events = run("--- { kind: waves }");
        assert_eq!(count_hr(&events), 1, "events: {events:?}");
        let (attrs, _body_range) = first_hr(&events).expect("HR event");
        assert_eq!(attrs.kind, Some("waves".to_string()));
        assert_eq!(attrs.legacy_style, None);
    }

    #[test]
    fn matched_multiple_attrs_with_star_markers() {
        let events = run("*** { kind: dots, weight: thick, color: blue }");
        assert_eq!(count_hr(&events), 1);
        let (attrs, _) = first_hr(&events).unwrap();
        assert_eq!(attrs.kind, Some("dots".to_string()));
        assert_eq!(attrs.weight, Some("thick".to_string()));
        assert_eq!(attrs.color, Some("blue".to_string()));
    }

    #[test]
    fn bare_dashes_pass_through_as_standard_rule() {
        // `---` on its own is `Event::Rule`, not a paragraph. The processor
        // must forward it as `Standard(Event::Rule, range)`.
        let events = run("---");
        assert_eq!(count_hr(&events), 0);
        let rule_count = events
            .iter()
            .filter(|e| matches!(e, BlockExtensionEvent::Standard(Event::Rule, _)))
            .count();
        assert_eq!(rule_count, 1, "expected one Standard(Rule, _): {events:?}");
    }

    #[test]
    fn regular_paragraph_passes_through() {
        let events = run("This is a regular paragraph.");
        assert_eq!(count_hr(&events), 0);
        assert!(matches!(
            events.first(),
            Some(BlockExtensionEvent::Standard(
                Event::Start(Tag::Paragraph),
                _
            )),
        ));
        assert!(matches!(
            events.last(),
            Some(BlockExtensionEvent::Standard(
                Event::End(TagEnd::Paragraph),
                _
            )),
        ));
    }

    #[test]
    fn paragraph_with_bold_is_not_rewritten() {
        let events = run("This is **bold** text.");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn paragraph_with_inline_code_is_not_rewritten() {
        let events = run("text with `code` inside");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn paragraph_with_emphasis_is_not_rewritten() {
        let events = run("text with *emphasis* inside");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn insufficient_markers_not_rewritten() {
        // `--` is only two markers; the matcher requires at least three.
        // pulldown-cmark emits this as a paragraph with text — it must flush
        // through unchanged.
        let events = run("-- { kind: waves }");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn mixed_markers_not_rewritten() {
        let events = run("-** { kind: waves }");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn mixed_dash_star_dash_not_rewritten() {
        let events = run("-*- { kind: dots }");
        assert_eq!(count_hr(&events), 0);
    }

    #[test]
    fn fenced_code_block_containing_hr_attribute_is_not_rewritten() {
        // pulldown-cmark wraps fenced code in CodeBlock tags, never as a
        // top-level paragraph, so the simple-paragraph matcher is naturally
        // safe. The regression test guards against future regressions.
        let input = "```\n--- { kind: waves }\n```\n";
        let events = run(input);
        assert_eq!(count_hr(&events), 0, "events: {events:?}");

        let has_code_start = events.iter().any(|e| {
            matches!(
                e,
                BlockExtensionEvent::Standard(Event::Start(Tag::CodeBlock(_)), _)
            )
        });
        let has_code_end = events.iter().any(|e| {
            matches!(
                e,
                BlockExtensionEvent::Standard(Event::End(TagEnd::CodeBlock), _)
            )
        });
        assert!(has_code_start, "expected CodeBlock start: {events:?}");
        assert!(has_code_end, "expected CodeBlock end: {events:?}");
    }

    #[test]
    fn blockquote_wrapped_hr_attribute_is_matched() {
        // pulldown-cmark emits a Paragraph inside the BlockQuote; the
        // processor must rewrite it just like a top-level paragraph. The HR
        // remains structurally wrapped by the BlockQuote start/end events.
        let events = run("> --- { kind: waves }");
        assert_eq!(count_hr(&events), 1, "events: {events:?}");

        let bq_start = events.iter().position(|e| {
            matches!(
                e,
                BlockExtensionEvent::Standard(Event::Start(Tag::BlockQuote(_)), _)
            )
        });
        let bq_end = events.iter().position(|e| {
            matches!(
                e,
                BlockExtensionEvent::Standard(Event::End(TagEnd::BlockQuote(_)), _)
            )
        });
        assert!(bq_start.is_some(), "expected BlockQuote start: {events:?}");
        assert!(bq_end.is_some(), "expected BlockQuote end: {events:?}");
    }

    #[test]
    fn list_item_wrapped_hr_attribute_is_not_matched() {
        // `- --- { kind: dots }` is a list item. The HR pattern is parsed by
        // pulldown-cmark as the item's content; the resulting event stream
        // must not contain a synthetic HR event. (The exact pulldown-cmark
        // structure for this input is delegated to the parser; the assertion
        // we care about is that no HR rewrite happens.)
        let events = run("- --- { kind: dots }");
        assert_eq!(count_hr(&events), 0, "events: {events:?}");
        let has_list = events.iter().any(|e| {
            matches!(
                e,
                BlockExtensionEvent::Standard(Event::Start(Tag::List(_)), _)
                    | BlockExtensionEvent::Standard(Event::Start(Tag::Item), _)
            )
        });
        assert!(has_list, "expected List/Item tags in output: {events:?}");
    }

    #[test]
    fn malformed_attribute_block_falls_back_gracefully() {
        // `{ kind: }` is not valid YAML (missing value). The shared parser
        // falls back to the legacy splitter (which also cannot extract any
        // pair from this body) and produces default attrs. The processor
        // must still emit a single HR event — no panic.
        let events = run("--- { kind: }");
        assert_eq!(count_hr(&events), 1);
        let (attrs, _) = first_hr(&events).unwrap();
        assert_eq!(attrs.kind, None);
        assert_eq!(attrs.legacy_style, None);
    }

    #[test]
    fn body_range_points_at_text_event_not_paragraph_end() {
        // The synthetic event's body range must cover the original text bytes,
        // not the `End(Paragraph)` range that may include a trailing newline.
        let src = "--- { kind: waves }\n";
        let events = run(src);
        assert_eq!(count_hr(&events), 1);
        let (_, body_range) = first_hr(&events).unwrap();

        // The text event range covers the literal source string without the
        // trailing newline. pulldown-cmark's End(Paragraph) range typically
        // includes the newline, so the two must differ for a source that has
        // one.
        let body_slice = &src[body_range.clone()];
        assert_eq!(body_slice, "--- { kind: waves }");
        assert!(
            !body_slice.ends_with('\n'),
            "body_range must not include the trailing paragraph newline: {body_slice:?}"
        );
    }

    #[test]
    fn standard_event_ranges_match_parser_output() {
        // Sanity: a regular paragraph's Standard events should carry the
        // exact ranges pulldown-cmark produces. Compare against a direct
        // offset-iter collection.
        let src = "hello world";
        let direct: Vec<_> = Parser::new(src).into_offset_iter().collect();
        let processed = run(src);

        assert_eq!(direct.len(), processed.len());
        for ((d_event, d_range), p) in direct.iter().zip(processed.iter()) {
            match p {
                BlockExtensionEvent::Standard(p_event, p_range) => {
                    assert_eq!(p_range, d_range);
                    // Discriminant comparison is enough; the Cow content
                    // already comes from the same parser instance lifetime.
                    assert_eq!(
                        std::mem::discriminant(p_event),
                        std::mem::discriminant(d_event)
                    );
                }
                other => panic!("expected Standard event, got {other:?}"),
            }
        }
    }

    #[test]
    fn legacy_style_records_deprecation_warning() {
        // Even though the HR event itself only carries `attrs`, the processor
        // must collect a deprecation `StyleWarning` for legacy `style` so
        // Phase 3 can surface it through the fold's diagnostic channel.
        let parser = Parser::new("--- { style: waves }").into_offset_iter();
        let mut processor = BlockExtensionProcessor::new(parser);
        let events: Vec<_> = processor.by_ref().collect();
        assert_eq!(count_hr(&events), 1);
        let (attrs, _) = first_hr(&events).unwrap();
        assert_eq!(attrs.legacy_style, Some("waves".to_string()));
        assert_eq!(processor.warnings().len(), 1);
        assert_eq!(processor.warnings()[0].path, "hr.inline.style");
    }
}

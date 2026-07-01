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
//! HR-attribute syntax is a whole-paragraph construct, but an earlier
//! implementation lived inside a span-aware inline transport that coupled the
//! HR path to per-event inline transport types, blocking the inline-span
//! replacement. Lifting the processor up to the offset-event layer lets
//! HR-attribute handling run before inline extension logic and removes the
//! dependency.
//!
//! ## Range Policy
//!
//! - **Standard events** keep `pulldown-cmark`'s offset range verbatim.
//! - **Generated HR events** point their `body_range` at the buffered
//!   `Event::Text` byte range — *not* the `End(Paragraph)` range, which
//!   `pulldown-cmark` may extend to include a trailing newline. This matches
//!   the retired span-aware HR policy so generated provenance stays
//!   byte-identical.

use std::collections::VecDeque;
use std::ops::Range;

use pulldown_cmark::{CowStr, Event, Tag, TagEnd};

use crate::markdown::block::{matches_horizontal_rule_pattern, parse_hr_attribute_block};
use crate::markdown::inline::HorizontalRuleAttrs;
use crate::markdown::render_tree::disclosure_style::parse_disclosure_opener_style;
use crate::markdown::MarkdownError;
use crate::style::schema::CommonStyle;

/// Single item produced by [`BlockExtensionProcessor`].
///
/// `Standard` wraps a parser event verbatim along with its byte range.
/// `HorizontalRule` is the synthetic event emitted when a simple paragraph
/// matched the HR-attribute pattern; `body_range` covers the original
/// `Event::Text` bytes (the paragraph body), excluding any trailing newline
/// `pulldown-cmark` may include in the `End(Paragraph)` range.
#[derive(Debug, Clone)]
#[allow(clippy::large_enum_variant)]
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
    /// Synthetic disclosure block lifted from a
    /// `::disclosure / ::details / ::end-disclosure` triple.
    Disclosure {
        /// Events forming the summary region (phrasing content only).
        summary_events: Vec<(Event<'a>, Range<usize>)>,
        /// Events forming the disclosed body region.
        body_events: Vec<(Event<'a>, Range<usize>)>,
        /// Inline style parsed from `::disclosure key=value ...` tokens.
        inline_style: Option<CommonStyle>,
        /// Byte range spanning the entire disclosure block.
        range: Range<usize>,
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
        /// inline event flips this to `false`, mirroring the `paragraph_is_simple`
        /// flag from the now-deleted legacy `RuleProcessor` iterator adapter.
        simple: bool,
    },
    /// Collecting events between `::disclosure` and the matching
    /// `::end-disclosure`.
    CollectingDisclosure {
        /// Byte range of the paragraph that opened the disclosure block.
        start: Range<usize>,
        /// Events belonging to the summary region.
        summary_events: Vec<(Event<'a>, Range<usize>)>,
        /// Events belonging to the disclosed body region.
        body_events: Vec<(Event<'a>, Range<usize>)>,
        /// Inline style parsed from the opener line.
        inline_style: Option<CommonStyle>,
        /// `true` until `::details` is seen.
        in_summary: bool,
        /// `true` once the summary has received non-whitespace inline content.
        summary_has_content: bool,
        /// Number of nested `::disclosure` openers seen in the body. The body
        /// closer only matches when this is zero.
        body_depth: usize,
    },
    /// The `::end-disclosure` marker has been seen; the next `End(Paragraph)`
    /// finalizes the block.
    ClosingDisclosure {
        /// Byte range of the paragraph that opened the disclosure block.
        start: Range<usize>,
        /// Events belonging to the summary region.
        summary_events: Vec<(Event<'a>, Range<usize>)>,
        /// Events belonging to the disclosed body region.
        body_events: Vec<(Event<'a>, Range<usize>)>,
        /// Inline style parsed from the opener line.
        inline_style: Option<CommonStyle>,
        /// `true` once the summary has received non-whitespace inline content.
        summary_has_content: bool,
    },
}

/// Opening marker for a render-time disclosure block.
const DISCLOSURE_OPENER: &str = "::disclosure";
/// Separator between summary and body in a disclosure block.
const DISCLOSURE_DETAILS: &str = "::details";
/// Closing marker for a render-time disclosure block.
const DISCLOSURE_CLOSER: &str = "::end-disclosure";

/// Builds a [`MarkdownError::MalformedDisclosure`] with a stable range.
fn malformed_disclosure(reason: impl Into<String>, range: Range<usize>) -> MarkdownError {
    MarkdownError::MalformedDisclosure {
        reason: reason.into(),
        range,
    }
}

/// Iterator adapter that consumes `pulldown_cmark`'s offset-event stream and
/// emits [`BlockExtensionEvent`]s.
///
/// The processor is a small state machine that buffers `Paragraph` regions,
/// runs the simple-paragraph HR-attribute matcher on close, and either emits
/// a synthetic [`BlockExtensionEvent::HorizontalRule`] or flushes the
/// buffered paragraph verbatim.
///
/// ## Warnings
///
/// This processor carries only parsed attributes, not warnings. HR
/// deprecation/style warnings (e.g. the legacy `style` key) reach callers
/// solely through the
/// [`scan_inline_hr_warnings`](crate::markdown::block::scan_inline_hr_warnings)
/// preflight, which shares the same
/// [`parse_hr_attribute_block`] parser. The render-tree fold does not surface
/// style warnings; exposing them through the tree entry points is a separate
/// public-entrypoint decision (see the block-extension spec's "Warning
/// Surface" section).
pub(crate) struct BlockExtensionProcessor<'a, I>
where
    I: Iterator<Item = (Event<'a>, Range<usize>)>,
{
    inner: I,
    pending: VecDeque<BlockExtensionEvent<'a>>,
    state: State<'a>,
    /// Tracks nested fenced/indented code-block depth so directive lines inside
    /// code blocks are never interpreted as disclosure markers.
    code_block_depth: usize,
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
            code_block_depth: 0,
        }
    }

    // ── disclosure keyword matching ────────────────────────────────────────

    /// `true` if `text` is the `::end-disclosure` closer (ignoring trailing
    /// ASCII whitespace) with no other non-whitespace content.
    fn is_closer(text: &str) -> bool {
        text.trim_end() == DISCLOSURE_CLOSER
    }

    /// Matches a directive keyword at the start of `text` followed by ASCII
    /// whitespace or end-of-line. Returns the remainder of the line after the
    /// keyword and its trailing whitespace, if any.
    fn match_keyword<'b>(text: &'b str, keyword: &str) -> Option<&'b str> {
        let rest = text.strip_prefix(keyword)?;
        if rest.is_empty() {
            return Some(rest);
        }
        let mut chars = rest.chars();
        match chars.next() {
            Some(c) if c.is_ascii_whitespace() => Some(chars.as_str()),
            _ => None,
        }
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
            // Only the parsed attributes are carried forward; style/deprecation
            // warnings are surfaced by the `scan_inline_hr_warnings` preflight,
            // which parses through the same `parse_hr_attribute_block` helper.
            let attrs = parse_hr_attribute_block(&attribute_str).attrs;
            self.pending.push_back(BlockExtensionEvent::HorizontalRule {
                attrs,
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
    type Item = Result<BlockExtensionEvent<'a>, MarkdownError>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                return Some(Ok(event));
            }

            let Some((event, range)) = self.inner.next() else {
                match std::mem::replace(&mut self.state, State::Idle) {
                    State::BufferingParagraph {
                        paragraph_start,
                        buffer,
                        ..
                    } => {
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
                    State::CollectingDisclosure { start, .. }
                    | State::ClosingDisclosure { start, .. } => {
                        return Some(Err(malformed_disclosure(
                            "disclosure block is missing `::end-disclosure`",
                            start.start..start.end,
                        )));
                    }
                    State::Idle => return None,
                }
            };

            // Track fenced/indented code blocks so directive lines inside them
            // are treated as literal content.
            match &event {
                Event::Start(Tag::CodeBlock(_)) => self.code_block_depth += 1,
                Event::End(TagEnd::CodeBlock) if self.code_block_depth > 0 => {
                    self.code_block_depth -= 1;
                }
                _ => {}
            }

            match std::mem::replace(&mut self.state, State::Idle) {
                State::Idle => match &event {
                    Event::Start(Tag::Paragraph) => {
                        self.state = State::BufferingParagraph {
                            paragraph_start: range,
                            buffer: Vec::new(),
                            simple: true,
                        };
                    }
                    _ => return Some(Ok(BlockExtensionEvent::Standard(event, range))),
                },
                State::BufferingParagraph {
                    paragraph_start,
                    mut buffer,
                    mut simple,
                } => match &event {
                    Event::End(TagEnd::Paragraph) => {
                        self.state = State::BufferingParagraph {
                            paragraph_start,
                            buffer,
                            simple,
                        };
                        self.close_paragraph(range);
                    }
                    Event::Text(text)
                        if self.code_block_depth == 0
                            && buffer.is_empty()
                            && let Some(rest) = Self::match_keyword(text.as_ref(), DISCLOSURE_OPENER) =>
                    {
                        let (inline_style, summary_text) = parse_disclosure_opener_style(rest);
                        let mut summary_events = Vec::new();
                        let mut summary_has_content = false;
                        if let Some(summary) = summary_text {
                            let prefix_len = text.len() - rest.len();
                            let summary_start = range.start + prefix_len
                                + rest.len().saturating_sub(summary.len());
                            summary_events.push((
                                Event::Text(CowStr::from(summary)),
                                summary_start..range.end,
                            ));
                            summary_has_content = true;
                        }
                        self.state = State::CollectingDisclosure {
                            start: paragraph_start,
                            summary_events,
                            body_events: Vec::new(),
                            inline_style,
                            in_summary: true,
                            summary_has_content,
                            body_depth: 0,
                        };
                    }
                    _ => {
                        if !matches!(event, Event::Text(_)) {
                            simple = false;
                        }
                        buffer.push((event, range));
                        self.state = State::BufferingParagraph {
                            paragraph_start,
                            buffer,
                            simple,
                        };
                    }
                },
                State::CollectingDisclosure {
                    start,
                    mut summary_events,
                    mut body_events,
                    inline_style,
                    in_summary,
                    mut summary_has_content,
                    mut body_depth,
                } => {
                    if self.code_block_depth > 0 {
                        if in_summary {
                            summary_has_content |= is_summary_content(&event);
                            summary_events.push((event, range));
                        } else {
                            body_events.push((event, range));
                        }
                        self.state = State::CollectingDisclosure {
                            start,
                            summary_events,
                            body_events,
                            inline_style,
                            in_summary,
                            summary_has_content,
                            body_depth,
                        };
                        continue;
                    }

                    if in_summary {
                        match &event {
                            Event::HardBreak => {
                                return Some(Err(malformed_disclosure(
                                    "summary region contains a hard line break",
                                    start.start..range.end,
                                )));
                            }
                            Event::End(TagEnd::Paragraph) => {
                                return Some(Err(malformed_disclosure(
                                    "summary region contains a paragraph break or block-level element",
                                    start.start..range.end,
                                )));
                            }
                            Event::Start(
                                Tag::Heading { .. }
                                | Tag::BlockQuote(_)
                                | Tag::CodeBlock(_)
                                | Tag::List(_)
                                | Tag::Item
                                | Tag::Table(_)
                                | Tag::HtmlBlock
                                | Tag::FootnoteDefinition(_)
                                | Tag::DefinitionList
                                | Tag::DefinitionListTitle
                                | Tag::DefinitionListDefinition
                                | Tag::MetadataBlock(_),
                            )
                            | Event::Rule
                            | Event::TaskListMarker(_) => {
                                return Some(Err(malformed_disclosure(
                                    "summary region contains a block-level element",
                                    start.start..range.end,
                                )));
                            }
                            _ => {}
                        }
                    }

                    match &event {
                        Event::Text(text) if in_summary => {
                            if let Some(rest) = Self::match_keyword(text.as_ref(), DISCLOSURE_DETAILS) {
                                if !rest.is_empty() {
                                    let prefix_len = text.len() - rest.len();
                                    body_events.push((
                                        Event::Text(CowStr::from(rest.to_string())),
                                        (range.start + prefix_len)..range.end,
                                    ));
                                }
                                self.state = State::CollectingDisclosure {
                                    start,
                                    summary_events,
                                    body_events,
                                    inline_style,
                                    in_summary: false,
                                    summary_has_content,
                                    body_depth: 0,
                                };
                            } else if Self::is_closer(text.as_ref()) {
                                return Some(Err(malformed_disclosure(
                                    "disclosure block is missing `::details` before `::end-disclosure`",
                                    start.start..range.end,
                                )));
                            } else if Self::match_keyword(text.as_ref(), DISCLOSURE_OPENER)
                                .is_some()
                            {
                                return Some(Err(malformed_disclosure(
                                    "summary region contains a nested disclosure opener",
                                    start.start..range.end,
                                )));
                            } else {
                                summary_has_content |= is_summary_content(&event);
                                summary_events.push((event, range));
                                self.state = State::CollectingDisclosure {
                                    start,
                                    summary_events,
                                    body_events,
                                    inline_style,
                                    in_summary,
                                    summary_has_content,
                                    body_depth,
                                };
                            }
                        }
                        Event::Text(text) if !in_summary => {
                            if Self::match_keyword(text.as_ref(), DISCLOSURE_OPENER).is_some() {
                                body_depth += 1;
                                body_events.push((event, range));
                                self.state = State::CollectingDisclosure {
                                    start,
                                    summary_events,
                                    body_events,
                                    inline_style,
                                    in_summary,
                                    summary_has_content,
                                    body_depth,
                                };
                            } else if Self::is_closer(text.as_ref()) {
                                if body_depth == 0 {
                                    self.state = State::ClosingDisclosure {
                                        start,
                                        summary_events,
                                        body_events,
                                        inline_style,
                                        summary_has_content,
                                    };
                                } else {
                                    body_depth -= 1;
                                    body_events.push((event, range));
                                    self.state = State::CollectingDisclosure {
                                        start,
                                        summary_events,
                                        body_events,
                                        inline_style,
                                        in_summary,
                                        summary_has_content,
                                        body_depth,
                                    };
                                }
                            } else {
                                body_events.push((event, range));
                                self.state = State::CollectingDisclosure {
                                    start,
                                    summary_events,
                                    body_events,
                                    inline_style,
                                    in_summary,
                                    summary_has_content,
                                    body_depth,
                                };
                            }
                        }
                        _ => {
                            if in_summary {
                                summary_has_content |= is_summary_content(&event);
                                summary_events.push((event, range));
                            } else {
                                body_events.push((event, range));
                            }
                            self.state = State::CollectingDisclosure {
                                start,
                                summary_events,
                                body_events,
                                inline_style,
                                in_summary,
                                summary_has_content,
                                body_depth,
                            };
                        }
                    }
                }
                State::ClosingDisclosure {
                    start,
                    summary_events,
                    body_events,
                    inline_style,
                    summary_has_content,
                } => match &event {
                    Event::End(TagEnd::Paragraph) => {
                        if !summary_has_content {
                            return Some(Err(malformed_disclosure(
                                "disclosure summary region is empty",
                                start.start..range.end,
                            )));
                        }
                        return Some(Ok(BlockExtensionEvent::Disclosure {
                            summary_events,
                            body_events,
                            inline_style,
                            range: start.start..range.end,
                        }));
                    }
                    Event::Text(text) if text.trim().is_empty() => {
                        // Trailing whitespace after the closer is ignored.
                        self.state = State::ClosingDisclosure {
                            start,
                            summary_events,
                            body_events,
                            inline_style,
                            summary_has_content,
                        };
                    }
                    _ => {
                        return Some(Err(malformed_disclosure(
                            "disclosure closer must end the paragraph",
                            start.start..range.end,
                        )));
                    }
                },
            }
        }
    }
}

/// Returns `true` for inline events that count as non-empty summary content.
fn is_summary_content(event: &Event<'_>) -> bool {
    match event {
        Event::Text(t) => !t.trim().is_empty(),
        Event::Code(_) | Event::InlineHtml(_) => true,
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulldown_cmark::{Options, Parser};

    /// Drains the processor over `markdown` into a flat vector, panicking on
    /// the first malformed disclosure. Use [`run_result`] when the test expects
    /// an error.
    fn run(markdown: &str) -> Vec<BlockExtensionEvent<'_>> {
        run_result(markdown).unwrap_or_else(|e| panic!("unexpected error: {e:?}"))
    }

    /// Drains the processor over `markdown` and returns the first error, if any.
    fn run_result(markdown: &str) -> Result<Vec<BlockExtensionEvent<'_>>, MarkdownError> {
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

    #[allow(clippy::type_complexity)]
    fn first_disclosure<'a>(
        events: &'a [BlockExtensionEvent<'a>],
    ) -> Option<(Vec<(Event<'a>, Range<usize>)>, Vec<(Event<'a>, Range<usize>)>, Option<CommonStyle>, Range<usize>)> {
        events.iter().find_map(|e| match e {
            BlockExtensionEvent::Disclosure {
                summary_events,
                body_events,
                inline_style,
                range,
            } => Some((summary_events.clone(), body_events.clone(), inline_style.clone(), range.clone())),
            _ => None,
        })
    }

    fn collect_text_from_events(events: &[(Event<'_>, Range<usize>)]) -> String {
        events
            .iter()
            .filter_map(|(e, _)| match e {
                Event::Text(t) => Some(t.as_ref()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
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

    // -------------------------------------------------------------------
    // Disclosure block recognition.
    // -------------------------------------------------------------------

    #[test]
    fn valid_disclosure_emits_single_synthetic_event() {
        let input = "::disclosure\nLicense Agreement\n::details\nKeep your hands off.\n::end-disclosure\n";
        let events = run(input);
        let disclosures: Vec<_> = events
            .iter()
            .filter(|e| matches!(e, BlockExtensionEvent::Disclosure { .. }))
            .collect();
        assert_eq!(disclosures.len(), 1, "expected one Disclosure event: {events:?}");
        let (summary, body, _inline_style, range) = first_disclosure(&events).expect("disclosure event");
        assert!(!summary.is_empty(), "summary events must not be empty");
        assert!(!body.is_empty(), "body events must not be empty");
        let summary_text = collect_text_from_events(&summary);
        let body_text = collect_text_from_events(&body);
        assert_eq!(summary_text, "License Agreement");
        assert_eq!(body_text, "Keep your hands off.");
        assert!(range.start < range.end);
    }

    #[test]
    fn empty_summary_is_rejected() {
        let input = "::disclosure\n::details\nbody\n::end-disclosure\n";
        let err = run_result(input).expect_err("empty summary must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { ref reason, .. } if reason.contains("empty")),
            "expected empty-summary error, got {err:?}"
        );
    }

    #[test]
    fn hard_line_break_in_summary_is_rejected() {
        let input = "::disclosure\nline one\\\nline two\n::details\nbody\n::end-disclosure\n";
        let err = run_result(input).expect_err("hard break in summary must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { ref reason, .. } if reason.contains("hard line break")),
            "expected hard-break error, got {err:?}"
        );
    }

    #[test]
    fn block_element_in_summary_is_rejected() {
        let input = "::disclosure\n# Heading\n::details\nbody\n::end-disclosure\n";
        let err = run_result(input).expect_err("block element in summary must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { .. }),
            "expected malformed-disclosure error, got {err:?}"
        );
    }

    #[test]
    fn missing_details_is_rejected() {
        let input = "::disclosure\nSummary\n::end-disclosure\n";
        let err = run_result(input).expect_err("missing details must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { ref reason, .. } if reason.contains("details")),
            "expected missing-details error, got {err:?}"
        );
    }

    #[test]
    fn missing_end_disclosure_is_rejected() {
        let input = "::disclosure\nSummary\n::details\nbody\n";
        let err = run_result(input).expect_err("missing closer must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { ref reason, .. } if reason.contains("end-disclosure")),
            "expected missing-closer error, got {err:?}"
        );
    }

    #[test]
    fn details_without_closer_is_rejected() {
        let input = "::disclosure\nSummary\n::details\nbody\n";
        let err = run_result(input).expect_err("details without closer must be rejected");
        assert!(
            matches!(err, MarkdownError::MalformedDisclosure { .. }),
            "expected malformed-disclosure error, got {err:?}"
        );
    }

    #[test]
    fn near_miss_keywords_are_literal_text() {
        let input = "::disclosure-extra\nSummary\n::details\nBody\n::end-disclosure\n";
        let events = run(input);
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, BlockExtensionEvent::Disclosure { .. })),
            "near-miss keywords must not form a disclosure: {events:?}"
        );
        let text: String = events
            .iter()
            .filter_map(|e| match e {
                BlockExtensionEvent::Standard(Event::Text(t), _) => Some(t.as_ref()),
                _ => None,
            })
            .collect();
        assert!(text.contains("::disclosure-extra"));
    }

    #[test]
    fn directives_inside_fenced_code_blocks_are_ignored() {
        let input = "```\n::disclosure\n::details\n::end-disclosure\n```\n";
        let events = run(input);
        assert!(
            events
                .iter()
                .all(|e| !matches!(e, BlockExtensionEvent::Disclosure { .. })),
            "directives inside a fenced code block must be ignored: {events:?}"
        );
        let text: String = events
            .iter()
            .filter_map(|e| match e {
                BlockExtensionEvent::Standard(Event::Text(t), _) => Some(t.as_ref()),
                _ => None,
            })
            .collect();
        assert!(text.contains("::disclosure"));
    }
}


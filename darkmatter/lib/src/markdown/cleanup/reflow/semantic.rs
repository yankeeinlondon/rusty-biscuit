use pulldown_cmark::{Event, Tag, TagEnd};
use std::ops::Range;

use super::{
    directive_trimmed, fence_marker, html_block_end, is_structural_line, join_separator,
    HtmlBlockEnd,
};
use super::super::cleanup_parser;
use super::super::opaque::OpaqueBodyScan;

#[derive(Debug, Clone)]
pub(super) struct SoftBreakModel {
    pub(super) boundaries: Vec<SemanticSoftBreak>,
    pub(super) protected: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
pub(super) struct ReflowMap {
    pub(super) blocks: Vec<ReflowBlock>,
    pub(super) breaks: Vec<ReflowBreak>,
    pub(super) protected: Vec<Range<usize>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReflowBlock {
    pub(super) span: Range<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct ReflowBreak {
    pub(super) span: Range<usize>,
    pub(super) next_prefix: Range<usize>,
    pub(super) kind: ReflowBreakKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum ReflowBreakKind {
    Soft(Option<char>),
    Hard,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct SemanticSoftBreak {
    pub(super) soft_break: Range<usize>,
    pub(super) next_prefix: Range<usize>,
    pub(super) protected: bool,
    pub(super) eligible: bool,
    pub(super) join_separator: Option<char>,
    pub(super) list_depth: usize,
    pub(super) item_depth: usize,
    pub(super) blockquote_depth: usize,
    pub(super) paragraph_depth: usize,
}

impl SoftBreakModel {
    pub(super) fn from_content(content: &str) -> Self {
        let events: Vec<_> = cleanup_parser(content).into_offset_iter().collect();
        Self::from_events(content, &events)
    }

    pub(super) fn from_events(content: &str, events: &[(Event<'_>, Range<usize>)]) -> Self {
        let protected_lines = ProtectedLines::scan(content);
        let mut context = ParserContext::default();
        let mut boundaries = Vec::new();

        for (event, span) in events {
            match event {
                Event::Start(tag) => context.enter(tag),
                Event::SoftBreak => {
                    let next_prefix = continuation_prefix_span(content, span);
                    let protected = context.in_protected_block()
                        || protected_lines.boundary_is_protected(span);
                    let eligible = context.in_prose() && !protected;
                    boundaries.push(SemanticSoftBreak {
                        join_separator: separator_at_boundary(content, span, &next_prefix),
                        soft_break: span.clone(),
                        next_prefix,
                        protected,
                        eligible,
                        list_depth: context.list_depth,
                        item_depth: context.item_depth,
                        blockquote_depth: context.blockquote_depth,
                        paragraph_depth: context.paragraph_depth,
                    });
                }
                Event::End(tag) => context.exit(*tag),
                _ => {}
            }
        }

        Self {
            boundaries,
            protected: protected_lines.opaque_ranges,
        }
    }
}

impl ReflowMap {
    pub(super) fn from_content(content: &str) -> Self {
        let protected_lines = ProtectedLines::scan(content);
        let mut context = ParserContext::default();
        let mut implicit = ImplicitBlock::default();
        let mut blocks = Vec::new();
        let mut breaks = Vec::new();

        let events = cleanup_parser(content).into_offset_iter();
        // Link-reference definitions are consumed by the first pass and never
        // reach the event stream, so their source ranges must be collected from
        // the parser's definition table or word wrapping would shred them into
        // invalid Markdown.
        let mut protected: Vec<Range<usize>> = protected_lines.opaque_ranges.clone();
        protected.extend(events
            .reference_definitions()
            .iter()
            .map(|(_, definition)| prose_block_span(content, definition.span.clone()))
        );

        for (event, span) in events {
            match event {
                Event::Start(tag) => {
                    if matches!(tag, Tag::CodeBlock(_) | Tag::Table(_) | Tag::HtmlBlock) {
                        protected.push(prose_block_span(content, span.clone()));
                    }
                    if matches!(tag, Tag::Paragraph) {
                        implicit.finish(content, &mut blocks);
                        if context.item_depth > 0 {
                            blocks.push(ReflowBlock {
                                span: prose_block_span(content, span.clone()),
                            });
                        }
                    } else if context.item_depth > 0
                        && context.paragraph_depth == 0
                        && is_inline_start(&tag)
                    {
                        implicit.observe(span.clone());
                    } else if !is_inline_start(&tag) {
                        implicit.finish(content, &mut blocks);
                    }
                    context.enter(&tag);
                }
                Event::End(tag) => {
                    if context.item_depth > 0
                        && context.paragraph_depth == 0
                        && is_inline_end(tag)
                    {
                        implicit.observe(span.clone());
                    }
                    if matches!(tag, TagEnd::Item | TagEnd::List(_)) {
                        implicit.finish(content, &mut blocks);
                    }
                    context.exit(tag);
                }
                Event::SoftBreak | Event::HardBreak => {
                    if context.item_depth > 0 && context.in_prose() {
                        if context.paragraph_depth == 0 {
                            implicit.observe(span.clone());
                        }
                        let next_prefix = continuation_prefix_span(content, &span);
                        let kind = if matches!(event, Event::SoftBreak) {
                            ReflowBreakKind::Soft(separator_at_boundary(
                                content,
                                &span,
                                &next_prefix,
                            ))
                        } else {
                            ReflowBreakKind::Hard
                        };
                        breaks.push(ReflowBreak {
                            span,
                            next_prefix,
                            kind,
                        });
                    }
                }
                Event::Text(_)
                | Event::Code(_)
                | Event::InlineMath(_)
                | Event::DisplayMath(_)
                | Event::InlineHtml(_)
                | Event::FootnoteReference(_)
                | Event::TaskListMarker(_) => {
                    if context.item_depth > 0
                        && context.paragraph_depth == 0
                        && !context.in_protected_block()
                    {
                        implicit.observe(span);
                    }
                }
                _ => {
                    implicit.finish(content, &mut blocks);
                }
            }
        }
        implicit.finish(content, &mut blocks);

        blocks.retain(|block| {
            !block.span.is_empty() && !protected_lines.range_is_protected(&block.span)
        });
        blocks.sort_unstable_by_key(|block| block.span.start);
        blocks.dedup_by(|left, right| left.span == right.span);
        breaks.retain(|boundary| {
            blocks.iter().any(|block| {
                block.span.start <= boundary.span.start && boundary.span.end <= block.span.end
            })
        });

        Self {
            blocks,
            breaks,
            protected,
        }
    }
}

#[derive(Debug, Clone, Default)]
struct ImplicitBlock {
    start: Option<usize>,
    end: usize,
}

impl ImplicitBlock {
    fn observe(&mut self, span: Range<usize>) {
        self.start.get_or_insert(span.start);
        self.end = self.end.max(span.end);
    }

    fn finish(&mut self, content: &str, blocks: &mut Vec<ReflowBlock>) {
        let Some(start) = self.start.take() else {
            return;
        };
        blocks.push(ReflowBlock {
            span: prose_block_span(content, start..self.end),
        });
        self.end = 0;
    }
}

fn is_inline_start(tag: &Tag<'_>) -> bool {
    matches!(
        tag,
        Tag::Emphasis | Tag::Strong | Tag::Strikethrough | Tag::Link { .. } | Tag::Image { .. }
    )
}

fn is_inline_end(tag: TagEnd) -> bool {
    matches!(
        tag,
        TagEnd::Emphasis | TagEnd::Strong | TagEnd::Strikethrough | TagEnd::Link | TagEnd::Image
    )
}

fn prose_block_span(content: &str, span: Range<usize>) -> Range<usize> {
    let start = content[..span.start]
        .rfind(['\n', '\r'])
        .map_or(0, |idx| idx + 1);
    let mut end = span.end;
    while end > start && matches!(content.as_bytes()[end - 1], b'\n' | b'\r') {
        end -= 1;
    }
    start..end
}

#[derive(Debug, Clone, Copy, Default)]
struct ParserContext {
    list_depth: usize,
    item_depth: usize,
    blockquote_depth: usize,
    paragraph_depth: usize,
    code_block_depth: usize,
    table_depth: usize,
    html_block_depth: usize,
}

impl ParserContext {
    fn enter(&mut self, tag: &Tag<'_>) {
        match tag {
            Tag::List(_) => self.list_depth += 1,
            Tag::Item => self.item_depth += 1,
            Tag::BlockQuote(_) => self.blockquote_depth += 1,
            Tag::Paragraph => self.paragraph_depth += 1,
            Tag::CodeBlock(_) => self.code_block_depth += 1,
            Tag::Table(_) => self.table_depth += 1,
            Tag::HtmlBlock => self.html_block_depth += 1,
            _ => {}
        }
    }

    fn exit(&mut self, tag: TagEnd) {
        match tag {
            TagEnd::List(_) => self.list_depth = self.list_depth.saturating_sub(1),
            TagEnd::Item => self.item_depth = self.item_depth.saturating_sub(1),
            TagEnd::BlockQuote(_) => {
                self.blockquote_depth = self.blockquote_depth.saturating_sub(1);
            }
            TagEnd::Paragraph => {
                self.paragraph_depth = self.paragraph_depth.saturating_sub(1);
            }
            TagEnd::CodeBlock => {
                self.code_block_depth = self.code_block_depth.saturating_sub(1);
            }
            TagEnd::Table => self.table_depth = self.table_depth.saturating_sub(1),
            TagEnd::HtmlBlock => {
                self.html_block_depth = self.html_block_depth.saturating_sub(1);
            }
            _ => {}
        }
    }

    fn in_prose(self) -> bool {
        self.paragraph_depth > 0 || self.item_depth > 0
    }

    fn in_protected_block(self) -> bool {
        self.code_block_depth > 0 || self.table_depth > 0 || self.html_block_depth > 0
    }
}

#[derive(Debug, Clone)]
struct ProtectedLines {
    lines: Vec<ProtectedLine>,
    opaque_ranges: Vec<Range<usize>>,
}

#[derive(Debug, Clone)]
struct ProtectedLine {
    span: Range<usize>,
    protected: bool,
}

impl ProtectedLines {
    fn scan(content: &str) -> Self {
        let opaque_scan = OpaqueBodyScan::scan(content);
        let opaque_ranges = opaque_scan.protected_ranges(content.len());
        let mut fence: Option<String> = None;
        let mut html_block: Option<HtmlBlockEnd> = None;
        let mut lines = Vec::new();

        for span in source_line_spans(content) {
            let line = &content[span.clone()];
            let trimmed = line.trim_start();
            let directive = directive_trimmed(line);
            let blank = trimmed.is_empty();
            let was_in_fence = fence.is_some();
            let was_in_html = html_block.is_some();
            let starts_fence = fence.is_none().then(|| fence_marker(trimmed)).flatten();
            let starts_html = html_block.is_none().then(|| html_block_end(trimmed)).flatten();
            let opaque = opaque_ranges.iter().any(|opaque| {
                opaque.start <= span.end && span.start < opaque.end
            });
            let protected = was_in_fence
                || was_in_html
                || starts_fence.is_some()
                || starts_html.is_some()
                || opaque
                || is_structural_line(trimmed)
                || is_structural_line(directive);
            lines.push(ProtectedLine { span, protected });

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

        }

        Self {
            lines,
            opaque_ranges,
        }
    }

    fn boundary_is_protected(&self, soft_break: &Range<usize>) -> bool {
        self.line_at(soft_break.start.saturating_sub(1))
            .is_some_and(|line| line.protected)
            || self
                .line_at(soft_break.end)
                .is_some_and(|line| line.protected)
    }

    fn line_at(&self, offset: usize) -> Option<&ProtectedLine> {
        let idx = self.lines.partition_point(|line| line.span.start <= offset);
        idx.checked_sub(1)
            .and_then(|idx| self.lines.get(idx))
            .filter(|line| offset <= line.span.end)
    }

    fn range_is_protected(&self, span: &Range<usize>) -> bool {
        self.lines.iter().any(|line| {
            line.protected && line.span.start < span.end && span.start <= line.span.end
        })
    }
}

fn source_line_spans(content: &str) -> Vec<Range<usize>> {
    let bytes = content.as_bytes();
    let mut spans = Vec::new();
    let mut start = 0usize;
    let mut idx = 0usize;

    while idx < bytes.len() {
        if matches!(bytes[idx], b'\n' | b'\r') {
            spans.push(start..idx);
            if bytes[idx] == b'\r' && bytes.get(idx + 1) == Some(&b'\n') {
                idx += 1;
            }
            start = idx + 1;
        }
        idx += 1;
    }
    spans.push(start..bytes.len());
    spans
}

fn continuation_prefix_span(content: &str, soft_break: &Range<usize>) -> Range<usize> {
    let bytes = content.as_bytes();
    let start = soft_break.end;
    let mut idx = start;

    loop {
        while matches!(bytes.get(idx), Some(b' ' | b'\t')) {
            idx += 1;
        }
        if bytes.get(idx) != Some(&b'>') {
            break;
        }
        idx += 1;
        if matches!(bytes.get(idx), Some(b' ' | b'\t')) {
            idx += 1;
        }
    }

    start..idx
}

fn separator_at_boundary(
    content: &str,
    soft_break: &Range<usize>,
    next_prefix: &Range<usize>,
) -> Option<char> {
    let previous_start = content[..soft_break.start]
        .rfind(['\n', '\r'])
        .map_or(0, |idx| idx + 1);
    let next_end = content[next_prefix.end..]
        .find(['\n', '\r'])
        .map_or(content.len(), |len| next_prefix.end + len);
    join_separator(
        &content[previous_start..soft_break.start],
        &content[next_prefix.end..next_end],
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn model_classifies_lazy_and_explicit_list_continuations() {
        let source = "- Lazy alpha\ncontinues beta\n\n- Explicit alpha\n    continues beta";
        let model = SoftBreakModel::from_content(source);

        assert_eq!(model.boundaries.len(), 2);
        assert_eq!(model.boundaries[0].soft_break, 12..13);
        assert_eq!(model.boundaries[0].next_prefix, 13..13);
        assert!(model.boundaries[0].eligible);
        assert!(!model.boundaries[0].protected);
        assert_eq!(model.boundaries[0].join_separator, Some(' '));
        assert_eq!(model.boundaries[0].list_depth, 1);
        assert_eq!(model.boundaries[0].item_depth, 1);

        assert_eq!(model.boundaries[1].soft_break, 45..46);
        assert_eq!(&source[model.boundaries[1].next_prefix.clone()], "    ");
        assert!(model.boundaries[1].eligible);
        assert_eq!(model.boundaries[1].join_separator, Some(' '));
    }

    #[test]
    fn model_keeps_parent_and_child_item_boundaries_independent() {
        let source = "- Parent alpha\n    parent continuation\n    - Child alpha\n      child continuation";
        let model = SoftBreakModel::from_content(source);

        assert_eq!(model.boundaries.len(), 2);
        assert_eq!(&source[model.boundaries[0].next_prefix.clone()], "    ");
        assert_eq!(model.boundaries[0].list_depth, 1);
        assert_eq!(&source[model.boundaries[1].next_prefix.clone()], "      ");
        assert_eq!(model.boundaries[1].list_depth, 2);
        assert!(model.boundaries.iter().all(|boundary| boundary.eligible));
    }

    #[test]
    fn model_records_composite_blockquote_list_prefix() {
        let source = "> - Quoted alpha\n>   quoted continuation";
        let model = SoftBreakModel::from_content(source);

        assert_eq!(model.boundaries.len(), 1);
        let boundary = &model.boundaries[0];
        assert_eq!(boundary.soft_break, 16..17);
        assert_eq!(&source[boundary.next_prefix.clone()], ">   ");
        assert_eq!(boundary.blockquote_depth, 1);
        assert_eq!(boundary.list_depth, 1);
        assert!(boundary.eligible);
    }

    #[test]
    fn model_limits_second_paragraph_boundary_to_that_paragraph() {
        let source = "- First paragraph.\n\n    Second alpha\n    second continuation";
        let model = SoftBreakModel::from_content(source);

        assert_eq!(model.boundaries.len(), 1);
        let boundary = &model.boundaries[0];
        assert_eq!(&source[boundary.next_prefix.clone()], "    ");
        assert_eq!(boundary.list_depth, 1);
        assert_eq!(boundary.item_depth, 1);
        assert_eq!(boundary.paragraph_depth, 1);
        assert!(boundary.eligible);
    }

    #[test]
    fn model_protects_structural_and_darkmatter_child_blocks() {
        let protected_children = [
            "- Before\n\n        indented one\n        indented two",
            "- Before\n\n    ```text\n    fenced one\n    fenced two\n    ```",
            "- Before\n\n    | A | B |\n    |---|---|\n    | 1 | 2 |",
            "- Before\n\n    <div>\n    html one\n    html two\n    </div>",
        ];

        for source in protected_children {
            let model = SoftBreakModel::from_content(source);
            assert!(
                model.boundaries.is_empty(),
                "parser-structured child exposed a candidate soft break: {source:?}"
            );
        }

        let source = "::shell-block\necho one\necho two\n::end-block";
        let model = SoftBreakModel::from_content(source);
        assert!(!model.boundaries.is_empty());
        assert!(model
            .boundaries
            .iter()
            .all(|boundary| boundary.protected && !boundary.eligible));
    }

    #[test]
    fn model_reuses_unicode_join_separator_policy() {
        let source = "- \u{6F22}\n  \u{5B57}";
        let model = SoftBreakModel::from_content(source);

        assert_eq!(model.boundaries.len(), 1);
        assert_eq!(model.boundaries[0].join_separator, None);
    }
}

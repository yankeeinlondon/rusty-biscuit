//! The shared Prose grammar's outer layers: line-break mode, the block
//! splitter, and the two entry points that feed [`super::tokens`].
//!
//! Parsing runs in a fixed order: fenced blocks are lifted from the whole
//! input ([`lift_fences`](super::markdown::lift_fences)), the remaining text
//! is split into paragraphs on blank lines ([`split_blocks`], `Prose` only),
//! and each paragraph's inline content is parsed on its own.
//!
//! A bracketed style or explicit `<a>` tag that spans a blank line keeps its
//! scope: the splitter closes it at the end of one paragraph and reopens the
//! same declaration at the start of the next, so a paragraph never sits
//! inside a phrasing wrapper. Code spans, quoted tag attributes, and
//! `<code-block>` bodies are opaque to the splitter.

use renderable::tree::RenderNode;

use super::markdown::{
    LIFT_MARK, Lifted, blank_run_end, is_escapable, lift_fences, parse_lift_placeholder,
    preprocess_inline,
};
use super::tokens::{InlineOptions, is_recognized_opening_tag, parse_opening_tag, parse_render_nodes};

/// What a single newline inside a paragraph means.
///
/// `\` immediately before a newline is a hard break in either mode, and two
/// or more newlines still separate `Prose` blocks in either mode.
///
/// ## Examples
///
/// ```rust
/// use biscuit_terminal::components::prose::{LineBreaks, Prose};
/// use renderable::markdown::MarkdownRenderable;
///
/// assert_eq!(Prose::new("a\nb").render_markdown(), "a\nb");
/// assert_eq!(
///     Prose::new("a\nb").with_line_breaks(LineBreaks::Hard).render_markdown(),
///     "a\\\nb"
/// );
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum LineBreaks {
    /// A single newline is a soft break: a space on the terminal and in
    /// HTML, a newline in Markdown.
    #[default]
    Soft,
    /// A single newline is a hard break.
    Hard,
}

/// One block of `Prose` input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Block {
    /// A paragraph's text, with any open style scope closed at its end and
    /// reopened at its start.
    Paragraph(String),
    /// A lifted fenced block, by index into the lifted-content table.
    Fence(usize),
}

/// A recognized tag open at the current position.
struct OpenTag {
    name: String,
    declaration: String,
}

/// Accumulates one paragraph while the splitter walks the input.
struct ParagraphBuilder {
    prefix: String,
    body: String,
    has_content: bool,
}

impl ParagraphBuilder {
    fn new(open: &[OpenTag]) -> Self {
        Self {
            prefix: open.iter().map(|tag| tag.declaration.as_str()).collect(),
            body: String::new(),
            has_content: false,
        }
    }

    fn push(&mut self, s: &str) {
        self.body.push_str(s);
    }

    fn push_content(&mut self, s: &str) {
        self.has_content |= s.chars().any(|c| !c.is_whitespace());
        self.body.push_str(s);
    }

    /// Close the paragraph, appending closers for every still-open tag.
    /// Whitespace-only paragraphs produce nothing.
    fn finish(self, open: &[OpenTag], blocks: &mut Vec<Block>) {
        if !self.has_content {
            return;
        }
        let mut text = self.prefix;
        text.push_str(trim_blank_edges(&self.body));
        for tag in open.iter().rev() {
            text.push_str("</");
            text.push_str(&tag.name);
            text.push('>');
        }
        blocks.push(Block::Paragraph(text));
    }
}

/// Remove blank lines (and the line ending before them) from both ends of a
/// paragraph body; spaces on non-blank lines are kept.
fn trim_blank_edges(body: &str) -> &str {
    let is_blank = |line: &str| line.chars().all(|c| c == ' ' || c == '\t');
    let mut s = body;
    while let Some(pos) = s.find('\n') {
        if !is_blank(&s[..pos]) {
            break;
        }
        s = &s[pos + 1..];
    }
    while let Some(pos) = s.rfind('\n') {
        if !is_blank(&s[pos + 1..]) {
            break;
        }
        s = &s[..pos];
    }
    s
}

/// Split fence-lifted text into paragraphs and fenced blocks.
///
/// A run of two or more newlines separated only by spaces or tabs ends a
/// paragraph, as does a fence placeholder. Leading and trailing blank lines
/// and whitespace-only paragraphs produce no block.
pub(super) fn split_blocks(text: &str, lifted: &[Lifted]) -> Vec<Block> {
    let chars: Vec<char> = text.chars().collect();
    let mut blocks = Vec::new();
    let mut open: Vec<OpenTag> = Vec::new();
    let mut paragraph = ParagraphBuilder::new(&open);
    let mut i = 0;

    while i < chars.len() {
        let ch = chars[i];

        if ch == '\\' && i + 1 < chars.len() && is_escapable(chars[i + 1]) {
            paragraph.push_content(&chars[i..i + 2].iter().collect::<String>());
            i += 2;
            continue;
        }

        if ch == '\n' {
            if let Some(end) = blank_run_end(&chars, i) {
                std::mem::replace(&mut paragraph, ParagraphBuilder::new(&open)).finish(&open, &mut blocks);
                i = end;
            } else {
                paragraph.push("\n");
                i += 1;
            }
            continue;
        }

        if ch == LIFT_MARK
            && let Some((index, next)) = parse_lift_placeholder(&chars, i)
            && matches!(lifted.get(index), Some(Lifted::Fence(_)))
        {
            std::mem::replace(&mut paragraph, ParagraphBuilder::new(&open)).finish(&open, &mut blocks);
            blocks.push(Block::Fence(index));
            i = next;
            continue;
        }

        if ch == '`' {
            let run = chars[i..].iter().take_while(|&&c| c == '`').count();
            let span_end = closing_run_in_paragraph(&chars, i + run, run).map(|close| close + run);
            let end = span_end.unwrap_or(i + run);
            paragraph.push_content(&chars[i..end].iter().collect::<String>());
            i = end;
            continue;
        }

        if ch == '<'
            && let Some(end) = tag_declaration_end(&chars, i)
        {
            let content: String = chars[i + 1..end - 1].iter().collect();
            if let Some(name) = content.strip_prefix('/') {
                let name = name.trim().to_lowercase();
                if let Some(pos) = open.iter().rposition(|tag| tag.name == name) {
                    open.remove(pos);
                    paragraph.push(&chars[i..end].iter().collect::<String>());
                    i = end;
                    continue;
                }
            } else if is_recognized_opening_tag(&content)
                && let Some((name, _)) = parse_opening_tag(&content)
            {
                if name == "code-block" {
                    // An explicit code block is opaque: blank lines inside it
                    // are code, not paragraph boundaries.
                    let close = find_ci(&chars, end, "</code-block>").unwrap_or(chars.len());
                    paragraph.push_content(&chars[i..close].iter().collect::<String>());
                    i = close;
                    continue;
                }
                let declaration: String = chars[i..end].iter().collect();
                paragraph.push(&declaration);
                open.push(OpenTag { name, declaration });
                i = end;
                continue;
            }
        }

        paragraph.push_content(&ch.to_string());
        i += 1;
    }

    paragraph.finish(&open, &mut blocks);
    blocks
}

/// Index of the closing backtick run of length `len` at or after `start`,
/// before the next blank line.
fn closing_run_in_paragraph(chars: &[char], start: usize, len: usize) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        match chars[i] {
            '\n' if blank_run_end(chars, i).is_some() => return None,
            '`' => {
                let run = chars[i..].iter().take_while(|&&c| c == '`').count();
                if run == len {
                    return Some(i);
                }
                i += run;
            }
            _ => i += 1,
        }
    }
    None
}

/// Index just past the `>` of the tag declaration opening at `chars[start]`,
/// scanning quote- and escape-aware like the token parser, so a newline
/// inside a quoted attribute belongs to the declaration. An unquoted blank
/// line ends the attempt: the `<` is then literal text.
fn tag_declaration_end(chars: &[char], start: usize) -> Option<usize> {
    let mut quote: Option<char> = None;
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\n' if quote.is_none() && blank_run_end(chars, i).is_some() => return None,
            '\\' => i += 2,
            c @ ('"' | '\'') => {
                match quote {
                    Some(q) if q == c => quote = None,
                    None => quote = Some(c),
                    Some(_) => {}
                }
                i += 1;
            }
            '>' if quote.is_none() => return Some(i + 1),
            _ => i += 1,
        }
    }
    None
}

/// Index just past the first case-insensitive occurrence of `needle` at or
/// after `start`.
fn find_ci(chars: &[char], start: usize, needle: &str) -> Option<usize> {
    let needle: Vec<char> = needle.chars().collect();
    (start..=chars.len().checked_sub(needle.len())?)
        .find(|&i| {
            chars[i..i + needle.len()]
                .iter()
                .zip(&needle)
                .all(|(a, b)| a.eq_ignore_ascii_case(b))
        })
        .map(|i| i + needle.len())
}

/// Parse `Prose` content into block nodes: a `Paragraph` per paragraph and a
/// `Code` per fenced block, in source order.
///
/// A `<code-block>` tag or a fenced block nested in a style still lands as a
/// sibling block: the paragraph is split around it.
pub(super) fn parse_blocks(content: &str, line_breaks: LineBreaks) -> Vec<RenderNode> {
    let (text, mut lifted) = lift_fences(content);
    let opts = InlineOptions {
        line_breaks,
        inline_only: false,
    };
    let mut out = Vec::new();
    for block in split_blocks(&text, &lifted) {
        match block {
            Block::Fence(index) => {
                if let Some(Lifted::Fence(code)) = lifted.get(index) {
                    out.push(RenderNode::code(
                        Some(code.lang.clone()).filter(|s| !s.is_empty()),
                        None,
                        code.body.clone(),
                    ));
                }
            }
            Block::Paragraph(paragraph) => {
                let pre = preprocess_inline(&paragraph, &mut lifted);
                let nodes = parse_render_nodes(&pre, &lifted, opts);
                push_paragraphs(nodes, &mut out);
            }
        }
    }
    out
}

/// Parse `InlineProse` content into phrasing nodes. Fenced blocks become
/// `InlineCode`; blank lines are ordinary breaks.
pub(super) fn parse_inline(content: &str, line_breaks: LineBreaks) -> Vec<RenderNode> {
    let (text, mut lifted) = lift_fences(content);
    let pre = preprocess_inline(&text, &mut lifted);
    parse_render_nodes(
        &pre,
        &lifted,
        InlineOptions {
            line_breaks,
            inline_only: true,
        },
    )
}

/// Wrap a paragraph's nodes in `Paragraph` blocks, splitting around any
/// block-level `Code` child so code never sits inside phrasing content.
/// A paragraph that parses to nothing (for example only an empty style tag)
/// adds no block.
fn push_paragraphs(nodes: Vec<RenderNode>, out: &mut Vec<RenderNode>) {
    let mut run: Vec<RenderNode> = Vec::new();
    for node in nodes {
        if matches!(node.kind, renderable::tree::NodeKind::Code { .. }) {
            if !run.is_empty() {
                out.push(RenderNode::paragraph(std::mem::take(&mut run)));
            }
            out.push(node);
        } else {
            run.push(node);
        }
    }
    if !run.is_empty() {
        out.push(RenderNode::paragraph(run));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn split(input: &str) -> Vec<Block> {
        let (text, lifted) = lift_fences(input);
        split_blocks(&text, &lifted)
    }

    fn para(s: &str) -> Block {
        Block::Paragraph(s.to_string())
    }

    #[test]
    fn blank_line_runs_split_paragraphs() {
        assert_eq!(split("a\nb"), [para("a\nb")]);
        assert_eq!(split("a\n\nb"), [para("a"), para("b")]);
        assert_eq!(split("a\n\n\n\nb"), [para("a"), para("b")]);
        assert_eq!(split("a\n \t \nb"), [para("a"), para("b")]);
    }

    #[test]
    fn edges_and_whitespace_only_input_produce_no_blocks() {
        assert_eq!(split(""), []);
        assert_eq!(split("  \n\t\n "), []);
        assert_eq!(split("\n\na\n\n"), [para("a")]);
        assert_eq!(split("  a  "), [para("  a  ")]);
    }

    #[test]
    fn styles_reopen_in_each_paragraph() {
        assert_eq!(split("<red>one\n\ntwo</red>"), [para("<red>one</red>"), para("<red>two</red>")]);
        assert_eq!(
            split("<a href=\"u\">x\n\ny</a> z"),
            [para("<a href=\"u\">x</a>"), para("<a href=\"u\">y</a> z")]
        );
    }

    #[test]
    fn fence_inside_a_style_is_a_sibling_block_and_the_style_resumes() {
        assert_eq!(
            split("<red>a\n```\nx\n\ny\n```\nb</red>"),
            [para("<red>a</red>"), Block::Fence(0), para("<red>b</red>")]
        );
    }

    #[test]
    fn quoted_attribute_newlines_belong_to_the_attribute() {
        assert_eq!(split("<a href=\"x\n\ny\">t</a>"), [para("<a href=\"x\n\ny\">t</a>")]);
    }

    #[test]
    fn code_spans_and_code_block_tags_are_opaque_to_the_splitter() {
        assert_eq!(split("`<red>` a\n\nb"), [para("`<red>` a"), para("b")]);
        assert_eq!(split("<code-block>a\n\nb</code-block>"), [para("<code-block>a\n\nb</code-block>")]);
    }

    #[test]
    fn unmatched_backticks_never_span_paragraphs() {
        assert_eq!(split("`a\n\nb`"), [para("`a"), para("b`")]);
    }

    #[test]
    fn unrecognized_angle_bracket_does_not_hide_a_boundary() {
        assert_eq!(split("a < b\n\nc > d"), [para("a < b"), para("c > d")]);
    }
}

//! Shared region pairing scanner for `::block`, `::shell-block`, `::map`, and
//! `::filter` directives.
//!
//! One stack-based scanner recognizes every region opener and pairs it with its
//! closer (`::end-block` for page and shell blocks, `::end-map`, `::end-filter`),
//! so interleaved nesting of all region kinds is handled unambiguously. Page and
//! shell parsing consume the pairs through adapters that ignore collection
//! kinds; the collection parser does the converse.
//!
//! Container ancestry comes from the CommonMark offset scan
//! ([`scan_code_regions`](super::parse_utils::scan_code_regions)): a collection
//! closer must sit in exactly the container path of its opener, so a quoted
//! collection is active inside its own block quote or list item only.

use std::ops::Range;

use super::body_origin::DataRanges;
use super::parse_utils::{
    CodeRegions, ContainerStep, authored_directive, is_in_code_region, scan_code_regions,
    strip_blockquote_prefix, structural_view,
};

/// Which kind of region opened a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum BlockOpenKind {
    /// A `::block` page-block opener.
    Page,
    /// A `::shell-block` shell-block opener.
    Shell,
    /// A `::map` collection opener, closed by `::end-map`.
    Map,
    /// A `::filter` collection opener, closed by `::end-filter`.
    Filter,
}

impl BlockOpenKind {
    /// The directive keyword that closes a region of this kind.
    pub(crate) fn closer(self) -> &'static str {
        match self {
            BlockOpenKind::Page | BlockOpenKind::Shell => "::end-block",
            BlockOpenKind::Map => "::end-map",
            BlockOpenKind::Filter => "::end-filter",
        }
    }

    /// Whether this is a `::map` or `::filter` region.
    pub(crate) fn is_collection(self) -> bool {
        matches!(self, BlockOpenKind::Map | BlockOpenKind::Filter)
    }
}

/// A fully-paired region discovered by the shared scanner.
///
/// The scanner returns these in closing order (inner before outer); callers
/// filter by `kind` for the family they handle and sort by `span.start` when
/// they need document order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct BlockPair {
    /// Which kind of region opened this pair.
    pub kind: BlockOpenKind,
    /// Full byte span including the opener and closer lines (and the closer's
    /// line break when it has one).
    pub span: Range<usize>,
    /// Byte span of the body (between opener and closer, excluding both
    /// directive lines).
    pub body_span: Range<usize>,
    /// 1-based line number of the opening directive.
    pub start_line: usize,
    /// 1-based line number of the closing directive.
    pub end_line: usize,
    /// The raw text of the opening directive line, container prefix included,
    /// without its line break.
    pub opening_text: String,
    /// Byte span of the opening directive line without its line break.
    pub opener_span: Range<usize>,
    /// Byte span of the closing directive line without its line break.
    pub closer_span: Range<usize>,
    /// Whether the opener carried a block-quote marker prefix.
    pub quoted: bool,
    /// Container ancestry of the opener, outermost first. A collection closer
    /// is guaranteed to have the same path.
    pub container: Vec<ContainerStep>,
}

/// An opener awaiting its closer.
struct OpenRegion {
    kind: BlockOpenKind,
    block_start: usize,
    body_start: usize,
    start_line: usize,
    opening_text: String,
    opener_end: usize,
    quoted: bool,
    container: Vec<ContainerStep>,
}

/// Scans `content` for paired region directives and returns an ordered vector
/// of [`BlockPair`].
///
/// ## Behaviour
///
/// * Skips fenced and indented code and inline code spans.
/// * Pushes `::block`, `::shell-block`, `::map`, and `::filter` openers onto a
///   single stack.
/// * `::end-block` closes the top page or shell region; `::end-map` and
///   `::end-filter` close only a region of their own kind in exactly the
///   container path of the opener.
/// * Block-quote markers are honored for `::shell-block`, `::map`, and
///   `::filter`. A quoted `::block` and its `::end-block` stay literal; a quoted
///   `::end-block` is a closer only for a quoted shell block.
/// * Rejects closers with trailing non-whitespace text.
///
/// ## Errors
///
/// * [`BlockPairError::UnmatchedEnd`] / [`BlockPairError::UnmatchedCollectionEnd`]
///   when a closer has no opener.
/// * [`BlockPairError::CrossedClose`] when a closer meets a different region
///   kind or container path on top of the stack.
/// * [`BlockPairError::UnterminatedBlock`] (anchored at the opener) when EOF
///   leaves a region open.
/// * [`BlockPairError::CollectionInShellBody`] for `::map` / `::filter` inside
///   a shell block.
pub(crate) fn scan_block_pairs(content: &str) -> Result<Vec<BlockPair>, BlockPairError> {
    scan_block_pairs_in(content, None)
}

/// [`scan_block_pairs`] over a body whose `data` bytes were inserted by an
/// earlier stage. An opener or closer counts only when its whole line,
/// including the line break that starts it, is authored; code regions and
/// containers are found in the masked view, so data can neither open, close,
/// nor hide a region.
pub(crate) fn scan_block_pairs_in(
    content: &str,
    data: Option<&DataRanges>,
) -> Result<Vec<BlockPair>, BlockPairError> {
    let regions = scan_code_regions(&structural_view(content, data));
    scan_block_pairs_with_regions(content, data, &regions)
}

/// [`scan_block_pairs_in`] with the code regions and containers of the
/// structural view already computed by the caller.
pub(crate) fn scan_block_pairs_with_regions(
    content: &str,
    data: Option<&DataRanges>,
    regions: &CodeRegions,
) -> Result<Vec<BlockPair>, BlockPairError> {
    let bytes = content.as_bytes();
    let code_regions = regions.all.as_slice();

    let mut stack: Vec<OpenRegion> = Vec::new();
    let mut pairs: Vec<BlockPair> = Vec::new();

    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let text_line_end = if line_end > line_start && bytes[line_end - 1] == b'\r' {
            line_end - 1
        } else {
            line_end
        };
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &content[line_start..text_line_end];
        let trimmed = line.trim();

        if !trimmed.is_empty() {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, code_regions)
                && authored_directive(data, line_start, text_line_end)
            {
                // `::block` matches the unstripped line so a quoted
                // `> ::block ... > ::end-block` stays a literal documentation
                // example; every other opener honors block-quote markers.
                let directive = strip_blockquote_prefix(trimmed);
                let is_quoted = directive.len() < trimmed.len();
                let at_boundary = |after: &str| after.is_empty() || after.starts_with(char::is_whitespace);

                let opener = if let Some(after) = trimmed.strip_prefix("::block") {
                    at_boundary(after).then_some((BlockOpenKind::Page, false))
                } else if let Some(after) = directive.strip_prefix("::shell-block") {
                    at_boundary(after).then_some((BlockOpenKind::Shell, is_quoted))
                } else if let Some(after) = directive.strip_prefix("::map") {
                    at_boundary(after).then_some((BlockOpenKind::Map, is_quoted))
                } else if let Some(after) = directive.strip_prefix("::filter") {
                    at_boundary(after).then_some((BlockOpenKind::Filter, is_quoted))
                } else {
                    None
                };

                if let Some((kind, quoted)) = opener {
                    if kind.is_collection()
                        && let Some(shell) = stack.iter().find(|open| open.kind == BlockOpenKind::Shell)
                    {
                        return Err(BlockPairError::CollectionInShellBody {
                            line: line_number,
                            directive: trimmed.to_string(),
                            shell_line: shell.start_line,
                        });
                    }
                    let container = if kind.is_collection() {
                        regions.container_path(content, line_start, first_non_ws + trimmed.len() - directive.len())
                    } else {
                        Vec::new()
                    };
                    stack.push(OpenRegion {
                        kind,
                        block_start: line_start,
                        body_start: span_end,
                        start_line: line_number,
                        opening_text: content[line_start..text_line_end].to_string(),
                        opener_end: text_line_end,
                        quoted,
                        container,
                    });
                } else if let Some(after) = directive.strip_prefix("::end-block") {
                    // A quoted `> ::end-block` closes only a quoted shell block;
                    // otherwise (empty stack, unquoted or collection top) it is
                    // literal quoted Markdown, not a closer.
                    let is_active_closer = !is_quoted
                        || matches!(stack.last(), Some(open) if open.kind == BlockOpenKind::Shell && open.quoted);

                    if is_active_closer {
                        let trailing = after.trim();
                        if !trailing.is_empty() {
                            return Err(BlockPairError::TrailingContent {
                                line: line_number,
                                content: trailing.to_string(),
                            });
                        }

                        let Some(open) = stack.pop() else {
                            return Err(BlockPairError::UnmatchedEnd { line: line_number });
                        };
                        if open.kind.is_collection() {
                            return Err(crossed(line_number, trimmed, &open));
                        }
                        pairs.push(finish(open, line_start, text_line_end, span_end, line_number));
                    }
                } else if let Some((expected, after)) = directive
                    .strip_prefix("::end-map")
                    .map(|after| (BlockOpenKind::Map, after))
                    .or_else(|| {
                        directive
                            .strip_prefix("::end-filter")
                            .map(|after| (BlockOpenKind::Filter, after))
                    })
                    .filter(|(_, after)| at_boundary(after))
                {
                    let trailing = after.trim();
                    if !trailing.is_empty() {
                        return Err(BlockPairError::TrailingCollectionContent {
                            line: line_number,
                            closer: expected.closer(),
                            content: trailing.to_string(),
                        });
                    }
                    let Some(top) = stack.last() else {
                        return Err(BlockPairError::UnmatchedCollectionEnd {
                            line: line_number,
                            closer: expected.closer(),
                        });
                    };
                    let path = regions.container_path(content, line_start, first_non_ws + trimmed.len() - directive.len());
                    if top.kind != expected || top.container != path {
                        return Err(crossed(line_number, trimmed, top));
                    }
                    let Some(open) = stack.pop() else {
                        unreachable!("stack top was just inspected");
                    };
                    pairs.push(finish(open, line_start, text_line_end, span_end, line_number));
                }
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    // Check for unterminated blocks (report the deepest open one)
    if let Some(open) = stack.last() {
        let opening_end = content[open.block_start..]
            .find('\n')
            .map(|pos| open.block_start + pos)
            .unwrap_or(content.len());
        let opening_text = content[open.block_start..opening_end].to_string();
        return Err(BlockPairError::UnterminatedBlock {
            line: open.start_line,
            opening_text,
            file_ends_at_line: line_number.saturating_sub(1),
        });
    }

    Ok(pairs)
}

fn finish(open: OpenRegion, closer_start: usize, closer_end: usize, span_end: usize, end_line: usize) -> BlockPair {
    BlockPair {
        kind: open.kind,
        span: open.block_start..span_end,
        body_span: open.body_start..closer_start,
        start_line: open.start_line,
        end_line,
        opening_text: open.opening_text,
        opener_span: open.block_start..open.opener_end,
        closer_span: closer_start..closer_end,
        quoted: open.quoted,
        container: open.container,
    }
}

fn crossed(line: usize, closer: &str, open: &OpenRegion) -> BlockPairError {
    BlockPairError::CrossedClose {
        line,
        closer: closer.to_string(),
        open_line: open.start_line,
        open_text: open.opening_text.trim().to_string(),
    }
}

/// Errors that can occur while scanning region pairs.
#[derive(Debug, thiserror::Error)]
pub(crate) enum BlockPairError {
    /// `::end-block` appeared without a matching opener.
    #[error("Unmatched ::end-block at line {line}")]
    UnmatchedEnd { line: usize },

    /// `::end-map` / `::end-filter` appeared with no open region.
    #[error("Unmatched {closer} at line {line}")]
    UnmatchedCollectionEnd { line: usize, closer: &'static str },

    /// A closer met the wrong region on top of the stack: a different kind, or
    /// the same kind in a different container path. `line` is the closer;
    /// `open_line` is the still-open inner region.
    #[error(
        "'{closer}' at line {line} does not close the open region '{open_text}' from line {open_line}"
    )]
    CrossedClose {
        line: usize,
        closer: String,
        open_line: usize,
        open_text: String,
    },

    /// EOF reached with an unclosed region; `line` anchors the opener.
    #[error("Unterminated block starting at line {line} (file ends at line {file_ends_at_line})")]
    UnterminatedBlock {
        line: usize,
        opening_text: String,
        file_ends_at_line: usize,
    },

    /// `::end-block` had trailing non-whitespace content.
    #[error("Unexpected content after ::end-block at line {line}: '{content}'")]
    TrailingContent { line: usize, content: String },

    /// `::end-map` / `::end-filter` had trailing non-whitespace content.
    #[error("Unexpected content after {closer} at line {line}: '{content}'")]
    TrailingCollectionContent {
        line: usize,
        closer: &'static str,
        content: String,
    },

    /// A `::map` / `::filter` opener appeared inside a shell-block command body.
    #[error("'{directive}' at line {line} is not allowed inside the shell block opened at line {shell_line}")]
    CollectionInShellBody {
        line: usize,
        directive: String,
        shell_line: usize,
    },
}

impl BlockPairError {
    /// The 1-based line the error is reported at: the closer for unmatched and
    /// crossed closers, the opener for an unterminated region.
    pub(crate) fn line(&self) -> usize {
        match self {
            BlockPairError::UnmatchedEnd { line }
            | BlockPairError::UnmatchedCollectionEnd { line, .. }
            | BlockPairError::CrossedClose { line, .. }
            | BlockPairError::UnterminatedBlock { line, .. }
            | BlockPairError::TrailingContent { line, .. }
            | BlockPairError::TrailingCollectionContent { line, .. }
            | BlockPairError::CollectionInShellBody { line, .. } => *line,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_shell_block() {
        let content = "::shell-block\necho hi\n::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].kind, BlockOpenKind::Shell);
        assert_eq!(pairs[0].start_line, 1);
        assert_eq!(pairs[0].end_line, 3);
        assert_eq!(&content[pairs[0].body_span.clone()], "echo hi\n");
    }

    #[test]
    fn single_page_block() {
        let content = "::block when=\"x\"\nbody\n::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].kind, BlockOpenKind::Page);
    }

    #[test]
    fn blockquote_marked_shell_block_is_paired() {
        // `>`-led opener and closer are recognized once their block-quote markers
        // are skipped, so a block-quoted shell block pairs like any other.
        let content = "> ::shell-block\n> echo hi\n> ::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].kind, BlockOpenKind::Shell);
        assert_eq!(&content[pairs[0].body_span.clone()], "> echo hi\n");
    }

    #[test]
    fn nested_blockquote_marked_shell_block_is_paired() {
        // A `> > `-led opener/closer pairs once its nested block-quote markers
        // are skipped, mirroring the single-marker case.
        let content = "> > ::shell-block\n> > echo hi\n> > ::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].kind, BlockOpenKind::Shell);
        assert_eq!(&content[pairs[0].body_span.clone()], "> > echo hi\n");
    }

    #[test]
    fn blockquote_marked_page_block_is_not_paired() {
        // Block-quote support is scoped to shell blocks: a quoted `> ::block`
        // region stays literal Markdown (e.g. a documentation example). The
        // opener is not recognized, and the quoted `> ::end-block` is skipped
        // rather than treated as a closer — so no pair and no error.
        let content = "> ::block when=\"x\"\n> body\n> ::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn mixed_page_and_shell_blocks() {
        let content = "::block\n::shell-block\nbody\n::end-block\n::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 2);
        // Inner shell block is popped first
        assert_eq!(pairs[0].kind, BlockOpenKind::Shell);
        assert_eq!(pairs[1].kind, BlockOpenKind::Page);
    }

    #[test]
    fn unmatched_end_block() {
        let content = "::end-block\n";
        let result = scan_block_pairs(content);
        assert!(matches!(
            result,
            Err(BlockPairError::UnmatchedEnd { line: 1 })
        ));
    }

    #[test]
    fn unterminated_block() {
        let content = "::shell-block\nbody\n";
        let result = scan_block_pairs(content);
        assert!(matches!(
            result,
            Err(BlockPairError::UnterminatedBlock { line: 1, .. })
        ));
    }

    #[test]
    fn trailing_content_on_end_block() {
        let content = "::shell-block\nbody\n::end-block extra\n";
        let result = scan_block_pairs(content);
        assert!(matches!(
            result,
            Err(BlockPairError::TrailingContent { line: 3, .. })
        ));
    }

    #[test]
    fn block_inside_fenced_code_ignored() {
        let content = "```\n::shell-block\n::end-block\n```\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert!(pairs.is_empty());
    }

    #[test]
    fn nested_blocks_correct_order() {
        let content = "::block\nL1\n::shell-block\nL2\n::end-block\n::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 2);
        // Inner shell block
        assert_eq!(pairs[0].kind, BlockOpenKind::Shell);
        assert_eq!(pairs[0].start_line, 3);
        assert_eq!(pairs[0].end_line, 5);
        // Outer page block
        assert_eq!(pairs[1].kind, BlockOpenKind::Page);
        assert_eq!(pairs[1].start_line, 1);
        assert_eq!(pairs[1].end_line, 6);
    }

    #[test]
    fn empty_block_body() {
        let content = "::shell-block\n::end-block\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(&content[pairs[0].body_span.clone()], "");
    }

    #[test]
    fn crlf_spans_include_actual_line_endings_without_polluting_opening_text() {
        let content = "::shell-block\r\necho hi\r\n::end-block\r\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].span, 0..content.len());
        assert_eq!(&content[pairs[0].body_span.clone()], "echo hi\r\n");
        assert_eq!(pairs[0].opening_text, "::shell-block");
    }

    #[test]
    fn block_at_start_no_trailing_newline() {
        let content = "::shell-block\nbody\n::end-block";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].span.end, content.len());
    }

    #[test]
    fn block_directive_not_confused_with_similar_names() {
        let content = "::blockquote something\ntext\n::end-block\n";
        let result = scan_block_pairs(content);
        assert!(matches!(result, Err(BlockPairError::UnmatchedEnd { .. })));
    }

    #[test]
    fn shell_block_not_confused_with_longer_prefix() {
        let content = "::shell-blocker\nbody\n::end-block\n";
        let result = scan_block_pairs(content);
        assert!(matches!(result, Err(BlockPairError::UnmatchedEnd { .. })));
    }

    fn kinds(pairs: &[BlockPair]) -> Vec<BlockOpenKind> {
        pairs.iter().map(|pair| pair.kind).collect()
    }

    #[test]
    fn map_and_filter_pair_with_distinct_closers() {
        let content = "::map x\nA\n::end-map\n::filter y\nB\n::end-filter\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(kinds(&pairs), vec![BlockOpenKind::Map, BlockOpenKind::Filter]);
        assert_eq!(&content[pairs[0].body_span.clone()], "A\n");
        assert_eq!(pairs[0].opening_text, "::map x");
        assert_eq!(&content[pairs[1].closer_span.clone()], "::end-filter");
    }

    #[test]
    fn collection_nests_with_page_and_shell_on_one_stack() {
        let content = "::map x\n::block\n::shell-block\necho\n::end-block\n::end-block\n::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(
            kinds(&pairs),
            vec![BlockOpenKind::Shell, BlockOpenKind::Page, BlockOpenKind::Map]
        );
    }

    #[test]
    fn collection_keyword_prefixes_are_not_directives() {
        let content = "::mapper x\n::filters\n::end-mapx\n";
        assert!(scan_block_pairs(content).unwrap().is_empty());
    }

    #[test]
    fn collection_closer_accepts_trailing_whitespace_only() {
        assert!(scan_block_pairs("::map x\n::end-map   \n").is_ok());
        let error = scan_block_pairs("::map x\n::end-map extra\n").unwrap_err();
        assert!(matches!(
            error,
            BlockPairError::TrailingCollectionContent { line: 2, closer: "::end-map", .. }
        ));
    }

    #[test]
    fn unclosed_collection_anchors_the_opener() {
        let error = scan_block_pairs("text\n::filter y\nbody\n").unwrap_err();
        assert!(matches!(
            error,
            BlockPairError::UnterminatedBlock { line: 2, ref opening_text, .. } if opening_text == "::filter y"
        ));
    }

    #[test]
    fn unmatched_collection_closer_is_reported() {
        let error = scan_block_pairs("::end-filter\n").unwrap_err();
        assert!(matches!(
            error,
            BlockPairError::UnmatchedCollectionEnd { line: 1, closer: "::end-filter" }
        ));
    }

    #[test]
    fn mismatched_closer_kind_names_closer_and_open_region() {
        let error = scan_block_pairs("::map x\nbody\n::end-filter\n").unwrap_err();
        match error {
            BlockPairError::CrossedClose { line, closer, open_line, open_text } => {
                assert_eq!((line, open_line), (3, 1));
                assert_eq!(closer, "::end-filter");
                assert_eq!(open_text, "::map x");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn crossed_collections_are_errors() {
        let error = scan_block_pairs("::map a\n::filter b\n::end-map\n::end-filter\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 3, open_line: 2, .. }));
    }

    #[test]
    fn end_block_cannot_close_a_collection() {
        let error = scan_block_pairs("::map a\n::end-block\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 2, open_line: 1, .. }));
    }

    #[test]
    fn end_map_cannot_close_a_page_block() {
        let error = scan_block_pairs("::block\n::end-map\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 2, open_line: 1, .. }));
    }

    #[test]
    fn collection_opener_in_shell_body_is_rejected() {
        let error = scan_block_pairs("::shell-block\n::map x\n::end-map\n::end-block\n").unwrap_err();
        assert!(matches!(
            error,
            BlockPairError::CollectionInShellBody { line: 2, shell_line: 1, .. }
        ));
        let quoted = scan_block_pairs("> ::shell-block\n> ::filter x\n").unwrap_err();
        assert!(matches!(quoted, BlockPairError::CollectionInShellBody { line: 2, .. }));
    }

    #[test]
    fn shell_block_inside_collection_is_allowed() {
        let content = "::map x\n::shell-block\necho\n::end-block\n::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(kinds(&pairs), vec![BlockOpenKind::Shell, BlockOpenKind::Map]);
    }

    #[test]
    fn list_item_continuation_body_pairs() {
        let content = "- item\n\n  ::map x\n  body\n\n  ::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(&content[pairs[0].body_span.clone()], "  body\n\n");
        assert!(pairs[0].container.len() >= 2);
    }

    #[test]
    fn closer_outside_list_item_does_not_close_inner_region() {
        let content = "- item\n\n  ::map x\n  body\n\n::end-map\n";
        let error = scan_block_pairs(content).unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 6, open_line: 3, .. }));
    }

    #[test]
    fn sibling_list_items_are_distinct_containers() {
        let content = "- a\n\n  ::map x\n\n- b\n\n  ::end-map\n";
        let error = scan_block_pairs(content).unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 7, .. }));
    }

    #[test]
    fn closer_after_a_list_in_the_body_is_not_a_lazy_continuation() {
        let content = "::map x\n- a\n- b\n::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(pairs.len(), 1);
        assert!(pairs[0].container.is_empty());
    }

    #[test]
    fn quoted_collection_is_active_within_its_quote() {
        let content = "> ::map x\n> body\n> ::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(kinds(&pairs), vec![BlockOpenKind::Map]);
        assert!(pairs[0].quoted);
        assert_eq!(&content[pairs[0].body_span.clone()], "> body\n");
    }

    #[test]
    fn quote_depth_must_match() {
        let error = scan_block_pairs("> > ::map x\n> > b\n> ::end-map\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 3, open_line: 1, .. }));
        assert!(scan_block_pairs("> > ::map x\n> > b\n> > ::end-map\n").is_ok());
    }

    #[test]
    fn unquoted_closer_cannot_close_quoted_collection() {
        let error = scan_block_pairs("> ::map x\n> b\n::end-map\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 3, open_line: 1, .. }));
    }

    #[test]
    fn separate_quotes_are_distinct_containers() {
        let error = scan_block_pairs("> ::map x\n\n> ::end-map\n").unwrap_err();
        assert!(matches!(error, BlockPairError::CrossedClose { line: 3, .. }));
    }

    #[test]
    fn quote_inside_list_item_and_list_inside_quote_pair() {
        let in_item = "- a\n\n  > ::map x\n  > b\n  > ::end-map\n";
        assert_eq!(scan_block_pairs(in_item).unwrap().len(), 1);
        let in_quote = "> - a\n>\n>   ::map x\n>   b\n>   ::end-map\n";
        let pairs = scan_block_pairs(in_quote).unwrap();
        assert_eq!(pairs.len(), 1);
        assert_eq!(pairs[0].container.len(), 3);
        let wrong = "> - a\n>\n>   ::map x\n>   b\n> ::end-map\n";
        assert!(matches!(
            scan_block_pairs(wrong).unwrap_err(),
            BlockPairError::CrossedClose { line: 5, .. }
        ));
    }

    #[test]
    fn quoted_page_example_stays_literal_inside_quoted_collection() {
        let content = "> ::map x\n> ::block when=\"a\"\n> body\n> ::end-block\n> ::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(kinds(&pairs), vec![BlockOpenKind::Map]);
    }

    #[test]
    fn quoted_shell_block_is_active_inside_quoted_collection() {
        let content = "> ::map x\n> ::shell-block\n> echo\n> ::end-block\n> ::end-map\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(kinds(&pairs), vec![BlockOpenKind::Shell, BlockOpenKind::Map]);
    }

    #[test]
    fn collection_in_fenced_and_indented_code_is_ignored() {
        let fenced = "```\n::map x\n```\n";
        assert!(scan_block_pairs(fenced).unwrap().is_empty());
        let indented = "text\n\n    ::map x\n\nmore\n";
        assert!(scan_block_pairs(indented).unwrap().is_empty());
        let in_item = "- a\n\n      ::map x\n";
        assert!(scan_block_pairs(in_item).unwrap().is_empty());
    }

    #[test]
    fn collection_crlf_and_eof_closer_without_newline() {
        let crlf = "::map x\r\nbody\r\n::end-map\r\n";
        let pairs = scan_block_pairs(crlf).unwrap();
        assert_eq!(&crlf[pairs[0].body_span.clone()], "body\r\n");
        assert_eq!(pairs[0].opening_text, "::map x");
        assert_eq!(pairs[0].span, 0..crlf.len());
        let bare = "::filter y\nbody\n::end-filter";
        let pairs = scan_block_pairs(bare).unwrap();
        assert_eq!(pairs[0].span.end, bare.len());
        assert_eq!(&bare[pairs[0].closer_span.clone()], "::end-filter");
    }

    #[test]
    fn empty_collection_body_is_empty_span() {
        let content = "a\n::map x\n::end-map\nb\n";
        let pairs = scan_block_pairs(content).unwrap();
        assert_eq!(&content[pairs[0].body_span.clone()], "");
        assert_eq!(&content[pairs[0].span.clone()], "::map x\n::end-map\n");
    }
}

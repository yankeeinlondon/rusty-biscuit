//! Shared block pairing scanner for `::block` and `::shell-block` directives.
//!
//! Provides a unified stack-based scanner that recognizes both page-block
//! openers (`::block`) and shell-block openers (`::shell-block`) and pairs
//! them with `::end-block` closers.  This ensures the two block families
//! share one stack so nesting interleaving is handled unambiguously.

use std::ops::Range;

use super::parse_utils::{find_code_regions, is_in_code_region};

/// Which kind of block opened a pair.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) enum BlockOpenKind {
    /// A `::block` page-block opener.
    Page,
    /// A `::shell-block` shell-block opener.
    Shell,
}

/// A fully-paired block region discovered by the shared scanner.
///
/// The scanner returns these in document order (by `span.start`).  Both
/// `Page` and `Shell` kinds are mixed in the same vector; callers filter
/// by `kind` when they only need one family.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub(crate) struct BlockPair {
    /// Which kind of block opened this pair.
    pub kind: BlockOpenKind,
    /// Full byte span including the opener and closer lines.
    pub span: Range<usize>,
    /// Byte span of the body (between opener and closer, excluding both
    /// directive lines).
    pub body_span: Range<usize>,
    /// 1-based line number of the opening directive.
    pub start_line: usize,
    /// 1-based line number of the closing directive.
    pub end_line: usize,
    /// The raw text of the opening directive line (useful for diagnostics).
    pub opening_text: String,
}

/// Scans `content` for paired `::block` / `::shell-block` / `::end-block`
/// directives and returns an ordered vector of [`BlockPair`].
///
/// ## Behaviour
///
/// * Skips fenced code regions and inline code spans via
///   [`find_code_regions`](super::parse_utils::find_code_regions).
/// * Pushes both `::block` and `::shell-block` openers onto a single stack.
/// * Pops the most recent opener on every `::end-block`.
/// * Rejects `::end-block` lines that contain trailing non-whitespace text.
/// * Returns [`BlockPairError::UnmatchedEnd`] when `::end-block` has no
///   matching opener.
/// * Returns [`BlockPairError::UnterminatedBlock`] when EOF is reached with
///   an open block still on the stack.
///
/// ## Examples
///
/// ```ignore
/// use darkmatter::markdown::compose::block_pairs::{scan_block_pairs, BlockOpenKind};
///
/// let content = "::shell-block\necho hi\n::end-block\n";
/// let pairs = scan_block_pairs(content).unwrap();
/// assert_eq!(pairs.len(), 1);
/// assert!(matches!(pairs[0].kind, BlockOpenKind::Shell));
/// ```
#[allow(dead_code)]
pub(crate) fn scan_block_pairs(content: &str) -> Result<Vec<BlockPair>, BlockPairError> {
    let code_regions = find_code_regions(content);
    let bytes = content.as_bytes();

    // Stack entries: (block_start_byte, body_start_byte, start_line, kind, opening_text)
    let mut stack: Vec<(usize, usize, usize, BlockOpenKind, String)> = Vec::new();
    let mut pairs: Vec<BlockPair> = Vec::new();

    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let span_end = if i < bytes.len() { i + 1 } else { i };
        let line = &content[line_start..line_end];
        let trimmed = line.trim();

        if !trimmed.is_empty() {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                if let Some(after) = trimmed.strip_prefix("::block") {
                    // Make sure it's not ::blockquote or similar
                    if after.is_empty() || after.starts_with(char::is_whitespace) {
                        let body_start = span_end;
                        let opening_text = content[line_start..line_end].to_string();
                        stack.push((
                            line_start,
                            body_start,
                            line_number,
                            BlockOpenKind::Page,
                            opening_text,
                        ));
                    }
                } else if let Some(after) = trimmed.strip_prefix("::shell-block") {
                    // Make sure it's not a longer prefix like ::shell-blocker
                    if after.is_empty() || after.starts_with(char::is_whitespace) {
                        let body_start = span_end;
                        let opening_text = content[line_start..line_end].to_string();
                        stack.push((
                            line_start,
                            body_start,
                            line_number,
                            BlockOpenKind::Shell,
                            opening_text,
                        ));
                    }
                } else if let Some(after) = trimmed.strip_prefix("::end-block") {
                    let trailing = after.trim();
                    if !trailing.is_empty() {
                        return Err(BlockPairError::TrailingContent {
                            line: line_number,
                            content: trailing.to_string(),
                        });
                    }

                    let Some((block_start, body_start, start_line, kind, opening_text)) =
                        stack.pop()
                    else {
                        return Err(BlockPairError::UnmatchedEnd { line: line_number });
                    };

                    pairs.push(BlockPair {
                        kind,
                        span: block_start..span_end,
                        body_span: body_start..line_start,
                        start_line,
                        end_line: line_number,
                        opening_text,
                    });
                }
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    // Check for unterminated blocks (report the deepest open one)
    if let Some((block_start, _, start_line, _, _)) = stack.last() {
        let opening_end = content[*block_start..]
            .find('\n')
            .map(|pos| block_start + pos)
            .unwrap_or(content.len());
        let opening_text = content[*block_start..opening_end].to_string();
        return Err(BlockPairError::UnterminatedBlock {
            line: *start_line,
            opening_text,
            file_ends_at_line: line_number.saturating_sub(1),
        });
    }

    Ok(pairs)
}

/// Errors that can occur while scanning block pairs.
#[derive(Debug, thiserror::Error)]
#[allow(dead_code)]
pub(crate) enum BlockPairError {
    /// `::end-block` appeared without a matching opener.
    #[error("Unmatched ::end-block at line {line}")]
    UnmatchedEnd { line: usize },

    /// EOF reached with an unclosed block.
    #[error("Unterminated block starting at line {line} (file ends at line {file_ends_at_line})")]
    UnterminatedBlock {
        line: usize,
        opening_text: String,
        file_ends_at_line: usize,
    },

    /// `::end-block` had trailing non-whitespace content.
    #[error("Unexpected content after ::end-block at line {line}: '{content}'")]
    TrailingContent { line: usize, content: String },
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
        assert!(matches!(result, Err(BlockPairError::UnmatchedEnd { line: 1 })));
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
}

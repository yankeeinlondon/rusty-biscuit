//! Parser for `::block` / `::end-block` paired directives.

use super::types::{PageBlockError, PageBlockOptions, PageBlockRegion};
use crate::markdown::compose::parse_utils::{Cursor, CursorError};

/// Internal helper for building the region tree.
struct UnfinishedRegion {
    end_line: usize,
    region: PageBlockRegion,
    children: Vec<PageBlockRegion>,
}

/// Parses page block directives from markdown content into a nested region tree.
///
/// Delegates the low-level block pairing to [`scan_block_pairs`](super::super::block_pairs::scan_block_pairs)
/// so that `::shell-block` regions are handled correctly and do not interfere with
/// page-block parsing.
///
/// ## Errors
///
/// Returns `PageBlockError::UnmatchedEnd` if `::end-block` appears without
/// a matching `::block`, and `PageBlockError::UnterminatedBlock` if EOF is
/// reached with an open block.
pub fn parse_page_blocks(content: &str) -> Result<Vec<PageBlockRegion>, PageBlockError> {
    let pairs = super::super::block_pairs::scan_block_pairs(content)
        .map_err(|e| match e {
            super::super::block_pairs::BlockPairError::UnmatchedEnd { line } => {
                PageBlockError::UnmatchedEnd { line }
            }
            super::super::block_pairs::BlockPairError::UnterminatedBlock {
                line,
                opening_text,
                file_ends_at_line,
            } => PageBlockError::UnterminatedBlock {
                line,
                opening_text,
                file_ends_at_line,
            },
            super::super::block_pairs::BlockPairError::TrailingContent { line, content } => {
                PageBlockError::ParseDirective {
                    line,
                    message: format!("Unexpected content after ::end-block: '{content}'"),
                }
            }
        })?;

    // Filter to page blocks only and sort by start line (document order).
    let mut page_pairs: Vec<_> = pairs
        .into_iter()
        .filter(|p| matches!(p.kind, super::super::block_pairs::BlockOpenKind::Page))
        .collect();
    page_pairs.sort_by_key(|p| p.start_line);

    let mut top_level: Vec<PageBlockRegion> = Vec::new();
    let mut unfinished: Vec<Option<UnfinishedRegion>> = Vec::new();
    let mut stack: Vec<usize> = Vec::new();

    for pair in page_pairs {
        let options = parse_block_options_from_opener(&pair.opening_text, pair.start_line)?;
        let region = PageBlockRegion {
            span: pair.span.clone(),
            body_span: pair.body_span.clone(),
            start_line: pair.start_line,
            end_line: pair.end_line,
            options,
            children: Vec::new(),
        };

        // Pop regions that ended before the current one starts (siblings, not ancestors).
        while let Some(&top_idx) = stack.last() {
            let top = unfinished[top_idx].as_ref().unwrap();
            if pair.start_line > top.end_line {
                stack.pop();
                let finished = unfinished[top_idx].take().unwrap();
                let final_region = PageBlockRegion {
                    children: finished.children,
                    ..finished.region
                };
                if let Some(&parent_idx) = stack.last() {
                    unfinished[parent_idx]
                        .as_mut()
                        .unwrap()
                        .children
                        .push(final_region);
                } else {
                    top_level.push(final_region);
                }
            } else {
                break;
            }
        }

        unfinished.push(Some(UnfinishedRegion {
            end_line: pair.end_line,
            region,
            children: Vec::new(),
        }));
        stack.push(unfinished.len() - 1);
    }

    // Drain any remaining regions on the stack.
    while let Some(top_idx) = stack.pop() {
        let finished = unfinished[top_idx].take().unwrap();
        let final_region = PageBlockRegion {
            children: finished.children,
            ..finished.region
        };
        if let Some(&parent_idx) = stack.last() {
            unfinished[parent_idx]
                .as_mut()
                .unwrap()
                .children
                .push(final_region);
        } else {
            top_level.push(final_region);
        }
    }

    Ok(top_level)
}

/// Parses options from the raw opening text of a `BlockPair`.
fn parse_block_options_from_opener(
    opener: &str,
    line: usize,
) -> Result<PageBlockOptions, PageBlockError> {
    let after = opener
        .find("::block")
        .map(|idx| &opener[idx + "::block".len()..])
        .unwrap_or(opener)
        .trim_start();
    parse_block_options(after, line)
}

/// Parses the option part of a `::block` directive line (everything after `::block`).
fn parse_block_options(input: &str, line: usize) -> Result<PageBlockOptions, PageBlockError> {
    let mut options = PageBlockOptions {
        when_expr: None,
        unknown_options: Vec::new(),
    };

    if input.is_empty() {
        return Ok(options);
    }

    let mut cursor = Cursor::new(input);

    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let key = cursor.read_identifier(line)?;
        cursor.skip_ws();
        cursor.expect_char('=', line)?;
        cursor.skip_ws();
        let value = cursor.read_value(line)?;

        match key.as_str() {
            "when" => {
                options.when_expr = Some(value);
            }
            _ => {
                options.unknown_options.push(key);
            }
        }
    }

    Ok(options)
}

impl From<CursorError> for PageBlockError {
    fn from(e: CursorError) -> Self {
        PageBlockError::ParseDirective {
            line: e.line,
            message: e.message,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_valid_block() {
        let content = "before\n::block when=\"x == 'y'\"\nbody line\n::end-block\nafter\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);

        let r = &regions[0];
        assert_eq!(r.start_line, 2);
        assert_eq!(r.end_line, 4);
        assert_eq!(r.options.when_expr, Some("x == 'y'".to_string()));
        assert_eq!(&content[r.body_span.clone()], "body line\n");
    }

    #[test]
    fn multiple_sibling_blocks() {
        let content =
            "::block when=\"a\"\nfirst\n::end-block\n::block when=\"b\"\nsecond\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 2);
        assert_eq!(regions[0].options.when_expr, Some("a".to_string()));
        assert_eq!(regions[1].options.when_expr, Some("b".to_string()));
        assert_eq!(&content[regions[0].body_span.clone()], "first\n");
        assert_eq!(&content[regions[1].body_span.clone()], "second\n");
    }

    #[test]
    fn nested_blocks() {
        let content = "::block when=\"outer\"\nouter body\n::block when=\"inner\"\ninner body\n::end-block\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].children.len(), 1);

        let inner = &regions[0].children[0];
        assert_eq!(inner.options.when_expr, Some("inner".to_string()));
        assert_eq!(&content[inner.body_span.clone()], "inner body\n");
    }

    #[test]
    fn block_inside_fenced_code_ignored() {
        let content = "```\n::block when=\"x\"\n::end-block\n```\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 0);
    }

    #[test]
    fn unmatched_end_block_error() {
        let content = "::end-block\n";
        let result = parse_page_blocks(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, PageBlockError::UnmatchedEnd { line: 1 }));
    }

    #[test]
    fn unterminated_block_error() {
        let content = "::block when=\"x\"\nbody\n";
        let result = parse_page_blocks(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(
            err,
            PageBlockError::UnterminatedBlock { line: 1, .. }
        ));
    }

    #[test]
    fn unknown_attributes_captured() {
        let content = "::block foo=\"bar\" when=\"x\" baz=\"qux\"\nbody\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].options.when_expr, Some("x".to_string()));
        assert_eq!(
            regions[0].options.unknown_options,
            vec!["foo".to_string(), "baz".to_string()]
        );
    }

    #[test]
    fn block_with_no_attributes() {
        let content = "::block\nbody\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert!(regions[0].options.when_expr.is_none());
    }

    #[test]
    fn end_block_with_trailing_content_error() {
        let content = "::block\nbody\n::end-block extra\n";
        let result = parse_page_blocks(content);
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PageBlockError::ParseDirective { line: 3, .. }
        ));
    }

    #[test]
    fn empty_block_body() {
        let content = "::block when=\"x\"\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(&content[regions[0].body_span.clone()], "");
    }

    // ── Regression / edge case tests ────────────────────────────────────

    #[test]
    fn block_at_very_start_of_file() {
        let content = "::block when=\"x\"\nfirst line\n::end-block\ntrailing\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].span.start, 0);
        assert_eq!(regions[0].start_line, 1);
    }

    #[test]
    fn block_at_very_end_of_file_no_trailing_newline() {
        let content = "leading\n::block\nbody\n::end-block";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].span.end, content.len());
    }

    #[test]
    fn adjacent_blocks_no_content_between() {
        let content = "::block when=\"a\"\nA\n::end-block\n::block when=\"b\"\nB\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 2);
        // First block's span end should equal second block's span start
        assert_eq!(regions[0].span.end, regions[1].span.start);
    }

    #[test]
    fn block_with_only_whitespace_body() {
        let content = "::block\n   \n\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(&content[regions[0].body_span.clone()], "   \n\n");
    }

    #[test]
    fn deeply_nested_blocks_three_levels() {
        let content =
            "::block\nL1\n::block\nL2\n::block\nL3\n::end-block\n::end-block\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].children.len(), 1);
        assert_eq!(regions[0].children[0].children.len(), 1);
        let l3 = &regions[0].children[0].children[0];
        assert_eq!(&content[l3.body_span.clone()], "L3\n");
    }

    #[test]
    fn multiple_unknown_attributes_all_captured() {
        let content = "::block alpha=\"1\" beta=\"2\" gamma=\"3\"\nbody\n::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(
            regions[0].options.unknown_options,
            vec!["alpha", "beta", "gamma"]
        );
    }

    #[test]
    fn block_directive_not_confused_with_similar_names() {
        // ::blockquote should NOT be treated as ::block
        let content = "::blockquote something\ntext\n::end-block\n";
        let result = parse_page_blocks(content);
        // Should get UnmatchedEnd since ::blockquote is not a block opener
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            PageBlockError::UnmatchedEnd { .. }
        ));
    }

    #[test]
    fn indented_block_directives() {
        let content = "  ::block when=\"x\"\n  body\n  ::end-block\n";
        let regions = parse_page_blocks(content).unwrap();
        assert_eq!(regions.len(), 1);
        assert_eq!(regions[0].options.when_expr, Some("x".to_string()));
    }
}

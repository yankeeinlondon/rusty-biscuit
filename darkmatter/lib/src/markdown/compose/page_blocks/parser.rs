//! Parser for `::block` / `::end-block` paired directives.

use super::types::{PageBlockError, PageBlockOptions, PageBlockRegion};
use crate::markdown::compose::parse_utils::{
    Cursor, CursorError, find_code_regions, is_in_code_region,
};

/// Parses page block directives from markdown content into a nested region tree.
///
/// Scans the document line by line, skipping fenced code regions, and builds
/// a tree of `PageBlockRegion` with exact byte spans for the full block
/// (including directive lines) and the body (excluding directive lines).
///
/// ## Errors
///
/// Returns `PageBlockError::UnmatchedEnd` if `::end-block` appears without
/// a matching `::block`, and `PageBlockError::UnterminatedBlock` if EOF is
/// reached with an open block.
pub fn parse_page_blocks(content: &str) -> Result<Vec<PageBlockRegion>, PageBlockError> {
    let code_regions = find_code_regions(content);
    let bytes = content.as_bytes();

    // Stack entries: (block_start_byte, body_start_byte, start_line, options, children)
    let mut stack: Vec<(usize, usize, usize, PageBlockOptions, Vec<PageBlockRegion>)> = Vec::new();
    let mut top_level: Vec<PageBlockRegion> = Vec::new();

    let mut line_start = 0usize;
    let mut line_number = 1usize;

    for i in 0..=bytes.len() {
        let is_eol = i == bytes.len() || bytes[i] == b'\n';
        if !is_eol {
            continue;
        }

        let line_end = i;
        let span_end = if i < bytes.len() { i + 1 } else { i }; // include the newline
        let line = &content[line_start..line_end];
        let trimmed = line.trim();

        if !trimmed.is_empty() {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                if let Some(after) = trimmed.strip_prefix("::block") {
                    // Check it's actually `::block` and not `::blockquote` etc.
                    if after.is_empty() || after.starts_with(char::is_whitespace) {
                        let options = parse_block_options(after.trim(), line_number)?;
                        let body_start = span_end; // body starts after the ::block line
                        stack.push((line_start, body_start, line_number, options, Vec::new()));
                    }
                } else if let Some(after) = trimmed.strip_prefix("::end-block") {
                    // Validate no trailing non-whitespace content
                    let trailing = after.trim();
                    if !trailing.is_empty() {
                        return Err(PageBlockError::ParseDirective {
                            line: line_number,
                            message: format!(
                                "Unexpected content after ::end-block: '{}'",
                                trailing
                            ),
                        });
                    }

                    let Some((block_start, body_start, start_line, options, children)) =
                        stack.pop()
                    else {
                        return Err(PageBlockError::UnmatchedEnd { line: line_number });
                    };

                    let region = PageBlockRegion {
                        span: block_start..span_end,
                        body_span: body_start..line_start, // body ends at start of ::end-block line
                        start_line,
                        end_line: line_number,
                        options,
                        children,
                    };

                    if let Some(parent) = stack.last_mut() {
                        parent.4.push(region);
                    } else {
                        top_level.push(region);
                    }
                }
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    // Check for unterminated blocks (report the deepest open one)
    if let Some(entry) = stack.last() {
        return Err(PageBlockError::UnterminatedBlock { line: entry.2 });
    }

    Ok(top_level)
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
        assert!(matches!(err, PageBlockError::UnterminatedBlock { line: 1 }));
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

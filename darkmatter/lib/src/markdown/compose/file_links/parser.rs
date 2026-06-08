//! Parser for `::file-links` directives.
//!
//! Recognizes exactly two source forms:
//!
//! - `::file-links <glob>`
//! - `::file-links --dir <path> [--depth <u32>]`
//!
//! Lines inside fenced or indented code blocks are ignored. Directive spans
//! and indentation are preserved for later replacement.

use super::types::{FileLinksDirective, FileLinksError, FileLinksMode, DEFAULT_DIR_DEPTH};
use crate::markdown::compose::parse_utils::{Cursor, find_code_regions, is_in_code_region};

/// Parses all `::file-links` directives from markdown content.
pub fn parse_file_links_directives(
    content: &str,
) -> Result<Vec<FileLinksDirective>, FileLinksError> {
    let code_regions = find_code_regions(content);
    let mut directives = Vec::new();

    let bytes = content.as_bytes();
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

        if trimmed.starts_with("::file-links") {
            let first_non_ws = line_start + line.len().saturating_sub(line.trim_start().len());
            if !is_in_code_region(first_non_ws, &code_regions) {
                let indent = &content[line_start..first_non_ws];
                let inferred_indent = if indent.is_empty() {
                    infer_indent_from_previous_line(content, line_start)
                } else {
                    None
                };
                let directive = parse_directive_line(
                    trimmed,
                    line_start..span_end,
                    line_number,
                    indent.to_string(),
                    inferred_indent,
                )?;
                directives.push(directive);
            }
        }

        line_start = i.saturating_add(1);
        line_number += 1;
    }

    Ok(directives)
}

/// Scan backward from `pos` to infer the indentation a flush-left directive
/// should inherit — mirrors the `::toc-linking` behaviour so list-item
/// continuations keep their nesting.
fn infer_indent_from_previous_line(content: &str, pos: usize) -> Option<String> {
    let before = &content[..pos];
    for line in before.lines().rev() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        if let Some(content_col) = list_item_content_column(line) {
            return Some(" ".repeat(content_col));
        }

        let leading_ws_len = line.len() - line.trim_start().len();
        if leading_ws_len > 0 {
            return Some(line[..leading_ws_len].to_string());
        }
        return None;
    }
    None
}

/// Byte column where a CommonMark list item's content begins.
fn list_item_content_column(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    let marker_start = i;

    if i < bytes.len() && matches!(bytes[i], b'-' | b'+' | b'*') {
        i += 1;
    } else {
        let digit_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if i == digit_start || i >= bytes.len() || !matches!(bytes[i], b'.' | b')') {
            return None;
        }
        i += 1;
    }

    if i == marker_start {
        return None;
    }

    if i >= bytes.len() {
        return Some(i);
    }
    if bytes[i] != b' ' && bytes[i] != b'\t' {
        return None;
    }

    while i < bytes.len() && (bytes[i] == b' ' || bytes[i] == b'\t') {
        i += 1;
    }
    Some(i)
}

fn parse_directive_line(
    input: &str,
    span: std::ops::Range<usize>,
    line: usize,
    indent: String,
    inferred_indent: Option<String>,
) -> Result<FileLinksDirective, FileLinksError> {
    let mut cursor = Cursor::new(input);

    cursor.expect_literal("::file-links", line)?;
    cursor.skip_ws();

    // No positional or flag token at all.
    if cursor.is_eof() {
        return Err(FileLinksError::ParseDirective {
            line,
            message: "Directive requires a glob pattern or '--dir <path>'".to_string(),
        });
    }

    // `--dir` form: `::file-links --dir <path> [--depth <u32>]`
    if cursor.current() == Some('-') {
        return parse_dir_form(&mut cursor, span, line, indent, inferred_indent);
    }

    // Glob form: the remainder is the glob (bare or quoted).
    let glob = read_glob_value(&mut cursor, line)?;
    let glob = glob.trim();
    if glob.is_empty() {
        return Err(FileLinksError::ParseDirective {
            line,
            message: "Glob pattern cannot be empty".to_string(),
        });
    }
    reject_trailing_tokens(&mut cursor, line)?;

    Ok(FileLinksDirective {
        mode: FileLinksMode::Glob(glob.to_string()),
        span,
        line,
        indent,
        inferred_indent,
    })
}

/// Parses `--dir <path> [--depth <u32>]` after the directive name.
fn parse_dir_form(
    cursor: &mut Cursor<'_>,
    span: std::ops::Range<usize>,
    line: usize,
    indent: String,
    inferred_indent: Option<String>,
) -> Result<FileLinksDirective, FileLinksError> {
    let mut path: Option<String> = None;
    let mut depth: Option<u32> = None;
    let mut seen_dir = false;

    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        let flag = read_flag(cursor, line)?;

        match flag.as_str() {
            "--dir" => {
                if seen_dir {
                    return Err(FileLinksError::ParseDirective {
                        line,
                        message: "'--dir' specified more than once".to_string(),
                    });
                }
                seen_dir = true;
                cursor.skip_ws();
                let value = cursor.read_value(line)?;
                let value = value.trim();
                if value.is_empty() {
                    return Err(FileLinksError::ParseDirective {
                        line,
                        message: "'--dir' requires a path".to_string(),
                    });
                }
                path = Some(value.to_string());
            }
            "--depth" => {
                if depth.is_some() {
                    return Err(FileLinksError::ParseDirective {
                        line,
                        message: "'--depth' specified more than once".to_string(),
                    });
                }
                cursor.skip_ws();
                let value = cursor.read_value(line)?;
                let value = value.trim();
                let parsed: u32 = value.parse().map_err(|_| {
                    FileLinksError::ParseDirective {
                        line,
                        message: format!("'--depth' expects a non-negative integer, got '{value}'"),
                    }
                })?;
                depth = Some(parsed);
            }
            other => {
                return Err(FileLinksError::ParseDirective {
                    line,
                    message: format!("Unknown option '{other}'"),
                });
            }
        }
    }

    let Some(path) = path else {
        return Err(FileLinksError::ParseDirective {
            line,
            message: "'--depth' can only be used with '--dir'".to_string(),
        });
    };

    Ok(FileLinksDirective {
        mode: FileLinksMode::Dir {
            path,
            depth: depth.unwrap_or(DEFAULT_DIR_DEPTH),
        },
        span,
        line,
        indent,
        inferred_indent,
    })
}

/// Reads a `--flag` token, rejecting `--depth`-without-`--dir` ordering issues
/// at the caller (the value is consumed by the match arm).
fn read_flag(cursor: &mut Cursor<'_>, line: usize) -> Result<String, FileLinksError> {
    let mut out = String::new();
    if cursor.current() != Some('-') {
        return Err(FileLinksError::ParseDirective {
            line,
            message: "Expected '--dir' or '--depth', found something else".to_string(),
        });
    }
    while let Some(ch) = cursor.current() {
        if ch.is_whitespace() {
            break;
        }
        out.push(ch);
        cursor.advance();
    }
    Ok(out)
}

/// Errors when extra tokens remain after the glob value.
fn reject_trailing_tokens(cursor: &mut Cursor<'_>, line: usize) -> Result<(), FileLinksError> {
    cursor.skip_ws();
    if !cursor.is_eof() {
        let remainder = cursor.current().unwrap_or(' ');
        // A second flag indicates the user mixed glob and --dir/--depth.
        if remainder == '-' {
            return Err(FileLinksError::ParseDirective {
                line,
                message: "Glob form cannot be combined with '--dir' or '--depth'".to_string(),
            });
        }
        return Err(FileLinksError::ParseDirective {
            line,
            message: format!("Unexpected trailing token '{remainder}'"),
        });
    }
    Ok(())
}

/// Reads the glob value from the cursor.
///
/// Quoted globs (`"..."` / `'...'`) use the standard quoted reader to
/// support paths with spaces. Bare globs — including those starting with `[`
/// (character classes) — are read as a single whitespace-delimited token so
/// glob syntax is preserved instead of being mistaken for a JSON array by
/// the generic value reader. Trailing `--dir`/`--depth` tokens are caught by
/// [`reject_trailing_tokens`].
fn read_glob_value(cursor: &mut Cursor<'_>, line: usize) -> Result<String, FileLinksError> {
    cursor.skip_ws();
    match cursor.current() {
        Some('\'') | Some('"') => cursor.read_value(line).map_err(FileLinksError::from),
        _ => {
            let mut out = String::new();
            while let Some(ch) = cursor.current() {
                if ch.is_whitespace() {
                    break;
                }
                out.push(ch);
                cursor.advance();
            }
            Ok(out)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_glob_form() {
        let directives =
            parse_file_links_directives("::file-links docs/**/*.md\n").unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Glob("docs/**/*.md".to_string())
        );
    }

    #[test]
    fn parses_quoted_glob_with_spaces() {
        let directives =
            parse_file_links_directives("::file-links \"my docs/**/*.md\"\n").unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Glob("my docs/**/*.md".to_string())
        );
    }

    #[test]
    fn parses_dir_form_default_depth() {
        let directives =
            parse_file_links_directives("::file-links --dir docs\n").unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Dir {
                path: "docs".to_string(),
                depth: 0,
            }
        );
    }

    #[test]
    fn parses_dir_form_with_depth() {
        let directives =
            parse_file_links_directives("::file-links --dir docs --depth 3\n").unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Dir {
                path: "docs".to_string(),
                depth: 3,
            }
        );
    }

    #[test]
    fn parses_dir_form_quoted_path() {
        let directives =
            parse_file_links_directives("::file-links --dir \"my docs\"\n").unwrap();
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Dir {
                path: "my docs".to_string(),
                depth: 0,
            }
        );
    }

    #[test]
    fn rejects_missing_target() {
        let err = parse_file_links_directives("::file-links\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_empty_glob() {
        let err = parse_file_links_directives("::file-links \"\"\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_glob_and_dir_combined() {
        let err = parse_file_links_directives("::file-links docs --dir other\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_depth_without_dir() {
        let err = parse_file_links_directives("::file-links --depth 2\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_duplicate_dir() {
        let err =
            parse_file_links_directives("::file-links --dir a --dir b\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_duplicate_depth() {
        let err =
            parse_file_links_directives("::file-links --dir a --depth 1 --depth 2\n")
                .unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_invalid_depth() {
        let err =
            parse_file_links_directives("::file-links --dir a --depth abc\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn rejects_unknown_option() {
        let err =
            parse_file_links_directives("::file-links --dir a --bogus 1\n").unwrap_err();
        assert!(matches!(err, FileLinksError::ParseDirective { .. }));
    }

    #[test]
    fn skips_directives_inside_code_blocks() {
        let content =
            "```\n::file-links docs/**/*.md\n```\n::file-links notes/*.md\n";
        let directives = parse_file_links_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].mode,
            FileLinksMode::Glob("notes/*.md".to_string())
        );
    }

    #[test]
    fn captures_indent() {
        let directives =
            parse_file_links_directives("  ::file-links docs/*.md\n").unwrap();
        assert_eq!(directives[0].indent, "  ");
    }

    #[test]
    fn infers_indent_from_list_item() {
        let directives =
            parse_file_links_directives("- Item\n::file-links docs/*.md\n").unwrap();
        assert_eq!(directives[0].indent, "");
        assert_eq!(directives[0].inferred_indent, Some("  ".to_string()));
    }

    #[test]
    fn multiple_directives_preserve_order() {
        let content = "::file-links a/*.md\n\n::file-links --dir b --depth 1\n";
        let directives = parse_file_links_directives(content).unwrap();
        assert_eq!(directives.len(), 2);
        assert!(matches!(directives[0].mode, FileLinksMode::Glob(_)));
        assert!(matches!(directives[1].mode, FileLinksMode::Dir { .. }));
    }
}

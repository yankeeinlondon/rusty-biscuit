//! Parameter parsing for `::shell-block` opener lines.

use super::types::{ShellBlockError, ShellBlockRegion, SourceExcerpt};
use crate::markdown::compose::parse_utils::directive_prefix_len;
use crate::markdown::compose::shell_expansion::types::ErrorHandling;
use std::time::Duration;

/// Parse a shell block region from a [`BlockPair`](super::super::block_pairs::BlockPair).
pub(crate) fn parse_shell_block_region(
    content: &str,
    pair: &super::super::block_pairs::BlockPair,
) -> Result<ShellBlockRegion, ShellBlockError> {
    let opening_text = &content[pair.span.start..pair.span.end];
    let opener_line = opening_text.lines().next().unwrap_or(opening_text);
    // Capture the opener's leading prefix — indentation whitespace plus any
    // block-quote markers (e.g. `> `). It is preserved verbatim so the block's
    // rendered output can be re-indented and re-quoted under its container, and
    // it must be stripped before parameter parsing, otherwise
    // `strip_prefix("::shell-block")` fails and the param parser chokes on the
    // leading `>` / `:`.
    let prefix_len = directive_prefix_len(opener_line);
    let indent = opener_line[..prefix_len].to_string();
    let (options, timeout_override, no_cache) =
        parse_opener_params(&opener_line[prefix_len..], pair.start_line)?;

    Ok(ShellBlockRegion {
        options,
        timeout_override,
        no_cache,
        indent,
    })
}

/// Parse key=value parameters from a `::shell-block` opener line.
///
/// Returns `(ErrorHandling, Option<Duration>, no_cache)`.
fn parse_opener_params(
    opener: &str,
    line: usize,
) -> Result<(ErrorHandling, Option<Duration>, bool), ShellBlockError> {
    let prefix = "::shell-block";
    let after_prefix = opener.strip_prefix(prefix).unwrap_or(opener).trim_start();

    let mut handling = ErrorHandling::default();
    let mut timeout_override = None;
    let mut no_cache = false;

    if after_prefix.is_empty() {
        return Ok((handling, timeout_override, no_cache));
    }

    let mut cursor = super::super::parse_utils::Cursor::new(after_prefix);

    while !cursor.is_eof() {
        cursor.skip_ws();
        if cursor.is_eof() {
            break;
        }

        // Detect wrong-style flags
        if let Some(ch) = cursor.current()
            && ch == '-'
        {
            // Read the flag text for the error message
            let mut flag = String::new();
            flag.push(ch);
            cursor.advance();
            while let Some(ch) = cursor.current() {
                if ch.is_whitespace() || ch == '=' {
                    break;
                }
                flag.push(ch);
                cursor.advance();
            }
            let hint = wrong_style_hint(&flag);
            return Err(ShellBlockError::Parse {
                line,
                message: format!("Invalid flag syntax '{flag}'. {hint}"),
                excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                source_file: None,
            });
        }

        // Read key
        let key = match read_key(&mut cursor, line) {
            Ok(k) => k,
            Err(e) => {
                return Err(ShellBlockError::Parse {
                    line,
                    message: e,
                    excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                    source_file: None,
                });
            }
        };

        cursor.skip_ws();

        // Expect '='
        if !cursor.try_consume_char('=') {
            // Maybe it's `key:value` or bare key
            if cursor.current() == Some(':') {
                return Err(ShellBlockError::Parse {
                    line,
                    message: format!(
                        "Expected '=' after '{key}', found ':'. Use {key}=\"<value>\" syntax."
                    ),
                    excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                    source_file: None,
                });
            }
            let hint = wrong_style_hint(&key);
            return Err(ShellBlockError::Parse {
                line,
                message: format!("Expected '=' after '{key}'. {hint}"),
                excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                source_file: None,
            });
        }

        cursor.skip_ws();

        // Read value
        let value = match cursor.read_value(line) {
            Ok(v) => v,
            Err(e) => {
                return Err(ShellBlockError::Parse {
                    line,
                    message: e.message,
                    excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                    source_file: None,
                });
            }
        };

        // Detect dash-separated keys (wrong style)
        if key.contains('-') {
            let underscore_version = key.replace('-', "_");
            return Err(ShellBlockError::Parse {
                line,
                message: format!(
                    "Unknown shell block option: '{key}'. Did you mean '{underscore_version}'?"
                ),
                excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                source_file: None,
            });
        }

        // Map key to ErrorHandling field
        match key.as_str() {
            "when_error" => {
                handling.when_error = Some(value);
            }
            "when_exit_code" => {
                if let Some((code, text)) = parse_code_pair(&value, line, &key, opener)? {
                    handling.when_exit_code.push((code, text));
                }
            }
            "except_exit_code" => {
                if let Some((code, text)) = parse_code_pair(&value, line, &key, opener)? {
                    handling.except_exit_code.push((code, text));
                }
            }
            "stderr_contains" => {
                if let Some((find, text)) = parse_string_pair(&value, line, &key, opener)? {
                    handling.stderr_contains.push((find, text));
                }
            }
            "stderr_lacks" => {
                if let Some((find, text)) = parse_string_pair(&value, line, &key, opener)? {
                    handling.stderr_lacks.push((find, text));
                }
            }
            "enrich_error" => {
                handling.enrich_error = Some(value);
            }
            "enrich_error_on" => {
                if let Some((code, text)) = parse_code_pair(&value, line, &key, opener)? {
                    handling.enrich_error_on.push((code, text));
                }
            }
            "timeout" => match value.parse::<u64>() {
                Ok(secs) => {
                    timeout_override = Some(Duration::from_secs(secs));
                }
                Err(_) => {
                    return Err(ShellBlockError::Parse {
                        line,
                        message: format!(
                            "Invalid timeout value '{value}'. Expected a number of seconds."
                        ),
                        excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                        source_file: None,
                    });
                }
            },
            "no_cache" => match value.as_str() {
                "true" => no_cache = true,
                "false" => no_cache = false,
                _ => {
                    return Err(ShellBlockError::Parse {
                        line,
                        message: format!(
                            "Invalid no_cache value '{value}'. Expected true or false."
                        ),
                        excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                        source_file: None,
                    });
                }
            },
            _ => {
                return Err(ShellBlockError::Parse {
                    line,
                    message: format!("Unknown shell block option: '{key}'"),
                    excerpt: SourceExcerpt::from_text(opener, line, line, 0),
                    source_file: None,
                });
            }
        }
    }

    Ok((handling, timeout_override, no_cache))
}

/// Read a key identifier from the cursor.
fn read_key(
    cursor: &mut super::super::parse_utils::Cursor,
    _line: usize,
) -> Result<String, String> {
    use super::super::parse_utils::is_identifier_char;
    use super::super::parse_utils::is_identifier_start;

    match cursor.current() {
        Some(ch) if is_identifier_start(ch) => {
            let mut out = String::new();
            out.push(ch);
            cursor.advance();
            while let Some(ch) = cursor.current() {
                if is_identifier_char(ch) {
                    out.push(ch);
                    cursor.advance();
                } else {
                    break;
                }
            }
            Ok(out)
        }
        Some(ch) => Err(format!("Expected identifier, found '{ch}'")),
        None => Err("Expected identifier, found end of directive".to_string()),
    }
}

/// Parse a "N,text" value for code-based options.
fn parse_code_pair(
    value: &str,
    line: usize,
    key: &str,
    opener: &str,
) -> Result<Option<(i32, String)>, ShellBlockError> {
    let Some(comma_idx) = value.find(',') else {
        return Err(ShellBlockError::Parse {
            line,
            message: format!("{key} value must be in format \"N,text\" where N is an exit code"),
            excerpt: SourceExcerpt::from_text(opener, line, line, 0),
            source_file: None,
        });
    };

    let code_str = &value[..comma_idx];
    let text = &value[comma_idx + 1..];

    match code_str.parse::<i32>() {
        Ok(code) => Ok(Some((code, text.to_string()))),
        Err(_) => Err(ShellBlockError::Parse {
            line,
            message: format!("Invalid exit code '{code_str}' in {key}. Expected an integer."),
            excerpt: SourceExcerpt::from_text(opener, line, line, 0),
            source_file: None,
        }),
    }
}

/// Parse a "find,text" value for string-based options.
fn parse_string_pair(
    value: &str,
    line: usize,
    key: &str,
    opener: &str,
) -> Result<Option<(String, String)>, ShellBlockError> {
    let Some(comma_idx) = value.find(',') else {
        return Err(ShellBlockError::Parse {
            line,
            message: format!("{key} value must be in format \"find,text\""),
            excerpt: SourceExcerpt::from_text(opener, line, line, 0),
            source_file: None,
        });
    };

    let find = value[..comma_idx].to_string();
    let text = value[comma_idx + 1..].to_string();
    Ok(Some((find, text)))
}

/// Generate a hint for wrong-style option syntax.
fn wrong_style_hint(input: &str) -> String {
    match input {
        "--when-error" | "-when-error" => "Use when_error=\"<text>\" instead.".to_string(),
        "--when-exit-code" | "-when-exit-code" => {
            "Use when_exit_code=\"<N>,<text>\" instead.".to_string()
        }
        "--except-exit-code" | "-except-exit-code" => {
            "Use except_exit_code=\"<N>,<text>\" instead.".to_string()
        }
        "--stderr-contains" | "-stderr-contains" => {
            "Use stderr_contains=\"<find>,<text>\" instead.".to_string()
        }
        "--stderr-lacks" | "-stderr-lacks" => {
            "Use stderr_lacks=\"<find>,<text>\" instead.".to_string()
        }
        "--enrich-error" | "-enrich-error" => "Use enrich_error=\"<text>\" instead.".to_string(),
        "--enrich-error-on" | "-enrich-error-on" => {
            "Use enrich_error_on=\"<N>,<text>\" instead.".to_string()
        }
        "--timeout" | "-timeout" => "Use timeout=<seconds> instead.".to_string(),
        "--no-cache" | "-no-cache" => "Use no_cache=true instead.".to_string(),
        s if s.contains('-') => format!("Use {}=\"<value>\" syntax.", s.replace('-', "_")),
        _ => "Shell block parameters use key=\"value\" syntax.".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_no_params() {
        let (handling, timeout, _no_cache) = parse_opener_params("::shell-block", 1).unwrap();
        assert!(handling.is_empty());
        assert!(timeout.is_none());
    }

    /// Build a `BlockPair` directly from `content` so `parse_shell_block_region`
    /// can be exercised without relying on the Markdown indented-code-block
    /// classification that `scan_block_pairs` applies (a column-0 indented line
    /// would otherwise be treated as a code block and skipped).
    fn shell_pair(content: &str) -> super::super::super::block_pairs::BlockPair {
        use super::super::super::block_pairs::{BlockOpenKind, BlockPair};
        let opener_line = content.lines().next().unwrap_or(content);
        BlockPair {
            kind: BlockOpenKind::Shell,
            span: 0..content.len(),
            body_span: 0..0,
            start_line: 1,
            end_line: content.lines().count(),
            opening_text: opener_line.to_string(),
        }
    }

    #[test]
    fn indented_opener_captures_indent_and_parses_params() {
        // Regression: an indented opener previously failed to parse because the
        // leading whitespace defeated `strip_prefix("::shell-block")`.
        let content = "    ::shell-block timeout=5\n    echo hi\n    ::end-block\n";
        let region = parse_shell_block_region(content, &shell_pair(content)).unwrap();
        assert_eq!(region.indent, "    ");
        assert_eq!(region.timeout_override, Some(Duration::from_secs(5)));
    }

    #[test]
    fn column1_opener_has_empty_indent() {
        let content = "::shell-block\necho hi\n::end-block\n";
        let region = parse_shell_block_region(content, &shell_pair(content)).unwrap();
        assert!(region.indent.is_empty());
    }

    #[test]
    fn tab_indented_opener_captures_tab_indent() {
        let content = "\t::shell-block\n\techo hi\n\t::end-block\n";
        let region = parse_shell_block_region(content, &shell_pair(content)).unwrap();
        assert_eq!(region.indent, "\t");
    }

    #[test]
    fn blockquote_opener_captures_marker_indent_and_parses_params() {
        // The `> ` block-quote marker is captured as the indent and stripped
        // before parameter parsing, so options after the marker still parse.
        let content = "> ::shell-block timeout=5\n> echo hi\n> ::end-block\n";
        let region = parse_shell_block_region(content, &shell_pair(content)).unwrap();
        assert_eq!(region.indent, "> ");
        assert_eq!(region.timeout_override, Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_when_error() {
        let (handling, timeout, _no_cache) =
            parse_opener_params("::shell-block when_error=\"fallback\"", 1).unwrap();
        assert_eq!(handling.when_error, Some("fallback".to_string()));
        assert!(timeout.is_none());
    }

    #[test]
    fn parse_when_exit_code() {
        let (handling, _, _) =
            parse_opener_params("::shell-block when_exit_code=\"1,oops\"", 1).unwrap();
        assert_eq!(handling.when_exit_code, vec![(1, "oops".to_string())]);
    }

    #[test]
    fn parse_stderr_contains() {
        let (handling, _, _) =
            parse_opener_params("::shell-block stderr_contains=\"warn,warning found\"", 1).unwrap();
        assert_eq!(
            handling.stderr_contains,
            vec![("warn".to_string(), "warning found".to_string())]
        );
    }

    #[test]
    fn parse_timeout() {
        let (handling, timeout, _no_cache) = parse_opener_params("::shell-block timeout=5", 1).unwrap();
        assert!(handling.is_empty());
        assert_eq!(timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn parse_multiple_params() {
        let (handling, timeout, _no_cache) =
            parse_opener_params("::shell-block when_error=\"fallback\" timeout=10", 1).unwrap();
        assert_eq!(handling.when_error, Some("fallback".to_string()));
        assert_eq!(timeout, Some(Duration::from_secs(10)));
    }

    #[test]
    fn reject_wrong_style_flag() {
        let err = parse_opener_params("::shell-block --when-error fallback", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid flag syntax"), "{msg}");
        assert!(msg.contains("when_error"), "{msg}");
    }

    #[test]
    fn reject_dash_in_key() {
        let err = parse_opener_params("::shell-block when-error=\"fallback\"", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown shell block option"), "{msg}");
        assert!(msg.contains("when_error"), "{msg}");
    }

    #[test]
    fn reject_colon_separator() {
        let err = parse_opener_params("::shell-block timeout:5", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Expected '='"), "{msg}");
        assert!(msg.contains("timeout="), "{msg}");
    }

    #[test]
    fn reject_unknown_option() {
        let err = parse_opener_params("::shell-block bogus=\"value\"", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Unknown shell block option"), "{msg}");
    }

    #[test]
    fn reject_invalid_exit_code() {
        let err = parse_opener_params("::shell-block when_exit_code=\"abc,text\"", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid exit code"), "{msg}");
    }

    #[test]
    fn reject_missing_comma_in_code_pair() {
        let err = parse_opener_params("::shell-block when_exit_code=\"1text\"", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("must be in format"), "{msg}");
    }

    #[test]
    fn reject_single_dash_flag() {
        let err = parse_opener_params("::shell-block -when-error fallback", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid flag syntax"), "{msg}");
    }

    #[test]
    fn reject_double_dash_timeout() {
        let err = parse_opener_params("::shell-block --timeout=5", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid flag syntax"), "{msg}");
        assert!(msg.contains("timeout"), "{msg}");
    }

    #[test]
    fn reject_bare_key_without_equals() {
        let err = parse_opener_params("::shell-block timeout", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Expected '='"), "{msg}");
    }

    #[test]
    fn parse_except_exit_code() {
        let (handling, _, _) =
            parse_opener_params("::shell-block except_exit_code=\"2,skip\"", 1).unwrap();
        assert_eq!(handling.except_exit_code, vec![(2, "skip".to_string())]);
    }

    #[test]
    fn parse_stderr_lacks() {
        let (handling, _, _) =
            parse_opener_params("::shell-block stderr_lacks=\"error,clean\"", 1).unwrap();
        assert_eq!(
            handling.stderr_lacks,
            vec![("error".to_string(), "clean".to_string())]
        );
    }

    #[test]
    fn parse_enrich_error() {
        let (handling, _, _) =
            parse_opener_params("::shell-block enrich_error=\"details\"", 1).unwrap();
        assert_eq!(handling.enrich_error, Some("details".to_string()));
    }

    #[test]
    fn parse_enrich_error_on() {
        let (handling, _, _) =
            parse_opener_params("::shell-block enrich_error_on=\"1,info\"", 1).unwrap();
        assert_eq!(handling.enrich_error_on, vec![(1, "info".to_string())]);
    }

    #[test]
    fn parse_no_cache_true() {
        let (handling, timeout, no_cache) =
            parse_opener_params("::shell-block no_cache=true", 1).unwrap();
        assert!(handling.is_empty());
        assert!(timeout.is_none());
        assert!(no_cache);
    }

    #[test]
    fn parse_no_cache_false() {
        let (_handling, _timeout, no_cache) =
            parse_opener_params("::shell-block no_cache=false", 1).unwrap();
        assert!(!no_cache);
    }

    #[test]
    fn parse_no_cache_default_is_false() {
        let (_handling, _timeout, no_cache) = parse_opener_params("::shell-block", 1).unwrap();
        assert!(!no_cache);
    }

    #[test]
    fn parse_no_cache_alongside_timeout() {
        let (_handling, timeout, no_cache) =
            parse_opener_params("::shell-block no_cache=true timeout=5", 1).unwrap();
        assert!(no_cache);
        assert_eq!(timeout, Some(Duration::from_secs(5)));
    }

    #[test]
    fn reject_no_cache_flag_style() {
        // `--no-cache` must remain a parse error: shell blocks use key=value.
        let err = parse_opener_params("::shell-block --no-cache", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid flag syntax"), "{msg}");
        assert!(msg.contains("no_cache=true"), "{msg}");
    }

    #[test]
    fn reject_invalid_no_cache_value() {
        let err = parse_opener_params("::shell-block no_cache=maybe", 1).unwrap_err();
        assert!(err.to_string().contains("Invalid no_cache value"));
    }

    #[test]
    fn reject_invalid_timeout_non_numeric() {
        let err = parse_opener_params("::shell-block timeout=abc", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid timeout value"), "{msg}");
    }

    #[test]
    fn reject_invalid_timeout_float() {
        let err = parse_opener_params("::shell-block timeout=1.5", 1).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Invalid timeout value"), "{msg}");
    }

    #[test]
    fn parse_all_error_handling_options() {
        let (handling, timeout, _no_cache) = parse_opener_params(
            "::shell-block when_error=\"fallback\" when_exit_code=\"1,oops\" except_exit_code=\"2,skip\" stderr_contains=\"warn,w\" stderr_lacks=\"err,c\" enrich_error=\"details\" enrich_error_on=\"3,info\" timeout=30",
            1,
        ).unwrap();
        assert_eq!(handling.when_error, Some("fallback".to_string()));
        assert_eq!(handling.when_exit_code, vec![(1, "oops".to_string())]);
        assert_eq!(handling.except_exit_code, vec![(2, "skip".to_string())]);
        assert_eq!(
            handling.stderr_contains,
            vec![("warn".to_string(), "w".to_string())]
        );
        assert_eq!(
            handling.stderr_lacks,
            vec![("err".to_string(), "c".to_string())]
        );
        assert_eq!(handling.enrich_error, Some("details".to_string()));
        assert_eq!(handling.enrich_error_on, vec![(3, "info".to_string())]);
        assert_eq!(timeout, Some(Duration::from_secs(30)));
    }
}

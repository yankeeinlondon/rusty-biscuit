//! Parser for `::shell` directives in markdown content.

use super::tokenize::{ShellToken, parse_pipeline, tokenize};
use super::types::{ErrorHandling, ShellCommandOrigin, ShellDirective, ShellExpansionError};
use crate::markdown::compose::parse_utils::{
    directive_prefix_len, find_code_regions, is_in_code_region,
};
use biscuit_terminal::errors::SourceContext;

/// Parses all `::shell` directives from markdown content.
///
/// ## Returns
///
/// A vector of parsed directives with their byte ranges and line numbers.
///
/// ## Errors
///
/// Returns an error if any directive has invalid syntax.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::errors::SourceContext;
/// use darkmatter::markdown::compose::shell_expansion::parser::parse_directives;
/// use std::path::PathBuf;
///
/// let content = "# Test\n::shell echo hello\nSome text\n";
/// let ctx = SourceContext::new(PathBuf::from("/t"), PathBuf::from("t"), content);
/// let directives = parse_directives(content, ctx, 0).unwrap();
/// assert_eq!(directives.len(), 1);
/// assert_eq!(directives[0].executable, "echo");
/// ```
pub fn parse_directives(
    content: &str,
    ctx: SourceContext,
    line_offset: usize,
) -> Result<Vec<ShellDirective>, ShellExpansionError> {
    let code_regions = find_code_regions(content);
    let mut directives = Vec::new();

    let mut byte_offset = 0;

    for (line_num, line_slice) in (1..).zip(content.split_inclusive('\n')) {
        let file_line = line_num + line_offset;
        let line_start = byte_offset;
        let line = line_slice
            .strip_suffix('\n')
            .and_then(|line| line.strip_suffix('\r').or(Some(line)))
            .unwrap_or(line_slice);
        let line_with_newline_end = byte_offset + line_slice.len();

        // Check if this line is inside a code region
        if !is_in_code_region(line_start, &code_regions) {
            // Detect the directive after any block-quote markers and indentation
            // whitespace. The captured prefix (e.g. `> > ` or `    `) is replayed
            // verbatim on every output line so the splice stays nested in its
            // container — a block quote, a list item, or the document root.
            let prefix_len = directive_prefix_len(line);
            let after = line[prefix_len..].trim_end();
            if let Some(command_text) = after.strip_prefix("::shell ") {
                let indent = line[..prefix_len].to_string();

                // Parse the command
                let tokens = tokenize(command_text, &ctx).map_err(|e| {
                    ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: file_line },
                        message: match e {
                            ShellExpansionError::ParseDirective { message, .. } => message,
                            _ => e.to_string(),
                        },
                    }
                })?;

                if tokens.is_empty() {
                    return Err(ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: file_line },
                        message: "Empty command".to_string(),
                    });
                }

                // Separate error handling options, timeout, and the cache opt-out
                // from shell tokens
                let (error_handling, shell_tokens, timeout_override, no_cache) =
                    extract_options_from_tokens(&tokens, file_line, &ctx)?;

                if shell_tokens.is_empty() {
                    return Err(ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: file_line },
                        message: "No command after error handling options".to_string(),
                    });
                }

                // Parse the pipeline from the shell tokens
                let pipeline = parse_pipeline(&shell_tokens, &ctx).map_err(|e| {
                    ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: file_line },
                        message: match e {
                            ShellExpansionError::ParseDirective { message, .. } => message,
                            _ => e.to_string(),
                        },
                    }
                })?;

                let raw_command = pipeline.display_string();
                let executable = pipeline.actions[0].command.executable.clone();
                let args = pipeline.actions[0].command.args.clone();

                directives.push(ShellDirective {
                    raw_command,
                    executable,
                    args,
                    span: line_start..line_with_newline_end,
                    indent,
                    origin: ShellCommandOrigin::Body { line: file_line },
                    error_handling,
                    timeout_override,
                    no_cache,
                    pipeline: Some(pipeline),
                    ctx: ctx.clone(),
                });
            }
        }

        byte_offset = line_with_newline_end;
    }

    Ok(directives)
}

/// Known directive option names for error handling.
const ERROR_HANDLING_OPTIONS: &[&str] = &[
    "--when-error",
    "--when-exit-code",
    "--except-exit-code",
    "--stderr-contains",
    "--stderr-lacks",
    "--enrich-error",
    "--enrich-error-on",
];

/// Returns the number of argument tokens consumed by a given option.
fn option_arg_count(option: &str) -> usize {
    match option {
        "--when-error" | "--enrich-error" => 1,
        "--when-exit-code" | "--except-exit-code" | "--enrich-error-on" => 2,
        "--stderr-contains" | "--stderr-lacks" => 2,
        _ => 0,
    }
}

/// Extracts error handling options, the timeout suffix, and the `--no-cache`
/// cache opt-out from a mixed token list, returning the remaining shell tokens.
fn extract_options_from_tokens(
    tokens: &[ShellToken],
    line: usize,
    ctx: &SourceContext,
) -> Result<(ErrorHandling, Vec<ShellToken>, Option<std::time::Duration>, bool), ShellExpansionError>
{
    let mut handling = ErrorHandling::default();
    let mut shell_tokens = Vec::new();
    let mut i = 0;
    let mut timeout_override = None;
    let mut no_cache = false;

    while i < tokens.len() {
        match &tokens[i] {
            ShellToken::Word(w) => {
                if ERROR_HANDLING_OPTIONS.contains(&w.as_str()) {
                    let option = w.as_str();
                    let argc = option_arg_count(option);

                    // Collect enough Word tokens after this one
                    let mut opt_args = Vec::new();
                    let mut j = i + 1;
                    while j < tokens.len() && opt_args.len() < argc {
                        if let ShellToken::Word(a) = &tokens[j] {
                            opt_args.push(a.clone());
                        } else {
                            // Non-word token interrupts option arguments
                            break;
                        }
                        j += 1;
                    }

                    if opt_args.len() < argc {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: format!("{option} requires {argc} argument(s)"),
                        });
                    }

                    match option {
                        "--when-error" => {
                            handling.when_error = Some(opt_args[0].clone());
                        }
                        "--when-exit-code" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.when_exit_code.push((code, opt_args[1].clone()));
                        }
                        "--except-exit-code" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.except_exit_code.push((code, opt_args[1].clone()));
                        }
                        "--stderr-contains" => {
                            handling
                                .stderr_contains
                                .push((opt_args[0].clone(), opt_args[1].clone()));
                        }
                        "--stderr-lacks" => {
                            handling
                                .stderr_lacks
                                .push((opt_args[0].clone(), opt_args[1].clone()));
                        }
                        "--enrich-error" => {
                            handling.enrich_error = Some(opt_args[0].clone());
                        }
                        "--enrich-error-on" => {
                            let code = parse_exit_code(&opt_args[0], option, line, ctx)?;
                            handling.enrich_error_on.push((code, opt_args[1].clone()));
                        }
                        _ => unreachable!(),
                    }

                    i = j;
                } else if w == "--no-cache" {
                    no_cache = true;
                    i += 1;
                } else if let Some(value_str) = w.strip_prefix("::timeout:") {
                    let seconds: u64 = value_str.parse().map_err(|_| {
                        ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: format!(
                                "::timeout requires a positive integer of seconds, got '{value_str}'"
                            ),
                        }
                    })?;
                    if seconds == 0 {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: "::timeout value must be greater than zero".to_string(),
                        });
                    }
                    // ::timeout must be the last token
                    if i != tokens.len() - 1 {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line },
                            message: "::timeout:<N> must be the last token on the ::shell line"
                                .to_string(),
                        });
                    }
                    timeout_override = Some(std::time::Duration::from_secs(seconds));
                    i += 1;
                } else {
                    shell_tokens.push(tokens[i].clone());
                    i += 1;
                }
            }
            // Non-word tokens (operators, redirections) pass through as shell tokens
            _ => {
                shell_tokens.push(tokens[i].clone());
                i += 1;
            }
        }
    }

    Ok((handling, shell_tokens, timeout_override, no_cache))
}

/// Parses an exit code string into an i32.
fn parse_exit_code(
    raw: &str,
    option_name: &str,
    line: usize,
    ctx: &SourceContext,
) -> Result<i32, ShellExpansionError> {
    raw.parse::<i32>()
        .map_err(|_| ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line },
            message: format!("{option_name} requires an integer exit code, got '{raw}'"),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::Arc;

    fn dummy_ctx(content: &str) -> SourceContext {
        SourceContext::new(
            PathBuf::from("/test.md"),
            PathBuf::from("test.md"),
            Arc::from(content),
        )
    }

    #[test]
    fn parse_simple_directive() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 1 });
        assert_eq!(directives[0].raw_command, "echo hello");
    }

    #[test]
    fn parse_multiple_directives() {
        let content = "::shell ls -la\nSome text\n::shell pwd\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].executable, "ls");
        assert_eq!(directives[0].args, vec!["-la"]);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 1 });
        assert_eq!(directives[1].executable, "pwd");
        assert_eq!(directives[1].args.len(), 0);
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 3 });
    }

    #[test]
    fn parse_directive_with_quotes() {
        let content = "::shell echo \"hello world\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello world"]);
    }

    #[test]
    fn parse_ignores_directives_in_code_blocks() {
        let content = r#"
# Test
::shell echo outside

```bash
::shell echo inside_fenced
```

And `::shell echo inline` should also be ignored.
"#;
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["outside"]);
    }

    #[test]
    fn parse_directive_with_leading_whitespace() {
        let content = "   ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
    }

    #[test]
    fn parse_directive_captures_space_indent() {
        let content = "    ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "    ");
    }

    #[test]
    fn parse_directive_captures_tab_indent() {
        let content = "\t::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "\t");
    }

    #[test]
    fn parse_directive_at_column_one_has_empty_indent() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].indent, "");
    }

    #[test]
    fn parse_directive_in_blockquote_captures_marker_indent() {
        let content = "> ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(directives[0].indent, "> ");
    }

    #[test]
    fn parse_directive_in_nested_blockquote_captures_marker_indent() {
        let content = "> > ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].indent, "> > ");
    }

    #[test]
    fn parse_directive_mid_line_after_blockquote_is_not_a_directive() {
        // `::shell` must be at the directive position (only markers/whitespace
        // before it); a `::shell` that follows prose stays literal text.
        let content = "> some text ::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_directive_span_includes_newline() {
        let content = "::shell echo hello\nNext line\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 19); // Includes newline
        assert_eq!(&content[directives[0].span.clone()], "::shell echo hello\n");
    }

    #[test]
    fn parse_directive_span_includes_crlf() {
        let content = "::shell echo hello\r\nNext line\r\n::shell pwd\r\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 20); // Includes \r\n
        assert_eq!(
            &content[directives[0].span.clone()],
            "::shell echo hello\r\n"
        );
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 3 });
        assert_eq!(&content[directives[1].span.clone()], "::shell pwd\r\n");
    }

    #[test]
    fn parse_directive_without_trailing_newline() {
        let content = "::shell echo hello";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, content.len());
    }

    #[test]
    fn parse_directive_span_includes_crlf_newline() {
        let content = "::shell echo hello\r\nNext line\r\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 20);
        assert_eq!(
            &content[directives[0].span.clone()],
            "::shell echo hello\r\n"
        );
    }

    #[test]
    fn parse_empty_content() {
        let content = "";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_no_directives() {
        let content = "# Heading\nSome text\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_directive_with_invalid_syntax_fails() {
        let content = "::shell echo | grep\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ShellExpansionError::ParseDirective {
                origin, message, ..
            } => {
                assert_eq!(origin, ShellCommandOrigin::Body { line: 1 });
                assert!(message.contains("pipes"));
            }
            _ => panic!("Expected ParseDirective error, got {:?}", err),
        }
    }

    #[test]
    fn parse_directive_line_numbers_are_correct() {
        let content = "Line 1\nLine 2\n::shell echo a\nLine 4\n::shell echo b\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].origin, ShellCommandOrigin::Body { line: 3 });
        assert_eq!(directives[1].origin, ShellCommandOrigin::Body { line: 5 });
    }

    #[test]
    fn parse_directive_ignores_incomplete_prefix() {
        let content = "::shel echo hello\n::shell echo world\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["world"]);
    }

    #[test]
    fn parse_when_error_option() {
        let content = "::shell --when-error \"fallback\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("fallback".to_string())
        );
        assert_eq!(directives[0].raw_command, "echo hello");
    }

    #[test]
    fn parse_when_exit_code_option() {
        let content = "::shell --when-exit-code 1 \"not found\" grep pattern file.txt\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(directives[0].args, vec!["pattern", "file.txt"]);
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
    }

    #[test]
    fn parse_except_exit_code_option() {
        let content = "::shell --except-exit-code 0 \"error occurred\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.except_exit_code,
            vec![(0, "error occurred".to_string())]
        );
    }

    #[test]
    fn parse_stderr_contains_option() {
        let content = "::shell --stderr-contains \"warning\" \"warnings found\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_contains,
            vec![("warning".to_string(), "warnings found".to_string())]
        );
    }

    #[test]
    fn parse_stderr_lacks_option() {
        let content = "::shell --stderr-lacks \"fatal\" \"non-fatal error\" echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_lacks,
            vec![("fatal".to_string(), "non-fatal error".to_string())]
        );
    }

    #[test]
    fn parse_enrich_error_option() {
        let content = "::shell --enrich-error \"Check if git is installed\" git log\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error,
            Some("Check if git is installed".to_string())
        );
    }

    #[test]
    fn parse_enrich_error_on_option() {
        let content = "::shell --enrich-error-on 127 \"Command not installed\" git log\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error_on,
            vec![(127, "Command not installed".to_string())]
        );
    }

    #[test]
    fn parse_multiple_error_handling_options() {
        let content =
            "::shell --when-exit-code 1 \"not found\" --when-error \"failed\" grep pattern\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("failed".to_string())
        );
    }

    #[test]
    fn parse_no_command_after_options_fails() {
        let content = "::shell --when-error \"fallback\"\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No command after error handling options")
        );
    }

    #[test]
    fn parse_missing_option_argument_fails() {
        let content = "::shell echo hello --when-error\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires"));
    }

    #[test]
    fn parse_invalid_exit_code_fails() {
        let content = "::shell --when-exit-code abc \"text\" echo hello\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("requires an integer exit code")
        );
    }

    #[test]
    fn parse_no_options_leaves_empty_error_handling() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(directives[0].error_handling.is_empty());
    }

    #[test]
    fn parse_options_at_end_of_command() {
        let content = "::shell sniff repo staged-packages --when-error \"no packages staged\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(directives[0].args, vec!["repo", "staged-packages"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("no packages staged".to_string())
        );
        assert_eq!(directives[0].raw_command, "sniff repo staged-packages");
    }

    #[test]
    fn parse_options_in_middle_of_command() {
        let content = "::shell sniff --when-error \"fallback\" repo staged-packages\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(directives[0].args, vec!["repo", "staged-packages"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("fallback".to_string())
        );
    }

    #[test]
    fn parse_multiple_options_at_end() {
        let content = "::shell grep pattern file.txt --when-exit-code 1 \"not found\" --enrich-error \"check path\"\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "grep");
        assert_eq!(directives[0].args, vec!["pattern", "file.txt"]);
        assert_eq!(
            directives[0].error_handling.when_exit_code,
            vec![(1, "not found".to_string())]
        );
        assert_eq!(
            directives[0].error_handling.enrich_error,
            Some("check path".to_string())
        );
    }

    #[test]
    fn parse_timeout_suffix_at_end() {
        let content = "::shell echo hello ::timeout:5\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(5))
        );
    }

    #[test]
    fn parse_timeout_suffix_after_error_handling() {
        let content = "::shell echo hello --when-error empty ::timeout:3\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("empty".to_string())
        );
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn parse_no_timeout_suffix_leaves_none() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(directives[0].timeout_override.is_none());
    }

    #[test]
    fn parse_no_cache_flag() {
        let content = "::shell --no-cache uuidgen\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "uuidgen");
        assert!(directives[0].no_cache);
    }

    #[test]
    fn parse_no_cache_defaults_false() {
        let content = "::shell uuidgen\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert!(!directives[0].no_cache);
    }

    #[test]
    fn parse_no_cache_alongside_error_handling_and_timeout() {
        let content = "::shell --no-cache --when-error empty uuidgen ::timeout:3\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives[0].executable, "uuidgen");
        assert!(directives[0].no_cache);
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("empty".to_string())
        );
        assert_eq!(
            directives[0].timeout_override,
            Some(std::time::Duration::from_secs(3))
        );
    }

    #[test]
    fn parse_timeout_suffix_before_error_handling_is_rejected() {
        let content = "::shell echo hello ::timeout:5 --when-error empty\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("::timeout") && err.contains("last"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_timeout_zero_is_rejected() {
        let content = "::shell echo hello ::timeout:0\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("greater than zero"), "got: {err}");
    }

    #[test]
    fn parse_timeout_non_integer_is_rejected() {
        let content = "::shell echo hello ::timeout:abc\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("integer"), "got: {err}");
    }

    #[test]
    fn parse_trailing_and_operator_is_rejected() {
        let content = "::shell echo ok &&\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Missing command after chain operator"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_trailing_or_operator_is_rejected() {
        let content = "::shell echo ok ||\n";
        let result = parse_directives(content, dummy_ctx(content), 0);
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(
            err.contains("Missing command after chain operator"),
            "got: {err}"
        );
    }

    #[test]
    fn parse_directive_raw_command_includes_redirection() {
        let content = "::shell ls > /dev/null\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].raw_command, "ls > /dev/null");
    }

    #[test]
    fn parse_directive_raw_command_includes_redirection_in_chain() {
        let content = "::shell echo hi > /dev/null && echo bye 2>&1\n";
        let directives = parse_directives(content, dummy_ctx(content), 0).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(
            directives[0].raw_command,
            "echo hi > /dev/null && echo bye 2>&1"
        );
    }
}

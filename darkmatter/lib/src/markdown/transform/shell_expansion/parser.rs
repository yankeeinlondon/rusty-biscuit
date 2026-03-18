//! Parser for `::shell` directives in markdown content.

use super::tokenize::tokenize;
use super::types::{ErrorHandling, ShellDirective, ShellExpansionError};
use crate::markdown::transform::parse_utils::{find_code_regions, is_in_code_region};

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
/// use darkmatter::markdown::transform::shell_expansion::parser::parse_directives;
///
/// let content = "# Test\n::shell echo hello\nSome text\n";
/// let directives = parse_directives(content).unwrap();
/// assert_eq!(directives.len(), 1);
/// assert_eq!(directives[0].executable, "echo");
/// ```
pub fn parse_directives(content: &str) -> Result<Vec<ShellDirective>, ShellExpansionError> {
    let code_regions = find_code_regions(content);
    let mut directives = Vec::new();

    let mut line_num = 1;
    let mut byte_offset = 0;

    for line_slice in content.split_inclusive('\n') {
        let line_start = byte_offset;
        let line = line_slice
            .strip_suffix('\n')
            .and_then(|line| line.strip_suffix('\r').or(Some(line)))
            .unwrap_or(line_slice);
        let line_with_newline_end = byte_offset + line_slice.len();

        // Check if this line is inside a code region
        if !is_in_code_region(line_start, &code_regions) {
            let trimmed = line.trim();
            if let Some(command_text) = trimmed.strip_prefix("::shell ") {
                // Parse the command
                let tokens =
                    tokenize(command_text).map_err(|e| ShellExpansionError::ParseDirective {
                        line: line_num,
                        message: match e {
                            ShellExpansionError::ParseDirective { message, .. } => message,
                            _ => e.to_string(),
                        },
                    })?;

                if tokens.is_empty() {
                    return Err(ShellExpansionError::ParseDirective {
                        line: line_num,
                        message: "Empty command".to_string(),
                    });
                }

                // Extract error handling options from anywhere in the token list
                let (error_handling, cmd_tokens) =
                    extract_error_handling(&tokens, line_num)?;

                if cmd_tokens.is_empty() {
                    return Err(ShellExpansionError::ParseDirective {
                        line: line_num,
                        message: "No command after error handling options".to_string(),
                    });
                }

                let executable = cmd_tokens[0].clone();
                let args = cmd_tokens[1..].to_vec();
                let raw_command = cmd_tokens.join(" ");

                directives.push(ShellDirective {
                    raw_command,
                    executable,
                    args,
                    span: line_start..line_with_newline_end,
                    line: line_num,
                    error_handling,
                });
            }
        }

        byte_offset = line_with_newline_end;
        line_num += 1;
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

/// Extracts error handling options from anywhere in a token list.
///
/// Scans all tokens for known directive options (e.g. `--when-error`,
/// `--when-exit-code`), extracts them and their arguments, and returns
/// the remaining tokens as the command.
///
/// Options can appear before, after, or interspersed with command tokens,
/// though the most common placement is at the end of the directive.
fn extract_error_handling(
    tokens: &[String],
    line: usize,
) -> Result<(ErrorHandling, Vec<String>), ShellExpansionError> {
    let mut handling = ErrorHandling::default();
    let mut cmd_tokens = Vec::new();
    let mut i = 0;

    while i < tokens.len() {
        if ERROR_HANDLING_OPTIONS.contains(&tokens[i].as_str()) {
            let option = tokens[i].as_str();
            let argc = option_arg_count(option);

            // Validate we have enough remaining tokens for the option's arguments
            if i + argc >= tokens.len() {
                return Err(ShellExpansionError::ParseDirective {
                    line,
                    message: format!("{option} requires {argc} argument(s)"),
                });
            }

            match option {
                "--when-error" => {
                    handling.when_error = Some(tokens[i + 1].clone());
                }
                "--when-exit-code" => {
                    let code = parse_exit_code(&tokens[i + 1], option, line)?;
                    handling.when_exit_code.push((code, tokens[i + 2].clone()));
                }
                "--except-exit-code" => {
                    let code = parse_exit_code(&tokens[i + 1], option, line)?;
                    handling.except_exit_code.push((code, tokens[i + 2].clone()));
                }
                "--stderr-contains" => {
                    handling
                        .stderr_contains
                        .push((tokens[i + 1].clone(), tokens[i + 2].clone()));
                }
                "--stderr-lacks" => {
                    handling
                        .stderr_lacks
                        .push((tokens[i + 1].clone(), tokens[i + 2].clone()));
                }
                "--enrich-error" => {
                    handling.enrich_error = Some(tokens[i + 1].clone());
                }
                "--enrich-error-on" => {
                    let code = parse_exit_code(&tokens[i + 1], option, line)?;
                    handling.enrich_error_on.push((code, tokens[i + 2].clone()));
                }
                _ => unreachable!(),
            }

            i += 1 + argc;
        } else {
            cmd_tokens.push(tokens[i].clone());
            i += 1;
        }
    }

    Ok((handling, cmd_tokens))
}

/// Parses an exit code string into an i32.
fn parse_exit_code(
    raw: &str,
    option_name: &str,
    line: usize,
) -> Result<i32, ShellExpansionError> {
    raw.parse::<i32>().map_err(|_| {
        ShellExpansionError::ParseDirective {
            line,
            message: format!("{option_name} requires an integer exit code, got '{raw}'"),
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_simple_directive() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["hello"]);
        assert_eq!(directives[0].line, 1);
        assert_eq!(directives[0].raw_command, "echo hello");
    }

    #[test]
    fn parse_multiple_directives() {
        let content = "::shell ls -la\nSome text\n::shell pwd\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].executable, "ls");
        assert_eq!(directives[0].args, vec!["-la"]);
        assert_eq!(directives[0].line, 1);
        assert_eq!(directives[1].executable, "pwd");
        assert_eq!(directives[1].args.len(), 0);
        assert_eq!(directives[1].line, 3);
    }

    #[test]
    fn parse_directive_with_quotes() {
        let content = "::shell echo \"hello world\"\n";
        let directives = parse_directives(content).unwrap();
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
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["outside"]);
    }

    #[test]
    fn parse_directive_with_leading_whitespace() {
        let content = "   ::shell echo hello\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
    }

    #[test]
    fn parse_directive_span_includes_newline() {
        let content = "::shell echo hello\nNext line\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 19); // Includes newline
        assert_eq!(&content[directives[0].span.clone()], "::shell echo hello\n");
    }

    #[test]
    fn parse_directive_span_includes_crlf() {
        let content = "::shell echo hello\r\nNext line\r\n::shell pwd\r\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, 20); // Includes \r\n
        assert_eq!(
            &content[directives[0].span.clone()],
            "::shell echo hello\r\n"
        );
        assert_eq!(directives[1].line, 3);
        assert_eq!(&content[directives[1].span.clone()], "::shell pwd\r\n");
    }

    #[test]
    fn parse_directive_without_trailing_newline() {
        let content = "::shell echo hello";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, content.len());
    }

    #[test]
    fn parse_directive_span_includes_crlf_newline() {
        let content = "::shell echo hello\r\nNext line\r\n";
        let directives = parse_directives(content).unwrap();
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
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_no_directives() {
        let content = "# Heading\nSome text\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 0);
    }

    #[test]
    fn parse_directive_with_invalid_syntax_fails() {
        let content = "::shell echo | grep\n";
        let result = parse_directives(content);
        assert!(result.is_err());
        let err = result.unwrap_err();
        match err {
            ShellExpansionError::ParseDirective { line, message } => {
                assert_eq!(line, 1);
                assert!(message.contains("pipes"));
            }
            _ => panic!("Expected ParseDirective error"),
        }
    }

    #[test]
    fn parse_directive_line_numbers_are_correct() {
        let content = "Line 1\nLine 2\n::shell echo a\nLine 4\n::shell echo b\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 2);
        assert_eq!(directives[0].line, 3);
        assert_eq!(directives[1].line, 5);
    }

    #[test]
    fn parse_directive_ignores_incomplete_prefix() {
        let content = "::shel echo hello\n::shell echo world\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].executable, "echo");
        assert_eq!(directives[0].args, vec!["world"]);
    }

    #[test]
    fn parse_when_error_option() {
        let content = "::shell --when-error \"fallback\" echo hello\n";
        let directives = parse_directives(content).unwrap();
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
        let directives = parse_directives(content).unwrap();
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
        let directives = parse_directives(content).unwrap();
        assert_eq!(
            directives[0].error_handling.except_exit_code,
            vec![(0, "error occurred".to_string())]
        );
    }

    #[test]
    fn parse_stderr_contains_option() {
        let content =
            "::shell --stderr-contains \"warning\" \"warnings found\" echo hello\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_contains,
            vec![("warning".to_string(), "warnings found".to_string())]
        );
    }

    #[test]
    fn parse_stderr_lacks_option() {
        let content =
            "::shell --stderr-lacks \"fatal\" \"non-fatal error\" echo hello\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(
            directives[0].error_handling.stderr_lacks,
            vec![("fatal".to_string(), "non-fatal error".to_string())]
        );
    }

    #[test]
    fn parse_enrich_error_option() {
        let content = "::shell --enrich-error \"Check if git is installed\" git log\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error,
            Some("Check if git is installed".to_string())
        );
    }

    #[test]
    fn parse_enrich_error_on_option() {
        let content = "::shell --enrich-error-on 127 \"Command not installed\" git log\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(
            directives[0].error_handling.enrich_error_on,
            vec![(127, "Command not installed".to_string())]
        );
    }

    #[test]
    fn parse_multiple_error_handling_options() {
        let content = "::shell --when-exit-code 1 \"not found\" --when-error \"failed\" grep pattern\n";
        let directives = parse_directives(content).unwrap();
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
        let result = parse_directives(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("No command after error handling options"));
    }

    #[test]
    fn parse_missing_option_argument_fails() {
        let content = "::shell echo hello --when-error\n";
        let result = parse_directives(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires"));
    }

    #[test]
    fn parse_invalid_exit_code_fails() {
        let content = "::shell --when-exit-code abc \"text\" echo hello\n";
        let result = parse_directives(content);
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("requires an integer exit code"));
    }

    #[test]
    fn parse_no_options_leaves_empty_error_handling() {
        let content = "::shell echo hello\n";
        let directives = parse_directives(content).unwrap();
        assert!(directives[0].error_handling.is_empty());
    }

    #[test]
    fn parse_options_at_end_of_command() {
        let content =
            "::shell sniff repo staged-packages --when-error \"no packages staged\"\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(
            directives[0].args,
            vec!["repo", "staged-packages"]
        );
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("no packages staged".to_string())
        );
        assert_eq!(directives[0].raw_command, "sniff repo staged-packages");
    }

    #[test]
    fn parse_options_in_middle_of_command() {
        let content =
            "::shell sniff --when-error \"fallback\" repo staged-packages\n";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives[0].executable, "sniff");
        assert_eq!(
            directives[0].args,
            vec!["repo", "staged-packages"]
        );
        assert_eq!(
            directives[0].error_handling.when_error,
            Some("fallback".to_string())
        );
    }

    #[test]
    fn parse_multiple_options_at_end() {
        let content = "::shell grep pattern file.txt --when-exit-code 1 \"not found\" --enrich-error \"check path\"\n";
        let directives = parse_directives(content).unwrap();
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
}

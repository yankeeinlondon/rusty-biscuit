//! Shell-like argv tokenizer for command parsing.
//!
//! Implements a simplified shell tokenizer that supports:
//! - Single-quoted strings (literal, no escapes)
//! - Double-quoted strings (backslash escaping for `\`, `"`, whitespace)
//! - Backslash escaping outside quotes
//! - Rejects shell metacharacters and redirections

use super::types::ShellExpansionError;

/// Tokenizes a shell command string into executable and arguments.
///
/// ## Supported Features
///
/// - Whitespace splitting
/// - Single-quoted strings (literal, no escape sequences)
/// - Double-quoted strings (backslash escaping for `\`, `"`, space)
/// - Backslash escaping outside quotes (for spaces, quotes, backslashes)
///
/// ## Rejected Patterns
///
/// - Empty input
/// - Unterminated quotes
/// - Shell metacharacters: `>`, `>>`, `<`, `|`, `;`, `&&`, `||`, backtick, `$(`
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::compose::shell_expansion::tokenize::tokenize;
///
/// let tokens = tokenize("echo hello world").unwrap();
/// assert_eq!(tokens, vec!["echo", "hello", "world"]);
///
/// let tokens = tokenize(r#"echo "hello world""#).unwrap();
/// assert_eq!(tokens, vec!["echo", "hello world"]);
/// ```
pub fn tokenize(input: &str) -> Result<Vec<String>, ShellExpansionError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            line: 0,
            message: "Empty command".to_string(),
        });
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = trimmed.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            // Whitespace ends current token
            ' ' | '\t' | '\n' | '\r' => {
                if !current.is_empty() {
                    tokens.push(current.clone());
                    current.clear();
                }
            }

            // Single quote: literal string until closing quote
            '\'' => {
                let mut found_close = false;
                for ch in chars.by_ref() {
                    if ch == '\'' {
                        found_close = true;
                        break;
                    }
                    current.push(ch);
                }
                if !found_close {
                    return Err(ShellExpansionError::ParseDirective {
                        line: 0,
                        message: "Unterminated single quote".to_string(),
                    });
                }
            }

            // Double quote: backslash escaping
            '"' => {
                let mut found_close = false;
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
                        // Escape next character
                        if let Some(next) = chars.next() {
                            match next {
                                '\\' | '"' | ' ' => current.push(next),
                                _ => {
                                    current.push('\\');
                                    current.push(next);
                                }
                            }
                        } else {
                            return Err(ShellExpansionError::ParseDirective {
                                line: 0,
                                message: "Unterminated escape sequence in double quote".to_string(),
                            });
                        }
                    } else if ch == '"' {
                        found_close = true;
                        break;
                    } else {
                        current.push(ch);
                    }
                }
                if !found_close {
                    return Err(ShellExpansionError::ParseDirective {
                        line: 0,
                        message: "Unterminated double quote".to_string(),
                    });
                }
            }

            // Backslash escaping outside quotes
            '\\' => {
                if let Some(next) = chars.next() {
                    match next {
                        ' ' | '\'' | '"' | '\\' => current.push(next),
                        _ => {
                            current.push('\\');
                            current.push(next);
                        }
                    }
                } else {
                    return Err(ShellExpansionError::ParseDirective {
                        line: 0,
                        message: "Trailing backslash".to_string(),
                    });
                }
            }

            // Reject metacharacters
            '|' => {
                return Err(ShellExpansionError::ParseDirective {
                    line: 0,
                    message: "Shell pipes are not allowed".to_string(),
                });
            }
            ';' => {
                return Err(ShellExpansionError::ParseDirective {
                    line: 0,
                    message: "Command chaining (;) is not allowed".to_string(),
                });
            }
            '<' => {
                return Err(ShellExpansionError::ParseDirective {
                    line: 0,
                    message: "Input redirection (<) is not allowed".to_string(),
                });
            }
            '>' => {
                return Err(ShellExpansionError::ParseDirective {
                    line: 0,
                    message: "Output redirection (>) is not allowed".to_string(),
                });
            }
            '`' => {
                return Err(ShellExpansionError::ParseDirective {
                    line: 0,
                    message: "Command substitution (`) is not allowed".to_string(),
                });
            }
            '$' => {
                // Check for $(
                if chars.peek() == Some(&'(') {
                    return Err(ShellExpansionError::ParseDirective {
                        line: 0,
                        message: "Command substitution $() is not allowed".to_string(),
                    });
                }
                current.push(ch);
            }
            '&' => {
                // Check for &&
                if chars.peek() == Some(&'&') {
                    return Err(ShellExpansionError::ParseDirective {
                        line: 0,
                        message: "Conditional execution (&&) is not allowed".to_string(),
                    });
                }
                current.push(ch);
            }

            // Regular character
            _ => {
                current.push(ch);
            }
        }
    }

    // Push final token
    if !current.is_empty() {
        tokens.push(current);
    }

    if tokens.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            line: 0,
            message: "Empty command".to_string(),
        });
    }

    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenize_simple_command() {
        let tokens = tokenize("echo hello").unwrap();
        assert_eq!(tokens, vec!["echo", "hello"]);
    }

    #[test]
    fn tokenize_multiple_args() {
        let tokens = tokenize("ls -la /tmp").unwrap();
        assert_eq!(tokens, vec!["ls", "-la", "/tmp"]);
    }

    #[test]
    fn tokenize_single_quoted_string() {
        let tokens = tokenize("echo 'hello world'").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_double_quoted_string() {
        let tokens = tokenize(r#"echo "hello world""#).unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_escaped_space_outside_quotes() {
        let tokens = tokenize(r"echo hello\ world").unwrap();
        assert_eq!(tokens, vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_escaped_quote_in_double_quotes() {
        let tokens = tokenize(r#"echo "say \"hello\"""#).unwrap();
        assert_eq!(tokens, vec!["echo", r#"say "hello""#]);
    }

    #[test]
    fn tokenize_escaped_backslash_in_double_quotes() {
        let tokens = tokenize(r#"echo "path\\to\\file""#).unwrap();
        assert_eq!(tokens, vec!["echo", r"path\to\file"]);
    }

    #[test]
    fn tokenize_single_quotes_are_literal() {
        let tokens = tokenize(r"echo 'no\escape'").unwrap();
        assert_eq!(tokens, vec!["echo", r"no\escape"]);
    }

    #[test]
    fn tokenize_mixed_quotes() {
        let tokens = tokenize(r#"echo 'single' "double" plain"#).unwrap();
        assert_eq!(tokens, vec!["echo", "single", "double", "plain"]);
    }

    #[test]
    fn tokenize_empty_input_fails() {
        let result = tokenize("");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Empty command"));
    }

    #[test]
    fn tokenize_whitespace_only_fails() {
        let result = tokenize("   \t\n  ");
        assert!(result.is_err());
    }

    #[test]
    fn tokenize_unterminated_single_quote_fails() {
        let result = tokenize("echo 'unterminated");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unterminated single quote")
        );
    }

    #[test]
    fn tokenize_unterminated_double_quote_fails() {
        let result = tokenize(r#"echo "unterminated"#);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unterminated double quote")
        );
    }

    #[test]
    fn tokenize_pipe_fails() {
        let result = tokenize("ls | grep test");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pipes"));
    }

    #[test]
    fn tokenize_semicolon_fails() {
        let result = tokenize("echo hello; echo world");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("chaining"));
    }

    #[test]
    fn tokenize_input_redirect_fails() {
        let result = tokenize("cat < file.txt");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Input redirection")
        );
    }

    #[test]
    fn tokenize_output_redirect_fails() {
        let result = tokenize("echo hello > file.txt");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Output redirection")
        );
    }

    #[test]
    fn tokenize_backtick_fails() {
        let result = tokenize("echo `date`");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Command substitution")
        );
    }

    #[test]
    fn tokenize_command_substitution_fails() {
        let result = tokenize("echo $(date)");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Command substitution")
        );
    }

    #[test]
    fn tokenize_double_ampersand_fails() {
        let result = tokenize("make && make install");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Conditional execution")
        );
    }

    #[test]
    fn tokenize_dollar_sign_alone_is_ok() {
        let tokens = tokenize("echo $PATH").unwrap();
        assert_eq!(tokens, vec!["echo", "$PATH"]);
    }

    #[test]
    fn tokenize_single_ampersand_is_ok() {
        let tokens = tokenize("echo foo&bar").unwrap();
        assert_eq!(tokens, vec!["echo", "foo&bar"]);
    }

    #[test]
    fn tokenize_trailing_backslash_fails() {
        let result = tokenize("echo hello\\");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Trailing backslash")
        );
    }

    #[test]
    fn tokenize_double_redirect_fails() {
        let result = tokenize("echo hello >> file.txt");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Output redirection")
        );
    }

    #[test]
    fn tokenize_double_pipe_fails() {
        let result = tokenize("echo hello || echo world");
        assert!(result.is_err());
        // The first `|` is caught, which triggers the pipe error
        assert!(result.unwrap_err().to_string().contains("pipes"));
    }
}

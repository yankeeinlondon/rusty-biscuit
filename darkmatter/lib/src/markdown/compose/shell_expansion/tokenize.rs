//! Shell-like tokenizer for command parsing.
//!
//! Implements a simplified shell tokenizer that supports:
//! - Single-quoted strings (literal, no escapes)
//! - Double-quoted strings (backslash escaping for `\`, `"`, whitespace)
//! - Backslash escaping outside quotes
//! - Chain operators: `&&`, `||`
//! - Curated redirections: `> /dev/null`, `2> /dev/null`, `2>&1`, `>&2`
//! - Literal backticks in arguments (not subshell expansion)
//!
//! Still rejects: bare `|`, `;`, `<`, `>>`, `$(`, and arbitrary file redirections.

use super::types::{
    ChainOperator, CommandAction, PipelineAction, RedirectionConfig, ShellCommandOrigin,
    ShellExpansionError, ShellPipeline, StderrTarget, StdoutTarget,
};
use biscuit_terminal::errors::SourceContext;

/// A single token produced by the shell tokenizer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShellToken {
    /// A word token (executable name or argument).
    Word(String),
    /// `&&` chain operator.
    And,
    /// `||` chain operator.
    Or,
    /// `> /dev/null` — redirect stdout to null.
    RedirectStdoutNull,
    /// `2> /dev/null` — redirect stderr to null.
    RedirectStderrNull,
    /// `2>&1` — merge stderr into stdout.
    RedirectStderrToStdout,
    /// `>&2` — route stdout to stderr.
    RedirectStdoutToStderr,
}

/// Tokenizes a shell command string into structured tokens.
///
/// ## Supported Features
///
/// - Whitespace splitting
/// - Single-quoted strings (literal, no escape sequences)
/// - Double-quoted strings (backslash escaping for `\`, `"`, space)
/// - Backslash escaping outside quotes (for spaces, quotes, backslashes)
/// - Chain operators: `&&`, `||`
/// - Curated redirections: `> /dev/null`, `2> /dev/null`, `2>&1`, `>&2`
/// - Literal backticks in arguments
///
/// ## Rejected Patterns
///
/// - Empty input
/// - Unterminated quotes
/// - Bare `|` (shell pipe)
/// - `;` (command chaining)
/// - `<` (input redirection)
/// - `>>` (append redirection)
/// - `$(` (command substitution)
/// - Redirection to arbitrary files
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::errors::SourceContext;
/// use darkmatter::markdown::compose::shell_expansion::tokenize::tokenize;
/// use std::path::PathBuf;
///
/// let ctx = SourceContext::new(PathBuf::from("/t"), PathBuf::from("t"), "");
/// let tokens = tokenize("echo hello world", &ctx).unwrap();
/// assert!(tokens.len() == 3);
///
/// let tokens = tokenize(r#"echo "hello world""#, &ctx).unwrap();
/// assert!(tokens.len() == 2);
/// ```
pub fn tokenize(input: &str, ctx: &SourceContext) -> Result<Vec<ShellToken>, ShellExpansionError> {
    let trimmed = input.trim();

    if trimmed.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: 0 },
            message: "Empty command".to_string(),
        });
    }

    let mut tokens = Vec::new();
    let mut current = String::new();
    let mut chars = trimmed.chars().peekable();

    while let Some(ch) = chars.next() {
        match ch {
            ' ' | '\t' | '\n' | '\r' => {
                if !current.is_empty() {
                    flush_token(&mut tokens, &mut current)?;
                }
            }

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
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Unterminated single quote".to_string(),
                    });
                }
            }

            '"' => {
                let mut found_close = false;
                while let Some(ch) = chars.next() {
                    if ch == '\\' {
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
                                ctx: Box::new(ctx.clone()),
                                origin: ShellCommandOrigin::Body { line: 0 },
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
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Unterminated double quote".to_string(),
                    });
                }
            }

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
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Trailing backslash".to_string(),
                    });
                }
            }

            '&' => {
                if chars.peek() == Some(&'&') {
                    chars.next();
                    if !current.is_empty() {
                        flush_token(&mut tokens, &mut current)?;
                    }
                    tokens.push(ShellToken::And);
                } else {
                    current.push(ch);
                }
            }

            '|' => {
                if chars.peek() == Some(&'|') {
                    chars.next();
                    if !current.is_empty() {
                        flush_token(&mut tokens, &mut current)?;
                    }
                    tokens.push(ShellToken::Or);
                } else {
                    return Err(ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Shell pipes are not allowed".to_string(),
                    });
                }
            }

            ';' => {
                return Err(ShellExpansionError::ParseDirective {
                    ctx: Box::new(ctx.clone()),
                    origin: ShellCommandOrigin::Body { line: 0 },
                    message: "Command chaining (;) is not allowed".to_string(),
                });
            }

            '<' => {
                return Err(ShellExpansionError::ParseDirective {
                    ctx: Box::new(ctx.clone()),
                    origin: ShellCommandOrigin::Body { line: 0 },
                    message: "Input redirection (<) is not allowed".to_string(),
                });
            }

            '>' => {
                if chars.peek() == Some(&'>') {
                    return Err(ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Append redirection (>>) is not allowed".to_string(),
                    });
                }

                if !current.is_empty() {
                    flush_token(&mut tokens, &mut current)?;
                }

                // Check for >&2 (stdout to stderr)
                if chars.peek() == Some(&'&') {
                    chars.next(); // consume '&'
                    if chars.peek() == Some(&'2') {
                        chars.next(); // consume '2'
                        tokens.push(ShellToken::RedirectStdoutToStderr);
                    } else {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line: 0 },
                            message: "Only >&2 redirection is supported after >&".to_string(),
                        });
                    }
                } else {
                    // Must be > /dev/null
                    skip_whitespace(&mut chars);
                    let target = read_bare_word(&mut chars);
                    if target == "/dev/null" {
                        tokens.push(ShellToken::RedirectStdoutNull);
                    } else {
                        return Err(ShellExpansionError::ParseDirective {
                            ctx: Box::new(ctx.clone()),
                            origin: ShellCommandOrigin::Body { line: 0 },
                            message: format!(
                                "Output redirection to arbitrary files is not allowed (>{target})"
                            ),
                        });
                    }
                }
            }

            '`' => {
                current.push(ch);
            }

            '$' => {
                if chars.peek() == Some(&'(') {
                    return Err(ShellExpansionError::ParseDirective {
                        ctx: Box::new(ctx.clone()),
                        origin: ShellCommandOrigin::Body { line: 0 },
                        message: "Command substitution $() is not allowed".to_string(),
                    });
                }
                current.push(ch);
            }

            _ => {
                // Handle the digit '2' specially: if followed by '>' and current is empty,
                // it's a fd prefix for stderr redirection
                if ch == '2' && chars.peek() == Some(&'>') && current.is_empty() {
                    chars.next(); // consume '>'
                    if chars.peek() == Some(&'&') {
                        chars.next(); // consume '&'
                        if chars.peek() == Some(&'1') {
                            chars.next(); // consume '1'
                            tokens.push(ShellToken::RedirectStderrToStdout);
                        } else {
                            return Err(ShellExpansionError::ParseDirective {
                                ctx: Box::new(ctx.clone()),
                                origin: ShellCommandOrigin::Body { line: 0 },
                                message: "Only 2>&1 redirection is supported".to_string(),
                            });
                        }
                    } else {
                        skip_whitespace(&mut chars);
                        let target = read_bare_word(&mut chars);
                        if target == "/dev/null" {
                            tokens.push(ShellToken::RedirectStderrNull);
                        } else {
                            return Err(ShellExpansionError::ParseDirective {
                                ctx: Box::new(ctx.clone()),
                                origin: ShellCommandOrigin::Body { line: 0 },
                                message: format!(
                                    "Stderr redirection to arbitrary files is not allowed (2>{target})"
                                ),
                            });
                        }
                    }
                } else {
                    current.push(ch);
                }
            }
        }
    }

    // Push final token
    if !current.is_empty() {
        flush_token(&mut tokens, &mut current)?;
    }

    if tokens.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: 0 },
            message: "Empty command".to_string(),
        });
    }

    Ok(tokens)
}

fn flush_token(
    tokens: &mut Vec<ShellToken>,
    current: &mut String,
) -> Result<(), ShellExpansionError> {
    if !current.is_empty() {
        tokens.push(ShellToken::Word(current.clone()));
        current.clear();
    }
    Ok(())
}

fn skip_whitespace(chars: &mut std::iter::Peekable<std::str::Chars>) {
    while chars.peek() == Some(&' ') || chars.peek() == Some(&'\t') {
        chars.next();
    }
}

/// Reads a bare (unquoted, unescaped) word from the character stream.
fn read_bare_word(chars: &mut std::iter::Peekable<std::str::Chars>) -> String {
    let mut word = String::new();
    while let Some(&ch) = chars.peek() {
        if ch.is_whitespace()
            || ch == '&'
            || ch == '|'
            || ch == ';'
            || ch == '<'
            || ch == '>'
            || ch == '\''
            || ch == '"'
            || ch == '`'
            || ch == '\\'
            || ch == '$'
        {
            break;
        }
        word.push(ch);
        chars.next();
    }
    word
}

/// Tokenizes input and returns only Word tokens as strings.
/// Used by callers that don't need operator/redirection tokens.
pub fn tokenize_simple(
    input: &str,
    ctx: &SourceContext,
) -> Result<Vec<String>, ShellExpansionError> {
    let tokens = tokenize(input, ctx)?;
    Ok(tokens
        .into_iter()
        .filter_map(|t| match t {
            ShellToken::Word(w) => Some(w),
            _ => None,
        })
        .collect())
}

/// Parses tokens into a pipeline structure for chain execution.
///
/// ## Combined Redirections
///
/// Multiple redirection tokens on the same command action are accepted and
/// resolved using shell-style left-to-right dup semantics:
///
/// - `cmd > /dev/null 2>&1` → both streams suppressed (stderr dups the
///   already-nulled stdout).
/// - `cmd 2>&1 > /dev/null` → only stdout suppressed; stderr keeps its
///   reference to the original (captured) stdout.
/// - `cmd 2> /dev/null >&2` → both streams suppressed (stdout dups the
///   already-nulled stderr).
/// - `cmd >&2 2> /dev/null` → only stderr suppressed; stdout keeps its
///   reference to the original (captured) stderr.
pub fn parse_pipeline(
    tokens: &[ShellToken],
    ctx: &SourceContext,
) -> Result<ShellPipeline, ShellExpansionError> {
    let mut actions = Vec::new();
    let mut current_words = Vec::new();
    let mut redirection = RedirectionConfig::default();
    let mut operator = ChainOperator::None;

    for token in tokens {
        match token {
            ShellToken::Word(w) => current_words.push(w.clone()),
            ShellToken::And => {
                let action = build_action(&mut current_words, &mut redirection, operator, ctx)?;
                actions.push(action);
                operator = ChainOperator::And;
            }
            ShellToken::Or => {
                let action = build_action(&mut current_words, &mut redirection, operator, ctx)?;
                actions.push(action);
                operator = ChainOperator::Or;
            }
            ShellToken::RedirectStdoutNull => {
                // Shell-style dup resolution: if stderr was previously duped
                // to stdout via `2>&1`, it captured stdout's prior target —
                // changing stdout now must not retroactively change stderr.
                if redirection.stderr == StderrTarget::ToStdout {
                    redirection.stderr = match redirection.stdout {
                        StdoutTarget::Capture => StderrTarget::Capture,
                        StdoutTarget::Null => StderrTarget::Null,
                        StdoutTarget::ToStderr => StderrTarget::Capture,
                    };
                }
                redirection.stdout = StdoutTarget::Null;
            }
            ShellToken::RedirectStderrNull => {
                if redirection.stdout == StdoutTarget::ToStderr {
                    redirection.stdout = match redirection.stderr {
                        StderrTarget::Capture => StdoutTarget::Capture,
                        StderrTarget::Null => StdoutTarget::Null,
                        StderrTarget::ToStdout => StdoutTarget::Capture,
                    };
                }
                redirection.stderr = StderrTarget::Null;
            }
            ShellToken::RedirectStderrToStdout => {
                // `2>&1` snapshots stdout's CURRENT target into stderr.
                redirection.stderr = match redirection.stdout {
                    StdoutTarget::Capture => StderrTarget::ToStdout,
                    StdoutTarget::Null => StderrTarget::Null,
                    StdoutTarget::ToStderr => StderrTarget::Capture,
                };
            }
            ShellToken::RedirectStdoutToStderr => {
                redirection.stdout = match redirection.stderr {
                    StderrTarget::Capture => StdoutTarget::ToStderr,
                    StderrTarget::Null => StdoutTarget::Null,
                    StderrTarget::ToStdout => StdoutTarget::Capture,
                };
            }
        }
    }

    if !current_words.is_empty() || redirection != RedirectionConfig::default() {
        let action = build_action(&mut current_words, &mut redirection, operator, ctx)?;
        actions.push(action);
    } else if operator != ChainOperator::None {
        let op_str = match operator {
            ChainOperator::And => "&&",
            ChainOperator::Or => "||",
            ChainOperator::None => unreachable!(),
        };
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: 0 },
            message: format!("Missing command after chain operator '{op_str}'"),
        });
    }

    if actions.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: 0 },
            message: "Empty command".to_string(),
        });
    }

    Ok(ShellPipeline { actions })
}

fn build_action(
    words: &mut Vec<String>,
    redirection: &mut RedirectionConfig,
    operator: ChainOperator,
    ctx: &SourceContext,
) -> Result<PipelineAction, ShellExpansionError> {
    if words.is_empty() {
        return Err(ShellExpansionError::ParseDirective {
            ctx: Box::new(ctx.clone()),
            origin: ShellCommandOrigin::Body { line: 0 },
            message: "Missing command before chain operator".to_string(),
        });
    }
    let executable = words.remove(0);
    let args: Vec<String> = std::mem::take(words);
    let redir = std::mem::take(redirection);
    Ok(PipelineAction {
        operator,
        command: CommandAction {
            executable,
            args,
            redirection: redir,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_ctx() -> SourceContext {
        SourceContext::new(
            std::path::PathBuf::from("/test"),
            std::path::PathBuf::from("test"),
            String::new(),
        )
    }

    fn tokenize(input: &str) -> Result<Vec<ShellToken>, ShellExpansionError> {
        super::tokenize(input, &test_ctx())
    }

    fn parse_pipeline(tokens: &[ShellToken]) -> Result<ShellPipeline, ShellExpansionError> {
        super::parse_pipeline(tokens, &test_ctx())
    }

    #[allow(dead_code)]
    fn tokenize_simple(input: &str) -> Result<Vec<String>, ShellExpansionError> {
        super::tokenize_simple(input, &test_ctx())
    }

    fn words(tokens: &[ShellToken]) -> Vec<String> {
        tokens
            .iter()
            .filter_map(|t| match t {
                ShellToken::Word(w) => Some(w.clone()),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn tokenize_simple_command() {
        let tokens = tokenize("echo hello").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "hello"]);
    }

    #[test]
    fn tokenize_multiple_args() {
        let tokens = tokenize("ls -la /tmp").unwrap();
        assert_eq!(words(&tokens), vec!["ls", "-la", "/tmp"]);
    }

    #[test]
    fn tokenize_single_quoted_string() {
        let tokens = tokenize("echo 'hello world'").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_double_quoted_string() {
        let tokens = tokenize(r#"echo "hello world""#).unwrap();
        assert_eq!(words(&tokens), vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_escaped_space_outside_quotes() {
        let tokens = tokenize(r"echo hello\ world").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "hello world"]);
    }

    #[test]
    fn tokenize_escaped_quote_in_double_quotes() {
        let tokens = tokenize(r#"echo "say \"hello\"""#).unwrap();
        assert_eq!(words(&tokens), vec!["echo", r#"say "hello""#]);
    }

    #[test]
    fn tokenize_escaped_backslash_in_double_quotes() {
        let tokens = tokenize(r#"echo "path\\to\\file""#).unwrap();
        assert_eq!(words(&tokens), vec!["echo", r"path\to\file"]);
    }

    #[test]
    fn tokenize_single_quotes_are_literal() {
        let tokens = tokenize(r"echo 'no\escape'").unwrap();
        assert_eq!(words(&tokens), vec!["echo", r"no\escape"]);
    }

    #[test]
    fn tokenize_mixed_quotes() {
        let tokens = tokenize(r#"echo 'single' "double" plain"#).unwrap();
        assert_eq!(words(&tokens), vec!["echo", "single", "double", "plain"]);
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
    fn tokenize_bare_pipe_fails() {
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
    fn tokenize_append_redirect_fails() {
        let result = tokenize("echo hello >> file.txt");
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Append redirection")
        );
    }

    #[test]
    fn tokenize_dollar_substitution_fails() {
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
    fn tokenize_dollar_sign_alone_is_ok() {
        let tokens = tokenize("echo $PATH").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "$PATH"]);
    }

    #[test]
    fn tokenize_single_ampersand_is_ok() {
        let tokens = tokenize("echo foo&bar").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "foo&bar"]);
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
    fn tokenize_arbitrary_redirect_fails() {
        let result = tokenize("echo hello > file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("arbitrary files"));
    }

    #[test]
    fn tokenize_arbitrary_stderr_redirect_fails() {
        let result = tokenize("cmd 2> file.txt");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("arbitrary files"));
    }

    // ── New: chain operators ──────────────────────────────────────

    #[test]
    fn tokenize_and_operator() {
        let tokens = tokenize("make && make install").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(tokens[0], ShellToken::Word("make".to_string()));
        assert_eq!(tokens[1], ShellToken::And);
        assert_eq!(tokens[2], ShellToken::Word("make".to_string()));
        assert_eq!(tokens[3], ShellToken::Word("install".to_string()));
    }

    #[test]
    fn tokenize_or_operator() {
        let tokens = tokenize("cmd1 || cmd2").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(tokens[0], ShellToken::Word("cmd1".to_string()));
        assert_eq!(tokens[1], ShellToken::Or);
        assert_eq!(tokens[2], ShellToken::Word("cmd2".to_string()));
    }

    #[test]
    fn tokenize_complex_chain() {
        let tokens = tokenize("npm install && npm run build || echo failed").unwrap();
        assert!(tokens.contains(&ShellToken::And));
        assert!(tokens.contains(&ShellToken::Or));
    }

    // ── New: redirections ─────────────────────────────────────────

    #[test]
    fn tokenize_stdout_to_dev_null() {
        let tokens = tokenize("ls > /dev/null").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], ShellToken::Word("ls".to_string()));
        assert_eq!(tokens[1], ShellToken::RedirectStdoutNull);
    }

    #[test]
    fn tokenize_stderr_to_dev_null() {
        let tokens = tokenize("cmd 2> /dev/null").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], ShellToken::Word("cmd".to_string()));
        assert_eq!(tokens[1], ShellToken::RedirectStderrNull);
    }

    #[test]
    fn tokenize_stderr_to_stdout() {
        let tokens = tokenize("cmd 2>&1").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], ShellToken::Word("cmd".to_string()));
        assert_eq!(tokens[1], ShellToken::RedirectStderrToStdout);
    }

    #[test]
    fn tokenize_stdout_to_stderr() {
        let tokens = tokenize("cmd >&2").unwrap();
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], ShellToken::Word("cmd".to_string()));
        assert_eq!(tokens[1], ShellToken::RedirectStdoutToStderr);
    }

    #[test]
    fn tokenize_redirect_with_chain() {
        let tokens = tokenize("cmd1 > /dev/null && cmd2").unwrap();
        assert_eq!(tokens[0], ShellToken::Word("cmd1".to_string()));
        assert_eq!(tokens[1], ShellToken::RedirectStdoutNull);
        assert_eq!(tokens[2], ShellToken::And);
        assert_eq!(tokens[3], ShellToken::Word("cmd2".to_string()));
    }

    // ── New: backticks ────────────────────────────────────────────

    #[test]
    fn tokenize_backtick_is_literal() {
        let tokens = tokenize("echo `hello`").unwrap();
        assert_eq!(words(&tokens), vec!["echo", "`hello`"]);
    }

    // ── Pipeline parsing ──────────────────────────────────────────

    #[test]
    fn pipeline_simple_command() {
        let tokens = tokenize("echo hello").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.actions.len(), 1);
        assert_eq!(pipeline.actions[0].command.executable, "echo");
        assert_eq!(pipeline.actions[0].command.args, vec!["hello"]);
    }

    #[test]
    fn pipeline_and_chain() {
        let tokens = tokenize("cmd1 && cmd2").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.actions.len(), 2);
        assert_eq!(pipeline.actions[0].operator, ChainOperator::None);
        assert_eq!(pipeline.actions[1].operator, ChainOperator::And);
        assert_eq!(pipeline.actions[1].command.executable, "cmd2");
    }

    #[test]
    fn pipeline_or_chain() {
        let tokens = tokenize("cmd1 || cmd2").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.actions.len(), 2);
        assert_eq!(pipeline.actions[1].operator, ChainOperator::Or);
    }

    #[test]
    fn pipeline_complex_chain() {
        let tokens = tokenize("a && b || c").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.actions.len(), 3);
        assert_eq!(pipeline.actions[0].operator, ChainOperator::None);
        assert_eq!(pipeline.actions[1].operator, ChainOperator::And);
        assert_eq!(pipeline.actions[2].operator, ChainOperator::Or);
    }

    #[test]
    fn pipeline_with_stdout_null() {
        let tokens = tokenize("ls > /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(
            pipeline.actions[0].command.redirection.stdout,
            StdoutTarget::Null
        );
    }

    #[test]
    fn pipeline_with_stderr_to_stdout() {
        let tokens = tokenize("cmd 2>&1").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(
            pipeline.actions[0].command.redirection.stderr,
            StderrTarget::ToStdout
        );
    }

    /// `> /dev/null 2>&1`: stdout is redirected to null first, then `2>&1`
    /// snapshots the now-null stdout into stderr — both go to /dev/null.
    #[test]
    fn pipeline_combined_stdout_null_then_merge_resolves_to_both_null() {
        let tokens = tokenize("cmd > /dev/null 2>&1").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let redir = &pipeline.actions[0].command.redirection;
        assert_eq!(redir.stdout, StdoutTarget::Null);
        assert_eq!(redir.stderr, StderrTarget::Null);
    }

    /// `2>&1 > /dev/null`: stderr captures stdout's CURRENT (Capture) target,
    /// then stdout is nulled — stderr keeps the Capture reference, matching
    /// real shell semantics.
    #[test]
    fn pipeline_combined_merge_then_stdout_null_keeps_stderr_captured() {
        let tokens = tokenize("cmd 2>&1 > /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let redir = &pipeline.actions[0].command.redirection;
        assert_eq!(redir.stdout, StdoutTarget::Null);
        assert_eq!(redir.stderr, StderrTarget::Capture);
    }

    /// `> /dev/null 2> /dev/null`: independent nullings of both streams.
    #[test]
    fn pipeline_combined_both_streams_to_null() {
        let tokens = tokenize("cmd > /dev/null 2> /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let redir = &pipeline.actions[0].command.redirection;
        assert_eq!(redir.stdout, StdoutTarget::Null);
        assert_eq!(redir.stderr, StderrTarget::Null);
    }

    /// `2> /dev/null >&2`: stderr nulled, then stdout dups the now-null
    /// stderr — both streams suppressed.
    #[test]
    fn pipeline_combined_stderr_null_then_route_resolves_to_both_null() {
        let tokens = tokenize("cmd 2> /dev/null >&2").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let redir = &pipeline.actions[0].command.redirection;
        assert_eq!(redir.stdout, StdoutTarget::Null);
        assert_eq!(redir.stderr, StderrTarget::Null);
    }

    /// `>&2 2> /dev/null`: stdout dups stderr (Capture), then stderr nulled —
    /// stdout keeps the Capture reference.
    #[test]
    fn pipeline_combined_route_then_stderr_null_keeps_stdout_captured() {
        let tokens = tokenize("cmd >&2 2> /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let redir = &pipeline.actions[0].command.redirection;
        assert_eq!(redir.stdout, StdoutTarget::Capture);
        assert_eq!(redir.stderr, StderrTarget::Null);
    }

    #[test]
    fn pipeline_display_string() {
        let tokens = tokenize("echo hello && ls").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        let display = pipeline.display_string();
        assert!(display.contains("&&"));
        assert!(display.contains("echo hello"));
        assert!(display.contains("ls"));
    }

    #[test]
    fn pipeline_missing_command_before_operator() {
        let _tokens = tokenize("&& cmd").unwrap();
        // tokenize will produce [And, Word("cmd")], but parse_pipeline should fail
        // because there's no command before And
        // Actually: tokenize("&& cmd") — first char is '&', peek is '&', so we push And.
        // Then we read "cmd" as Word. parse_pipeline will see And first, try to build action
        // from empty words → error.
        // Wait — operator is None initially, so first action has operator None.
        // Let me trace: tokens = [And, Word("cmd")]
        // - And: current_words is empty → build_action fails? No, because operator is None initially
        //   Wait no. The first token is And. We enter ShellToken::And branch.
        //   current_words is empty. build_action(&mut [], ...) will fail with "Missing command"
        let result = parse_pipeline(&tokenize("&& cmd").unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_trailing_and_rejected() {
        let result = parse_pipeline(&tokenize("echo ok &&").unwrap());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Missing command after chain operator") && msg.contains("&&"),
            "got: {msg}"
        );
    }

    #[test]
    fn pipeline_trailing_or_rejected() {
        let result = parse_pipeline(&tokenize("echo ok ||").unwrap());
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(
            msg.contains("Missing command after chain operator") && msg.contains("||"),
            "got: {msg}"
        );
    }

    #[test]
    fn pipeline_doubled_operator_rejected() {
        let result = parse_pipeline(&tokenize("echo ok && && echo no").unwrap());
        assert!(result.is_err());
    }

    #[test]
    fn pipeline_display_string_includes_stdout_null() {
        let tokens = tokenize("ls > /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.display_string(), "ls > /dev/null");
    }

    #[test]
    fn pipeline_display_string_includes_stderr_null() {
        let tokens = tokenize("cmd 2> /dev/null").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.display_string(), "cmd 2> /dev/null");
    }

    #[test]
    fn pipeline_display_string_includes_stderr_to_stdout() {
        let tokens = tokenize("cmd 2>&1").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.display_string(), "cmd 2>&1");
    }

    #[test]
    fn pipeline_display_string_includes_stdout_to_stderr() {
        let tokens = tokenize("cmd >&2").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.display_string(), "cmd >&2");
    }

    #[test]
    fn pipeline_display_string_includes_redirection_in_chain() {
        let tokens = tokenize("a > /dev/null && b 2>&1").unwrap();
        let pipeline = parse_pipeline(&tokens).unwrap();
        assert_eq!(pipeline.display_string(), "a > /dev/null && b 2>&1");
    }
}

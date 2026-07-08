//! Body splitting for shell blocks.
//!
//! Splits a shell block body into logical commands, handling line continuations.

use std::ops::Range;

use super::types::{ShellBlockCommand, ShellBlockError, SourceExcerpt};
use crate::markdown::compose::parse_utils::strip_blockquote_prefix;

/// A raw logical command grouped from a shell-block body, before tokenization.
///
/// Carries the continuation-folded command text and its byte extent within the
/// body so both the compose executor and the passive
/// [`scan_shell_block_commands`](crate::markdown::compose::directives_api::scan_shell_block_commands)
/// scanner share one splitting geometry.
pub(crate) struct RawShellCommand {
    /// The continuation-folded command text (block-quote markers stripped).
    pub raw_command: String,
    /// Byte span of the command's physical extent within the body.
    pub physical_span: Range<usize>,
    /// 1-based line number where the command starts.
    pub start_line: usize,
}

/// Groups a shell-block body into raw logical commands without tokenizing.
///
/// This is the single splitting authority shared by the compose executor
/// ([`split_logical_commands`]) and the passive language-server scanner.
///
/// ## Rules
///
/// - Blank physical lines are ignored.
/// - Block-quote markers (`> `) are stripped so a quoted block exposes the bare
///   command; byte offsets still index into `body`.
/// - A line ending with an unescaped `\` joins the next non-blank line with one
///   space separator.
///
/// The returned `Option` is a trailing command whose final physical line ended
/// in an unterminated `\` continuation. It is reported separately so the
/// executor can reject it as a parse error while a passive scan can still
/// surface the best-effort command it collected.
///
/// `body_start_line` is the absolute 1-based line number where the body begins.
pub(crate) fn group_logical_commands(
    body: &str,
    body_start_line: usize,
) -> (Vec<RawShellCommand>, Option<RawShellCommand>) {
    let mut commands = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut current_start_line: Option<usize> = None;
    let mut current_start_byte: Option<usize> = None;
    let mut last_end_byte: usize = 0;
    let mut in_continuation = false;

    for (i, (raw_line, raw_line_start)) in physical_lines(body).into_iter().enumerate() {
        let line_number = body_start_line + i;

        // Strip any block-quote markers so a block-quoted shell block exposes the
        // bare command; lines without `>` markers (and their significant leading
        // whitespace) are returned untouched. The byte offset is advanced past
        // the stripped markers so spans still index into `body`.
        let line = strip_blockquote_prefix(raw_line);
        let line_start = raw_line_start + (raw_line.len() - line.len());

        let trimmed = line.trim();

        if trimmed.is_empty() {
            continue;
        }

        // Check for continuation: last non-whitespace char is backslash
        // and it's NOT preceded by another backslash.
        let trimmed_right = line.trim_end();
        let is_continuation = trimmed_right.ends_with('\\') && !trimmed_right.ends_with("\\\\");

        if current_start_line.is_none() {
            current_start_line = Some(line_number);
            current_start_byte = Some(line_start);
        }
        last_end_byte = line_start + line.len();

        if is_continuation {
            // Remove trailing backslash and any whitespace before it
            let without_backslash = trimmed_right[..trimmed_right.len() - 1].trim_end();
            current_lines.push(without_backslash.to_string());
            in_continuation = true;
        } else {
            current_lines.push(line.to_string());
            in_continuation = false;

            commands.push(RawShellCommand {
                raw_command: current_lines.join(" "),
                physical_span: current_start_byte.take().unwrap()..last_end_byte,
                start_line: current_start_line.take().unwrap(),
            });

            current_lines.clear();
        }
    }

    let dangling = if in_continuation || !current_lines.is_empty() {
        let start_byte = current_start_byte.unwrap_or(0);
        Some(RawShellCommand {
            raw_command: current_lines.join(" "),
            physical_span: start_byte..last_end_byte.max(start_byte),
            start_line: current_start_line.unwrap_or(body_start_line),
        })
    } else {
        None
    };

    (commands, dangling)
}

/// Split a shell block body into logical commands.
///
/// Groups body lines via [`group_logical_commands`] (the shared splitting
/// geometry), then tokenizes each command with the shell expansion tokenizer.
///
/// `body_start_line` is the absolute 1-based line number where the body begins.
pub(crate) fn split_logical_commands(
    body: &str,
    body_start_line: usize,
) -> Result<Vec<ShellBlockCommand>, ShellBlockError> {
    let (groups, dangling) = group_logical_commands(body, body_start_line);
    let mut commands = Vec::new();

    for group in groups {
        let RawShellCommand {
            raw_command: raw,
            physical_span,
            start_line,
        } = group;

        let synthetic_ctx = biscuit_terminal::errors::SourceContext::new(
            std::path::PathBuf::from("<shell-block>"),
            std::path::PathBuf::from("<shell-block>"),
            raw.clone(),
        );
        let shell_tokens =
            crate::markdown::compose::shell_expansion::tokenize::tokenize(&raw, &synthetic_ctx)
                .map_err(|e| ShellBlockError::Parse {
                    line: start_line,
                    message: format!("Command tokenization failed: {e}"),
                    excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
                    source_file: None,
                })?;

        let pipeline = crate::markdown::compose::shell_expansion::tokenize::parse_pipeline(
            &shell_tokens,
            &synthetic_ctx,
        )
        .map_err(|e| ShellBlockError::Parse {
            line: start_line,
            message: format!("Command parsing failed: {e}"),
            excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
            source_file: None,
        })?;

        if pipeline.actions.is_empty() {
            return Err(ShellBlockError::Parse {
                line: start_line,
                message: "Empty command after tokenization".to_string(),
                excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
                source_file: None,
            });
        }

        let executable = pipeline.actions[0].command.executable.clone();
        let args = pipeline.actions[0].command.args.clone();

        commands.push(ShellBlockCommand {
            raw_command: raw,
            executable,
            args,
            pipeline,
            physical_span,
            start_line,
        });
    }

    // Handle unterminated continuation
    if let Some(dangling) = dangling {
        return Err(ShellBlockError::Parse {
            line: dangling.start_line,
            message: "Unterminated line continuation".to_string(),
            excerpt: SourceExcerpt::from_text(body, dangling.start_line, body_start_line, 2),
            source_file: None,
        });
    }

    Ok(commands)
}

fn physical_lines(input: &str) -> Vec<(&str, usize)> {
    let mut lines = Vec::new();
    let mut offset = 0usize;
    for segment in input.split_inclusive('\n') {
        let start = offset;
        offset += segment.len();
        let without_lf = segment.strip_suffix('\n').unwrap_or(segment);
        let line = without_lf.strip_suffix('\r').unwrap_or(without_lf);
        lines.push((line, start));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn single_command() {
        let cmds = split_logical_commands("echo hello", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[0].executable, "echo");
        assert_eq!(cmds[0].args, vec!["hello"]);
        assert_eq!(cmds[0].start_line, 1);
    }

    #[test]
    fn multiple_commands() {
        let cmds = split_logical_commands("echo hello\necho world", 1).unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[1].raw_command, "echo world");
    }

    #[test]
    fn strips_blockquote_markers_from_commands() {
        // Block-quoted body lines expose the bare command, and the physical span
        // points past the stripped `> ` markers into the real command bytes.
        let body = "> echo hello\n> echo world\n";
        let cmds = split_logical_commands(body, 1).unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[1].raw_command, "echo world");
        assert_eq!(&body[cmds[0].physical_span.clone()], "echo hello");
        assert_eq!(&body[cmds[1].physical_span.clone()], "echo world");
    }

    #[test]
    fn crlf_physical_spans_track_actual_bytes() {
        let body = "echo hello\r\necho world\r\n";
        let cmds = split_logical_commands(body, 10).unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].physical_span, 0..10);
        assert_eq!(cmds[1].physical_span, 12..22);
        assert_eq!(&body[cmds[0].physical_span.clone()], "echo hello");
        assert_eq!(&body[cmds[1].physical_span.clone()], "echo world");
        assert_eq!(cmds[0].start_line, 10);
        assert_eq!(cmds[1].start_line, 11);
    }

    #[test]
    fn blank_lines_ignored() {
        let cmds = split_logical_commands("echo hello\n\n\necho world", 1).unwrap();
        assert_eq!(cmds.len(), 2);
    }

    #[test]
    fn continuation_line() {
        let cmds = split_logical_commands("echo \\\n  hello", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].raw_command, "echo   hello");
        assert_eq!(cmds[0].executable, "echo");
        assert_eq!(cmds[0].args, vec!["hello"]);
        assert_eq!(cmds[0].start_line, 1);
    }

    #[test]
    fn continuation_skips_blank_lines() {
        let cmds = split_logical_commands("echo \\\n\n  hello", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].raw_command, "echo   hello");
    }

    #[test]
    fn escaped_backslash_not_continuation() {
        let cmds = split_logical_commands("echo \\\\\necho hello", 1).unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].raw_command, "echo \\\\");
    }

    #[test]
    fn rejects_pipe() {
        let result = split_logical_commands("ls | grep test", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("pipes"));
    }

    #[test]
    fn rejects_semicolon() {
        let result = split_logical_commands("echo a; echo b", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("chaining"));
    }

    #[test]
    fn rejects_redirect() {
        let result = split_logical_commands("echo hello > file.txt", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("redirection"));
    }

    #[test]
    fn accepts_stdout_null_redirect() {
        let cmds = split_logical_commands("echo hello > /dev/null", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].executable, "echo");
        assert_eq!(cmds[0].args, vec!["hello"]);
        assert_eq!(cmds[0].pipeline.actions.len(), 1);
    }

    #[test]
    fn empty_body() {
        let cmds = split_logical_commands("", 1).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn only_blank_lines() {
        let cmds = split_logical_commands("\n\n\n", 1).unwrap();
        assert!(cmds.is_empty());
    }

    #[test]
    fn unterminated_continuation() {
        let result = split_logical_commands("echo \\", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Unterminated"));
    }

    #[test]
    fn command_substitution_rejected() {
        let result = split_logical_commands("echo $(date)", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("substitution"));
    }

    #[test]
    fn double_ampersand_accepted_as_pipeline() {
        let cmds = split_logical_commands("make && make install", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].executable, "make");
        assert_eq!(cmds[0].pipeline.actions.len(), 2);
        assert_eq!(cmds[0].pipeline.actions[1].command.executable, "make");
        assert_eq!(cmds[0].pipeline.actions[1].command.args, vec!["install"]);
    }

    #[test]
    fn mixed_empty_and_non_empty() {
        let cmds = split_logical_commands("echo hello\n\necho world\n\necho done", 1).unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[1].raw_command, "echo world");
        assert_eq!(cmds[2].raw_command, "echo done");
    }
}

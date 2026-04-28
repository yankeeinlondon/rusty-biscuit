//! Body splitting for shell blocks.
//!
//! Splits a shell block body into logical commands, handling line continuations.

use super::types::{ShellBlockCommand, ShellBlockError, SourceExcerpt};

/// Split a shell block body into logical commands.
///
/// ## Rules
///
/// - Blank physical lines are ignored.
/// - A non-empty line starts a new logical command (or continues the current one).
/// - A line ending with an unescaped `\` joins with the next non-blank line
///   (one space separator).
/// - Each logical command is tokenized using the shell expansion tokenizer.
///
/// `body_start_line` is the absolute 1-based line number where the body begins.
pub(crate) fn split_logical_commands(
    body: &str,
    body_start_line: usize,
) -> Result<Vec<ShellBlockCommand>, ShellBlockError> {
    let lines: Vec<&str> = body.lines().collect();
    let mut commands = Vec::new();
    let mut current_lines: Vec<String> = Vec::new();
    let mut current_start_line: Option<usize> = None;
    let mut current_start_byte: Option<usize> = None;
    let mut in_continuation = false;

    let mut byte_offset = 0usize;

    for (i, line) in lines.iter().enumerate() {
        let line_number = body_start_line + i;
        let trimmed = line.trim();

        if trimmed.is_empty() {
            // Blank line: ignore, but track byte offset
            byte_offset += line.len() + 1; // +1 for newline
            continue;
        }

        // Check for continuation: last non-whitespace char is backslash
        // and it's NOT preceded by another backslash.
        let trimmed_right = line.trim_end();
        let is_continuation = trimmed_right.ends_with('\\')
            && !trimmed_right.ends_with("\\\\");

        if current_start_line.is_none() {
            current_start_line = Some(line_number);
            current_start_byte = Some(byte_offset);
        }

        if is_continuation {
            // Remove trailing backslash and any whitespace before it
            let without_backslash = trimmed_right[..trimmed_right.len() - 1].trim_end();
            current_lines.push(without_backslash.to_string());
            in_continuation = true;
        } else {
            current_lines.push(line.to_string());
            in_continuation = false;

            // End of command
            let raw = current_lines.join(" ");
            let start_line = current_start_line.unwrap();
            let end_line = line_number;
            let start_byte = current_start_byte.unwrap();
            let end_byte = byte_offset + line.len();

            // Tokenize
            let tokens = crate::markdown::compose::shell_expansion::tokenize::tokenize(&raw
            ).map_err(|e| ShellBlockError::Parse {
                line: start_line,
                message: format!("Command tokenization failed: {e}"),
                excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
            })?;

            if tokens.is_empty() {
                return Err(ShellBlockError::Parse {
                    line: start_line,
                    message: "Empty command after tokenization".to_string(),
                    excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
                });
            }

            let executable = tokens[0].clone();
            let args = tokens[1..].to_vec();

            commands.push(ShellBlockCommand {
                raw_command: raw,
                executable,
                args,
                physical_span: start_byte..end_byte,
                start_line,
                end_line,
            });

            current_lines.clear();
            current_start_line = None;
            current_start_byte = None;
        }

        byte_offset += line.len() + 1; // +1 for newline
    }

    // Handle unterminated continuation
    if in_continuation || !current_lines.is_empty() {
        let start_line = current_start_line.unwrap_or(body_start_line);
        return Err(ShellBlockError::Parse {
            line: start_line,
            message: "Unterminated line continuation".to_string(),
            excerpt: SourceExcerpt::from_text(body, start_line, body_start_line, 2),
        });
    }

    Ok(commands)
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
        assert_eq!(cmds[0].end_line, 1);
    }

    #[test]
    fn multiple_commands() {
        let cmds = split_logical_commands("echo hello\necho world", 1).unwrap();
        assert_eq!(cmds.len(), 2);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[1].raw_command, "echo world");
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
        assert_eq!(cmds[0].end_line, 2);
    }

    #[test]
    fn continuation_skips_blank_lines() {
        let cmds = split_logical_commands("echo \\\n\n  hello", 1).unwrap();
        assert_eq!(cmds.len(), 1);
        assert_eq!(cmds[0].raw_command, "echo   hello");
        assert_eq!(cmds[0].end_line, 3);
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
    fn double_ampersand_rejected() {
        let result = split_logical_commands("make && make install", 1);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Conditional execution"));
    }

    #[test]
    fn mixed_empty_and_non_empty() {
        let cmds = split_logical_commands(
            "echo hello\n\necho world\n\necho done",
            1,
        ).unwrap();
        assert_eq!(cmds.len(), 3);
        assert_eq!(cmds[0].raw_command, "echo hello");
        assert_eq!(cmds[1].raw_command, "echo world");
        assert_eq!(cmds[2].raw_command, "echo done");
    }
}

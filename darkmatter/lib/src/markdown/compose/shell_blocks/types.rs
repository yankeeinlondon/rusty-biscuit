//! Type definitions for shell blocks.

use std::ops::Range;
use std::path::PathBuf;
use std::time::Duration;

use crate::markdown::compose::shell_expansion::types::{ErrorHandling, ShellExpansionError};

/// A discovered shell block region with parsed options.
#[derive(Debug, Clone)]
pub(crate) struct ShellBlockRegion {
    /// Parsed error handling options from the opener line.
    pub options: ErrorHandling,
    /// Optional timeout override for all commands in this block.
    pub timeout_override: Option<Duration>,
}

/// A single logical command within a shell block.
#[derive(Debug, Clone)]
pub(crate) struct ShellBlockCommand {
    /// The raw command string after continuation folding.
    pub raw_command: String,
    /// The resolved executable.
    pub executable: String,
    /// Tokenized arguments.
    pub args: Vec<String>,
    /// Byte span within the block body.
    pub physical_span: Range<usize>,
    /// 1-based line number where this command starts.
    pub start_line: usize,
}

/// Result of executing a single logical command.
#[derive(Debug, Clone)]
pub(crate) struct ShellBlockCommandResult {
    /// The rendered output for this command.
    pub output: String,
}

/// Compact owned structure containing surrounding body lines for diagnostics.
#[derive(Debug, Clone, Default)]
pub struct SourceExcerpt {
    /// Lines around the relevant location: (absolute_line_number, text).
    pub lines: Vec<(usize, String)>,
    /// The line to highlight.
    pub highlight_line: usize,
}

impl SourceExcerpt {
    /// Build an excerpt from a text block.
    ///
    /// `highlight_line` is an absolute 1-based line number.
    /// `body_start_line` is the absolute line number where `text` starts.
    pub fn from_text(
        text: &str,
        highlight_line: usize,
        body_start_line: usize,
        context_lines: usize,
    ) -> Self {
        let all_lines: Vec<(usize, &str)> = text
            .lines()
            .enumerate()
            .map(|(i, line)| (body_start_line + i, line))
            .collect();

        let relative_highlight = highlight_line.saturating_sub(body_start_line);
        let start = relative_highlight.saturating_sub(context_lines);
        let end = (relative_highlight + context_lines + 1).min(all_lines.len());

        let lines = all_lines[start..end]
            .iter()
            .map(|(n, line)| (*n, line.to_string()))
            .collect();

        Self {
            lines,
            highlight_line,
        }
    }
}

/// Errors from shell block parsing or execution.
#[derive(Debug, thiserror::Error)]
pub enum ShellBlockError {
    /// Parse error in a shell block directive or body.
    #[error("Shell block parse error at line {line}: {message}")]
    Parse {
        line: usize,
        message: String,
        excerpt: SourceExcerpt,
        /// Source file where the error occurred, if known.
        source_file: Option<PathBuf>,
    },

    /// EOF reached with an unclosed shell block.
    #[error("Unterminated shell block opened at line {line}")]
    Unterminated {
        line: usize,
        opening_text: String,
        excerpt: SourceExcerpt,
        /// Source file where the error occurred, if known.
        source_file: Option<PathBuf>,
    },

    /// A command within a shell block failed with no matching error handler.
    #[error("Shell block command failed at line {command_line}: {source}")]
    Command {
        block_start_line: usize,
        command_line: usize,
        partial_output: Box<Vec<String>>,
        excerpt: SourceExcerpt,
        #[source]
        source: Box<ShellExpansionError>,
        /// Source file where the error occurred, if known.
        source_file: Option<PathBuf>,
    },
}

impl ShellBlockError {
    /// Attach a source file path to this error.
    ///
    /// Returns a new error with `source_file` set; consumes `self`.
    pub fn with_source_file(self, path: Option<PathBuf>) -> Self {
        match self {
            Self::Parse {
                line,
                message,
                excerpt,
                ..
            } => Self::Parse {
                line,
                message,
                excerpt,
                source_file: path,
            },
            Self::Unterminated {
                line,
                opening_text,
                excerpt,
                ..
            } => Self::Unterminated {
                line,
                opening_text,
                excerpt,
                source_file: path,
            },
            Self::Command {
                block_start_line,
                command_line,
                partial_output,
                excerpt,
                source,
                ..
            } => Self::Command {
                block_start_line,
                command_line,
                partial_output,
                excerpt,
                source,
                source_file: path,
            },
        }
    }
}

impl biscuit_terminal::errors::BlockError for ShellBlockError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            ShellBlockError::Parse {
                line,
                message,
                excerpt,
                source_file,
            } => {
                let mut body = String::new();
                if let Some(path) = source_file {
                    body.push_str(&format!("<dim>Source:</dim> {}\n", path.display()));
                }
                body.push_str(&format!(
                    "<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ));
                if !excerpt.lines.is_empty() {
                    body.push_str("\n<dim>Context:</dim>\n");
                    for (ln, text) in &excerpt.lines {
                        if *ln == excerpt.highlight_line {
                            body.push_str(&format!("  <red>> {ln:>4} | {text}</red>\n"));
                        } else {
                            body.push_str(&format!("  <dim>{ln:>4} |</dim> {text}\n"));
                        }
                    }
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ShellBlockError", "parse failed"))
                    .body(body)
                    .hint("Shell block parameters use <cyan>key=\"value\"</cyan> syntax, not <cyan>--flag</cyan> syntax.")
            }

            ShellBlockError::Unterminated {
                line,
                opening_text,
                excerpt,
                source_file,
            } => {
                let mut body = String::new();
                if let Some(path) = source_file {
                    body.push_str(&format!("<dim>Source:</dim> {}\n", path.display()));
                }
                body.push_str(&format!("<dim>Opened at line:</dim> {line}\n<dim>Opener:</dim> <cyan>{opening_text}</cyan>"));
                if !excerpt.lines.is_empty() {
                    body.push_str("\n<dim>Context:</dim>\n");
                    for (ln, text) in &excerpt.lines {
                        if *ln == excerpt.highlight_line {
                            body.push_str(&format!("  <red>> {ln:>4} | {text}</red>\n"));
                        } else {
                            body.push_str(&format!("  <dim>{ln:>4} |</dim> {text}\n"));
                        }
                    }
                }
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ShellBlockError", "unterminated block"))
                    .body(body)
                    .hint("Add <cyan>::end-block</cyan> to close the block.")
            }

            ShellBlockError::Command {
                block_start_line,
                command_line,
                partial_output,
                excerpt,
                source,
                source_file,
            } => {
                let mut body = String::new();
                if let Some(path) = source_file {
                    body.push_str(&format!("<dim>Source:</dim> {}\n", path.display()));
                }
                body.push_str(&format!("<dim>Block opened at line:</dim> {block_start_line}\n<dim>Command at line:</dim> {command_line}"));
                if !partial_output.is_empty() {
                    body.push_str("\n<dim>Partial output from earlier commands:</dim>\n");
                    for output in partial_output.iter() {
                        let dimmed = dim_output_for_error(output);
                        body.push_str(&format!("<dim>---</dim>\n{dimmed}\n"));
                    }
                }
                if !excerpt.lines.is_empty() {
                    body.push_str("\n<dim>Context:</dim>\n");
                    for (ln, text) in &excerpt.lines {
                        if *ln == excerpt.highlight_line {
                            body.push_str(&format!("  <red>> {ln:>4} | {text}</red>\n"));
                        } else {
                            body.push_str(&format!("  <dim>{ln:>4} |</dim> {text}\n"));
                        }
                    }
                }
                let mut block = StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new("ShellBlockError", "command failed"))
                    .body(body);

                let hint = match source.as_ref() {
                    ShellExpansionError::ParseDirective { .. } => "Check the command syntax.",
                    ShellExpansionError::CommandNotFound { .. } => {
                        "Install the binary or update <cyan>$PATH</cyan>."
                    }
                    ShellExpansionError::Blacklisted { .. } => {
                        "Remove the entry from your blacklist file if you trust this command."
                    }
                    ShellExpansionError::ApprovalRequired { .. } => {
                        "Re-run with <cyan>--approve-shell</cyan> or add the command to your whitelist."
                    }
                    ShellExpansionError::Denied { .. } => {
                        "The user declined to approve this shell command."
                    }
                    ShellExpansionError::Timeout { .. } => {
                        "Raise the timeout, or pass <cyan>--allow-shell-timeout</cyan> to warn instead of fail."
                    }
                    ShellExpansionError::ExecutionFailed { .. } => {
                        "Run the command directly to reproduce the failure."
                    }
                    _ => "Check the command and its error handling options.",
                };
                block = block.hint(hint);
                block
            }
        }
    }

    fn block_source(&self) -> Option<&(dyn biscuit_terminal::errors::BlockError + 'static)> {
        None
    }
}

/// Truncate output for display in error blocks.
fn truncate_output(text: &str) -> String {
    const MAX_LINES: usize = 10;
    const MAX_BYTES: usize = 1024;

    let truncated = if text.len() > MAX_BYTES {
        let cut = text
            .char_indices()
            .map(|(i, _)| i)
            .take_while(|&i| i <= MAX_BYTES)
            .last()
            .unwrap_or(0);
        format!("{}\n<dim>... output truncated</dim>", &text[..cut])
    } else {
        text.to_string()
    };

    let lines: Vec<&str> = truncated.lines().collect();
    if lines.len() > MAX_LINES {
        let head = lines[..MAX_LINES].join("\n");
        format!(
            "{head}\n<dim>... {} more lines</dim>",
            lines.len() - MAX_LINES
        )
    } else {
        truncated
    }
}

/// Wrap each line of output with `<dim>` tags for visual demotion in error
/// displays. Preserves the structure while making it visually secondary.
fn dim_output_for_error(text: &str) -> String {
    let truncated = truncate_output(text);
    truncated
        .lines()
        .map(|line| format!("<dim>{line}</dim>"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_excerpt_from_text() {
        let text = "line1\nline2\nline3\nline4\nline5";
        let excerpt = SourceExcerpt::from_text(text, 3, 1, 1);
        assert_eq!(excerpt.lines.len(), 3);
        assert_eq!(excerpt.lines[0], (2, "line2".to_string()));
        assert_eq!(excerpt.lines[1], (3, "line3".to_string()));
        assert_eq!(excerpt.lines[2], (4, "line4".to_string()));
        assert_eq!(excerpt.highlight_line, 3);
    }

    #[test]
    fn source_excerpt_clamps_to_bounds() {
        let text = "line1\nline2\nline3";
        let excerpt = SourceExcerpt::from_text(text, 1, 1, 5);
        assert_eq!(excerpt.lines.len(), 3);
    }

    #[test]
    fn dim_output_wraps_lines() {
        let output = "line1\nline2\nline3";
        let dimmed = dim_output_for_error(output);
        assert!(dimmed.contains("<dim>line1</dim>"));
        assert!(dimmed.contains("<dim>line2</dim>"));
        assert!(dimmed.contains("<dim>line3</dim>"));
    }

    #[test]
    fn dim_output_truncates_long_output() {
        let long = "x\n".repeat(20);
        let dimmed = dim_output_for_error(&long);
        assert!(dimmed.contains("more lines"));
    }
}

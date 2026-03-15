//! Parser for `::shell` directives in markdown content.

use super::tokenize::tokenize;
use super::types::{ShellDirective, ShellExpansionError};
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

    for line in content.lines() {
        let line_start = byte_offset;
        let line_with_newline_end = if byte_offset + line.len() < content.len() {
            byte_offset + line.len() + 1 // Include newline
        } else {
            byte_offset + line.len()
        };

        // Check if this line is inside a code region
        if !is_in_code_region(line_start, &code_regions) {
            let trimmed = line.trim();
            if let Some(command_text) = trimmed.strip_prefix("::shell ") {
                // Parse the command
                let tokens = tokenize(command_text).map_err(|e| {
                    ShellExpansionError::ParseDirective {
                        line: line_num,
                        message: match e {
                            ShellExpansionError::ParseDirective { message, .. } => message,
                            _ => e.to_string(),
                        },
                    }
                })?;

                if tokens.is_empty() {
                    return Err(ShellExpansionError::ParseDirective {
                        line: line_num,
                        message: "Empty command".to_string(),
                    });
                }

                let executable = tokens[0].clone();
                let args = tokens[1..].to_vec();

                directives.push(ShellDirective {
                    raw_command: command_text.to_string(),
                    executable,
                    args,
                    span: line_start..line_with_newline_end,
                    line: line_num,
                });
            }
        }

        byte_offset = line_with_newline_end;
        line_num += 1;
    }

    Ok(directives)
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
    fn parse_directive_without_trailing_newline() {
        let content = "::shell echo hello";
        let directives = parse_directives(content).unwrap();
        assert_eq!(directives.len(), 1);
        assert_eq!(directives[0].span.start, 0);
        assert_eq!(directives[0].span.end, content.len());
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
}

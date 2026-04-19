use crate::discovery::eval::strip_ansi_codes;

/// Produces a vector where each element represents a line's length
/// after all escape codes have been stripped.
pub fn content_length(content: &str) -> Vec<u32> {
    content
        .lines()
        .map(|line| strip_ansi_codes(line).len() as u32)
        .collect()
}

/// Escape a string for display in shell-like contexts.
///
/// Safe alphanumeric characters and common shell-safe punctuation are left
/// unquoted. Everything else is single-quoted with internal quotes escaped.
/// Control characters (`\n`, `\r`, `\t`) are replaced with their visible
/// escape sequences.
///
/// This is intended for **display purposes** (logging, dry-run output), not
/// for constructing commands that will be executed. For execution safety, use
/// `claudine::actions::bash_executor::shell_escape`.
pub fn shell_escape_for_display(arg: &str) -> String {
    if arg.is_empty() {
        return "''".to_string();
    }

    if arg
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '/' | ':' | '='))
    {
        return arg.to_string();
    }

    let escaped = arg
        .replace('\'', "'\\''")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
        .replace('\t', "\\t");
    format!("'{escaped}'")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_escape_safe_chars_unquoted() {
        assert_eq!(shell_escape_for_display("hello"), "hello");
        assert_eq!(
            shell_escape_for_display("path/to/file.rs"),
            "path/to/file.rs"
        );
    }

    #[test]
    fn display_escape_empty() {
        assert_eq!(shell_escape_for_display(""), "''");
    }

    #[test]
    fn display_escape_spaces() {
        assert_eq!(shell_escape_for_display("hello world"), "'hello world'");
    }

    #[test]
    fn display_escape_single_quotes() {
        assert_eq!(shell_escape_for_display("it's"), "'it'\\''s'");
    }

    #[test]
    fn display_escape_control_chars() {
        assert_eq!(shell_escape_for_display("line1\nline2"), "'line1\\nline2'");
        assert_eq!(shell_escape_for_display("col1\tcol2"), "'col1\\tcol2'");
    }
}

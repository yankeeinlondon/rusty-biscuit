//! Output rendering for shell blocks.

use super::types::ShellBlockCommandResult;

/// Render the combined output of a shell block's commands.
///
/// ## Contract
///
/// Concatenates each command's captured combined output verbatim. No trimming,
/// blank-line insertion, or other normalization is applied — the only
/// transformation a shell block performs on captured output is the container
/// indentation added by the caller at the splice boundary (spec requirement 5).
/// Separation between commands therefore comes from the commands' own trailing
/// newlines, exactly as a single `::shell` directive preserves its output.
pub(crate) fn render_block_output(results: &[ShellBlockCommandResult]) -> String {
    results.iter().map(|result| result.output.as_str()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_result(output: &str) -> ShellBlockCommandResult {
        ShellBlockCommandResult {
            output: output.to_string(),
        }
    }

    #[test]
    fn empty_results() {
        assert_eq!(render_block_output(&[]), "");
    }

    #[test]
    fn single_output_is_verbatim() {
        // Each command's own trailing newline is preserved, never added or stripped.
        let results = vec![make_result("hello\n")];
        assert_eq!(render_block_output(&results), "hello\n");
    }

    #[test]
    fn multiple_outputs_concatenate_verbatim() {
        // Separation comes from the commands' own trailing newlines, not an
        // inserted blank line.
        let results = vec![make_result("hello\n"), make_result("world\n")];
        assert_eq!(render_block_output(&results), "hello\nworld\n");
    }

    #[test]
    fn empty_output_contributes_nothing() {
        let results = vec![make_result("hello\n"), make_result(""), make_result("world\n")];
        assert_eq!(render_block_output(&results), "hello\nworld\n");
    }

    #[test]
    fn all_empty() {
        let results = vec![make_result(""), make_result(""), make_result("")];
        assert_eq!(render_block_output(&results), "");
    }

    #[test]
    fn leading_and_trailing_whitespace_preserved() {
        // Byte-for-byte preservation: surrounding spaces survive unchanged.
        let results = vec![make_result("  hello  \n"), make_result("world")];
        assert_eq!(render_block_output(&results), "  hello  \nworld");
    }

    #[test]
    fn output_without_trailing_newline_preserved() {
        // A command that emits no trailing newline keeps that shape; the next
        // command's output is appended directly (no normalization).
        let results = vec![make_result("a"), make_result("b\n")];
        assert_eq!(render_block_output(&results), "ab\n");
    }
}

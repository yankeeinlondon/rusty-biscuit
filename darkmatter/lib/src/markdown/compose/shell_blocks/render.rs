//! Output rendering for shell blocks.

use super::types::ShellBlockCommandResult;

/// Render the combined output of a shell block's commands.
///
/// ## Contract
///
/// - Trim each command's combined output.
/// - Drop empty command outputs entirely.
/// - Append one newline after each non-empty command output.
/// - Insert one blank line between non-empty command outputs.
/// - Return empty string if all outputs are empty.
pub(crate) fn render_block_output(results: &[ShellBlockCommandResult]) -> String {
    let mut non_empty: Vec<&str> = Vec::new();

    for result in results {
        let trimmed = result.output.trim();
        if !trimmed.is_empty() {
            non_empty.push(trimmed);
        }
    }

    if non_empty.is_empty() {
        return String::new();
    }

    let mut output = String::new();
    for (i, text) in non_empty.iter().enumerate() {
        if i > 0 {
            output.push('\n');
        }
        output.push_str(text);
        output.push('\n');
    }

    output
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
    fn single_output() {
        let results = vec![make_result("hello")];
        assert_eq!(render_block_output(&results), "hello\n");
    }

    #[test]
    fn multiple_outputs() {
        let results = vec![make_result("hello"), make_result("world")];
        assert_eq!(render_block_output(&results), "hello\n\nworld\n");
    }

    #[test]
    fn empty_output_omitted() {
        let results = vec![make_result("hello"), make_result(""), make_result("world")];
        assert_eq!(render_block_output(&results), "hello\n\nworld\n");
    }

    #[test]
    fn all_empty() {
        let results = vec![make_result(""), make_result("  "), make_result("")];
        assert_eq!(render_block_output(&results), "");
    }

    #[test]
    fn trimmed_output() {
        let results = vec![make_result("  hello  "), make_result("world\n")];
        assert_eq!(render_block_output(&results), "hello\n\nworld\n");
    }

    #[test]
    fn mixed_empty_and_non_empty() {
        let results = vec![
            make_result(""),
            make_result("a"),
            make_result(""),
            make_result(""),
            make_result("b"),
            make_result(""),
        ];
        assert_eq!(render_block_output(&results), "a\n\nb\n");
    }

    #[test]
    fn single_empty_amidst_empty() {
        let results = vec![make_result(""), make_result("only"), make_result("")];
        assert_eq!(render_block_output(&results), "only\n");
    }
}

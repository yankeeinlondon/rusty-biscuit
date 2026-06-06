//! Inline-compose / sequence mismatch detection and authored-YAML capture.
//!
//! A document that authors **both** a non-null `prompt` and a non-null
//! `sequence` defines an *inline sequence* and must be run with
//! `claudine sequence`, not `claudine inline-compose`. This module provides the
//! pure detection predicate plus a verbatim frontmatter-YAML capture used by
//! the mismatch diagnostic. See
//! [`claudine/docs/topics/composition.md`](../../../docs/topics/composition.md).

use super::types::ResolvedCompositionSource;

/// Whether a resolved source is an inline-compose / sequence mismatch.
///
/// True iff the *authored* frontmatter contains a `prompt` key whose value is
/// not `null` **and** a `sequence` key whose value is not `null`. Absent,
/// null, and non-null are distinguished via `as_map().get(key)`: an absent key
/// is not a mismatch, and an explicit `null` is not a mismatch. Value type or
/// validity is never inspected — any non-null value counts.
///
/// Detection reads authored frontmatter only; command-line property overrides
/// never participate.
pub fn is_inline_sequence_mismatch(source: &ResolvedCompositionSource) -> bool {
    let fm = source.markdown.frontmatter();
    let map = fm.as_map();
    let prompt_non_null = matches!(map.get("prompt"), Some(v) if !v.is_null());
    let sequence_non_null = matches!(map.get("sequence"), Some(v) if !v.is_null());
    prompt_non_null && sequence_non_null
}

/// Capture the authored frontmatter YAML interior verbatim, to spec
/// boundaries.
///
/// Returns the text between the first `---` delimiter line and the closing
/// `---` delimiter line, **excluding** both delimiter lines and **excluding**
/// the single line-ending that separates the last YAML line from the closing
/// delimiter. Every interior line-ending is preserved exactly (LF stays LF,
/// CRLF stays CRLF), and the YAML is never parsed or reserialized.
///
/// ## Returns
///
/// `None` when `original_text` has no well-formed frontmatter block (no leading
/// `---` line, or no closing `---` line).
pub fn capture_frontmatter_yaml(original_text: &str) -> Option<String> {
    let mut lines = original_text.split_inclusive('\n');

    let opening = lines.next()?;
    if trim_line_ending(opening) != "---" {
        return None;
    }

    let yaml_start = opening.len();
    let mut offset = yaml_start;
    for line in lines {
        if trim_line_ending(line) == "---" {
            // `offset` is the start of the closing delimiter line; the interior
            // YAML is everything between the opening line and here. Strip the
            // single delimiter-adjacent line-ending so the closing `---`'s
            // preceding separator is not part of the captured payload.
            let interior = &original_text[yaml_start..offset];
            return Some(strip_final_line_ending(interior).to_string());
        }
        offset += line.len();
    }

    None
}

/// Strip a single trailing `\r\n` or `\n` from `text`, leaving all interior
/// line-endings untouched.
fn strip_final_line_ending(text: &str) -> &str {
    if let Some(stripped) = text.strip_suffix("\r\n") {
        stripped
    } else if let Some(stripped) = text.strip_suffix('\n') {
        stripped
    } else {
        text
    }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composition::types::ResolvedCompositionSource;
    use std::path::PathBuf;

    fn source_from(text: &str) -> ResolvedCompositionSource {
        ResolvedCompositionSource {
            original_ref: "test.md".to_string(),
            resolved_path: PathBuf::from("test.md"),
            original_text: text.to_string(),
            markdown: text.to_string().into(),
        }
    }

    // --- is_inline_sequence_mismatch truth table (spec criteria 1-8) ---

    #[test]
    fn mismatch_string_prompt_nonempty_list_sequence() {
        // Criterion 1: valid string prompt + nonempty list → true.
        let src = source_from("---\nprompt: Do something\nsequence:\n  - name: Hello\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn mismatch_empty_sequence_list() {
        // Criterion 2: `sequence: []` is non-null → true.
        let src = source_from("---\nprompt: Do something\nsequence: []\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn mismatch_scalar_sequence() {
        // Criterion 3: prompt + scalar sequence (wrong type, non-null) → true.
        let src = source_from("---\nprompt: Do something\nsequence: nope\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn mismatch_mapping_sequence() {
        // Criterion 3: prompt + mapping sequence (wrong type, non-null) → true.
        let src = source_from("---\nprompt: Do something\nsequence:\n  key: value\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn no_mismatch_sequence_null() {
        // Criterion 4: prompt + `sequence: null` → false.
        let src = source_from("---\nprompt: Do something\nsequence: null\n---\nbody\n");
        assert!(!is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn no_mismatch_no_sequence_key() {
        // Criterion 5: prompt + no sequence key → false.
        let src = source_from("---\nprompt: Do something\n---\nbody\n");
        assert!(!is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn no_mismatch_sequence_without_prompt() {
        // Criterion 6: non-null sequence + no prompt key → false.
        let src = source_from("---\nsequence:\n  - name: Hello\n---\nbody\n");
        assert!(!is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn mismatch_empty_string_prompt_nonnull_sequence() {
        // Criterion 7: empty (non-null) prompt + non-null sequence → true.
        let src = source_from("---\nprompt: \"\"\nsequence: []\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn mismatch_wrong_type_prompt_nonnull_sequence() {
        // Criterion 7: wrong-type (non-null) prompt + non-null sequence → true.
        let src = source_from("---\nprompt: 42\nsequence: []\n---\nbody\n");
        assert!(is_inline_sequence_mismatch(&src));
    }

    #[test]
    fn no_mismatch_prompt_null_nonnull_sequence() {
        // Criterion 8: `prompt: null` + non-null sequence → false.
        let src = source_from("---\nprompt: null\nsequence:\n  - name: Hello\n---\nbody\n");
        assert!(!is_inline_sequence_mismatch(&src));
    }

    // --- capture_frontmatter_yaml fidelity (spec criteria 13, 14) ---

    #[test]
    fn capture_basic_excludes_delimiters_and_final_separator() {
        let text = "---\nprompt: x\nsequence: []\n---\nbody\n";
        assert_eq!(
            capture_frontmatter_yaml(text).as_deref(),
            Some("prompt: x\nsequence: []"),
        );
    }

    #[test]
    fn capture_preserves_comments_order_anchors_block_scalar() {
        // Criterion 13: comments, non-canonical order, anchors/aliases, and a
        // block scalar are preserved verbatim.
        let yaml = "# a comment\n\
sequence: &seq\n\
  - name: Hello\n\
prompt: |-\n\
  multi\n\
  line\n\
alias: *seq";
        let text = format!("---\n{yaml}\n---\nbody\n");
        assert_eq!(capture_frontmatter_yaml(&text).as_deref(), Some(yaml));
    }

    #[test]
    fn capture_preserves_interior_lf() {
        // Criterion 14: LF interior line-endings preserved; only the final
        // separator stripped.
        let text = "---\na: 1\nb: 2\n---\nbody\n";
        let captured = capture_frontmatter_yaml(text).unwrap();
        assert_eq!(captured, "a: 1\nb: 2");
        assert!(!captured.ends_with('\n'));
    }

    #[test]
    fn capture_preserves_interior_crlf() {
        // Criterion 14: CRLF interior line-endings preserved; only the final
        // CRLF separator stripped.
        let text = "---\r\na: 1\r\nb: 2\r\n---\r\nbody\r\n";
        let captured = capture_frontmatter_yaml(text).unwrap();
        assert_eq!(captured, "a: 1\r\nb: 2");
        assert!(!captured.ends_with('\n'));
    }

    #[test]
    fn capture_returns_none_without_opening_delimiter() {
        assert_eq!(capture_frontmatter_yaml("no frontmatter here\n"), None);
    }

    #[test]
    fn capture_returns_none_without_closing_delimiter() {
        assert_eq!(capture_frontmatter_yaml("---\nprompt: x\nbody\n"), None);
    }
}

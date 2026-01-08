//! Line-level diff computation with inline change detection.
//!
//! Uses the `similar` crate to compute line-level diffs and identify
//! character-level changes within modified lines.

use similar::{ChangeTag, TextDiff};

/// A span within a line that represents a change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlineSpan {
    /// Start byte offset within the line.
    pub start: usize,
    /// End byte offset within the line.
    pub end: usize,
    /// Whether this span represents the changed portion (true) or unchanged (false).
    pub emphasized: bool,
}

/// Represents a single line in the diff output.
#[derive(Debug, Clone)]
pub enum DiffLine {
    /// Unchanged context line present in both versions.
    Context {
        /// Line number in the original file.
        line_no_old: usize,
        /// Line number in the updated file.
        line_no_new: usize,
        /// The line content (without trailing newline).
        content: String,
    },
    /// Line only in the original (was removed).
    Removed {
        /// Line number in the original file.
        line_no: usize,
        /// The line content (without trailing newline).
        content: String,
        /// Character-level change spans within this line.
        inline_changes: Vec<InlineSpan>,
    },
    /// Line only in the updated version (was added).
    Added {
        /// Line number in the updated file.
        line_no: usize,
        /// The line content (without trailing newline).
        content: String,
        /// Character-level change spans within this line.
        inline_changes: Vec<InlineSpan>,
    },
}

impl DiffLine {
    /// Returns true if this is a context line.
    pub fn is_context(&self) -> bool {
        matches!(self, DiffLine::Context { .. })
    }

    /// Returns true if this is a removed line.
    pub fn is_removed(&self) -> bool {
        matches!(self, DiffLine::Removed { .. })
    }

    /// Returns true if this is an added line.
    pub fn is_added(&self) -> bool {
        matches!(self, DiffLine::Added { .. })
    }

    /// Returns the content of this line.
    pub fn content(&self) -> &str {
        match self {
            DiffLine::Context { content, .. } => content,
            DiffLine::Removed { content, .. } => content,
            DiffLine::Added { content, .. } => content,
        }
    }
}

/// Compute a visual diff between two strings.
///
/// Returns a list of diff lines with context, additions, and removals.
/// Adjacent remove/add pairs are analyzed for inline character changes.
pub fn compute_visual_diff(original: &str, updated: &str) -> Vec<DiffLine> {
    let text_diff = TextDiff::from_lines(original, updated);
    let changes: Vec<_> = text_diff.iter_all_changes().collect();

    let mut result = Vec::new();
    let mut old_line = 1usize;
    let mut new_line = 1usize;
    let mut i = 0;

    while i < changes.len() {
        let change = &changes[i];

        match change.tag() {
            ChangeTag::Equal => {
                result.push(DiffLine::Context {
                    line_no_old: old_line,
                    line_no_new: new_line,
                    content: change.value().trim_end_matches('\n').to_string(),
                });
                old_line += 1;
                new_line += 1;
                i += 1;
            }
            ChangeTag::Delete => {
                // Look ahead for a matching Insert to compute inline changes
                let removed_content = change.value().trim_end_matches('\n');

                // Collect consecutive deletes
                let mut deletes = vec![(old_line, removed_content.to_string())];
                let mut j = i + 1;
                while j < changes.len() && changes[j].tag() == ChangeTag::Delete {
                    deletes.push((
                        old_line + deletes.len(),
                        changes[j].value().trim_end_matches('\n').to_string(),
                    ));
                    j += 1;
                }

                // Collect consecutive inserts after deletes
                let mut inserts = vec![];
                let _insert_start = j;
                while j < changes.len() && changes[j].tag() == ChangeTag::Insert {
                    inserts.push((
                        new_line + inserts.len(),
                        changes[j].value().trim_end_matches('\n').to_string(),
                    ));
                    j += 1;
                }

                // Pair up deletes and inserts for inline diff
                let max_pairs = deletes.len().min(inserts.len());

                for k in 0..deletes.len() {
                    let (line_no, del_content) = &deletes[k];
                    if k < max_pairs {
                        // Has a matching insert - compute inline changes
                        let (_, ins_content) = &inserts[k];
                        let inline = compute_inline_changes(del_content, ins_content);
                        result.push(DiffLine::Removed {
                            line_no: *line_no,
                            content: del_content.clone(),
                            inline_changes: inline.0,
                        });
                    } else {
                        // Pure removal, no matching insert
                        result.push(DiffLine::Removed {
                            line_no: *line_no,
                            content: del_content.clone(),
                            inline_changes: vec![InlineSpan {
                                start: 0,
                                end: del_content.len(),
                                emphasized: true,
                            }],
                        });
                    }
                }

                for k in 0..inserts.len() {
                    let (line_no, ins_content) = &inserts[k];
                    if k < max_pairs {
                        // Has a matching delete - compute inline changes
                        let (_, del_content) = &deletes[k];
                        let inline = compute_inline_changes(del_content, ins_content);
                        result.push(DiffLine::Added {
                            line_no: *line_no,
                            content: ins_content.clone(),
                            inline_changes: inline.1,
                        });
                    } else {
                        // Pure addition, no matching delete
                        result.push(DiffLine::Added {
                            line_no: *line_no,
                            content: ins_content.clone(),
                            inline_changes: vec![InlineSpan {
                                start: 0,
                                end: ins_content.len(),
                                emphasized: true,
                            }],
                        });
                    }
                }

                old_line += deletes.len();
                new_line += inserts.len();
                i = j;
            }
            ChangeTag::Insert => {
                // Standalone insert (no preceding delete)
                let content = change.value().trim_end_matches('\n').to_string();
                result.push(DiffLine::Added {
                    line_no: new_line,
                    content: content.clone(),
                    inline_changes: vec![InlineSpan {
                        start: 0,
                        end: content.len(),
                        emphasized: true,
                    }],
                });
                new_line += 1;
                i += 1;
            }
        }
    }

    result
}

/// Compute character-level inline changes between two strings.
///
/// Returns a tuple of (spans for old, spans for new) indicating which
/// portions of each string differ.
fn compute_inline_changes(old: &str, new: &str) -> (Vec<InlineSpan>, Vec<InlineSpan>) {
    use similar::Algorithm;

    let diff = TextDiff::configure()
        .algorithm(Algorithm::Patience)
        .diff_chars(old, new);

    let mut old_spans = Vec::new();
    let mut new_spans = Vec::new();
    let mut old_pos = 0usize;
    let mut new_pos = 0usize;

    for change in diff.iter_all_changes() {
        let value = change.value();
        let len = value.len();

        match change.tag() {
            ChangeTag::Equal => {
                // Unchanged portion
                if len > 0 {
                    old_spans.push(InlineSpan {
                        start: old_pos,
                        end: old_pos + len,
                        emphasized: false,
                    });
                    new_spans.push(InlineSpan {
                        start: new_pos,
                        end: new_pos + len,
                        emphasized: false,
                    });
                }
                old_pos += len;
                new_pos += len;
            }
            ChangeTag::Delete => {
                // Removed from old
                if len > 0 {
                    old_spans.push(InlineSpan {
                        start: old_pos,
                        end: old_pos + len,
                        emphasized: true,
                    });
                }
                old_pos += len;
            }
            ChangeTag::Insert => {
                // Added to new
                if len > 0 {
                    new_spans.push(InlineSpan {
                        start: new_pos,
                        end: new_pos + len,
                        emphasized: true,
                    });
                }
                new_pos += len;
            }
        }
    }

    // Merge adjacent spans with same emphasis
    (merge_spans(old_spans), merge_spans(new_spans))
}

/// Merge adjacent spans with the same emphasis value.
fn merge_spans(spans: Vec<InlineSpan>) -> Vec<InlineSpan> {
    if spans.is_empty() {
        return spans;
    }

    let mut result = Vec::with_capacity(spans.len());
    let mut current = spans[0].clone();

    for span in spans.into_iter().skip(1) {
        if span.emphasized == current.emphasized && span.start == current.end {
            // Merge with current
            current.end = span.end;
        } else {
            result.push(current);
            current = span;
        }
    }
    result.push(current);

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identical_content() {
        let content = "Hello\nWorld";
        let diff = compute_visual_diff(content, content);

        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|d| d.is_context()));
    }

    #[test]
    fn test_empty_files() {
        let diff = compute_visual_diff("", "");
        assert!(diff.is_empty());
    }

    #[test]
    fn test_pure_addition() {
        let diff = compute_visual_diff("", "Hello\nWorld");

        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|d| d.is_added()));

        if let DiffLine::Added { line_no, .. } = &diff[0] {
            assert_eq!(*line_no, 1);
        }
    }

    #[test]
    fn test_pure_removal() {
        let diff = compute_visual_diff("Hello\nWorld", "");

        assert_eq!(diff.len(), 2);
        assert!(diff.iter().all(|d| d.is_removed()));
    }

    #[test]
    fn test_line_modification() {
        let diff = compute_visual_diff("Hello World", "Hello Universe");

        // Should have one removed and one added
        assert_eq!(diff.len(), 2);

        let removed = diff.iter().find(|d| d.is_removed()).unwrap();
        let added = diff.iter().find(|d| d.is_added()).unwrap();

        assert_eq!(removed.content(), "Hello World");
        assert_eq!(added.content(), "Hello Universe");

        // Check inline changes highlight the differing part
        if let DiffLine::Removed { inline_changes, .. } = removed {
            let emphasized: Vec<_> = inline_changes.iter().filter(|s| s.emphasized).collect();
            assert!(!emphasized.is_empty());
        }
    }

    #[test]
    fn test_mixed_changes() {
        // Use consistent line endings (with trailing newlines)
        let original = "Line 1\nLine 2\nLine 3\n";
        let updated = "Line 1\nModified 2\nLine 3\nLine 4\n";

        let diff = compute_visual_diff(original, updated);

        // Line 1: context
        // Line 2 -> Modified 2: removed + added
        // Line 3: context
        // Line 4: added
        let context_count = diff.iter().filter(|d| d.is_context()).count();
        let removed_count = diff.iter().filter(|d| d.is_removed()).count();
        let added_count = diff.iter().filter(|d| d.is_added()).count();

        assert_eq!(context_count, 2); // Line 1 and Line 3
        assert_eq!(removed_count, 1); // Line 2
        assert_eq!(added_count, 2); // Modified 2 and Line 4
    }

    #[test]
    fn test_inline_changes() {
        let (old_spans, new_spans) = compute_inline_changes("Hello World", "Hello Universe");

        // "Hello " is unchanged, "World" -> "Universe"
        let old_emphasized: Vec<_> = old_spans.iter().filter(|s| s.emphasized).collect();
        let new_emphasized: Vec<_> = new_spans.iter().filter(|s| s.emphasized).collect();

        assert!(!old_emphasized.is_empty());
        assert!(!new_emphasized.is_empty());

        // The emphasized part in old should cover "World"
        // The emphasized part in new should cover "Universe"
    }

    #[test]
    fn test_unicode_content() {
        let original = "Hello 世界";
        let updated = "Hello 宇宙";

        let diff = compute_visual_diff(original, updated);
        assert_eq!(diff.len(), 2); // One removed, one added

        // Verify content is preserved
        let removed = diff.iter().find(|d| d.is_removed()).unwrap();
        let added = diff.iter().find(|d| d.is_added()).unwrap();

        assert_eq!(removed.content(), "Hello 世界");
        assert_eq!(added.content(), "Hello 宇宙");
    }
}

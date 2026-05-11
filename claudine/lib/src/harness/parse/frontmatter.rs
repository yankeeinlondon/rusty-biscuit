//! Frontmatter text extraction and rule source reconstruction.

use std::ops::RangeInclusive;
use std::path::Path;

use serde_json::Value;

use crate::harness::model::RuleSource;

/// Extract the YAML frontmatter slice from a markdown source, returning the
/// text between the opening and closing `---` fences and the 1-indexed line
/// number of the first frontmatter content line.
///
/// ## Returns
///
/// `Some((yaml_text, base_line))` where `base_line` is the 1-indexed line
/// number of the first body line of the frontmatter (the line immediately
/// after the opening `---`). `None` if the file does not start with a `---`
/// fence on its first line, or if no closing fence is found.
pub(super) fn extract_frontmatter_text(source: &str) -> Option<(&str, usize)> {
    // Frontmatter must start at the very first byte of the file with `---`
    // followed by a newline. Anything else (BOMs, leading whitespace, CRLF
    // before the first delimiter) falls through to the `None` fallback.
    let rest = source.strip_prefix("---\n")?;
    // Find the closing `---` line. Accept either `---\n` or a trailing
    // `---` at end of string. Look for it as a line by itself.
    let mut byte_offset = 0usize;
    for line in rest.split_inclusive('\n') {
        let trimmed = line.strip_suffix('\n').unwrap_or(line);
        if trimmed == "---" {
            // Slice inside the fences.
            let yaml = &rest[..byte_offset];
            return Some((yaml, 2));
        }
        byte_offset += line.len();
    }
    None
}

/// Build a [`RuleSource`] for an author-declared validation rule.
///
/// Prefers the original source slice authored by the user when a
/// [`SpanIndex`] line range and the raw frontmatter text are both
/// available, preserving comments, quoting, anchors, and indentation
/// exactly as written. Falls back to a `serde_yaml_ng`-reconstructed
/// single-key mapping when span recovery is unavailable.
///
/// ## Returns
///
/// `None` only when the reconstruction fallback also fails to produce
/// a YAML snippet (e.g. the JSON value is not representable as YAML),
/// leaving the reporter to fall back to its legacy single-line failure
/// rendering.
pub(super) fn build_rule_source(
    source_path: &Path,
    rule_name: &str,
    value: &Value,
    line_range: Option<RangeInclusive<usize>>,
    frontmatter_slice: Option<(&str, usize)>,
) -> Option<RuleSource> {
    let yaml_snippet = original_yaml_slice(line_range.as_ref(), frontmatter_slice)
        .or_else(|| reconstruct_yaml_snippet(rule_name, value))?;
    Some(RuleSource {
        file: source_path.to_path_buf(),
        line_range,
        yaml_snippet,
    })
}

/// Slice the original YAML lines for a rule out of the raw frontmatter text.
///
/// `line_range` is the 1-indexed inclusive line range of the rule **within
/// the source file**, as recovered by the conservative span finder.
/// `frontmatter_slice` carries the frontmatter text and the 1-indexed line
/// number of its first body line (i.e. the line immediately after the
/// opening `---` fence). When either is absent, or the requested range
/// falls outside the available frontmatter body, returns `None` so the
/// caller can fall back to YAML reconstruction.
///
/// The returned slice preserves all interior whitespace, comments,
/// quoting, and indentation exactly as authored, with at most a single
/// trailing newline trimmed.
fn original_yaml_slice(
    line_range: Option<&RangeInclusive<usize>>,
    frontmatter_slice: Option<(&str, usize)>,
) -> Option<String> {
    let range = line_range?;
    let (text, base_line) = frontmatter_slice?;
    let start = *range.start();
    let end = *range.end();
    if start < base_line || end < start {
        return None;
    }
    // Translate 1-indexed source-file line numbers into 0-indexed offsets
    // within `text` (whose first line corresponds to `base_line`).
    let start_idx = start - base_line;
    let end_idx = end - base_line;
    let lines: Vec<&str> = text.split('\n').collect();
    if end_idx >= lines.len() {
        return None;
    }
    let mut snippet = lines[start_idx..=end_idx].join("\n");
    // Trim a single trailing newline only; preserve all other whitespace.
    if snippet.ends_with('\n') {
        snippet.pop();
    }
    Some(snippet)
}

/// Reconstruct a YAML mapping (`name: value`) for the rule via
/// `serde_yaml_ng`. Used as the fallback when the original source slice
/// is unavailable.
fn reconstruct_yaml_snippet(rule_name: &str, value: &Value) -> Option<String> {
    use biscuit_file::serde_yaml_ng;
    let mut map = serde_yaml_ng::Mapping::new();
    map.insert(
        serde_yaml_ng::Value::String(rule_name.to_string()),
        serde_yaml_ng::to_value(value).ok()?,
    );
    serde_yaml_ng::to_string(&serde_yaml_ng::Value::Mapping(map)).ok()
}

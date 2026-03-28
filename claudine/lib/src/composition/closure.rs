//! Inline composition closure: body extraction, document reconstruction,
//! and atomic file write-back.
//!
//! These functions centralise the file-mutation side of inline composition
//! so that both the harness loop and the non-harness path share one
//! deterministic rewrite pipeline.

use std::collections::BTreeSet;
use std::path::Path;

use crate::composition::error::CompositionError;
use crate::composition::types::InlineClosurePlan;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Strip accidental frontmatter fences from provider output and validate
/// that the resulting body is non-empty.
pub fn extract_replacement_body(provider_output: &str) -> Result<String, CompositionError> {
    let trimmed = provider_output.trim();
    if trimmed.is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "provider returned an empty response".into(),
        ));
    }

    // If the provider ignored the guardrail and wrapped its output in
    // frontmatter fences, strip them.
    let body = strip_leading_frontmatter(trimmed);
    let body = body.trim();
    if body.is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "provider response contained only frontmatter with no body".into(),
        ));
    }

    Ok(body.to_string())
}

/// Validate the replacement body, reconstruct the document preserving
/// original frontmatter, and write atomically to `target_path`.
pub fn apply_inline_closure(
    plan: &InlineClosurePlan,
    replacement_body: &str,
    target_path: &Path,
    today: &str,
) -> Result<(), CompositionError> {
    if replacement_body.trim().is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "replacement body is empty".into(),
        ));
    }

    let replacement_markdown: darkmatter::markdown::Markdown = replacement_body.to_string().into();
    if replacement_markdown.hash_body(false) == plan.original_body_hash {
        return Err(CompositionError::InvalidInlineResponse(
            "replacement body is unchanged".into(),
        ));
    }

    let doc_string = rewrite_inline_document(&plan.original_document_text, replacement_body, today)
        .map_err(CompositionError::InvalidInlineResponse)?;

    crate::config::atomic::atomic_write(target_path, doc_string.as_bytes())
        .map_err(|e| CompositionError::AtomicWriteFailed(e.to_string()))?;

    Ok(())
}

/// Reconstruct a Markdown document from `frontmatter_source` (for its
/// frontmatter) and `body` (new body), updating `last_updated` to `today`.
pub fn rewrite_inline_document(
    frontmatter_source: &str,
    body: &str,
    today: &str,
) -> Result<String, String> {
    if let Some(parts) = split_frontmatter_parts(frontmatter_source) {
        let newline = detect_newline(frontmatter_source);
        let yaml = upsert_last_updated_in_frontmatter(parts.yaml, today, newline);
        let mut document = String::with_capacity(
            parts.opening.len() + yaml.len() + parts.closing.len() + body.len(),
        );
        document.push_str(parts.opening);
        document.push_str(&yaml);
        document.push_str(parts.closing);
        document.push_str(body);
        return Ok(document);
    }

    let mut markdown: darkmatter::markdown::Markdown = frontmatter_source.to_string().into();
    markdown
        .fm_insert("last_updated", today)
        .map_err(|e| format!("failed to update last_updated: {e}"))?;
    *markdown.content_mut() = body.to_string();
    Ok(markdown.as_string())
}

/// Return the set of frontmatter field names that Claudine manages
/// automatically during inline closure.
pub fn default_managed_fields() -> BTreeSet<String> {
    BTreeSet::from(["last_updated".into()])
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

/// Strip a leading frontmatter block (```---\n...\n---\n```) from text,
/// returning only the body that follows.
fn strip_leading_frontmatter(text: &str) -> &str {
    let mut lines = text.split_inclusive('\n');
    let first = match lines.next() {
        Some(l) => l,
        None => return text,
    };
    if trim_line_ending(first) != "---" {
        return text;
    }

    let mut offset = first.len();
    for line in lines {
        offset += line.len();
        if trim_line_ending(line) == "---" {
            return &text[offset..];
        }
    }

    // No closing delimiter — return as-is.
    text
}

struct FrontmatterParts<'a> {
    opening: &'a str,
    yaml: &'a str,
    closing: &'a str,
}

fn split_frontmatter_parts(text: &str) -> Option<FrontmatterParts<'_>> {
    let mut lines = text.split_inclusive('\n');
    let opening = lines.next()?;
    if trim_line_ending(opening) != "---" {
        return None;
    }

    let yaml_start = opening.len();
    let mut offset = yaml_start;
    for line in lines {
        let next_offset = offset + line.len();
        if trim_line_ending(line) == "---" {
            return Some(FrontmatterParts {
                opening: &text[..yaml_start],
                yaml: &text[yaml_start..offset],
                closing: &text[offset..next_offset],
            });
        }
        offset = next_offset;
    }

    None
}

fn upsert_last_updated_in_frontmatter(yaml: &str, today: &str, newline: &str) -> String {
    let mut updated = String::with_capacity(yaml.len() + today.len() + 32);
    let mut found = false;
    let mut had_trailing_newline = yaml.is_empty();

    for line in yaml.split_inclusive('\n') {
        let line_ending = if line.ends_with("\r\n") {
            "\r\n"
        } else if line.ends_with('\n') {
            "\n"
        } else {
            ""
        };
        let content = trim_line_ending(line);

        if let Some(rewritten) = rewrite_last_updated_line(content, today) {
            updated.push_str(&rewritten);
            updated.push_str(line_ending);
            found = true;
        } else {
            updated.push_str(line);
        }

        had_trailing_newline = !line_ending.is_empty();
    }

    if !found {
        if !updated.is_empty() && !had_trailing_newline {
            updated.push_str(newline);
        }
        updated.push_str("last_updated: ");
        updated.push_str(today);
        updated.push_str(newline);
    }

    updated
}

fn rewrite_last_updated_line(line: &str, today: &str) -> Option<String> {
    let trimmed = line.trim_start();
    let rest = trimmed.strip_prefix("last_updated:")?;
    let indent = &line[..line.len() - trimmed.len()];
    if !indent.is_empty() {
        return None;
    }
    let quote = rest
        .trim_start()
        .chars()
        .next()
        .filter(|quote| matches!(quote, '"' | '\''));

    let mut rewritten = String::from(indent);
    rewritten.push_str("last_updated: ");
    match quote {
        Some(quote) => {
            rewritten.push(quote);
            rewritten.push_str(today);
            rewritten.push(quote);
        }
        None => rewritten.push_str(today),
    }
    Some(rewritten)
}

fn detect_newline(text: &str) -> &str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    // -- extract_replacement_body -------------------------------------------

    #[test]
    fn extract_body_returns_trimmed_content() {
        let body = extract_replacement_body("  Hello world  ").unwrap();
        assert_eq!(body, "Hello world");
    }

    #[test]
    fn extract_body_rejects_empty_input() {
        assert!(extract_replacement_body("").is_err());
        assert!(extract_replacement_body("   ").is_err());
    }

    #[test]
    fn extract_body_strips_accidental_frontmatter() {
        let output = "---\ntitle: oops\n---\nActual body content\n";
        let body = extract_replacement_body(output).unwrap();
        assert_eq!(body, "Actual body content");
    }

    #[test]
    fn extract_body_rejects_frontmatter_only() {
        let output = "---\ntitle: oops\n---\n";
        assert!(extract_replacement_body(output).is_err());
    }

    #[test]
    fn apply_inline_closure_rejects_unchanged_body() {
        let original = "---\nprompt: write\n---\nOriginal body\n";
        let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_frontmatter_hash: original_markdown.hash_frontmatter(false),
            original_body_hash: original_markdown.hash_body(false),
            managed_fields: default_managed_fields(),
        };

        let err = apply_inline_closure(
            &plan,
            "Original body",
            Path::new("/tmp/nonexistent"),
            "2026-03-27",
        )
        .unwrap_err();

        assert!(matches!(err, CompositionError::InvalidInlineResponse(_)));
        assert!(err.to_string().contains("unchanged"));
    }

    #[test]
    fn extract_body_preserves_dashes_that_are_not_frontmatter() {
        let output = "Some text\n---\nMore text\n";
        let body = extract_replacement_body(output).unwrap();
        assert_eq!(body, "Some text\n---\nMore text");
    }

    // -- rewrite_inline_document --------------------------------------------

    #[test]
    fn rewrite_preserves_block_scalar_frontmatter_layout() {
        let original = concat!(
            "---\n",
            "prompt: |-\n",
            "  First line\n",
            "  Second line\n",
            "last_updated: 2026-03-18\n",
            "---\n",
            "Old body\n",
        );

        let rewritten = rewrite_inline_document(original, "Fresh body\n", "2026-03-19").unwrap();

        assert!(rewritten.contains("prompt: |-"));
        assert!(rewritten.contains("  First line\n  Second line\n"));
        assert!(rewritten.contains("last_updated: 2026-03-19"));
        assert!(rewritten.ends_with("---\nFresh body\n"));
    }

    #[test]
    fn rewrite_adds_last_updated_without_reserializing_frontmatter() {
        let original = concat!(
            "---\n",
            "prompt: |-\n",
            "  Keep this formatting\n",
            "---\n",
            "Body\n",
        );

        let rewritten = rewrite_inline_document(original, "Updated body\n", "2026-03-19").unwrap();

        assert!(rewritten.contains("prompt: |-"));
        assert!(rewritten.contains("  Keep this formatting\n"));
        assert!(rewritten.contains("last_updated: 2026-03-19\n---\nUpdated body\n"));
    }

    #[test]
    fn rewrite_updates_quoted_last_updated() {
        let original = "---\nlast_updated: \"2026-01-01\"\n---\nBody\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27").unwrap();
        assert!(rewritten.contains("last_updated: \"2026-03-27\""));
    }

    #[test]
    fn rewrite_updates_single_quoted_last_updated() {
        let original = "---\nlast_updated: '2026-01-01'\n---\nBody\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27").unwrap();
        assert!(rewritten.contains("last_updated: '2026-03-27'"));
    }

    #[test]
    fn rewrite_preserves_crlf_line_endings() {
        let original = "---\r\nlast_updated: 2026-01-01\r\n---\r\nBody\r\n";
        let rewritten = rewrite_inline_document(original, "New body\r\n", "2026-03-27").unwrap();
        assert!(rewritten.contains("last_updated: 2026-03-27\r\n"));
    }

    // -- apply_inline_closure -----------------------------------------------

    #[test]
    fn apply_closure_rejects_empty_body() {
        let plan = InlineClosurePlan {
            original_document_text: "---\nprompt: test\n---\nOld\n".into(),
            original_frontmatter_hash: 0,
            original_body_hash: 0,
            managed_fields: default_managed_fields(),
        };
        let err = apply_inline_closure(&plan, "  ", Path::new("/tmp/nonexistent"), "2026-03-27");
        assert!(err.is_err());
    }

    // -- default_managed_fields ---------------------------------------------

    #[test]
    fn managed_fields_contains_last_updated() {
        let fields = default_managed_fields();
        assert!(fields.contains("last_updated"));
        assert_eq!(fields.len(), 1);
    }

    // -- strip_leading_frontmatter ------------------------------------------

    #[test]
    fn strip_frontmatter_removes_leading_fences() {
        let text = "---\ntitle: test\n---\nBody here\n";
        assert_eq!(strip_leading_frontmatter(text), "Body here\n");
    }

    #[test]
    fn strip_frontmatter_preserves_text_without_fences() {
        let text = "Just plain text\nwith lines\n";
        assert_eq!(strip_leading_frontmatter(text), text);
    }

    #[test]
    fn strip_frontmatter_preserves_unclosed_fences() {
        let text = "---\ntitle: test\nNo closing fence\n";
        assert_eq!(strip_leading_frontmatter(text), text);
    }

    // -- upsert_last_updated ------------------------------------------------

    #[test]
    fn upsert_adds_when_missing() {
        let yaml = "prompt: test\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n");
        assert!(result.contains("last_updated: 2026-03-27\n"));
        assert!(result.contains("prompt: test\n"));
    }

    #[test]
    fn upsert_replaces_existing() {
        let yaml = "last_updated: 2026-01-01\nprompt: test\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n");
        assert!(result.contains("last_updated: 2026-03-27\n"));
        assert!(!result.contains("2026-01-01"));
    }

    #[test]
    fn upsert_ignores_indented_last_updated() {
        let yaml = "  last_updated: 2026-01-01\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n");
        // Indented line should not be rewritten (it's nested YAML)
        assert!(result.contains("  last_updated: 2026-01-01"));
        // A new top-level entry should be appended
        assert!(result.contains("last_updated: 2026-03-27\n"));
    }
}

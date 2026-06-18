//! Inline composition closure: body extraction, document reconstruction,
//! and atomic file write-back.
//!
//! These functions centralise the file-mutation side of inline composition
//! so that both the harness loop and the non-harness path share one
//! deterministic rewrite pipeline.

use std::path::Path;

use indexmap::IndexMap;

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

/// Result of applying inline closure, reporting frontmatter changes.
#[derive(Debug, Clone, Default)]
pub struct InlineClosureResult {
    /// Keys that were added by the agent and merged into the document.
    pub new_properties: Vec<String>,
    /// Keys that were modified by the agent and reverted to original values.
    pub reverted_properties: Vec<String>,
}

/// Validate the replacement body, reconstruct the document preserving
/// original frontmatter, and write atomically to `target_path`.
pub fn apply_inline_closure(
    plan: &InlineClosurePlan,
    replacement_body: &str,
    target_path: &Path,
    today: &str,
    post_run_frontmatter: Option<&IndexMap<String, serde_json::Value>>,
) -> Result<InlineClosureResult, CompositionError> {
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

    // Compare frontmatter to detect new and modified properties
    let (new_properties, reverted_properties) = match post_run_frontmatter {
        Some(post_run_fm) => compare_frontmatter(&plan.original_document_text, post_run_fm),
        None => (vec![], vec![]),
    };

    let serialized_props: Vec<(String, String)> = new_properties
        .iter()
        .filter_map(|key| {
            post_run_frontmatter
                .and_then(|fm| fm.get(key))
                .map(|value| (key.clone(), serialize_frontmatter_property(key, value)))
        })
        .collect();

    let doc_string = rewrite_inline_document(
        &plan.original_document_text,
        replacement_body,
        today,
        &serialized_props,
    )
    .map_err(CompositionError::InvalidInlineResponse)?;

    crate::config::atomic::atomic_write(target_path, doc_string.as_bytes())
        .map_err(|e| CompositionError::AtomicWriteFailed(e.to_string()))?;

    Ok(InlineClosureResult {
        new_properties,
        reverted_properties,
    })
}

/// Reconstruct a Markdown document from `frontmatter_source` (for its
/// frontmatter) and `body` (new body), updating `last_updated` to `today`.
pub fn rewrite_inline_document(
    frontmatter_source: &str,
    body: &str,
    today: &str,
    new_properties: &[(String, String)],
) -> Result<String, String> {
    if let Some(parts) = split_frontmatter_parts(frontmatter_source) {
        let newline = detect_newline(frontmatter_source);
        let prop_lines: Vec<String> = new_properties.iter().map(|(_, v)| v.clone()).collect();
        let yaml = upsert_last_updated_in_frontmatter(parts.yaml, today, newline, &prop_lines);
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

fn upsert_last_updated_in_frontmatter(
    yaml: &str,
    today: &str,
    newline: &str,
    new_properties: &[String],
) -> String {
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
            // Inject new properties just before last_updated
            for prop in new_properties {
                updated.push_str(prop);
            }
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
        // Inject new properties before last_updated
        for prop in new_properties {
            updated.push_str(prop);
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

/// Compare post-run frontmatter against the original document's frontmatter.
///
/// Returns `(new_keys, modified_keys)` where:
/// - `new_keys`: present in post-run but absent in original
/// - `modified_keys`: present in both but with different values
fn compare_frontmatter(
    original_document_text: &str,
    post_run_fm: &IndexMap<String, serde_json::Value>,
) -> (Vec<String>, Vec<String>) {
    let original_md: darkmatter::markdown::Markdown = original_document_text.to_string().into();
    let original_fm = original_md.frontmatter().as_map();

    let mut new_keys = Vec::new();
    let mut modified_keys = Vec::new();

    for (key, post_value) in post_run_fm {
        // Skip last_updated — managed by the closure itself
        if key == "last_updated" {
            continue;
        }
        match original_fm.get(key) {
            None => new_keys.push(key.clone()),
            Some(original_value) if original_value != post_value => {
                modified_keys.push(key.clone());
            }
            Some(_) => {} // unchanged
        }
    }

    (new_keys, modified_keys)
}

/// Serialize a single frontmatter property as a YAML fragment.
///
/// Simple scalars produce `key: value\n`. Complex types (arrays, objects)
/// delegate to `serde_yaml_ng` for the value portion.
fn serialize_frontmatter_property(key: &str, value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::String(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::Bool(_)
        | serde_json::Value::Null => {
            let yaml_value = biscuit_file::serde_yaml_ng::to_string(value)
                .unwrap_or_else(|_| format!("{value}"));
            let yaml_value = yaml_value.trim_end_matches('\n');
            format!("{key}: {yaml_value}\n")
        }
        complex => {
            let yaml_value = biscuit_file::serde_yaml_ng::to_string(complex)
                .unwrap_or_else(|_| format!("{complex}"));
            let yaml_value = yaml_value.trim_end_matches('\n');
            let indented = yaml_value
                .lines()
                .map(|line| format!("  {line}"))
                .collect::<Vec<_>>()
                .join("\n");
            format!("{key}:\n{indented}\n")
        }
    }
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
            original_body_hash: original_markdown.hash_body(false),
        };

        let err = apply_inline_closure(
            &plan,
            "Original body",
            Path::new("/tmp/nonexistent"),
            "2026-03-27",
            None,
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

        let rewritten =
            rewrite_inline_document(original, "Fresh body\n", "2026-03-19", &[]).unwrap();

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

        let rewritten =
            rewrite_inline_document(original, "Updated body\n", "2026-03-19", &[]).unwrap();

        assert!(rewritten.contains("prompt: |-"));
        assert!(rewritten.contains("  Keep this formatting\n"));
        assert!(rewritten.contains("last_updated: 2026-03-19\n---\nUpdated body\n"));
    }

    #[test]
    fn rewrite_updates_quoted_last_updated() {
        let original = "---\nlast_updated: \"2026-01-01\"\n---\nBody\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();
        assert!(rewritten.contains("last_updated: \"2026-03-27\""));
    }

    #[test]
    fn rewrite_updates_single_quoted_last_updated() {
        let original = "---\nlast_updated: '2026-01-01'\n---\nBody\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-03-27", &[]).unwrap();
        assert!(rewritten.contains("last_updated: '2026-03-27'"));
    }

    #[test]
    fn rewrite_preserves_schema_property_with_inline_value() {
        // Phase 5 Task 2: `$schema` must survive the inline rewrite so the
        // document continues to validate on subsequent runs.
        let original = concat!(
            "---\n",
            "$schema:\n",
            "  title: 'string(required)'\n",
            "prompt: |-\n",
            "  write a title\n",
            "title: Hello\n",
            "---\n",
            "Old body\n",
        );

        let rewritten =
            rewrite_inline_document(original, "Fresh body\n", "2026-05-26", &[]).unwrap();

        assert!(rewritten.contains("$schema:"));
        assert!(rewritten.contains("title: 'string(required)'"));
        assert!(rewritten.contains("last_updated: 2026-05-26"));
        assert!(rewritten.ends_with("---\nFresh body\n"));
    }

    #[test]
    fn rewrite_does_not_persist_set_only_keys() {
        // Phase 5 Task 2: interactive-collected values flow through
        // `--set` overrides during composition only; they must not appear
        // in the rewritten document. The rewrite reuses the original
        // frontmatter text and only adds `last_updated` + any new keys
        // explicitly handed to it. Pass an empty `new_properties` slice
        // to simulate the inline closure path.
        let original = "---\n$schema:\n  title: 'string(required)'\n---\nOld\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-05-26", &[]).unwrap();

        // The collected `title` value is never in the original document
        // and should not be inserted on the way out.
        assert!(!rewritten.contains("title: Plan"));
        assert!(rewritten.contains("$schema:"));
        assert!(rewritten.contains("last_updated: 2026-05-26"));
    }

    #[test]
    fn rewrite_preserves_crlf_line_endings() {
        let original = "---\r\nlast_updated: 2026-01-01\r\n---\r\nBody\r\n";
        let rewritten =
            rewrite_inline_document(original, "New body\r\n", "2026-03-27", &[]).unwrap();
        assert!(rewritten.contains("last_updated: 2026-03-27\r\n"));
    }

    // -- apply_inline_closure -----------------------------------------------

    #[test]
    fn apply_closure_rejects_empty_body() {
        let plan = InlineClosurePlan {
            original_document_text: "---\nprompt: test\n---\nOld\n".into(),
            original_body_hash: 0,
        };
        let err = apply_inline_closure(
            &plan,
            "  ",
            Path::new("/tmp/nonexistent"),
            "2026-03-27",
            None,
        );
        assert!(err.is_err());
    }

    // -- apply_inline_closure with post-run frontmatter -----------------------

    #[test]
    fn rewrite_merges_new_properties_before_last_updated() {
        let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
        let new_props = vec![("tags".to_string(), "tags: research\n".to_string())];
        let rewritten =
            rewrite_inline_document(original, "New body\n", "2026-04-02", &new_props).unwrap();
        assert!(rewritten.contains("prompt: test\n"));
        assert!(rewritten.contains("tags: research\n"));
        assert!(rewritten.contains("last_updated: 2026-04-02\n"));
        assert!(rewritten.contains("New body\n"));
        let tags_pos = rewritten.find("tags:").unwrap();
        let lu_pos = rewritten.find("last_updated:").unwrap();
        assert!(tags_pos < lu_pos);
    }

    #[test]
    fn rewrite_no_new_properties_backward_compatible() {
        let original = "---\nprompt: test\nlast_updated: 2026-03-18\n---\nOld body\n";
        let rewritten = rewrite_inline_document(original, "New body\n", "2026-04-02", &[]).unwrap();
        assert!(rewritten.contains("prompt: test\n"));
        assert!(rewritten.contains("last_updated: 2026-04-02\n"));
        assert!(rewritten.contains("New body\n"));
        // No extra properties injected
        assert!(!rewritten.contains("tags:"));
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

    // -- serialize_frontmatter_property ----------------------------------------

    #[test]
    fn serialize_string_property() {
        let value = serde_json::json!("hello world");
        let result = serialize_frontmatter_property("title", &value);
        assert_eq!(result, "title: hello world\n");
    }

    #[test]
    fn serialize_string_with_special_chars_is_quoted() {
        let value = serde_json::json!("key: value");
        let result = serialize_frontmatter_property("note", &value);
        assert!(
            result.contains("'key: value'") || result.contains("\"key: value\""),
            "expected quoted YAML value, got: {result}"
        );
    }

    #[test]
    fn serialize_string_with_colon_hash() {
        let value = serde_json::json!("item #1: top");
        let result = serialize_frontmatter_property("desc", &value);
        assert!(
            result.contains('#'),
            "hash should be preserved in quoted value: {result}"
        );
    }

    #[test]
    fn serialize_number_property() {
        let result = serialize_frontmatter_property("count", &serde_json::json!(42));
        assert_eq!(result, "count: 42\n");
    }

    #[test]
    fn serialize_bool_property() {
        let result = serialize_frontmatter_property("draft", &serde_json::json!(true));
        assert_eq!(result, "draft: true\n");
    }

    #[test]
    fn serialize_null_property() {
        let result = serialize_frontmatter_property("removed", &serde_json::json!(null));
        assert_eq!(result, "removed: null\n");
    }

    #[test]
    fn serialize_array_property() {
        let value = serde_json::json!(["rust", "markdown"]);
        let result = serialize_frontmatter_property("tags", &value);
        assert!(result.starts_with("tags:\n"));
        assert!(result.contains("  - rust"));
        assert!(result.contains("  - markdown"));
    }

    #[test]
    fn serialize_object_property() {
        let value = serde_json::json!({"version": "1.0"});
        let result = serialize_frontmatter_property("meta", &value);
        assert!(result.starts_with("meta:\n"));
        assert!(result.contains("version"));
    }

    // -- upsert_last_updated ------------------------------------------------

    #[test]
    fn upsert_adds_when_missing() {
        let yaml = "prompt: test\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
        assert!(result.contains("last_updated: 2026-03-27\n"));
        assert!(result.contains("prompt: test\n"));
    }

    #[test]
    fn upsert_replaces_existing() {
        let yaml = "last_updated: 2026-01-01\nprompt: test\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
        assert!(result.contains("last_updated: 2026-03-27\n"));
        assert!(!result.contains("2026-01-01"));
    }

    #[test]
    fn upsert_ignores_indented_last_updated() {
        let yaml = "  last_updated: 2026-01-01\n";
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-03-27", "\n", &[]);
        // Indented line should not be rewritten (it's nested YAML)
        assert!(result.contains("  last_updated: 2026-01-01"));
        // A new top-level entry should be appended
        assert!(result.contains("last_updated: 2026-03-27\n"));
    }

    // -- upsert with new properties -------------------------------------------

    #[test]
    fn upsert_injects_new_properties_before_last_updated() {
        let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
        let new_props = vec!["tags: research\n".to_string()];
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
        assert!(result.contains("prompt: test\n"));
        assert!(result.contains("tags: research\n"));
        assert!(result.contains("last_updated: 2026-04-02\n"));
        // tags must appear before last_updated
        let tags_pos = result.find("tags:").unwrap();
        let lu_pos = result.find("last_updated:").unwrap();
        assert!(
            tags_pos < lu_pos,
            "new property should appear before last_updated"
        );
    }

    #[test]
    fn upsert_injects_new_properties_when_last_updated_missing() {
        let yaml = "prompt: test\n";
        let new_props = vec!["tags: research\n".to_string()];
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &new_props);
        assert!(result.contains("tags: research\n"));
        assert!(result.contains("last_updated: 2026-04-02\n"));
        let tags_pos = result.find("tags:").unwrap();
        let lu_pos = result.find("last_updated:").unwrap();
        assert!(tags_pos < lu_pos);
    }

    #[test]
    fn upsert_empty_new_properties_behaves_as_before() {
        let yaml = "prompt: test\nlast_updated: 2026-01-01\n";
        let no_props: Vec<String> = vec![];
        let result = upsert_last_updated_in_frontmatter(yaml, "2026-04-02", "\n", &no_props);
        assert_eq!(result, "prompt: test\nlast_updated: 2026-04-02\n");
    }

    // -- end-to-end integration tests -----------------------------------------

    #[test]
    fn apply_closure_merges_new_and_reports_reverted() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        let original = "---\nprompt: original prompt\ntitle: My Doc\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();

        let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_body_hash: original_markdown.hash_body(false),
        };

        // Simulate post-run state: title changed, tags added
        let mut post_run_fm = indexmap::IndexMap::new();
        post_run_fm.insert("prompt".to_string(), serde_json::json!("original prompt"));
        post_run_fm.insert("title".to_string(), serde_json::json!("Changed Title"));
        post_run_fm.insert("tags".to_string(), serde_json::json!("research"));

        let result = apply_inline_closure(
            &plan,
            "Brand new body content\n",
            &file,
            "2026-04-02",
            Some(&post_run_fm),
        )
        .unwrap();

        assert_eq!(result.new_properties, vec!["tags"]);
        assert_eq!(result.reverted_properties, vec!["title"]);

        let written = std::fs::read_to_string(&file).unwrap();
        // Original title preserved
        assert!(written.contains("title: My Doc\n"));
        // New property merged
        assert!(written.contains("tags: research\n"));
        // last_updated set
        assert!(written.contains("last_updated: 2026-04-02\n"));
        // New body applied
        assert!(written.contains("Brand new body content\n"));
        // tags appears before last_updated
        let tags_pos = written.find("tags:").unwrap();
        let lu_pos = written.find("last_updated:").unwrap();
        assert!(tags_pos < lu_pos);
    }

    #[test]
    fn apply_closure_none_post_run_backward_compatible() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();

        let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_body_hash: original_markdown.hash_body(false),
        };

        let result =
            apply_inline_closure(&plan, "Updated body\n", &file, "2026-04-02", None).unwrap();

        assert!(result.new_properties.is_empty());
        assert!(result.reverted_properties.is_empty());

        let written = std::fs::read_to_string(&file).unwrap();
        assert!(written.contains("last_updated: 2026-04-02\n"));
        assert!(written.contains("Updated body\n"));
    }

    #[test]
    fn apply_closure_writes_dirty_body_for_downstream_cleanup() {
        use tempfile::TempDir;

        let dir = TempDir::new().unwrap();
        let file = dir.path().join("test.md");
        let original = "---\nprompt: test\nlast_updated: 2026-01-01\n---\nOld body\n";
        std::fs::write(&file, original).unwrap();

        let original_markdown: darkmatter::markdown::Markdown = original.to_string().into();
        let plan = InlineClosurePlan {
            original_document_text: original.to_string(),
            original_body_hash: original_markdown.hash_body(false),
        };

        let dirty_body = "# Generated\nNo blank line before paragraph\n";
        apply_inline_closure(&plan, dirty_body, &file, "2026-04-02", None).unwrap();

        let written = std::fs::read_to_string(&file).unwrap();
        // apply_inline_closure writes the raw body — cleanup is the caller's job.
        // Verify the dirty body IS present so the downstream cleanup test is meaningful.
        assert!(
            written.contains("# Generated\nNo blank line before paragraph\n"),
            "raw replacement body must be on disk for downstream cleanup; got:\n{written}"
        );
        // Now simulate the cleanup step that callers (inline_cleanup, try_inline_closure) perform
        let (fm_prefix, body) = split_frontmatter(&written);
        let cleaned = darkmatter::markdown::cleanup::cleanup_content(body);
        assert_ne!(
            cleaned, body,
            "cleanup_content must transform the dirty body"
        );
        assert!(
            cleaned.contains("# Generated\n\nNo blank line before paragraph"),
            "cleaned body must insert blank line between header and paragraph; got:\n{cleaned}"
        );
        let _ = fm_prefix;
    }

    fn split_frontmatter(text: &str) -> (&str, &str) {
        let mut lines = text.split_inclusive('\n');
        let first = match lines.next() {
            Some(l) => l,
            None => return ("", text),
        };
        if first.trim_end_matches(['\r', '\n']) != "---" {
            return ("", text);
        }
        let mut offset = first.len();
        for line in lines {
            offset += line.len();
            if line.trim_end_matches(['\r', '\n']) == "---" {
                return (&text[..offset], &text[offset..]);
            }
        }
        ("", text)
    }
}

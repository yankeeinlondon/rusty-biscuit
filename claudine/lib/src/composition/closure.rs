//! Inline composition closure: body extraction, document reconstruction,
//! and atomic file write-back.
//!
//! These functions centralise the file-mutation side of inline composition
//! so that both the harness loop and the non-harness path share one
//! deterministic rewrite pipeline.

use std::path::Path;

use darkmatter::markdown::MarkdownResult;
use darkmatter::markdown::hash::{ComputedHash, MdHashKind, MdHashOptions, StoredHash};
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
    /// Whether the frontmatter segment differed from the pre-run baseline.
    ///
    /// `hash` and `last_updated` are excluded from this comparison, so the
    /// stamp itself cannot pollute the signal.
    pub frontmatter_changed: bool,
    /// Whether Darkmatter's markdown cleanup pass rewrote the replacement body.
    ///
    /// Cleanup runs inside the closure so the hashed-and-written body is the
    /// cleaned body (one atomic write). The CLI surfaces this as the
    /// "Cleaned up generated markdown formatting" status line.
    pub body_cleaned: bool,
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

    // Clean the body up front so the body that is hashed (D2 unchanged check),
    // stamped (D3), and atomically written is the final on-disk body. This
    // keeps the stored `hash:` consistent with the post-cleanup document and
    // preserves the single atomic write. `cleanup_content` operates on body
    // text only; frontmatter is assembled separately below.
    let cleaned_body = darkmatter::markdown::cleanup::cleanup_content(replacement_body);
    let body_cleaned = cleaned_body != replacement_body;
    let replacement_body = cleaned_body.as_str();

    let replacement_markdown: darkmatter::markdown::Markdown = replacement_body.to_string().into();
    let post_hash = replacement_markdown.compute_hash(MdHashKind::Simple, &inline_hash_options());
    if simple_body(&post_hash) == simple_body(&plan.original_hash) {
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

    // Stamp a Darkmatter Simple hash into the `hash:` frontmatter property
    // in the same atomic write that persists the body.
    //
    // We avoid parsing `doc_string` directly into `Markdown` because
    // Darkmatter's frontmatter parser splits on `lines()` and rejoins with
    // `\n`, which strips trailing newlines and normalizes CRLF. Instead, parse
    // only the frontmatter block and build the Markdown with the verbatim body.
    let md = if let Some(parts) = split_frontmatter_parts(&doc_string) {
        let fm_only = format!("{}{}{}", parts.opening, parts.yaml, parts.closing);
        let fm_md: darkmatter::markdown::Markdown = fm_only.into();
        let body_start = parts.opening.len() + parts.yaml.len() + parts.closing.len();
        let body = &doc_string[body_start..];
        darkmatter::markdown::Markdown::with_frontmatter(fm_md.frontmatter().clone(), body)
    } else {
        doc_string.into()
    };

    let opts = inline_hash_options();
    let stored = parse_inline_stored_hash(&md, &opts)
        .map_err(CompositionError::InlineHashMalformed)?;
    let decision = md
        .plan_hash_save(stored.as_ref(), &opts)
        .map_err(CompositionError::InlineHashMalformed)?;
    let final_text = md
        .apply_hash_save(&decision, &opts, today)
        .unwrap_or_else(|| md.as_string());

    crate::config::atomic::atomic_write(target_path, final_text.as_bytes())
        .map_err(|e| CompositionError::AtomicWriteFailed {
            path: target_path.to_path_buf(),
            source: Box::new(e),
        })?;

    // Compute the post-write fm-segment-change signal for tooling that wants
    // to distinguish frontmatter drift from body drift. The `hash` and
    // `last_updated` managed keys are excluded by `inline_hash_options()`, so
    // the stamp itself cannot influence the comparison.
    let final_md: darkmatter::markdown::Markdown = final_text.clone().into();
    let final_hash = final_md.compute_hash(MdHashKind::Simple, &opts);
    let frontmatter_changed = simple_fm(&final_hash) != simple_fm(&plan.original_hash);

    Ok(InlineClosureResult {
        new_properties,
        reverted_properties,
        frontmatter_changed,
        body_cleaned,
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

/// Hash options used for every inline-compose hash computation.
///
/// Forces [`MdHashKind::Simple`] so any pre-existing `structured` or `detailed`
/// stored hash is normalized to the `Simple` shape on the next run. Excludes
/// the managed `hash` and `last_updated` keys from the frontmatter segment so
/// the stamp itself cannot influence the hash.
pub fn inline_hash_options() -> MdHashOptions {
    MdHashOptions {
        forced_kind: Some(MdHashKind::Simple),
        ..MdHashOptions::default()
    }
}

/// Returns the body segment of a [`ComputedHash::Simple`] value.
///
/// # Panics
///
/// Panics if `hash` is not `Simple`. The inline closure only ever stores
/// `Simple` hashes, so this is an unreachable invariant violation.
fn simple_body(hash: &ComputedHash) -> &str {
    match hash {
        ComputedHash::Simple { body, .. } => body,
        _ => unreachable!("inline closure hashes are always ComputedHash::Simple"),
    }
}

/// Returns the frontmatter segment of a [`ComputedHash::Simple`] value.
///
/// # Panics
///
/// Panics if `hash` is not `Simple`. The inline closure only ever stores
/// `Simple` hashes, so this is an unreachable invariant violation.
fn simple_fm(hash: &ComputedHash) -> &str {
    match hash {
        ComputedHash::Simple { fm, .. } => fm,
        _ => unreachable!("inline closure hashes are always ComputedHash::Simple"),
    }
}

/// Parses the document's stored `hash` property, or returns `None` when it is
/// absent or null.
///
/// Mirrors the CLI pattern in `darkmatter/cli/src/commands/hash.rs:115` so
/// inline-compose shares the same stored-hash contract as `md hash --save`.
fn parse_inline_stored_hash(
    md: &darkmatter::markdown::Markdown,
    opts: &MdHashOptions,
) -> MarkdownResult<Option<StoredHash>> {
    match md.frontmatter().as_map().get(opts.property.as_str()) {
        None | Some(serde_json::Value::Null) => Ok(None),
        Some(value) => StoredHash::parse(value, &opts.property).map(Some),
    }
}

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
mod tests;

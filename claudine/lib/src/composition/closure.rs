//! Inline composition closure: body extraction, document reconstruction,
//! and atomic file write-back.
//!
//! These functions centralise the file-mutation side of inline composition
//! so that both the harness loop and the non-harness path share one
//! deterministic rewrite pipeline.

use std::path::Path;

use darkmatter::markdown::MarkdownResult;
use darkmatter::markdown::hash::{ComputedHash, MdHashKind, MdHashOptions, StoredHash};
use indexmap::{IndexMap, IndexSet};

use crate::composition::error::CompositionError;
use crate::composition::types::InlineClosurePlan;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Provider output split into its replacement body and optional metadata.
#[derive(Debug, Clone)]
pub struct InlineReplacementParts {
    body: String,
    frontmatter: Option<IndexMap<String, ResponseProperty>>,
}

#[derive(Debug, Clone)]
struct ResponseProperty {
    value: serde_json::Value,
    line: usize,
}

/// An ignored response-frontmatter proposal and its response line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InlinePropertyNotice {
    /// Proposed property name.
    pub key: String,
    /// One-based line in the provider response.
    pub line: usize,
}

/// Parse the provider's final response into replacement body and metadata.
pub fn extract_replacement_parts(
    provider_output: &str,
) -> Result<InlineReplacementParts, CompositionError> {
    if provider_output.trim().is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "provider returned an empty response".into(),
        ));
    }

    let Some(parts) = split_frontmatter_parts(provider_output) else {
        return Ok(InlineReplacementParts {
            body: provider_output.trim().to_string(),
            frontmatter: None,
        });
    };

    let body = parts.body.trim();
    if body.is_empty() {
        return Err(CompositionError::InvalidInlineResponse(
            "provider response contained only frontmatter with no body".into(),
        ));
    }

    let parsed: serde_json::Value = biscuit_file::serde_yaml_ng::from_str(parts.yaml)
        .map_err(|source| CompositionError::InlineResponseFrontmatterYaml { source })?;
    let map = parsed.as_object().ok_or_else(|| {
        CompositionError::InvalidInlineResponse(
            "response frontmatter must be a YAML mapping".into(),
        )
    })?;
    let locations = top_level_key_locations(parts.yaml)?;
    let frontmatter = map
        .iter()
        .map(|(key, value)| {
            (
                key.clone(),
                ResponseProperty {
                    value: value.clone(),
                    line: locations.get(key).copied().unwrap_or(2),
                },
            )
        })
        .collect();

    Ok(InlineReplacementParts {
        body: body.to_string(),
        frontmatter: Some(frontmatter),
    })
}

/// Result of applying inline closure, reporting frontmatter changes.
#[derive(Debug, Clone, Default)]
pub struct InlineClosureResult {
    /// Authorized response properties inserted into the document.
    pub inserted_properties: Vec<String>,
    /// Authorized response properties refreshed in their existing positions.
    pub refreshed_properties: Vec<String>,
    /// Unauthorized response properties that were ignored.
    pub ignored_properties: Vec<InlinePropertyNotice>,
    /// Authorized properties omitted from the response.
    pub missing_properties: Vec<String>,
    /// Authored properties whose on-disk values drifted and were restored.
    pub restored_frontmatter_properties: Vec<String>,
    /// Whether frontmatter drift was restored but could not be classified by property.
    pub unclassified_frontmatter_drift_restored: bool,
    /// Whether on-disk body drift was overwritten by the replacement body.
    pub body_drift_restored: bool,
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
    replacement: &InlineReplacementParts,
    target_path: &Path,
    today: &str,
) -> Result<InlineClosureResult, CompositionError> {
    let replacement_body = replacement.body.as_str();
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

    let original_md: darkmatter::markdown::Markdown =
        plan.original_document_text.clone().into();
    let original_fm = original_md.frontmatter().as_map();
    let allowed: IndexSet<&str> = plan
        .response_frontmatter
        .iter()
        .map(String::as_str)
        .collect();
    let mut harvested = IndexMap::new();
    let mut ignored_properties = Vec::new();
    if let Some(response_fm) = &replacement.frontmatter {
        for (key, property) in response_fm {
            if matches!(key.as_str(), "hash" | "last_updated") {
                continue;
            }
            if allowed.contains(key.as_str()) {
                harvested.insert(key.clone(), serialize_frontmatter_property(key, &property.value)?);
            } else {
                ignored_properties.push(InlinePropertyNotice {
                    key: key.clone(),
                    line: property.line,
                });
            }
        }
    }
    let inserted_properties = plan
        .response_frontmatter
        .iter()
        .filter(|key| harvested.contains_key(*key) && !original_fm.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let refreshed_properties = plan
        .response_frontmatter
        .iter()
        .filter(|key| harvested.contains_key(*key) && original_fm.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();
    let missing_properties = plan
        .response_frontmatter
        .iter()
        .filter(|key| !harvested.contains_key(*key))
        .cloned()
        .collect::<Vec<_>>();

    let doc_string = rewrite_inline_document(
        &plan.original_document_text,
        replacement_body,
        &harvested,
        &plan.response_frontmatter,
    )
    .map_err(CompositionError::InlineRewriteFailed)?;

    let md: darkmatter::markdown::Markdown = doc_string.clone().into();

    let opts = inline_hash_options();
    let stored = parse_inline_stored_hash(&md, &opts)
        .map_err(CompositionError::InlineHashMalformed)?;
    let mut decision = md
        .plan_hash_save(stored.as_ref(), &opts)
        .map_err(CompositionError::InlineHashMalformed)?;
    // Hash-save treats a missing stored hash as baseline creation, but every
    // successful inline closure is a known body mutation and must date it.
    decision.bump_last_updated = true;
    let final_text = darkmatter::markdown::hash::apply_hash_save_text(
        &doc_string,
        &decision,
        &opts,
        today,
    )
    .map_err(CompositionError::InlineHashMalformed)?
    .unwrap_or(doc_string);

    let source_drift = detect_source_drift(&plan.original_document_text, target_path);

    crate::config::atomic::atomic_write(target_path, final_text.as_bytes())
        .map_err(|e| CompositionError::AtomicWriteFailed {
            path: target_path.to_path_buf(),
            source: e,
        })?;

    // Compute the post-write fm-segment-change signal for tooling that wants
    // to distinguish frontmatter drift from body drift. The `hash` and
    // `last_updated` managed keys are excluded by `inline_hash_options()`, so
    // the stamp itself cannot influence the comparison.
    let final_md: darkmatter::markdown::Markdown = final_text.clone().into();
    let final_hash = final_md.compute_hash(MdHashKind::Simple, &opts);
    let frontmatter_changed = simple_fm(&final_hash) != simple_fm(&plan.original_hash);

    Ok(InlineClosureResult {
        inserted_properties,
        refreshed_properties,
        ignored_properties,
        missing_properties,
        restored_frontmatter_properties: source_drift.restored_frontmatter_properties,
        unclassified_frontmatter_drift_restored: source_drift
            .unclassified_frontmatter_drift_restored,
        body_drift_restored: source_drift.body_drift_restored,
        frontmatter_changed,
        body_cleaned,
    })
}

/// Reconstruct a Markdown document from authored frontmatter and a new body.
///
/// ## Errors
///
/// The typed [`MarkdownError`] is retained for API compatibility with the
/// fallback Markdown reconstruction path.
///
/// [`MarkdownError`]: darkmatter::markdown::MarkdownError
pub fn rewrite_inline_document(
    frontmatter_source: &str,
    body: &str,
    harvested: &IndexMap<String, String>,
    declaration_order: &[String],
) -> Result<String, darkmatter::markdown::MarkdownError> {
    if let Some(parts) = split_frontmatter_parts(frontmatter_source) {
        let newline = detect_newline(frontmatter_source);
        let yaml = rewrite_harvested_frontmatter(parts.yaml, harvested, declaration_order, newline);
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

struct FrontmatterParts<'a> {
    opening: &'a str,
    yaml: &'a str,
    closing: &'a str,
    body: &'a str,
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
                body: &text[next_offset..],
            });
        }
        offset = next_offset;
    }

    None
}

#[derive(Debug)]
struct TopLevelNode {
    key: String,
    start: usize,
    end: usize,
}

fn rewrite_harvested_frontmatter(
    yaml: &str,
    harvested: &IndexMap<String, String>,
    declaration_order: &[String],
    newline: &str,
) -> String {
    let nodes = top_level_nodes(yaml);
    let existing: IndexSet<&str> = nodes.iter().map(|node| node.key.as_str()).collect();
    let mut edits = nodes
        .iter()
        .filter_map(|node| {
            harvested.get(&node.key).map(|fragment| {
                (
                    node.start,
                    node.end,
                    with_newline_style(fragment, newline),
                )
            })
        })
        .collect::<Vec<_>>();

    let insertion = nodes
        .iter()
        .find(|node| node.key == "last_updated")
        .map_or(yaml.len(), |node| node.start);
    let inserted = declaration_order
        .iter()
        .filter(|key| !existing.contains(key.as_str()))
        .filter_map(|key| harvested.get(key))
        .map(|fragment| with_newline_style(fragment, newline))
        .collect::<String>();
    if !inserted.is_empty() {
        edits.push((insertion, insertion, inserted));
    }

    edits.sort_by_key(|(start, _, _)| *start);
    let mut rewritten = yaml.to_string();
    for (start, end, replacement) in edits.into_iter().rev() {
        rewritten.replace_range(start..end, &replacement);
    }
    rewritten
}

fn top_level_nodes(yaml: &str) -> Vec<TopLevelNode> {
    let lines = yaml
        .split_inclusive('\n')
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, *offset, trim_line_ending(line)))
        })
        .collect::<Vec<_>>();
    let roots = lines
        .iter()
        .enumerate()
        .filter_map(|(index, (start, _, line))| {
            semantic_top_level_key(line).map(|key| (index, *start, key))
        })
        .collect::<Vec<_>>();

    roots
        .iter()
        .enumerate()
        .map(|(root_index, (line_index, start, key))| {
            let next_line = roots
                .get(root_index + 1)
                .map_or(lines.len(), |(index, _, _)| *index);
            let mut end = roots
                .get(root_index + 1)
                .map_or(yaml.len(), |(_, start, _)| *start);
            for (line_start, _, line) in &lines[line_index + 1..next_line] {
                if line.starts_with('#') {
                    end = *line_start;
                    break;
                }
            }
            TopLevelNode {
                key: key.clone(),
                start: *start,
                end,
            }
        })
        .collect()
}

fn semantic_top_level_key(line: &str) -> Option<String> {
    if line.is_empty()
        || line.chars().next().is_some_and(char::is_whitespace)
        || line.starts_with('#')
    {
        return None;
    }
    let mut quote = None;
    let mut escaped = false;
    let colon = line.char_indices().find_map(|(index, character)| {
        match quote {
            Some('"') if escaped => escaped = false,
            Some('"') if character == '\\' => escaped = true,
            Some(active) if character == active => quote = None,
            Some(_) => {}
            None if matches!(character, '"' | '\'') => quote = Some(character),
            None if character == ':' => return Some(index),
            None => {}
        }
        None
    })?;
    let key_source = line[..colon].trim_end();
    let probe = format!("{key_source}: null");
    let parsed: serde_json::Value = biscuit_file::serde_yaml_ng::from_str(&probe).ok()?;
    parsed.as_object()?.keys().next().cloned()
}

fn top_level_key_locations(
    yaml: &str,
) -> Result<IndexMap<String, usize>, CompositionError> {
    let mut locations = IndexMap::new();
    for (index, line) in yaml.lines().enumerate() {
        let Some(key) = semantic_top_level_key(line) else {
            continue;
        };
        if locations.insert(key.clone(), index + 2).is_some() {
            return Err(CompositionError::InvalidInlineResponse(format!(
                "response frontmatter contains duplicate property {key:?}"
            )));
        }
    }
    Ok(locations)
}

fn with_newline_style(fragment: &str, newline: &str) -> String {
    if newline == "\r\n" {
        fragment.replace('\n', "\r\n")
    } else {
        fragment.to_string()
    }
}

fn detect_newline(text: &str) -> &str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}

fn trim_line_ending(line: &str) -> &str {
    line.trim_end_matches(['\r', '\n'])
}

#[derive(Default)]
struct SourceDrift {
    restored_frontmatter_properties: Vec<String>,
    unclassified_frontmatter_drift_restored: bool,
    body_drift_restored: bool,
}

fn detect_source_drift(original: &str, target_path: &Path) -> SourceDrift {
    let Ok(current) = std::fs::read_to_string(target_path) else {
        return SourceDrift::default();
    };
    if current == original {
        return SourceDrift::default();
    }
    let original_parts = split_frontmatter_parts(original);
    let current_parts = split_frontmatter_parts(&current);
    let original_body = original_parts.as_ref().map_or(original, |parts| parts.body);
    let current_body = source_body_region(&current, original_body);
    let body_drift_restored = original_body != current_body;

    let comparable = original_parts
        .as_ref()
        .zip(current_parts.as_ref())
        .and_then(|(original, current)| {
            let original: serde_json::Value =
                biscuit_file::serde_yaml_ng::from_str(original.yaml).ok()?;
            let current: serde_json::Value =
                biscuit_file::serde_yaml_ng::from_str(current.yaml).ok()?;
            Some((original.as_object()?.clone(), current.as_object()?.clone()))
        });

    let Some((original, current)) = comparable else {
        return SourceDrift {
            unclassified_frontmatter_drift_restored: source_frontmatter_region(
                original,
                original_body,
            ) != source_frontmatter_region(&current, current_body),
            body_drift_restored,
            ..SourceDrift::default()
        };
    };

    let mut keys: IndexSet<&str> = original.keys().map(String::as_str).collect();
    keys.extend(current.keys().map(String::as_str));
    let restored_frontmatter_properties = keys
        .into_iter()
        .filter(|key| {
            !matches!(*key, "hash" | "last_updated") && original.get(*key) != current.get(*key)
        })
        .map(str::to_string)
        .collect();

    SourceDrift {
        restored_frontmatter_properties,
        body_drift_restored,
        ..SourceDrift::default()
    }
}

fn source_body_region<'a>(text: &'a str, original_body: &str) -> &'a str {
    if let Some(parts) = split_frontmatter_parts(text) {
        return parts.body;
    }
    if text.ends_with(original_body) {
        return &text[text.len() - original_body.len()..];
    }

    let mut offset = 0;
    let mut body_start = None;
    for line in text.split_inclusive('\n') {
        offset += line.len();
        if trim_line_ending(line) == "---" {
            body_start = Some(offset);
        }
    }
    body_start.map_or(text, |start| &text[start..])
}

fn source_frontmatter_region<'a>(text: &'a str, body: &'a str) -> &'a str {
    &text[..text.len() - body.len()]
}

/// Serialize a single frontmatter property as a YAML fragment.
///
/// Simple scalars produce `key: value\n`. Complex types (arrays, objects)
/// delegate to `serde_yaml_ng` for the value portion.
fn serialize_frontmatter_property(
    key: &str,
    value: &serde_json::Value,
) -> Result<String, CompositionError> {
    let mut map = serde_json::Map::new();
    map.insert(key.to_string(), value.clone());
    let serialized = biscuit_file::serde_yaml_ng::to_string(&serde_json::Value::Object(map))
        .map_err(|source| CompositionError::InlineResponseFrontmatterSerialize {
            key: key.to_string(),
            source,
        })?;
    let mut lines = serialized.lines();
    let Some(first) = lines.next() else {
        return Err(CompositionError::InvalidInlineResponse(format!(
            "response frontmatter property {key:?} serialized to an empty node"
        )));
    };
    let mut node = String::from(first);
    node.push('\n');
    for line in lines {
        node.push_str("  ");
        node.push_str(line);
        node.push('\n');
    }
    Ok(node)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests;

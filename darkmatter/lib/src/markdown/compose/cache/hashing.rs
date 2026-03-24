//! Cache key generation for compose pipeline artifacts.
//!
//! Provides content-aware hashing using `biscuit_hash` xxHash functions.
//! All hashes are `u64` values suitable for cache keys, manifest fields,
//! and Merkle-style closure hash computation.

use biscuit_hash::{HashVariant, xx_hash, xx_hash_bytes, xx_hash_variant};
use serde_json::{Map, Value};
use std::path::Path;

use super::types::DependencyRef;
use crate::markdown::compose::state::EffectiveState;
use crate::markdown::compose::types::{ComposeContext, ComposeOptions};

// ── Source identification ──────────────────────────────────────────

/// Generates a cache key from a filesystem path (canonicalized).
///
/// Used by Phase 1 run-local cache and as the `source_id` for persistent cache.
pub(crate) fn compose_cache_key(source_path: &Path) -> String {
    std::fs::canonicalize(source_path)
        .unwrap_or_else(|_| source_path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

/// Hash of a canonical source identifier string.
pub(crate) fn source_id_hash(canonical_source: &str) -> u64 {
    xx_hash(canonical_source)
}

// ── Content hashing ────────────────────────────────────────────────

/// Hash of raw file bytes (detects any byte-level change).
pub(crate) fn raw_bytes_hash(content: &[u8]) -> u64 {
    xx_hash_bytes(content)
}

/// Semantic hash of a markdown body.
///
/// Normalizes by trimming blocks and stripping leading/trailing whitespace
/// per line, so pure formatting changes (indentation, trailing spaces)
/// don't invalidate the cache.
pub(crate) fn body_semantic_hash(body: &str) -> u64 {
    xx_hash_variant(
        body,
        vec![
            HashVariant::BlockTrimming,
            HashVariant::LeadingWhitespace,
            HashVariant::TrailingWhitespace,
        ],
    )
}

/// Template hash of a markdown body (more aggressive normalization).
///
/// In addition to semantic normalization, collapses interior whitespace
/// and removes blank lines entirely. Useful for detecting when the
/// "shape" of a document changes while ignoring all whitespace variance.
pub(crate) fn body_template_hash(body: &str) -> u64 {
    xx_hash_variant(
        body,
        vec![
            HashVariant::BlockTrimming,
            HashVariant::LeadingWhitespace,
            HashVariant::TrailingWhitespace,
            HashVariant::InteriorWhitespace,
            HashVariant::BlankLine,
        ],
    )
}

// ── Structured data hashing ────────────────────────────────────────

/// Hash of a frontmatter map using canonical JSON with sorted keys.
pub(crate) fn frontmatter_hash(fm: &Map<String, Value>) -> u64 {
    let canonical = canonical_json_sorted(&Value::Object(fm.clone()));
    xx_hash(&canonical)
}

/// Hash of the effective state (merged frontmatter + external state).
///
/// Hashes the data map using canonical JSON. Environment variables
/// and context are excluded — they're covered by `context_hash`.
pub(crate) fn effective_state_hash(state: &EffectiveState) -> u64 {
    let data = state.data();
    let as_map: Map<String, Value> = data.iter().map(|(k, v)| (k.clone(), v.clone())).collect();
    let canonical = canonical_json_sorted(&Value::Object(as_map));
    xx_hash(&canonical)
}

/// Hash of the runtime context (timestamps, env vars).
///
/// Only includes fields that affect compose output. We hash `today`
/// (which covers date-dependent interpolations) and skip volatile
/// fields like `now`/`utc` that change every second.
pub(crate) fn context_hash(ctx: &ComposeContext) -> u64 {
    // Hash only stable, output-affecting context fields
    let mut parts = Vec::new();
    parts.push(format!("today={}", ctx.today));
    parts.push(format!("yesterday={}", ctx.yesterday));
    parts.push(format!("tomorrow={}", ctx.tomorrow));

    // Sort env vars for determinism
    let mut env_pairs: Vec<_> = ctx.env.iter().collect();
    env_pairs.sort_by_key(|(k, _)| *k);
    for (k, v) in env_pairs {
        parts.push(format!("env.{}={}", k, v));
    }

    xx_hash(&parts.join("\0"))
}

/// Hash of compose options that affect output.
///
/// Only includes fields that change the composed result. Internal
/// bookkeeping fields (approval handler, policy root, etc.) are excluded.
pub(crate) fn options_hash(options: &ComposeOptions) -> u64 {
    let mut parts = Vec::new();

    // Operation set
    let ops: Vec<String> = options
        .enabled_operations
        .iter()
        .map(|op| format!("{:?}", op))
        .collect();
    parts.push(format!("ops={}", ops.join(",")));

    parts.push(format!("fail_fast={}", options.fail_fast));
    parts.push(format!("max_depth={}", options.max_transclusion_depth));
    parts.push(format!("allow_remote={}", options.allow_remote_transclusion));
    parts.push(format!("allow_local_md={}", options.allow_local_markdown));
    parts.push(format!("allow_local_code={}", options.allow_local_code));
    parts.push(format!(
        "code_fallback_lang={}",
        options.code_fallback_language
    ));
    parts.push(format!(
        "ignore_invalid={:?}",
        options.ignore_invalid_references
    ));
    parts.push(format!("resolve_repo_root={}", options.resolve_repo_root));
    parts.push(format!("list_spacing={:?}", options.list_spacing));
    parts.push(format!("indent_size={}", options.indent_size));
    parts.push(format!(
        "replace_parent_wins={}",
        options.replace_parent_wins
    ));

    if let Some(ref one_off) = options.one_off_replace {
        let canonical = canonical_json_sorted(&Value::Object(one_off.clone()));
        parts.push(format!("one_off_replace={}", canonical));
    }

    if let Some(ref ext) = options.external_state {
        let canonical = canonical_json_sorted(ext);
        parts.push(format!("external_state={}", canonical));
    }

    if let Some(ref overrides) = options.set_overrides {
        let canonical = canonical_json_sorted(overrides);
        parts.push(format!("set_overrides={}", canonical));
    }

    xx_hash(&parts.join("\0"))
}

// ── Merkle closure hash ────────────────────────────────────────────

/// Computes a Merkle-style closure hash from a self-hash and dependency hashes.
///
/// The closure hash captures the complete state of a document and all its
/// transitive dependencies. If any dependency changes, the closure hash changes.
pub(crate) fn closure_hash(self_hash: u64, deps: &[DependencyRef]) -> u64 {
    let mut data = Vec::with_capacity(8 + deps.len() * 16);
    data.extend_from_slice(&self_hash.to_le_bytes());
    for dep in deps {
        data.extend_from_slice(&dep.source_id_hash.to_le_bytes());
        data.extend_from_slice(&dep.closure_hash.to_le_bytes());
    }
    xx_hash_bytes(&data)
}

/// Computes a combined entry key from multiple hash dimensions.
///
/// Used to look up a composed document manifest: combines source identity,
/// content state, options, and context into a single key.
pub(crate) fn compose_entry_key(
    source_id: u64,
    body_semantic: u64,
    state: u64,
    context: u64,
    options: u64,
) -> u64 {
    let mut data = Vec::with_capacity(40);
    data.extend_from_slice(&source_id.to_le_bytes());
    data.extend_from_slice(&body_semantic.to_le_bytes());
    data.extend_from_slice(&state.to_le_bytes());
    data.extend_from_slice(&context.to_le_bytes());
    data.extend_from_slice(&options.to_le_bytes());
    xx_hash_bytes(&data)
}

// ── Operation entry key ──────────────────────────────────────────────

/// Computes a cache key for an operation result.
///
/// Combines the operation kind, source identity, and variant parameter
/// hash into a single lookup key.
pub(crate) fn operation_entry_key(op_kind: &str, source_id: u64, variant_hash: u64) -> u64 {
    let mut data = Vec::new();
    data.extend_from_slice(op_kind.as_bytes());
    data.push(0); // separator
    data.extend_from_slice(&source_id.to_le_bytes());
    data.extend_from_slice(&variant_hash.to_le_bytes());
    xx_hash_bytes(&data)
}

// ── Helpers ────────────────────────────────────────────────────────

/// Produces a canonical JSON string with recursively sorted object keys.
///
/// This ensures deterministic hashing regardless of insertion order.
/// Arrays preserve their element order (order is semantically meaningful).
pub(crate) fn canonical_json_sorted(value: &Value) -> String {
    match value {
        Value::Object(map) => {
            let mut sorted: Vec<_> = map.iter().collect();
            sorted.sort_by_key(|(k, _)| *k);
            let entries: Vec<String> = sorted
                .into_iter()
                .map(|(k, v)| format!("{}:{}", serde_json::to_string(k).unwrap(), canonical_json_sorted(v)))
                .collect();
            format!("{{{}}}", entries.join(","))
        }
        Value::Array(arr) => {
            let entries: Vec<String> = arr.iter().map(canonical_json_sorted).collect();
            format!("[{}]", entries.join(","))
        }
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn canonical_json_sorts_keys() {
        let value: Value = serde_json::json!({
            "zebra": 1,
            "alpha": 2,
            "middle": {"z": true, "a": false}
        });
        let result = canonical_json_sorted(&value);
        assert_eq!(
            result,
            r#"{"alpha":2,"middle":{"a":false,"z":true},"zebra":1}"#
        );
    }

    #[test]
    fn canonical_json_preserves_array_order() {
        let value: Value = serde_json::json!({"items": [3, 1, 2]});
        let result = canonical_json_sorted(&value);
        assert_eq!(result, r#"{"items":[3,1,2]}"#);
    }

    #[test]
    fn canonical_json_handles_nested_objects() {
        let value: Value = serde_json::json!({
            "b": {"d": 4, "c": 3},
            "a": {"f": 6, "e": 5}
        });
        let result = canonical_json_sorted(&value);
        assert_eq!(result, r#"{"a":{"e":5,"f":6},"b":{"c":3,"d":4}}"#);
    }

    #[test]
    fn canonical_json_empty_object() {
        assert_eq!(canonical_json_sorted(&serde_json::json!({})), "{}");
    }

    #[test]
    fn canonical_json_scalars() {
        assert_eq!(canonical_json_sorted(&serde_json::json!(null)), "null");
        assert_eq!(canonical_json_sorted(&serde_json::json!(true)), "true");
        assert_eq!(canonical_json_sorted(&serde_json::json!("hello")), r#""hello""#);
        assert_eq!(canonical_json_sorted(&serde_json::json!(42)), "42");
    }

    #[test]
    fn source_id_hash_deterministic() {
        let h1 = source_id_hash("/foo/bar/doc.md");
        let h2 = source_id_hash("/foo/bar/doc.md");
        assert_eq!(h1, h2);
    }

    #[test]
    fn source_id_hash_sensitive_to_path() {
        let h1 = source_id_hash("/foo/bar/doc.md");
        let h2 = source_id_hash("/foo/bar/other.md");
        assert_ne!(h1, h2);
    }

    #[test]
    fn body_semantic_hash_ignores_whitespace() {
        let body1 = "# Hello\n\nWorld\n";
        let body2 = "  # Hello  \n\n  World  \n\n";
        assert_eq!(body_semantic_hash(body1), body_semantic_hash(body2));
    }

    #[test]
    fn body_semantic_hash_sensitive_to_content() {
        let body1 = "# Hello\nWorld";
        let body2 = "# Hello\nDifferent";
        assert_ne!(body_semantic_hash(body1), body_semantic_hash(body2));
    }

    #[test]
    fn body_template_hash_more_aggressive() {
        // Template hash collapses interior whitespace and blank lines
        let body1 = "# Hello\n\n\nWorld";
        let body2 = "# Hello\nWorld";
        assert_eq!(body_template_hash(body1), body_template_hash(body2));
    }

    #[test]
    fn frontmatter_hash_ignores_key_order() {
        let mut fm1 = Map::new();
        fm1.insert("alpha".into(), Value::from(1));
        fm1.insert("beta".into(), Value::from(2));

        let mut fm2 = Map::new();
        fm2.insert("beta".into(), Value::from(2));
        fm2.insert("alpha".into(), Value::from(1));

        assert_eq!(frontmatter_hash(&fm1), frontmatter_hash(&fm2));
    }

    #[test]
    fn frontmatter_hash_sensitive_to_values() {
        let mut fm1 = Map::new();
        fm1.insert("key".into(), Value::from("value1"));

        let mut fm2 = Map::new();
        fm2.insert("key".into(), Value::from("value2"));

        assert_ne!(frontmatter_hash(&fm1), frontmatter_hash(&fm2));
    }

    #[test]
    fn closure_hash_changes_with_deps() {
        let self_hash = 12345u64;
        let deps1 = vec![DependencyRef {
            artifact_class: crate::markdown::compose::cache::types::ArtifactClass::ComposeDocumentCore,
            entry_key: 1,
            source_id_hash: 100,
            closure_hash: 200,
        }];
        let deps2 = vec![DependencyRef {
            artifact_class: crate::markdown::compose::cache::types::ArtifactClass::ComposeDocumentCore,
            entry_key: 1,
            source_id_hash: 100,
            closure_hash: 300, // Different closure hash
        }];

        assert_ne!(closure_hash(self_hash, &deps1), closure_hash(self_hash, &deps2));
    }

    #[test]
    fn closure_hash_no_deps_uses_self() {
        let h1 = closure_hash(12345, &[]);
        let h2 = closure_hash(12345, &[]);
        assert_eq!(h1, h2);

        let h3 = closure_hash(99999, &[]);
        assert_ne!(h1, h3);
    }

    #[test]
    fn compose_entry_key_deterministic() {
        let k1 = compose_entry_key(1, 2, 3, 4, 5);
        let k2 = compose_entry_key(1, 2, 3, 4, 5);
        assert_eq!(k1, k2);
    }

    #[test]
    fn compose_entry_key_sensitive_to_each_dimension() {
        let base = compose_entry_key(1, 2, 3, 4, 5);
        assert_ne!(base, compose_entry_key(99, 2, 3, 4, 5)); // source_id
        assert_ne!(base, compose_entry_key(1, 99, 3, 4, 5)); // body_semantic
        assert_ne!(base, compose_entry_key(1, 2, 99, 4, 5)); // state
        assert_ne!(base, compose_entry_key(1, 2, 3, 99, 5)); // context
        assert_ne!(base, compose_entry_key(1, 2, 3, 4, 99)); // options
    }

    #[test]
    fn operation_entry_key_deterministic() {
        let k1 = operation_entry_key("code", 123, 456);
        let k2 = operation_entry_key("code", 123, 456);
        assert_eq!(k1, k2);
    }

    #[test]
    fn operation_entry_key_sensitive_to_kind() {
        let code = operation_entry_key("code", 123, 456);
        let toc = operation_entry_key("toc-linking", 123, 456);
        assert_ne!(code, toc);
    }

    #[test]
    fn operation_entry_key_sensitive_to_source() {
        let k1 = operation_entry_key("code", 100, 456);
        let k2 = operation_entry_key("code", 200, 456);
        assert_ne!(k1, k2);
    }

    #[test]
    fn operation_entry_key_sensitive_to_variant() {
        let k1 = operation_entry_key("code", 123, 100);
        let k2 = operation_entry_key("code", 123, 200);
        assert_ne!(k1, k2);
    }
}

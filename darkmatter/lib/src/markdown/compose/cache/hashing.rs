//! Cache key generation for compose pipeline artifacts.
//!
//! Provides content-aware hashing using `biscuit_hash` xxHash functions.
//! All hashes are `u64` values suitable for cache keys, manifest fields,
//! and Merkle-style closure hash computation.

use biscuit_hash::{HashVariant, xx_hash, xx_hash_bytes, xx_hash_variant};
use serde::Serialize;
use serde::ser::{SerializeMap, SerializeSeq};
use serde_json::{Map, Value};
use std::path::Path;

use super::types::DependencyRef;
use crate::markdown::compose::EffectiveState;
use crate::markdown::compose::{ComposeContext, ComposeOptions};

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
/// Hashes the data map using canonical JSON, excluding the `ctx`
/// namespace which is covered separately by `context_hash`.
pub(crate) fn effective_state_hash(state: &EffectiveState) -> u64 {
    let data = state.data();
    let as_map: Map<String, Value> = data
        .iter()
        .filter(|(k, _)| k.as_str() != "ctx")
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    let canonical = canonical_json_sorted(&Value::Object(as_map));
    xx_hash(&canonical)
}

/// Hash of the runtime context (all stable output-affecting fields + env vars).
///
/// Hashes the full normalized values map from `ComposeContext`, excluding
/// volatile per-second fields (`now`, `utc`, `time`, `timestamp`, etc.)
/// and volatile system state (`memory_used`, `memory_avail`).
pub(crate) fn context_hash(ctx: &ComposeContext) -> u64 {
    // Clone the values map and remove volatile fields
    let mut values = ctx.values().clone();
    // Per-second volatile fields
    values.remove("now");
    values.remove("now_utc");
    values.remove("utc");
    values.remove("time");
    values.remove("time_military");
    values.remove("timestamp");
    values.remove("timestamp_ms");
    // Volatile system state
    values.remove("memory_used");
    values.remove("memory_avail");

    let canonical = canonical_json_sorted(&Value::Object(values));
    let mut parts = vec![canonical];

    // Sort env vars for determinism (kept separate for clarity)
    let mut env_pairs: Vec<_> = ctx.env().iter().collect();
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
///
/// Delegates to [`ComposeOptions::compose_cache_fingerprint`], the compose-cache
/// product of the single exhaustive options field-classification authority
/// (`compose/context/options.rs`). The reference-graph options identity is the
/// other product of that same classification, so cache and graph identity share
/// one field inventory.
pub(crate) fn options_hash(options: &ComposeOptions) -> u64 {
    options.compose_cache_fingerprint()
}

/// Combines the base options hash with a directive's set-overlay hash.
///
/// Returns `base` unchanged when `overlay` is `0` so the no-overlay path
/// remains neutral against pre-existing cache entries. Otherwise produces
/// a derived hash that cleanly distinguishes distinct overlay payloads.
pub(crate) fn combine_options_overlay_hash(base: u64, overlay: u64) -> u64 {
    if overlay == 0 {
        return base;
    }
    let mut data = [0u8; 16];
    data[..8].copy_from_slice(&base.to_le_bytes());
    data[8..].copy_from_slice(&overlay.to_le_bytes());
    xx_hash_bytes(&data)
}

/// Hash of the per-directive set overlay applied during markdown transclusion.
///
/// The returned hash is stable across ordering variations inside the object
/// overlay (keys are canonicalized) and across equivalent values, but
/// preserves the ordering of the property list because the spec treats
/// property-form ordering as semantically significant under permissive
/// duplicate-handling. Returns `0` when both inputs are empty so the
/// no-overlay path remains neutral inside composed cache keys.
pub(crate) fn set_overlay_hash(
    set_object: Option<&Map<String, Value>>,
    set_properties: &[(String, Value)],
) -> u64 {
    let object_empty = set_object.is_none_or(Map::is_empty);
    if object_empty && set_properties.is_empty() {
        return 0;
    }

    let mut parts = Vec::with_capacity(1 + set_properties.len());

    if let Some(map) = set_object {
        let canonical = canonical_json_sorted(&Value::Object(map.clone()));
        parts.push(format!("obj={}", canonical));
    } else {
        parts.push("obj=".to_string());
    }

    for (index, (name, value)) in set_properties.iter().enumerate() {
        let canonical = canonical_json_sorted(value);
        parts.push(format!("p{}:{}={}", index, name, canonical));
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
    let mut bytes = Vec::new();
    serde_json::to_writer(&mut bytes, &CanonicalJson(value)).unwrap();
    String::from_utf8(bytes).unwrap_or_default()
}

struct CanonicalJson<'a>(&'a Value);

impl Serialize for CanonicalJson<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Value::Null => serializer.serialize_unit(),
            Value::Bool(value) => serializer.serialize_bool(*value),
            Value::Number(value) => value.serialize(serializer),
            Value::String(value) => serializer.serialize_str(value),
            Value::Array(values) => {
                let mut seq = serializer.serialize_seq(Some(values.len()))?;
                for value in values {
                    seq.serialize_element(&CanonicalJson(value))?;
                }
                seq.end()
            }
            Value::Object(map) => {
                let mut entries: Vec<_> = map.iter().collect();
                entries.sort_by(|left, right| left.0.cmp(right.0));

                let mut object = serializer.serialize_map(Some(entries.len()))?;
                for (key, value) in entries {
                    object.serialize_entry(key, &CanonicalJson(value))?;
                }
                object.end()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds `ComposeOptions` without the full environment capture that
    /// `opts()` performs (git/repo/docs/OS/hardware detection via
    /// sniff). `options_hash` reads only option fields — never the runtime
    /// context — so that capture is pure waste in these tests. Empty content
    /// makes `capture_for_content` do date/time only (zero I/O); calling the
    /// full `new()` several times per test otherwise exceeded nextest's 30s
    /// terminate ceiling on CI's large working tree.
    fn opts() -> ComposeOptions {
        ComposeOptions::new_with_context(ComposeContext::capture_for_content(Path::new("."), ""))
    }

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
        assert_eq!(
            canonical_json_sorted(&serde_json::json!("hello")),
            r#""hello""#
        );
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
            artifact_class:
                crate::markdown::compose::cache::types::ArtifactClass::ComposeDocumentCore,
            entry_key: 1,
            source_id_hash: 100,
            closure_hash: 200,
        }];
        let deps2 = vec![DependencyRef {
            artifact_class:
                crate::markdown::compose::cache::types::ArtifactClass::ComposeDocumentCore,
            entry_key: 1,
            source_id_hash: 100,
            closure_hash: 300, // Different closure hash
        }];

        assert_ne!(
            closure_hash(self_hash, &deps1),
            closure_hash(self_hash, &deps2)
        );
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

    #[test]
    fn options_hash_sensitive_to_magic_paths() {
        let base = opts();
        let with_magic = opts()
            .with_magic_path("/custom/root", biscuit_file::PathPosition::Start);

        assert_ne!(options_hash(&base), options_hash(&with_magic));
    }

    #[test]
    fn options_hash_sensitive_to_magic_path_position() {
        let start =
            opts().with_magic_path("/path", biscuit_file::PathPosition::Start);
        let end = opts().with_magic_path("/path", biscuit_file::PathPosition::End);

        assert_ne!(options_hash(&start), options_hash(&end));
    }

    #[test]
    fn set_overlay_hash_empty_inputs_return_zero() {
        assert_eq!(set_overlay_hash(None, &[]), 0);
        let empty = Map::new();
        assert_eq!(set_overlay_hash(Some(&empty), &[]), 0);
    }

    #[test]
    fn set_overlay_hash_sensitive_to_object_overlay() {
        let mut m = Map::new();
        m.insert("name".to_string(), Value::from("Bob"));
        let with_object = set_overlay_hash(Some(&m), &[]);
        assert_ne!(with_object, 0);
        assert_ne!(with_object, set_overlay_hash(None, &[]));
    }

    #[test]
    fn set_overlay_hash_sensitive_to_property_value() {
        let a = set_overlay_hash(None, &[("name".to_string(), Value::from("Bob"))]);
        let b = set_overlay_hash(None, &[("name".to_string(), Value::from("Mary"))]);
        assert_ne!(a, b);
    }

    #[test]
    fn set_overlay_hash_stable_for_equivalent_object_key_order() {
        let mut m1 = Map::new();
        m1.insert("a".to_string(), Value::from(1));
        m1.insert("b".to_string(), Value::from(2));

        let mut m2 = Map::new();
        m2.insert("b".to_string(), Value::from(2));
        m2.insert("a".to_string(), Value::from(1));

        assert_eq!(
            set_overlay_hash(Some(&m1), &[]),
            set_overlay_hash(Some(&m2), &[])
        );
    }

    #[test]
    fn set_overlay_hash_distinguishes_object_vs_property() {
        let mut m = Map::new();
        m.insert("name".to_string(), Value::from("Bob"));
        let obj_form = set_overlay_hash(Some(&m), &[]);
        let prop_form = set_overlay_hash(None, &[("name".to_string(), Value::from("Bob"))]);
        assert_ne!(obj_form, prop_form);
    }

    #[test]
    fn combine_options_overlay_hash_neutral_when_overlay_zero() {
        assert_eq!(combine_options_overlay_hash(0xDEAD_BEEF, 0), 0xDEAD_BEEF);
    }

    #[test]
    fn combine_options_overlay_hash_distinguishes_payloads() {
        let base = 0xABCD_EF01_u64;
        let h1 = combine_options_overlay_hash(base, 0x1111);
        let h2 = combine_options_overlay_hash(base, 0x2222);
        assert_ne!(h1, h2);
        assert_ne!(h1, base);
    }

    #[test]
    fn options_hash_sensitive_to_baseline_schema() {
        use crate::markdown::schemas::{
            Constraint, PropertyAtom, PropertyDef, SchemaShape, SimplifiedSchema, SimplifiedType,
            TypeExpr,
        };
        use indexmap::IndexMap;

        let base = opts();

        let mut props_a = IndexMap::new();
        props_a.insert(
            "title".into(),
            PropertyDef::Single(PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        let schema_a = SimplifiedSchema::Single(SchemaShape {
            properties: props_a,
            ..Default::default()
        });

        let mut props_b = IndexMap::new();
        props_b.insert(
            "owner".into(),
            PropertyDef::Single(PropertyAtom {
                ty: TypeExpr::Primitive(SimplifiedType::String),
                is_array: false,
                constraints: vec![Constraint::Required],
                array_constraints: vec![],
                description: None,
            }),
        );
        let schema_b = SimplifiedSchema::Single(SchemaShape {
            properties: props_b,
            ..Default::default()
        });

        let with_a = opts().with_baseline_schema(schema_a);
        let with_b = opts().with_baseline_schema(schema_b);

        assert_ne!(options_hash(&base), options_hash(&with_a));
        assert_ne!(options_hash(&base), options_hash(&with_b));
        assert_ne!(options_hash(&with_a), options_hash(&with_b));
    }

    #[test]
    fn options_hash_sensitive_to_file_ref_fallback_dir() {
        let base = opts();
        let with_a = opts().with_file_ref_fallback_dir("/launch/area-a");
        let with_b = opts().with_file_ref_fallback_dir("/launch/area-b");

        // None vs Some, and Some(a) vs Some(b), must all differ.
        assert_ne!(options_hash(&base), options_hash(&with_a));
        assert_ne!(options_hash(&base), options_hash(&with_b));
        assert_ne!(options_hash(&with_a), options_hash(&with_b));

        // Identical anchors must hash identically.
        let with_a_again = opts().with_file_ref_fallback_dir("/launch/area-a");
        assert_eq!(options_hash(&with_a), options_hash(&with_a_again));
    }
}

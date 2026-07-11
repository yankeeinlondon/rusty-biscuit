//! Effective-schema assembly: trigger payload resolution, matching, and
//! cycle detection.
//!
//! This module wires the Phase 1–3 primitives (matcher, discovery, bare-name
//! resolution) into the helpers [`super::super`] (`DarkmatterSchemas`) consumes
//! in `effective_for`. The assembly order is:
//!
//! 1. Caller-configured baseline.
//! 2. Matching trigger payloads — nearest root first, filename-lexicographic
//!    within a root (the registry's built-in order).
//! 3. Document `$schema` (always wins on conflict).
//!
//! Shadowing is applied **before** matching (a shadowed trigger file is never
//! in the registry). Later layers win per top-level property via the existing
//! [`super::super::resolve::merge_baseline`] contract.
//!
//! See `darkmatter/features/2026-07-10-schema-triggers/spec.md`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::resolve::{self, ResolvedSchema, validate_simple_object_schema};

use super::discovery::{LoadedTrigger, TriggerRegistry};
use super::envelope::parse_trigger_envelope_from_str;
use super::matcher;

// ── Pure matching entry ─────────────────────────────────────────────────────

/// The result of evaluating one trigger against a document snapshot.
#[derive(Debug, Clone)]
pub struct TriggerEvaluation<'a> {
    /// The evaluated trigger.
    pub trigger: &'a LoadedTrigger,
    /// Whether the trigger matched (any arm held).
    pub matched: bool,
    /// The first defeating condition for a non-matching trigger, for the
    /// `md schema triggers` trace. `None` when the trigger matched.
    pub defeat: Option<String>,
}

/// Owned, presentation-neutral explanation of trigger discovery and matching.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerTrace {
    /// Discovery boundary.
    pub boundary: PathBuf,
    /// Schema roots, nearest first.
    pub roots: Vec<PathBuf>,
    /// Shadowed envelope paths paired with their nearer winner.
    pub shadowed: Vec<(PathBuf, PathBuf)>,
    /// Evaluated unshadowed triggers in registry order.
    pub triggers: Vec<TriggerTraceEntry>,
}

/// One trigger's arm-by-arm trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerTraceEntry {
    /// Trigger envelope path.
    pub source: PathBuf,
    /// Whether any arm matched.
    pub matched: bool,
    /// Results in authored arm order.
    pub arms: Vec<TriggerArmTrace>,
}

/// One authored match arm's result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerArmTrace {
    /// Zero-based authored arm index.
    pub index: usize,
    /// Whether this arm matched.
    pub matched: bool,
    /// First defeating condition when the arm did not match.
    pub defeat: Option<String>,
}

/// Builds the shared structured trace consumed by inspection surfaces.
pub fn trace_registry(
    registry: &TriggerRegistry,
    frontmatter: &Value,
    normalized_path: &str,
) -> TriggerTrace {
    let triggers = registry
        .triggers
        .iter()
        .map(|trigger| {
            let arms = trigger
                .envelope
                .match_arms
                .0
                .iter()
                .enumerate()
                .map(|(index, arm)| {
                    let matched = matcher::matches(arm, frontmatter, normalized_path);
                    TriggerArmTrace {
                        index,
                        matched,
                        defeat: (!matched)
                            .then(|| matcher::first_defeat(arm, frontmatter, normalized_path))
                            .flatten(),
                    }
                })
                .collect::<Vec<_>>();
            TriggerTraceEntry {
                source: trigger.source.clone(),
                matched: arms.iter().any(|arm| arm.matched),
                arms,
            }
        })
        .collect();
    TriggerTrace {
        boundary: registry.boundary.clone(),
        roots: registry.roots.clone(),
        shadowed: registry
            .shadowed
            .iter()
            .map(|item| (item.path.clone(), item.shadowed_by.clone()))
            .collect(),
        triggers,
    }
}

/// Evaluates every trigger in the registry against the given frontmatter
/// snapshot and normalized path.
///
/// Returns results in registry order (nearest root first, filename-lexicographic
/// within a root). This is the pure matching entry both surfaces (CLI trace +
/// DMLS) consume — it performs no I/O.
pub fn evaluate_registry<'a>(
    registry: &'a TriggerRegistry,
    frontmatter: &Value,
    normalized_path: &str,
) -> Vec<TriggerEvaluation<'a>> {
    registry
        .triggers
        .iter()
        .map(|trigger| {
            let arms = &trigger.envelope.match_arms;
            let matched = arms
                .0
                .iter()
                .any(|arm| matcher::matches(arm, frontmatter, normalized_path));
            let defeat = if matched {
                None
            } else {
                arms.0
                    .iter()
                    .filter_map(|arm| matcher::first_defeat(arm, frontmatter, normalized_path))
                    .next()
            };
            TriggerEvaluation {
                trigger,
                matched,
                defeat,
            }
        })
        .collect()
}

/// Returns only the matching triggers in registry order (nearest root first,
/// filename-lexicographic within a root). Convenience over
/// [`evaluate_registry`].
pub fn matched_triggers<'a>(
    registry: &'a TriggerRegistry,
    frontmatter: &Value,
    normalized_path: &str,
) -> Vec<&'a LoadedTrigger> {
    evaluate_registry(registry, frontmatter, normalized_path)
        .into_iter()
        .filter(|eval| eval.matched)
        .map(|eval| eval.trigger)
        .collect()
}

// ── Payload resolution + merge-compatibility gate ───────────────────────────

/// A resolved trigger payload ready for layering.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedPayload {
    /// The resolved JSON Schema (guaranteed simple-object by the gate).
    pub json_schema: Value,
    /// Dependency edges: the payload file + its imports + its examples
    /// (sorted, deduplicated).
    pub dependencies: Vec<PathBuf>,
}

/// Resolves a trigger's `$schema` payload, enforcing the merge-compatibility
/// gate and cycle detection.
///
/// The payload resolves via the same value forms as a document `$schema`
/// (inline mapping, string reference, bare name), but must resolve to a
/// **merge-compatible object schema** — a SimplifiedSchema single object or a
/// raw JSON Schema satisfying [`validate_simple_object_schema`]. Root unions
/// and non-object schemas are rejected as trigger payloads.
///
/// ## Cycle detection
///
/// A payload that resolves to a trigger-schema envelope file (directly or
/// through an import chain) is a hard load error. Directly referencing a
/// `*.trigger.yaml` as a payload is always wrong — the payload should reference
/// the schema file, not the activation metadata.
pub(crate) fn resolve_trigger_payload(
    trigger: &LoadedTrigger,
    schema_roots: &[PathBuf],
) -> Result<ResolvedPayload, SchemaError> {
    let payload = trigger.envelope.payload.as_ref().ok_or_else(|| {
        SchemaError::TriggerPayloadNotMergeable {
            path: trigger.source.clone(),
            payload: "<none>".into(),
            reason: "trigger envelope has no `$schema:` payload".into(),
        }
    })?;

    let base_dir = trigger
        .source
        .parent()
        .unwrap_or_else(|| Path::new("."));
    let resolved = resolve::resolve_yaml_schema_with_roots(payload, base_dir, schema_roots)?;

    // Cycle detection: the payload must not resolve to a trigger envelope.
    check_payload_cycles(trigger, &resolved)?;

    // Merge-compatibility gate: must be a simple-object schema.
    enforce_merge_compatible(&trigger.source, payload, &resolved.json_schema)?;

    // Collect dependency edges: the payload file(s) + imports + examples.
    let mut deps: Vec<PathBuf> = resolved
        .referenced_files
        .iter()
        .chain(resolved.imports.iter())
        .chain(resolved.examples.iter())
        .cloned()
        .collect();
    deps.sort();
    deps.dedup();

    Ok(ResolvedPayload {
        json_schema: resolved.json_schema,
        dependencies: deps,
    })
}

/// Rejects payloads that are root unions, non-objects, or otherwise not
/// merge-compatible. The contract is [`validate_simple_object_schema`].
fn enforce_merge_compatible(
    trigger_source: &Path,
    payload: &YamlValue,
    json_schema: &Value,
) -> Result<(), SchemaError> {
    let payload_desc = describe_payload(payload);
    let json_obj = json_schema.as_object().ok_or_else(|| {
        SchemaError::TriggerPayloadNotMergeable {
            path: trigger_source.to_path_buf(),
            payload: payload_desc.clone(),
            reason: "payload did not resolve to a JSON object".into(),
        }
    })?;
    validate_simple_object_schema(json_obj).map_err(|err| {
        let reason = match &err {
            SchemaError::Baseline { message, .. } => message.clone(),
            _ => err.to_string(),
        };
        SchemaError::TriggerPayloadNotMergeable {
            path: trigger_source.to_path_buf(),
            payload: payload_desc,
            reason: format!(
                "{reason}. Root unions and non-object schemas are rejected as trigger payloads"
            ),
        }
    })
}

/// Checks whether any file the payload resolved to is a trigger-schema
/// envelope (direct self-reference or referencing another trigger file).
fn check_payload_cycles(
    trigger: &LoadedTrigger,
    resolved: &ResolvedSchema,
) -> Result<(), SchemaError> {
    let trigger_canonical = canonicalize(&trigger.source);
    for ref_path in &resolved.referenced_files {
        if *ref_path == trigger_canonical {
            return Err(SchemaError::TriggerPayloadCycle {
                trigger: trigger.source.clone(),
                payload_path: ref_path.clone(),
                chain: format!(
                    "{} -> {} (self-reference)",
                    trigger.source.display(),
                    ref_path.display()
                ),
            });
        }
        if file_claims_trigger_envelope(ref_path)? {
            return Err(SchemaError::TriggerPayloadCycle {
                trigger: trigger.source.clone(),
                payload_path: ref_path.clone(),
                chain: format!(
                    "{} -> {}",
                    trigger.source.display(),
                    ref_path.display()
                ),
            });
        }
    }
    Ok(())
}

/// Returns `true` when `path`'s contents claim the `kind: trigger-schema`
/// envelope. I/O errors and malformed YAML return `Ok(false)` — they are not
/// the cycle detector's concern (the resolver surfaces those separately).
fn file_claims_trigger_envelope(path: &Path) -> Result<bool, SchemaError> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(false);
    };
    match parse_trigger_envelope_from_str(&content) {
        Ok(Some(_)) => Ok(true),
        Ok(None) | Err(_) => Ok(false),
    }
}

/// Checks whether the document `$schema` resolved to a trigger-schema file.
/// Directly referencing a trigger envelope as a document schema is an error —
/// trigger schemas activate by placement and match, never by reference.
///
/// `referenced_files` comes from the resolved document `$schema`;
/// `suggestion` is the payload filename to reference instead.
pub(crate) fn check_document_schema_not_trigger(
    referenced_files: &[PathBuf],
) -> Result<(), SchemaError> {
    for ref_path in referenced_files {
        if file_claims_trigger_envelope(ref_path)? {
            let suggestion = payload_suggestion(ref_path);
            return Err(SchemaError::TriggerSchemaReferenced {
                reference: ref_path.to_string_lossy().into_owned(),
                suggestion,
            });
        }
    }
    Ok(())
}

/// Derives the payload filename suggestion from a `.trigger.yaml` path by
/// stripping the `.trigger` infix: `claudine.trigger.yaml` → `claudine.yaml`.
fn payload_suggestion(trigger_path: &Path) -> String {
    let name = trigger_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("schema.yaml");
    if let Some(rest) = name.strip_suffix(".trigger.yaml") {
        format!("{rest}.yaml")
    } else if let Some(rest) = name.strip_suffix(".trigger.yml") {
        format!("{rest}.yml")
    } else {
        // Not a `.trigger.*` file — suggest the base name minus the trigger part.
        name.replace(".trigger", "")
    }
}

fn describe_payload(payload: &YamlValue) -> String {
    match payload {
        YamlValue::String(s) => s.clone(),
        YamlValue::Mapping(_) => "<inline>".into(),
        YamlValue::Sequence(_) => "<inline-union>".into(),
        _ => "<unknown>".into(),
    }
}

fn canonicalize(path: &Path) -> PathBuf {
    path.canonicalize().unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::errors::SchemaError;
    use crate::markdown::schemas::triggers::discovery;
    use std::fs;
    use tempfile::TempDir;

    fn repo_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    fn write(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    const TRIGGER_WITH_PAYLOAD: &str = "kind: trigger-schema\n\
        match:\n  prompt: string(required)\n\
        $schema: claudine.yaml\n";
    const PAYLOAD: &str = "$schema:\n  model: string(required)\n";

    fn frontmatter(pairs: &[(&str, Value)]) -> Value {
        let mut map = serde_json::Map::new();
        for (k, v) in pairs {
            map.insert(k.to_string(), v.clone());
        }
        Value::Object(map)
    }

    /// Builds a `LoadedTrigger` directly from envelope source text, bypassing
    /// [`discovery::scan`] (which now eagerly resolves payloads). Used to unit-
    /// test [`resolve_trigger_payload`] against payloads that would be rejected
    /// at load time.
    fn make_trigger(content: &str, source: &Path) -> LoadedTrigger {
        let envelope = parse_trigger_envelope_from_str(content)
            .expect("envelope should parse")
            .expect("content should claim trigger-schema");
        LoadedTrigger { source: source.to_path_buf(), envelope }
    }

    // ── Matching ────────────────────────────────────────────────────────

    #[test]
    fn matched_triggers_returns_only_matching() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/a.trigger.yaml"),
            TRIGGER_WITH_PAYLOAD,
        );
        write(
            &root.join("schemas/b.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema: other.yaml\n",
        );
        write(&root.join("schemas/claudine.yaml"), PAYLOAD);
        write(&root.join("schemas/other.yaml"), PAYLOAD);

        let doc = root.join("doc.md");
        let registry = discovery::scan(&doc, root).unwrap();

        // `prompt` present → a matches; b does not.
        let fm = frontmatter(&[("prompt", Value::String("x".into()))]);
        let matched = matched_triggers(&registry, &fm, "doc.md");
        assert_eq!(matched.len(), 1);
        assert!(matched[0].source.ends_with("a.trigger.yaml"));
    }

    #[test]
    fn evaluate_registry_returns_defeat_for_non_matching() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/a.trigger.yaml"),
            TRIGGER_WITH_PAYLOAD,
        );
        write(&root.join("schemas/claudine.yaml"), PAYLOAD);

        let doc = root.join("doc.md");
        let registry = discovery::scan(&doc, root).unwrap();

        let fm = frontmatter(&[]);
        let evals = evaluate_registry(&registry, &fm, "doc.md");
        assert_eq!(evals.len(), 1);
        assert!(!evals[0].matched);
        assert!(evals[0].defeat.as_deref().unwrap().contains("required"));
    }

    // ── Payload resolution + merge-compat gate ──────────────────────────

    #[test]
    fn resolves_mergeable_payload() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write(
            &root.join("schemas/claudine.trigger.yaml"),
            TRIGGER_WITH_PAYLOAD,
        );
        write(&root.join("schemas/claudine.yaml"), PAYLOAD);

        let doc = root.join("doc.md");
        let registry = discovery::scan(&doc, root).unwrap();
        let trigger = &registry.triggers[0];
        let resolved = resolve_trigger_payload(trigger, &registry.roots).unwrap();
        assert_eq!(
            resolved.json_schema["type"], "object",
            "payload must resolve to an object schema"
        );
        assert!(
            resolved.json_schema["properties"]["model"].is_object(),
            "payload must carry the `model` property"
        );
    }

    #[test]
    fn rejects_root_union_payload() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        let source = root.join("schemas/union.trigger.yaml");
        write(
            &source,
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema:\n  - model: string(required)\n  - title: string(required)\n",
        );

        let trigger = make_trigger(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema:\n  - model: string(required)\n  - title: string(required)\n",
            &source,
        );
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")]).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerPayloadNotMergeable { .. }),
            "root union payload must be rejected: {err:?}"
        );
    }

    #[test]
    fn rejects_payload_with_no_schema() {
        let repo = repo_fixture();
        let root = repo.path();
        let source = root.join("schemas/empty.trigger.yaml");

        let trigger = make_trigger(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n",
            &source,
        );
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")]).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerPayloadNotMergeable { .. }),
            "missing payload must be rejected: {err:?}"
        );
    }

    // ── Cycle detection ─────────────────────────────────────────────────

    #[test]
    fn rejects_payload_self_reference() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Payload references its own envelope file.
        let source = root.join("schemas/self.trigger.yaml");
        write(
            &source,
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: self.trigger.yaml\n",
        );

        let trigger = make_trigger(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: self.trigger.yaml\n",
            &source,
        );
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")]).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerPayloadCycle { .. }),
            "self-referencing payload must be rejected: {err:?}"
        );
    }

    #[test]
    fn rejects_payload_referencing_another_trigger() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        let a_source = root.join("schemas/a.trigger.yaml");
        write(
            &a_source,
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: b.trigger.yaml\n",
        );
        write(
            &root.join("schemas/b.trigger.yaml"),
            TRIGGER_WITH_PAYLOAD,
        );
        write(&root.join("schemas/claudine.yaml"), PAYLOAD);

        let trigger = make_trigger(
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: b.trigger.yaml\n",
            &a_source,
        );
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")]).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerPayloadCycle { .. }),
            "payload referencing another trigger must be rejected: {err:?}"
        );
    }

    // ── Document-schema trigger check ───────────────────────────────────

    #[test]
    fn check_document_schema_not_trigger_passes_for_normal_file() {
        let repo = repo_fixture();
        let root = repo.path();
        let normal = root.join("schema.yaml");
        write(&normal, PAYLOAD);
        let canonical = canonicalize(&normal);
        check_document_schema_not_trigger(&[canonical]).unwrap();
    }

    #[test]
    fn check_document_schema_not_trigger_rejects_trigger_file() {
        let repo = repo_fixture();
        let root = repo.path();
        let trigger = root.join("claudine.trigger.yaml");
        write(&trigger, TRIGGER_WITH_PAYLOAD);
        let canonical = canonicalize(&trigger);
        let err = check_document_schema_not_trigger(&[canonical]).unwrap_err();
        match err {
            SchemaError::TriggerSchemaReferenced {
                suggestion, ..
            } => {
                assert!(
                    suggestion.contains("claudine.yaml"),
                    "suggestion should name the payload: {suggestion}"
                );
            }
            other => panic!("expected TriggerSchemaReferenced, got {other:?}"),
        }
    }

    // ── payload_suggestion ──────────────────────────────────────────────

    #[test]
    fn payload_suggestion_strips_trigger_infix() {
        assert_eq!(
            payload_suggestion(Path::new("/repo/schemas/claudine.trigger.yaml")),
            "claudine.yaml"
        );
        assert_eq!(
            payload_suggestion(Path::new("/repo/schemas/x.trigger.yml")),
            "x.yml"
        );
    }
}

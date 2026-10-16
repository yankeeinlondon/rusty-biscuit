//! Effective-schema assembly: trigger payload resolution, matching, and
//! cycle detection.
//!
//! This module wires the Phase 1–3 primitives (matcher, discovery, bare-name
//! resolution) into the helpers [`super::super`] (`DarkmatterSchemas`) consumes
//! in `effective_for`. The assembly order is:
//!
//! 1. Caller-configured baseline.
//! 2. Matching trigger payloads — first schema root first,
//!    filename-lexicographic within a root, then registered embedded bundles
//!    (the registry's built-in order).
//! 3. Document `$schema` (always wins on conflict).
//!
//! Shadowing is applied **before** matching (a shadowed trigger file is never
//! in the registry). Later layers win per top-level property via the existing
//! [`super::super::resolve::merge_baseline`] contract.
//!
//! The user-facing contract is the "Repository Trigger Schemas" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use std::path::{Path, PathBuf};

use serde_json::Value;
use serde_yaml_ng::Value as YamlValue;

use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::resolve::{self, ResolvedSchema, validate_simple_object_schema};

use crate::markdown::schemas::roots::SchemaRoots;

use crate::markdown::schemas::embedded::EmbeddedSchemaBundle;

use super::discovery::{LoadedTrigger, TriggerRegistry, TriggerSource};
use super::envelope::parse_trigger_envelope_from_str;
use super::matcher::{self, PathSubject};

// ── Matching entry ──────────────────────────────────────────────────────────

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
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TriggerTrace {
    /// The five schema roots in search order, including the ones that
    /// contribute nothing.
    pub roots: SchemaRoots,
    /// Shadowed envelope paths paired with the earlier root's winner.
    pub shadowed: Vec<(PathBuf, PathBuf)>,
    /// Evaluated unshadowed triggers in registry order.
    pub triggers: Vec<TriggerTraceEntry>,
}

/// One trigger's arm-by-arm trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerTraceEntry {
    /// The trigger envelope's schema-root file or bundle document.
    pub source: TriggerSource,
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

/// The subject one trigger's `$path` patterns judge: `document` in the
/// registry's context, with the trigger's own `cwd` for bare, `./`, and
/// `../` patterns.
fn subject_for<'a>(
    registry: &'a TriggerRegistry,
    trigger: &'a LoadedTrigger,
    document: Option<&'a Path>,
) -> PathSubject<'a> {
    match document {
        Some(document) => {
            PathSubject::new(document, registry.context()).with_pattern_cwd(&trigger.pattern_cwd)
        }
        None => PathSubject::detached(),
    }
}

/// Builds the shared structured trace consumed by inspection surfaces.
///
/// `document` is the checked document's path; without one, every `$path`
/// predicate is defeated.
pub fn trace_registry(
    registry: &TriggerRegistry,
    frontmatter: &Value,
    document: Option<&Path>,
) -> TriggerTrace {
    let triggers = registry
        .triggers
        .iter()
        .map(|trigger| {
            let subject = subject_for(registry, trigger, document);
            let arms = trigger
                .envelope
                .match_arms
                .0
                .iter()
                .enumerate()
                .map(|(index, arm)| {
                    let matched = matcher::matches(arm, frontmatter, &subject);
                    TriggerArmTrace {
                        index,
                        matched,
                        defeat: (!matched)
                            .then(|| matcher::first_defeat(arm, frontmatter, &subject))
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
/// snapshot and document path, judged in the registry's context.
///
/// Returns results in registry order (first root first, filename-lexicographic
/// within a root). This is the matching entry both surfaces (CLI trace +
/// DMLS) consume; it judges the known path without a walk.
pub fn evaluate_registry<'a>(
    registry: &'a TriggerRegistry,
    frontmatter: &Value,
    document: Option<&Path>,
) -> Vec<TriggerEvaluation<'a>> {
    registry
        .triggers
        .iter()
        .map(|trigger| {
            let subject = subject_for(registry, trigger, document);
            let arms = &trigger.envelope.match_arms;
            let matched = arms
                .0
                .iter()
                .any(|arm| matcher::matches(arm, frontmatter, &subject));
            let defeat = if matched {
                None
            } else {
                arms.0
                    .iter()
                    .filter_map(|arm| matcher::first_defeat(arm, frontmatter, &subject))
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

/// Returns only the matching triggers in registry order (first root first,
/// filename-lexicographic within a root). Convenience over
/// [`evaluate_registry`].
pub fn matched_triggers<'a>(
    registry: &'a TriggerRegistry,
    frontmatter: &Value,
    document: Option<&Path>,
) -> Vec<&'a LoadedTrigger> {
    evaluate_registry(registry, frontmatter, document)
        .into_iter()
        .filter(|eval| eval.matched)
        .map(|eval| eval.trigger)
        .collect()
}

// ── Payload resolution + merge-compatibility gate ───────────────────────────

/// A resolved trigger payload ready for layering.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ResolvedPayload {
    /// The resolved JSON Schema (guaranteed simple-object by the gate),
    /// shared by every effective schema layered from it.
    pub json_schema: std::sync::Arc<Value>,
    /// The payload's SimplifiedSchema object shape, retained so editor
    /// projections see the properties validation layers in. `None` for a raw
    /// JSON Schema payload, which has no SimplifiedSchema projection.
    pub shape: Option<std::sync::Arc<crate::markdown::schemas::SchemaShape>>,
    /// Dependency edges: the payload file + its imports + its examples
    /// (sorted, deduplicated). Empty for an embedded trigger.
    pub dependencies: Vec<PathBuf>,
    /// Non-fatal findings produced while resolving payload references.
    pub advisories: Vec<super::super::SchemaAdvisory>,
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
///
/// ## Embedded triggers
///
/// An embedded trigger's payload resolves inside its bundle only (see
/// [`resolve_embedded_payload`]); `schema_roots` and `context` are not
/// consulted, and the result has no dependency edges.
pub(crate) fn resolve_trigger_payload(
    trigger: &LoadedTrigger,
    schema_roots: &[PathBuf],
    context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedPayload, SchemaError> {
    let source = match (&trigger.source, trigger.bundle) {
        (TriggerSource::File(path), _) => path,
        (TriggerSource::Embedded { name, .. }, Some(bundle)) => {
            return resolve_embedded_payload(trigger, bundle, name).map(|(payload, _)| payload);
        }
        (TriggerSource::Embedded { bundle, name }, None) => {
            return Err(SchemaError::EmbeddedBundle {
                bundle: bundle.clone(),
                message: format!("trigger `{name}` was loaded without its bundle"),
            });
        }
    };
    let payload = trigger.envelope.payload.as_ref().ok_or_else(|| {
        SchemaError::TriggerPayloadNotMergeable {
            path: source.clone(),
            payload: "<none>".into(),
            reason: "trigger envelope has no `$schema:` payload".into(),
        }
    })?;
    file_payload_memo::resolve(source, payload, schema_roots, context)
}

/// [`resolve_trigger_payload`] for a schema-root file, uncached.
fn resolve_file_payload(
    source: &Path,
    payload: &YamlValue,
    schema_roots: &[PathBuf],
    context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedPayload, SchemaError> {
    let base_dir = source.parent().unwrap_or_else(|| Path::new("."));
    let resolved = resolve::resolve_yaml_schema_with_roots(payload, base_dir, schema_roots, context)?;

    // Cycle detection: the payload must not resolve to a trigger envelope.
    check_payload_cycles(source, &resolved)?;

    // Merge-compatibility gate: must be a simple-object schema.
    enforce_merge_compatible(source, payload, &resolved.json_schema)?;

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
        shape: payload_shape(resolved.simplified),
        json_schema: std::sync::Arc::new(resolved.json_schema),
        dependencies: deps,
        advisories: resolved.advisories,
    })
}

/// A process-wide memo of resolved schema-root trigger payloads.
///
/// A long-running host (DMLS) scans the trigger registry for every request,
/// and a large payload (Claudine's resolves to megabytes) takes hundreds of
/// milliseconds to expand. Every downstream memo (the trigger merge, root-arm
/// wrapping, and the validator cache's digest) is keyed by the payload's
/// `Arc` identity, so returning the same `Arc` for unchanged inputs also
/// keeps those warm.
///
/// An entry is reused only when the trigger path, the payload value, the
/// schema roots, and the request context are equal, every dependency file
/// still hashes the same, and the entry list of every searched directory (the
/// schema roots and each dependency's folder) is unchanged, so a file added
/// in front of a bare-name target is noticed. Only successes are kept.
mod file_payload_memo {
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use biscuit_hash::xx_hash_bytes;
    use serde_yaml_ng::Value as YamlValue;

    use super::{ResolvedPayload, resolve_file_payload};
    use crate::markdown::schemas::errors::SchemaError;

    const RECENT_PAYLOADS: usize = 16;

    struct Entry {
        source: PathBuf,
        payload: YamlValue,
        schema_roots: Vec<PathBuf>,
        context: biscuit_file::FileResolutionContext,
        fingerprint: u64,
        resolved: ResolvedPayload,
    }

    static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

    pub(super) fn resolve(
        source: &Path,
        payload: &YamlValue,
        schema_roots: &[PathBuf],
        original: &biscuit_file::FileResolutionContext,
    ) -> Result<ResolvedPayload, SchemaError> {
        let context = &folder_context(original);
        let same_inputs = |entry: &Entry| {
            entry.source == source
                && entry.schema_roots == schema_roots
                && entry.payload == *payload
                && entry.context == *context
        };
        let held = {
            let entries = ENTRIES.lock().expect("trigger payload memo poisoned");
            entries
                .iter()
                .find(|entry| same_inputs(entry))
                .map(|entry| (entry.fingerprint, entry.resolved.clone()))
        };
        if let Some((fingerprint, resolved)) = held
            && fingerprint == inputs_fingerprint(schema_roots, &resolved.dependencies)
        {
            return Ok(resolved);
        }

        let resolved = resolve_file_payload(source, payload, schema_roots, original)?;
        let fingerprint = inputs_fingerprint(schema_roots, &resolved.dependencies);
        let mut entries = ENTRIES.lock().expect("trigger payload memo poisoned");
        entries.retain(|entry| !same_inputs(entry));
        if entries.len() == RECENT_PAYLOADS {
            entries.remove(0);
        }
        entries.push(Entry {
            source: source.to_path_buf(),
            payload: payload.clone(),
            schema_roots: schema_roots.to_vec(),
            context: context.clone(),
            fingerprint,
            resolved: resolved.clone(),
        });
        Ok(resolved)
    }

    /// The memo key's context: a document context with its source path
    /// replaced by its folder. Every document in one folder shares the
    /// derived `cwd`, tree, and anchors; the source path itself only names
    /// the resolution source in diagnostics and the containment check, which
    /// the folder decides alike. Any other context is kept whole.
    fn folder_context(context: &biscuit_file::FileResolutionContext) -> biscuit_file::FileResolutionContext {
        match context.source_path() {
            Some(source) if source.parent() == Some(context.cwd()) => {
                context.clone().with_source_path(context.cwd())
            }
            _ => context.clone(),
        }
    }

    /// Hashes each dependency's bytes and the sorted entry names of every
    /// schema root and dependency folder. An unreadable file or folder hashes
    /// as absent, which differs from any readable state.
    fn inputs_fingerprint(schema_roots: &[PathBuf], dependencies: &[PathBuf]) -> u64 {
        let mut folders: Vec<&Path> = schema_roots
            .iter()
            .map(PathBuf::as_path)
            .chain(dependencies.iter().filter_map(|path| path.parent()))
            .collect();
        folders.sort();
        folders.dedup();
        let mut fingerprint = 0u64;
        let mut fold = |bytes: &[u8]| fingerprint = fingerprint.rotate_left(5) ^ xx_hash_bytes(bytes);
        for folder in folders {
            fold(folder.as_os_str().as_encoded_bytes());
            match std::fs::read_dir(folder) {
                Ok(entries) => {
                    let mut names: Vec<_> = entries.filter_map(|entry| entry.ok().map(|entry| entry.file_name())).collect();
                    names.sort();
                    for name in names {
                        fold(name.as_encoded_bytes());
                    }
                }
                Err(_) => fold(b"\0absent folder"),
            }
        }
        for dependency in dependencies {
            fold(dependency.as_os_str().as_encoded_bytes());
            match std::fs::read(dependency) {
                Ok(bytes) => fold(&bytes),
                Err(_) => fold(b"\0absent file"),
            }
        }
        fingerprint
    }
}

/// The object shape of a payload that passed the merge-compatibility gate.
/// A gated payload is never a root union, so only a single-object projection
/// is kept.
fn payload_shape(
    simplified: Option<crate::markdown::schemas::SimplifiedSchema>,
) -> Option<std::sync::Arc<crate::markdown::schemas::SchemaShape>> {
    match simplified? {
        crate::markdown::schemas::SimplifiedSchema::Single(shape) => Some(std::sync::Arc::new(shape)),
        crate::markdown::schemas::SimplifiedSchema::Union(_) => None,
    }
}

/// [`resolve_trigger_payload`] for an embedded trigger, which needs neither
/// schema roots nor a document context, alongside the virtual paths
/// (`{bundle}/{file}`) of every bundle document the payload read.
pub(crate) fn resolve_embedded_trigger_payload(
    trigger: &LoadedTrigger,
) -> Result<(ResolvedPayload, Vec<PathBuf>), SchemaError> {
    match (&trigger.source, trigger.bundle) {
        (TriggerSource::Embedded { name, .. }, Some(bundle)) => resolve_embedded_payload(trigger, bundle, name),
        (source, _) => Err(SchemaError::EmbeddedBundle {
            bundle: String::new(),
            message: format!("trigger `{source}` is not an embedded bundle trigger"),
        }),
    }
}

/// Resolves an embedded trigger's `$schema` inside `bundle`.
///
/// A string payload is a relative reference to another bundle document
/// (`payload.yaml`, `./payload.yaml`); a mapping is an inline schema whose
/// `Name@./file.yaml` imports and `@this` resolve among the bundle's
/// documents. Nothing falls back to the disk, the schema roots, or the
/// network. Errors carry the virtual path `{bundle}/{name}` where a
/// [`SchemaError`] field needs a path.
fn resolve_embedded_payload(
    trigger: &LoadedTrigger,
    bundle: EmbeddedSchemaBundle,
    name: &str,
) -> Result<(ResolvedPayload, Vec<PathBuf>), SchemaError> {
    let importer = bundle.virtual_path(name);
    let importer_path = PathBuf::from(&importer);
    let payload = trigger.envelope.payload.as_ref().ok_or_else(|| {
        SchemaError::TriggerPayloadNotMergeable {
            path: importer_path.clone(),
            payload: "<none>".into(),
            reason: "trigger envelope has no `$schema:` payload".into(),
        }
    })?;
    let resolved = bundle.with_documents(|documents| match payload {
        YamlValue::String(reference) => {
            let target = crate::markdown::schemas::embedded::resolve_bundle_path(
                bundle.name(),
                &importer,
                reference,
            )
            .map_err(|reason| SchemaError::SchemaDocument {
                path: importer_path.clone(),
                message: format!("embedded `$schema` `{}`: {reason}", reference.trim()),
            })?;
            let claims_trigger = crate::markdown::schemas::embedded::find(documents, &target)
                .is_some_and(|document| matches!(parse_trigger_envelope_from_str(document.text), Ok(Some(_))));
            if target == importer || claims_trigger {
                let suffix = if target == importer { " (self-reference)" } else { "" };
                return Err(SchemaError::TriggerPayloadCycle {
                    trigger: importer_path.clone(),
                    payload_path: PathBuf::from(&target),
                    chain: format!("{importer} -> {target}{suffix}"),
                });
            }
            resolve::prepare_embedded_schema_in(&target, documents, bundle.name())
        }
        YamlValue::Mapping(_) => {
            resolve::prepare_embedded_inline_schema(payload, &importer, documents, bundle.name())
        }
        YamlValue::Sequence(_) => Err(SchemaError::TriggerPayloadNotMergeable {
            path: importer_path.clone(),
            payload: describe_payload(payload),
            reason: "a root union is not an object schema. Root unions and non-object schemas \
                     are rejected as trigger payloads"
                .into(),
        }),
        other => Err(SchemaError::FrontmatterShape {
            message: format!(
                "$schema must be a mapping, sequence, or string; got {}",
                resolve::describe_yaml(other)
            ),
        }),
    })?;
    let (resolved, reads) = resolved;
    enforce_merge_compatible(&importer_path, payload, &resolved.json_schema)?;
    let payload = ResolvedPayload {
        shape: payload_shape(resolved.simplified),
        json_schema: std::sync::Arc::new(resolved.json_schema),
        dependencies: Vec::new(),
        advisories: resolved.advisories,
    };
    Ok((payload, reads))
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
    trigger: &Path,
    resolved: &ResolvedSchema,
) -> Result<(), SchemaError> {
    let trigger_canonical = canonicalize(trigger);
    for ref_path in &resolved.referenced_files {
        if *ref_path == trigger_canonical {
            return Err(SchemaError::TriggerPayloadCycle {
                trigger: trigger.to_path_buf(),
                payload_path: ref_path.clone(),
                chain: format!(
                    "{} -> {} (self-reference)",
                    trigger.display(),
                    ref_path.display()
                ),
            });
        }
        if file_claims_trigger_envelope(ref_path)? {
            return Err(SchemaError::TriggerPayloadCycle {
                trigger: trigger.to_path_buf(),
                payload_path: ref_path.clone(),
                chain: format!(
                    "{} -> {}",
                    trigger.display(),
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
    biscuit_file::canonicalize_simplified(path).unwrap_or_else(|_| path.to_path_buf())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::errors::SchemaError;
    use crate::markdown::schemas::triggers::discovery;
    use std::fs;
    use tempfile::TempDir;

    /// A document context at the repository root, with no home directory.
    fn doc_context(root: &Path) -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(root, None, std::collections::HashMap::new())
            .with_repository_root(root)
    }

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
        LoadedTrigger {
            source: TriggerSource::File(source.to_path_buf()),
            envelope,
            pattern_cwd: source.parent().and_then(Path::parent).unwrap_or(source).to_path_buf(),
            bundle: None,
        }
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
        let registry = discovery::scan(&doc_context(root)).unwrap();

        // `prompt` present → a matches; b does not.
        let fm = frontmatter(&[("prompt", Value::String("x".into()))]);
        let matched = matched_triggers(&registry, &fm, Some(&doc));
        assert_eq!(matched.len(), 1);
        assert!(matched[0].source.path().unwrap().ends_with("a.trigger.yaml"));
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
        let registry = discovery::scan(&doc_context(root)).unwrap();

        let fm = frontmatter(&[]);
        let evals = evaluate_registry(&registry, &fm, Some(&doc));
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

        let registry = discovery::scan(&doc_context(root)).unwrap();
        let trigger = &registry.triggers[0];
        let resolved = resolve_trigger_payload(trigger, registry.roots.search_paths(), &biscuit_file::FileResolutionContext::new(root)).unwrap();
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
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")], &biscuit_file::FileResolutionContext::new(root)).unwrap_err();
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
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")], &biscuit_file::FileResolutionContext::new(root)).unwrap_err();
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
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")], &biscuit_file::FileResolutionContext::new(root)).unwrap_err();
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
        let err = resolve_trigger_payload(&trigger, &[root.join("schemas")], &biscuit_file::FileResolutionContext::new(root)).unwrap_err();
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

    // ── Payload memo ────────────────────────────────────────────────────

    /// A repository whose `schemas/` holds the trigger and its payload.
    fn payload_repo() -> TempDir {
        let repo = repo_fixture();
        write(&repo.path().join("schemas/claudine.trigger.yaml"), TRIGGER_WITH_PAYLOAD);
        write(&repo.path().join("schemas/claudine.yaml"), PAYLOAD);
        repo
    }

    fn payload_properties(registry: &TriggerRegistry) -> Vec<String> {
        registry.payloads[0].json_schema["properties"].as_object().unwrap().keys().cloned().collect()
    }

    #[test]
    fn documents_in_one_folder_share_the_resolved_payload() {
        let repo = payload_repo();
        let root = repo.path();

        let first = discovery::scan(&doc_context(root).for_source(root.join("a.md"))).unwrap();
        let second = discovery::scan(&doc_context(root).for_source(root.join("b.md"))).unwrap();

        assert!(std::sync::Arc::ptr_eq(&first.payloads[0].json_schema, &second.payloads[0].json_schema));
    }

    #[test]
    fn an_edited_payload_file_is_resolved_again() {
        let repo = payload_repo();
        let root = repo.path();
        assert_eq!(payload_properties(&discovery::scan(&doc_context(root)).unwrap()), ["model"]);

        write(&root.join("schemas/claudine.yaml"), "$schema:\n  owner: string(required)\n");

        assert_eq!(payload_properties(&discovery::scan(&doc_context(root)).unwrap()), ["owner"]);
    }

    #[test]
    fn a_bare_name_target_added_in_a_nearer_root_is_resolved_again() {
        let repo = payload_repo();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        let context = || doc_context(root).with_package_root(root.join("pkg")).for_source(root.join("pkg/doc.md"));
        assert_eq!(payload_properties(&discovery::scan(&context()).unwrap()), ["model"]);

        write(&root.join("pkg/schemas/claudine.yaml"), "$schema:\n  owner: string(required)\n");

        assert_eq!(payload_properties(&discovery::scan(&context()).unwrap()), ["owner"]);
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

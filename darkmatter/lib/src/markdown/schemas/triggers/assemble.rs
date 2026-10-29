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
    /// Whether the trigger matched (every top-level condition held).
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

/// One trigger's condition-by-condition trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerTraceEntry {
    /// The trigger envelope's schema-root file or bundle document.
    pub source: TriggerSource,
    /// Whether every top-level condition matched.
    pub matched: bool,
    /// Results in authored order: one per `match:` list entry, or one for a
    /// mapping `match:`.
    pub conditions: Vec<TriggerConditionTrace>,
}

/// One authored top-level condition's result.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
pub struct TriggerConditionTrace {
    /// Zero-based authored condition index.
    pub index: usize,
    /// Whether this condition matched.
    pub matched: bool,
    /// First defeating condition when this one did not match.
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
            let conditions = trigger
                .envelope
                .conditions
                .0
                .iter()
                .enumerate()
                .map(|(index, condition)| {
                    let matched = matcher::matches(condition, frontmatter, &subject);
                    TriggerConditionTrace {
                        index,
                        matched,
                        defeat: (!matched)
                            .then(|| matcher::first_defeat(condition, frontmatter, &subject))
                            .flatten(),
                    }
                })
                .collect::<Vec<_>>();
            TriggerTraceEntry {
                source: trigger.source.clone(),
                matched: conditions.iter().all(|condition| condition.matched),
                conditions,
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
            let conditions = &trigger.envelope.conditions;
            let matched = conditions.matches(frontmatter, &subject);
            let defeat = if matched {
                None
            } else {
                conditions
                    .0
                    .iter()
                    .find_map(|condition| matcher::first_defeat(condition, frontmatter, &subject))
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
    /// The cyclic named types `shape` reaches through
    /// [`TypeExpr::Ref`](crate::markdown::schemas::simplified::TypeExpr::Ref);
    /// empty for a raw payload or one that does not recurse.
    pub definitions: std::sync::Arc<crate::markdown::schemas::simplified::DefinitionTable>,
    /// Dependency edges: the payload file + its imports + its examples
    /// (sorted, deduplicated). Empty for an embedded trigger.
    pub dependencies: Vec<PathBuf>,
    /// Every location probed while resolving the payload and its references
    /// ([`ResolvedSchema::searched`](crate::markdown::schemas::resolve::ResolvedSchema::searched)):
    /// lexical, sorted, deduplicated, and possibly absent, so it is never read
    /// or hashed as a dependency file. Empty for an embedded trigger.
    pub searched: Vec<PathBuf>,
    /// Non-fatal findings produced while resolving payload references.
    pub advisories: Vec<super::super::SchemaAdvisory>,
}

impl ResolvedPayload {
    /// The stand-in for a matcher-only trigger (no `$schema`): no schema, no
    /// dependencies. Effective-schema assembly layers nothing for such a
    /// trigger, so `json_schema` is never read.
    fn matcher_only() -> Self {
        Self {
            json_schema: std::sync::Arc::new(Value::Object(serde_json::Map::new())),
            shape: None,
            definitions: std::sync::Arc::default(),
            dependencies: Vec::new(),
            searched: Vec::new(),
            advisories: Vec::new(),
        }
    }
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
/// A payload that resolves to a schema-trigger envelope file (directly or
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
    let _resolve = crate::markdown::schemas::timings::stage(crate::markdown::schemas::ValidationStage::Resolve);
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
    let Some(payload) = trigger.envelope.payload.as_ref() else {
        return Ok(ResolvedPayload::matcher_only());
    };
    file_payload_memo::resolve(source, payload, schema_roots, context)
}

/// Empties the process-wide trigger payload memo (see
/// [`crate::markdown::schemas::reset_process_caches_for_bench`]).
pub(crate) fn reset_process_caches() {
    file_payload_memo::clear();
}

/// [`resolve_trigger_payload`] for a schema-root file, uncached.
fn resolve_file_payload(
    source: &Path,
    payload: &YamlValue,
    schema_roots: &[PathBuf],
    context: &biscuit_file::FileResolutionContext,
) -> Result<ResolvedPayload, SchemaError> {
    let base_dir = source.parent().unwrap_or_else(|| Path::new("."));
    // The trigger file owns an inline payload's named types, so its
    // definitions never share keys with the document's or another trigger's.
    let resolved = resolve::resolve_yaml_schema_owned_by(
        payload,
        resolve::InlineOwner::Trigger(source),
        base_dir,
        schema_roots,
        context,
    )?;

    // Cycle detection: the payload must not resolve to a trigger envelope.
    check_payload_cycles(source, &resolved)?;

    // The file that authored the resolved schema, for a gate rejection to
    // name: through an alias chain (`$schema: ./alias.yaml` whose own
    // `$schema` names `./raw.json`) the resolver attributes the schema to
    // the last file, so that file is named, not merely the first one read.
    let payload_path = match payload {
        YamlValue::String(_) => resolved.origin.file().map(Path::to_path_buf),
        _ => None,
    };

    // Merge-compatibility gate: must be a simple-object schema.
    enforce_merge_compatible(source, payload, payload_path.as_deref(), &resolved.json_schema)?;
    let json_schema = std::sync::Arc::new(resolved.json_schema);
    enforce_compiles(source, payload, payload_path, &json_schema)?;

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
        definitions: resolved.definitions,
        json_schema,
        dependencies: deps,
        searched: resolved.searched,
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
/// The key is what resolution reads, not who asked. Resolution never reads
/// the requesting context directly: it derives `for_cwd(folder)` for the
/// trigger's folder and every nested file's folder, and
/// `for_trusted_external_cwd(root)` for each schema root. The key therefore
/// holds the trigger path, the payload value, the schema roots, the context
/// derived onto the trigger's folder, the context derived onto each schema
/// root, and the requesting context's own tree. The tree is kept because two
/// requesters can derive the same context for the trigger's folder yet
/// derive differently for a nested file's folder when one keeps a boundary
/// tree and the other selects a tree per folder. Documents in different
/// package folders of one repository share a key, since derivation
/// recomputes the folder-dependent scopes.
///
/// An entry is reused only when its key is equal and its inputs fingerprint
/// still matches: every dependency file hashes the same; the entry names of
/// each schema root, each dependency's folder, and each searched candidate's
/// folder are unchanged (a missing folder observed as missing); and every
/// searched candidate ([`ResolvedPayload::searched`]) is still the same kind
/// (absent, file, directory, other). A file created in front of a bare-name
/// or implicit-path target, even in a folder that did not exist, is therefore
/// noticed, as is a candidate swapped between directory and file under the
/// same name. A recursive `%` reference's search roots are not candidates, so
/// a file added below them is not noticed. When any observation fails for a
/// reason other than absence, the entry is not reused and a fresh result is
/// not kept, so resolution reports its own typed error. Only successes are
/// kept, at most 32 of them; a reuse refreshes an entry's recency and the
/// least recently used entry is evicted.
mod file_payload_memo {
    use std::io::ErrorKind;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    use biscuit_file::{BaseDirOrigin, FileResolutionContext};
    use biscuit_hash::xx_hash_bytes;
    use serde_yaml_ng::Value as YamlValue;

    use super::{ResolvedPayload, resolve_file_payload, tally};
    use crate::markdown::schemas::errors::SchemaError;

    /// Room for one DMLS workspace pass over this repository (18 distinct
    /// inputs before keys stopped naming the requester) with headroom.
    /// Expanded payloads can be megabytes, so the bound is fixed.
    pub(super) const CAPACITY: usize = 32;

    #[derive(PartialEq)]
    struct Key {
        source: PathBuf,
        payload: YamlValue,
        schema_roots: Vec<PathBuf>,
        tree: PathBuf,
        tree_origin: BaseDirOrigin,
        folder_context: FileResolutionContext,
        root_contexts: Vec<FileResolutionContext>,
    }

    impl Key {
        fn new(source: &Path, payload: &YamlValue, schema_roots: &[PathBuf], original: &FileResolutionContext) -> Self {
            // Mirrors `resolve_file_payload`'s base folder.
            let folder = source.parent().unwrap_or_else(|| Path::new("."));
            Self {
                source: source.to_path_buf(),
                payload: payload.clone(),
                schema_roots: schema_roots.to_vec(),
                tree: original.base_dir().to_path_buf(),
                tree_origin: original.base_dir_origin().clone(),
                folder_context: original.for_cwd(folder),
                root_contexts: schema_roots.iter().map(|root| original.for_trusted_external_cwd(root)).collect(),
            }
        }
    }

    struct Entry {
        key: Key,
        fingerprint: u64,
        resolved: ResolvedPayload,
    }

    /// Least recently used first.
    static ENTRIES: Mutex<Vec<Entry>> = Mutex::new(Vec::new());

    pub(super) fn clear() {
        ENTRIES.lock().expect("trigger payload memo poisoned").clear();
    }

    pub(super) fn resolve(
        source: &Path,
        payload: &YamlValue,
        schema_roots: &[PathBuf],
        original: &FileResolutionContext,
    ) -> Result<ResolvedPayload, SchemaError> {
        let key = Key::new(source, payload, schema_roots, original);
        let held = {
            let entries = ENTRIES.lock().expect("trigger payload memo poisoned");
            entries
                .iter()
                .find(|entry| entry.key == key)
                .map(|entry| (entry.fingerprint, entry.resolved.clone()))
        };
        if let Some((fingerprint, resolved)) = held
            && inputs_fingerprint(schema_roots, &resolved) == Some(fingerprint)
        {
            let mut entries = ENTRIES.lock().expect("trigger payload memo poisoned");
            if let Some(position) = entries.iter().position(|entry| entry.key == key) {
                let entry = entries.remove(position);
                entries.push(entry);
            }
            tally::record_hit();
            return Ok(resolved);
        }

        tally::record_miss();
        let resolved = resolve_file_payload(source, payload, schema_roots, original)?;
        let fingerprint = inputs_fingerprint(schema_roots, &resolved);
        let mut entries = ENTRIES.lock().expect("trigger payload memo poisoned");
        entries.retain(|entry| entry.key != key);
        let Some(fingerprint) = fingerprint else {
            return Ok(resolved);
        };
        if entries.len() == CAPACITY {
            entries.remove(0);
        }
        entries.push(Entry {
            key,
            fingerprint,
            resolved: resolved.clone(),
        });
        Ok(resolved)
    }

    /// Hashes each dependency's bytes, the sorted entry names of every
    /// schema root, dependency folder, and searched candidate's folder, and
    /// each searched candidate's kind. A missing path (or one below a file)
    /// is an expected observation with its own marker, distinct from an empty
    /// folder. `None` when any other read fails, including one entry of a
    /// listing: such a state can neither validate nor seed an entry.
    fn inputs_fingerprint(schema_roots: &[PathBuf], resolved: &ResolvedPayload) -> Option<u64> {
        let mut folders: Vec<&Path> = schema_roots
            .iter()
            .map(PathBuf::as_path)
            .chain(resolved.dependencies.iter().chain(&resolved.searched).filter_map(|path| path.parent()))
            .collect();
        folders.sort();
        folders.dedup();
        let mut fingerprint = 0u64;
        let mut fold = |bytes: &[u8]| fingerprint = fingerprint.rotate_left(5) ^ xx_hash_bytes(bytes);
        for folder in folders {
            fold(folder.as_os_str().as_encoded_bytes());
            match observe::read_dir(folder) {
                Ok(entries) => {
                    let mut names = entries.map(|entry| entry.map(|entry| entry.file_name())).collect::<Result<Vec<_>, _>>().ok()?;
                    names.sort();
                    for name in names {
                        fold(name.as_encoded_bytes());
                    }
                }
                Err(error) => fold(expected_absence(&error)?),
            }
        }
        for candidate in &resolved.searched {
            fold(candidate.as_os_str().as_encoded_bytes());
            match observe::metadata(candidate) {
                Ok(metadata) if metadata.is_file() => fold(b"\0file"),
                Ok(metadata) if metadata.is_dir() => fold(b"\0directory"),
                Ok(_) => fold(b"\0other"),
                Err(error) => fold(expected_absence(&error)?),
            }
        }
        for dependency in &resolved.dependencies {
            fold(dependency.as_os_str().as_encoded_bytes());
            match observe::read(dependency) {
                Ok(bytes) => fold(&bytes),
                Err(error) => fold(expected_absence(&error)?),
            }
        }
        Some(fingerprint)
    }

    /// The marker for an error that means "nothing is there", or `None` for
    /// a failed observation.
    fn expected_absence(error: &std::io::Error) -> Option<&'static [u8]> {
        match error.kind() {
            ErrorKind::NotFound => Some(b"\0absent"),
            // A component of the path is a file (`ENOTDIR`; Windows reports
            // a file used as a folder as `ERROR_DIRECTORY`).
            ErrorKind::NotADirectory => Some(b"\0below a file"),
            _ => None,
        }
    }

    /// The filesystem reads of [`inputs_fingerprint`]. Tests can make one
    /// path's reads fail without permission bits, which behave differently
    /// under elevated users and on native Windows.
    mod observe {
        use std::path::Path;

        #[cfg(test)]
        thread_local! {
            static FAILING: std::cell::RefCell<Option<std::path::PathBuf>> = const { std::cell::RefCell::new(None) };
        }

        /// Makes every observation of `path` on this thread fail until the
        /// guard drops.
        #[cfg(test)]
        pub(in crate::markdown::schemas::triggers) fn fail_observing(path: &Path) -> impl Drop + use<> {
            struct Restore;
            impl Drop for Restore {
                fn drop(&mut self) {
                    FAILING.with_borrow_mut(|failing| *failing = None);
                }
            }
            FAILING.with_borrow_mut(|failing| *failing = Some(path.to_path_buf()));
            Restore
        }

        fn injected(_path: &Path) -> std::io::Result<()> {
            #[cfg(test)]
            if FAILING.with_borrow(|failing| failing.as_deref() == Some(_path)) {
                return Err(std::io::Error::other("injected observation failure"));
            }
            Ok(())
        }

        pub(super) fn read_dir(path: &Path) -> std::io::Result<std::fs::ReadDir> {
            injected(path)?;
            std::fs::read_dir(path)
        }

        pub(super) fn metadata(path: &Path) -> std::io::Result<std::fs::Metadata> {
            injected(path)?;
            std::fs::metadata(path)
        }

        pub(super) fn read(path: &Path) -> std::io::Result<Vec<u8>> {
            injected(path)?;
            std::fs::read(path)
        }
    }

    #[cfg(test)]
    pub(in crate::markdown::schemas::triggers) use observe::fail_observing;
}

#[cfg(test)]
pub(super) use file_payload_memo::fail_observing;

/// Trigger-payload memo outcomes counted by a [`PayloadMemoTally`].
///
/// A hit is a reuse whose dependency fingerprint still matched; a miss is an
/// attempted uncached resolution, counted whether or not it succeeded. A
/// registry lookup that never resolves a payload records neither.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PayloadMemoCounts {
    pub hits: u64,
    pub misses: u64,
}

/// Counts the trigger-payload memo's hits and misses on the calling thread
/// while it is alive.
///
/// Only the most recently started tally on a thread counts, so a host that
/// interrupts one unit of work with another on the same thread starts a
/// second tally for the interruption and the first one excludes it. Tallies
/// nest lexically: drop the inner one before reading the outer one again.
/// Work on other threads is never counted.
///
/// ## Examples
///
/// ```
/// use darkmatter::markdown::schemas::triggers::PayloadMemoTally;
///
/// let pass = PayloadMemoTally::start();
/// {
///     let interruption = PayloadMemoTally::start();
///     // ... a scan here counts only toward `interruption`
///     # let _ = interruption.counts();
/// }
/// assert_eq!(pass.counts().misses, 0);
/// ```
#[derive(Debug)]
pub struct PayloadMemoTally {
    depth: usize,
    /// Thread-bound: the counts live in a thread-local stack.
    _thread_bound: std::marker::PhantomData<*const ()>,
}

impl PayloadMemoTally {
    /// Start counting on this thread; this tally takes over from any tally
    /// already running here until it is dropped.
    #[must_use]
    pub fn start() -> Self {
        let depth = tally::TALLIES.with_borrow_mut(|tallies| {
            tallies.push(PayloadMemoCounts::default());
            tallies.len() - 1
        });
        Self {
            depth,
            _thread_bound: std::marker::PhantomData,
        }
    }

    /// The counts recorded so far.
    #[must_use]
    pub fn counts(&self) -> PayloadMemoCounts {
        tally::TALLIES.with_borrow(|tallies| tallies.get(self.depth).copied().unwrap_or_default())
    }
}

impl Drop for PayloadMemoTally {
    fn drop(&mut self) {
        tally::TALLIES.with_borrow_mut(|tallies| tallies.truncate(self.depth));
    }
}

mod tally {
    use std::cell::RefCell;

    use super::PayloadMemoCounts;

    thread_local! {
        /// One slot per live [`super::PayloadMemoTally`]; the last one counts.
        pub(super) static TALLIES: RefCell<Vec<PayloadMemoCounts>> = const { RefCell::new(Vec::new()) };
    }

    pub(super) fn record_hit() {
        TALLIES.with_borrow_mut(|tallies| {
            if let Some(counts) = tallies.last_mut() {
                counts.hits += 1;
            }
        });
    }

    pub(super) fn record_miss() {
        TALLIES.with_borrow_mut(|tallies| {
            if let Some(counts) = tallies.last_mut() {
                counts.misses += 1;
            }
        });
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
    let _resolve = crate::markdown::schemas::timings::stage(crate::markdown::schemas::ValidationStage::Resolve);
    let importer = bundle.virtual_path(name);
    let importer_path = PathBuf::from(&importer);
    let Some(payload) = trigger.envelope.payload.as_ref() else {
        return Ok((ResolvedPayload::matcher_only(), Vec::new()));
    };
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
            payload_path: None,
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
    enforce_merge_compatible(&importer_path, payload, None, &resolved.json_schema)?;
    let json_schema = std::sync::Arc::new(resolved.json_schema);
    enforce_compiles(&importer_path, payload, None, &json_schema)?;
    let payload = ResolvedPayload {
        shape: payload_shape(resolved.simplified),
        definitions: resolved.definitions,
        json_schema,
        dependencies: Vec::new(),
        searched: Vec::new(),
        advisories: resolved.advisories,
    };
    Ok((payload, reads))
}

/// Rejects a payload whose JSON Schema cannot be compiled into a validator,
/// before its trigger is installed: a later layer could otherwise hide the bad
/// property when the merged schema compiles. The structural validator is
/// built through the shared validator cache, so an unchanged payload is
/// compiled once per process, not once per document pass.
fn enforce_compiles(
    trigger_source: &Path,
    payload: &YamlValue,
    payload_path: Option<PathBuf>,
    json_schema: &std::sync::Arc<Value>,
) -> Result<(), SchemaError> {
    let cache = &*crate::markdown::schemas::SHARED_VALIDATOR_CACHE;
    let hash = cache.shared_schema_hash(json_schema);
    match cache.structural_validator_hashed(json_schema, hash) {
        Ok(_) => Ok(()),
        Err(error) => Err(SchemaError::TriggerPayloadCompile {
            path: trigger_source.to_path_buf(),
            payload: describe_payload(payload),
            payload_path,
            message: match error {
                SchemaError::BuildValidator { message } => message,
                other => other.to_string(),
            },
        }),
    }
}

/// Rejects payloads that are root unions, non-objects, or otherwise not
/// merge-compatible. The contract is [`validate_simple_object_schema`].
fn enforce_merge_compatible(
    trigger_source: &Path,
    payload: &YamlValue,
    payload_path: Option<&Path>,
    json_schema: &Value,
) -> Result<(), SchemaError> {
    let payload_desc = describe_payload(payload);
    if !json_schema.is_object() {
        return Err(SchemaError::TriggerPayloadNotMergeable {
            path: trigger_source.to_path_buf(),
            payload: payload_desc,
            payload_path: payload_path.map(Path::to_path_buf),
            reason: "payload did not resolve to a JSON object".into(),
        });
    }
    let label = format!("the payload `{payload_desc}`");
    validate_simple_object_schema(json_schema, &label).map_err(|err| {
        // A reference error already says what is wrong with the payload's
        // definitions; the shape hint applies to the root-shape errors.
        let reason = match &err {
            SchemaError::Baseline { message, .. } => format!(
                "{message}. Root unions and non-object schemas are rejected as trigger payloads"
            ),
            _ => err.to_string(),
        };
        SchemaError::TriggerPayloadNotMergeable {
            path: trigger_source.to_path_buf(),
            payload: payload_desc,
            payload_path: payload_path.map(Path::to_path_buf),
            reason,
        }
    })
}

/// Checks whether any file the payload resolved to is a schema-trigger
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

/// Returns `true` when `path`'s contents claim the `kind: schema-trigger`
/// envelope, valid or not: a claimed envelope that discovery skips as invalid
/// is still a trigger file, never a schema. An unreadable file returns
/// `Ok(false)` — it is not the cycle detector's concern (the resolver
/// surfaces it separately) — as does unclaimed YAML, malformed or not.
fn file_claims_trigger_envelope(path: &Path) -> Result<bool, SchemaError> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return Ok(false);
    };
    match parse_trigger_envelope_from_str(&content) {
        Ok(Some(_)) | Err(_) => Ok(true),
        Ok(None) => Ok(false),
    }
}

/// Checks whether the document `$schema` resolved to a schema-trigger file.
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

    const TRIGGER_WITH_PAYLOAD: &str = "kind: schema-trigger\n\
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
            .expect("content should claim schema-trigger");
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
            "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema: other.yaml\n",
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
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
             $schema:\n  - model: string(required)\n  - title: string(required)\n",
        );

        let trigger = make_trigger(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
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
    fn a_trigger_with_no_schema_is_matcher_only() {
        let repo = repo_fixture();
        let root = repo.path();
        let source = root.join("schemas/empty.trigger.yaml");

        let trigger = make_trigger(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n",
            &source,
        );
        let payload = resolve_trigger_payload(&trigger, &[root.join("schemas")], &biscuit_file::FileResolutionContext::new(root))
            .expect("a matcher-only trigger loads");
        assert!(payload.dependencies.is_empty() && payload.shape.is_none(), "{payload:?}");
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
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
             $schema: self.trigger.yaml\n",
        );

        let trigger = make_trigger(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
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
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
             $schema: b.trigger.yaml\n",
        );
        write(
            &root.join("schemas/b.trigger.yaml"),
            TRIGGER_WITH_PAYLOAD,
        );
        write(&root.join("schemas/claudine.yaml"), PAYLOAD);

        let trigger = make_trigger(
            "kind: schema-trigger\nmatch:\n  prompt: string(required)\n\
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

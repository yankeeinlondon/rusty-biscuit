//! Trigger discovery in the five schema roots and the per-document trigger
//! registry.
//!
//! Trigger schemas live in the [schema roots](crate::markdown::schemas::roots):
//! the document's package, package-area, and tree root `schemas/` folders,
//! the `SCHEMAS_DIR` folder, and `~/schemas`, most local first. Folders
//! between the document and those roots are not searched.
//!
//! ## Discovery contract
//!
//! - Regular `.yaml`/`.yml` files only; directory and file symlinks inside a
//!   root are never followed.
//! - Within a root, files are ordered by UTF-8 filename bytes (not locale or
//!   host filesystem collation).
//! - Two names that collide after case-folding are a load error so a repository
//!   cannot activate differently on case-sensitive and case-insensitive
//!   filesystems.
//! - The first root wins by trigger **filename** with no merging; a shadowed
//!   file is never evaluated.
//! - A file that does not claim `kind: trigger-schema` is silently ignored.
//! - A file that claims the envelope but is malformed is a hard load error.
//! - Loading is **transactional**: if any unshadowed opted-in trigger is
//!   invalid, no registry is installed from that scan.
//! - A root folder whose metadata or entries cannot be read fails the scan
//!   with an I/O error naming it; it is never treated as absent.
//! - A host's [`EmbeddedSchemaBundle`]s are a last-resort location after the
//!   five roots: a bundle trigger is loaded only when no file of the same
//!   name was enumerated in any root (whatever that file holds), and only a
//!   bundle file claiming `kind: trigger-schema` becomes a trigger. Its
//!   payload and imports resolve inside its bundle.
//!
//! The user-facing contract is the "Repository Trigger Schemas" section of
//! `darkmatter/docs/topics/schemas/definition.md`.

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;

use crate::markdown::schemas::embedded::EmbeddedSchemaBundle;
use crate::markdown::schemas::errors::SchemaError;
use crate::markdown::schemas::roots::SchemaRoots;
use crate::markdown::schemas::simplified::SchemaSourceId;

use super::envelope::{TriggerEnvelope, parse_trigger_envelope_from_str};

/// Valid YAML extensions for trigger-schema files.
const YAML_EXTENSIONS: &[&str] = &["yaml", "yml"];

// ── Types ───────────────────────────────────────────────────────────────────

/// Where a trigger envelope was loaded from.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TriggerSource {
    /// A file in one of the five schema roots.
    File(PathBuf),
    /// A document in a host's [`EmbeddedSchemaBundle`]. It names no host
    /// file and is never read, watched, or linked as one.
    Embedded {
        /// The bundle's [name](EmbeddedSchemaBundle::name).
        bundle: String,
        /// The document's file name inside the bundle.
        name: String,
    },
}

impl TriggerSource {
    /// The host path of a [`File`](Self::File) source.
    #[must_use]
    pub fn path(&self) -> Option<&Path> {
        match self {
            Self::File(path) => Some(path),
            Self::Embedded { .. } => None,
        }
    }

    /// The file name that shadows and is shadowed by same-named triggers.
    #[must_use]
    pub fn file_name(&self) -> Option<&str> {
        match self {
            Self::File(path) => path.file_name().and_then(|name| name.to_str()),
            Self::Embedded { name, .. } => Some(name),
        }
    }

    /// The schema-source identity: [`SchemaSourceId::File`] for a file and
    /// [`SchemaSourceId::Embedded`] with the virtual path `{bundle}/{name}`
    /// for a bundle document.
    #[must_use]
    pub fn source_id(&self) -> SchemaSourceId {
        match self {
            Self::File(path) => SchemaSourceId::File(path.clone()),
            Self::Embedded { bundle, name } => SchemaSourceId::Embedded(format!("{bundle}/{name}")),
        }
    }
}

impl std::fmt::Display for TriggerSource {
    /// A file displays as its path; a bundle document as
    /// `embedded:{bundle}/{name}`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::File(path) => write!(f, "{}", path.display()),
            Self::Embedded { bundle, name } => write!(f, "embedded:{bundle}/{name}"),
        }
    }
}

impl serde::Serialize for TriggerSource {
    /// Serialized as its [`Display`](std::fmt::Display) text, so a file
    /// source keeps the plain path string it had before embedded sources.
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

/// A loaded trigger envelope with its source.
#[derive(Debug, Clone, PartialEq)]
pub struct LoadedTrigger {
    /// The schema-root file or bundle document the envelope was read from.
    pub source: TriggerSource,
    /// The parsed, lint-clean trigger envelope.
    pub envelope: TriggerEnvelope,
    /// The `cwd` of the trigger's bare, `./`, and `../` `$path` patterns (see
    /// [`SearchedRoot::pattern_cwd`](crate::markdown::schemas::roots::SearchedRoot::pattern_cwd));
    /// the document's tree root for an embedded trigger.
    pub pattern_cwd: PathBuf,
    /// The bundle an [`Embedded`](TriggerSource::Embedded) trigger's payload
    /// resolves in.
    pub(crate) bundle: Option<EmbeddedSchemaBundle>,
}

/// A file shadowed by an earlier root's file of the same name.
#[derive(Debug, Clone, PartialEq)]
pub struct ShadowedFile {
    /// The shadowed file's path.
    pub path: PathBuf,
    /// The earlier root's file that shadowed it.
    pub shadowed_by: PathBuf,
}

/// The unshadowed, loaded trigger envelopes for one document.
///
/// Built by [`scan`] from the document's context, which the registry keeps:
/// its `$path` patterns are judged in that context. Loading is
/// transactional: if any unshadowed opted-in trigger is invalid, [`scan`]
/// returns `Err` and no registry is installed.
#[derive(Debug, Clone, PartialEq)]
pub struct TriggerRegistry {
    /// The five schema roots, for bare-name lookup and the
    /// `md schema triggers` trace.
    pub roots: SchemaRoots,
    /// The ordered, deduped set of loaded triggers (first root first,
    /// filename-lexicographic within a root, then embedded bundle triggers in
    /// registration and filename order). Shadowed filenames are absent.
    pub triggers: Vec<LoadedTrigger>,
    /// Resolved payloads parallel to [`triggers`](Self::triggers) — one per
    /// trigger, in the same order. Populated by [`scan`] so `effective_for`
    /// does not resolve payloads twice. Empty for programmatically
    /// constructed registries, which fall back to on-demand resolution.
    pub(crate) payloads: Vec<super::assemble::ResolvedPayload>,
    /// Files shadowed by an earlier root's file of the same name.
    pub shadowed: Vec<ShadowedFile>,
    /// Every file name a later embedded trigger of that name defers to: each
    /// file enumerated in a root (trigger or not, shadowed or not) and each
    /// embedded trigger already loaded.
    claimed_names: BTreeSet<String>,
    /// The document context the roots were computed in and `$path` patterns
    /// are judged in.
    context: FileResolutionContext,
}

impl TriggerRegistry {
    /// A registry of `triggers`, with no schema roots, judged in `context`.
    /// For callers (and tests) that build triggers without discovery.
    #[must_use]
    pub fn new(triggers: Vec<LoadedTrigger>, context: FileResolutionContext) -> Self {
        let claimed_names = triggers
            .iter()
            .filter_map(|trigger| trigger.source.file_name().map(str::to_string))
            .collect();
        Self {
            roots: SchemaRoots::none(),
            triggers,
            payloads: Vec::new(),
            shadowed: Vec::new(),
            claimed_names,
            context,
        }
    }

    /// Appends `bundle`'s triggers after every trigger already loaded.
    ///
    /// A bundle document whose file name is already claimed (see
    /// [`scan_with_embedded`]) is skipped unread. Every other bundle document
    /// is classified like a root file: one that does not claim
    /// `kind: trigger-schema` is an import target only. Each embedded
    /// trigger's payload is resolved now, inside the bundle.
    ///
    /// Transactional: on error the registry is unchanged.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::EmbeddedBundle`] for an unusable bundle shape;
    /// [`SchemaError::EmbeddedTriggerLoad`] naming the first malformed
    /// unclaimed trigger or unresolvable payload.
    pub fn append_embedded_bundle(&mut self, bundle: EmbeddedSchemaBundle) -> Result<(), SchemaError> {
        bundle.check()?;
        let base_dir = self.context.base_dir().to_path_buf();
        let mut loaded = Vec::new();
        for (name, text) in bundle.sorted_files() {
            if self.claimed_names.contains(name) {
                continue;
            }
            let Some((envelope, payload)) = load_embedded_trigger(bundle, name, text)? else {
                continue;
            };
            let trigger = LoadedTrigger {
                source: TriggerSource::Embedded {
                    bundle: bundle.name().to_string(),
                    name: name.to_string(),
                },
                envelope,
                pattern_cwd: base_dir.clone(),
                bundle: Some(bundle),
            };
            loaded.push((trigger, payload));
        }
        // A programmatic registry resolves payloads on demand; keep its
        // `payloads` empty so the two vectors stay parallel or empty.
        let keep_payloads = self.payloads.len() == self.triggers.len();
        for (trigger, payload) in loaded {
            if let Some(name) = trigger.source.file_name() {
                self.claimed_names.insert(name.to_string());
            }
            self.triggers.push(trigger);
            if keep_payloads {
                self.payloads.push(payload);
            }
        }
        Ok(())
    }

    /// `true` when no triggers were loaded.
    pub fn is_empty(&self) -> bool {
        self.triggers.is_empty()
    }

    /// The document context `$path` patterns are judged in.
    #[must_use]
    pub fn context(&self) -> &FileResolutionContext {
        &self.context
    }

    /// The files an effective schema layered from this registry depends on:
    /// each file trigger's envelope, and each resolved payload file with its
    /// imports and examples. Embedded triggers contribute none; a registry
    /// built without [`scan`] contributes only its envelopes.
    pub fn dependencies(&self) -> impl Iterator<Item = &Path> {
        self.triggers
            .iter()
            .filter_map(|trigger| trigger.source.path())
            .chain(
                self.payloads
                    .iter()
                    .flat_map(|payload| payload.dependencies.iter().map(PathBuf::as_path)),
            )
    }

    /// The locations each resolved payload probed while selecting its schema
    /// file, imports, and examples, including absent nearer candidates whose
    /// creation would change the selection. Paths are lexical and may not
    /// exist; a path may repeat across payloads. Embedded triggers contribute
    /// none.
    pub fn searched_locations(&self) -> impl Iterator<Item = &Path> {
        self.payloads
            .iter()
            .flat_map(|payload| payload.searched.iter().map(PathBuf::as_path))
    }
}

/// An embedded trigger's envelope and resolved payload, or `None` for a bundle
/// document that does not claim `kind: trigger-schema`.
type EmbeddedTriggerLoad = Option<(TriggerEnvelope, super::assemble::ResolvedPayload)>;

/// Parses bundle document `name` and resolves its payload inside `bundle`.
///
/// Memoized for the process: a bundle's texts are `'static` and its payloads
/// resolve without the disk or the document context, so the result for one
/// bundle document depends only on texts. An entry is keyed by the bundle
/// name, the document name, and its text, and records the text of every
/// bundle document its payload read (its imports, transitively); it is reused
/// only while each of those documents still holds the same text. Editing a
/// bundle document the payload never reads, such as another trigger, keeps
/// the entry. Only successes are kept; a failure is recomputed (and reported)
/// on every load.
fn load_embedded_trigger(
    bundle: EmbeddedSchemaBundle,
    name: &str,
    text: &'static str,
) -> Result<EmbeddedTriggerLoad, SchemaError> {
    type Key = (&'static str, String, &'static str);
    /// The `(virtual path, text)` of each bundle document a payload read.
    type Reads = Vec<(PathBuf, &'static str)>;
    type Memo = std::sync::Mutex<HashMap<Key, Vec<(Reads, EmbeddedTriggerLoad)>>>;
    static LOADED: std::sync::OnceLock<Memo> = std::sync::OnceLock::new();
    let memo = LOADED.get_or_init(Default::default);
    let current_text = |path: &Path| {
        bundle.files().iter().find(|(file, _)| Path::new(&bundle.virtual_path(file)) == path).map(|(_, text)| *text)
    };
    let key: Key = (bundle.name(), name.to_string(), text);
    if let Some(entries) = memo.lock().expect("embedded trigger memo poisoned").get(&key)
        && let Some((_, hit)) = entries
            .iter()
            .find(|(reads, _)| reads.iter().all(|(path, read)| current_text(path) == Some(*read)))
    {
        return Ok(hit.clone());
    }
    let load_error = |source: SchemaError| SchemaError::EmbeddedTriggerLoad {
        bundle: bundle.name().to_string(),
        name: name.to_string(),
        source: Box::new(source),
    };
    let (loaded, reads) = match parse_trigger_envelope_from_str(text).map_err(load_error)? {
        None => (None, Vec::new()),
        Some(envelope) => {
            let trigger = LoadedTrigger {
                source: TriggerSource::Embedded {
                    bundle: bundle.name().to_string(),
                    name: name.to_string(),
                },
                envelope,
                pattern_cwd: PathBuf::new(),
                bundle: Some(bundle),
            };
            let (payload, read_paths) =
                super::assemble::resolve_embedded_trigger_payload(&trigger).map_err(load_error)?;
            let reads: Reads = read_paths
                .into_iter()
                .filter_map(|path| current_text(&path).map(|read| (path, read)))
                .collect();
            (Some((trigger.envelope, payload)), reads)
        }
    };
    memo.lock()
        .expect("embedded trigger memo poisoned")
        .entry(key)
        .or_default()
        .push((reads, loaded.clone()));
    Ok(loaded)
}

// ── Per-root file enumeration ───────────────────────────────────────────────

/// A regular `.yaml`/`.yml` file discovered in a schema root.
#[derive(Debug, Clone)]
struct RootFile {
    /// Full filesystem path.
    path: PathBuf,
    /// UTF-8 filename (last path component).
    name: String,
}

/// The regular `.yaml`/`.yml` files directly in schema root `root`, ordered by
/// UTF-8 filename bytes.
///
/// This is discovery's own enumeration, under the [discovery
/// contract](self#discovery-contract): symlinks and non-UTF-8 names are
/// skipped, and two names that collide after case-folding are an error.
///
/// ## Errors
///
/// [`SchemaError::Io`] when the folder or one of its entries cannot be read;
/// [`SchemaError::TriggerCaseFoldCollision`] for colliding names.
pub fn enumerate_root_files(root: &Path) -> Result<Vec<PathBuf>, SchemaError> {
    Ok(enumerate_root(root)?.into_iter().map(|file| file.path).collect())
}

/// Where [`scan_with_source`] reads schema-root folders and files from.
///
/// [`scan`] reads the filesystem directly. A host that keeps schema-root
/// contents in memory (DMLS) supplies them here, so its registry is the one
/// discovery computes from those contents. Either method may read the
/// filesystem; the trait only decides where the bytes come from.
pub trait SchemaRootSource {
    /// The files [`enumerate_root_files`] returns for `root`.
    ///
    /// ## Errors
    ///
    /// Every [`enumerate_root_files`] error.
    fn root_files(&mut self, root: &Path) -> Result<Vec<PathBuf>, SchemaError>;

    /// The UTF-8 text of root file `path`.
    ///
    /// ## Errors
    ///
    /// [`SchemaError::Io`] naming `path` when it cannot be read as UTF-8 text.
    fn root_file_text(&mut self, path: &Path) -> Result<std::sync::Arc<str>, SchemaError>;
}

/// The filesystem [`SchemaRootSource`] that [`scan`] uses.
struct FilesystemRoots;

impl SchemaRootSource for FilesystemRoots {
    fn root_files(&mut self, root: &Path) -> Result<Vec<PathBuf>, SchemaError> {
        enumerate_root_files(root)
    }

    fn root_file_text(&mut self, path: &Path) -> Result<std::sync::Arc<str>, SchemaError> {
        std::fs::read_to_string(path).map(Into::into).map_err(|source| SchemaError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Enumerates regular `.yaml`/`.yml` files in `root`, rejecting symlinks and
/// case-fold collisions.
///
/// Files are returned sorted by UTF-8 filename bytes. Two names that collide
/// after case-folding are a load error.
fn enumerate_root(root: &Path) -> Result<Vec<RootFile>, SchemaError> {
    let entries = match std::fs::read_dir(root) {
        Ok(entries) => entries,
        Err(e) => {
            return Err(SchemaError::Io {
                path: root.to_path_buf(),
                source: e,
            });
        }
    };

    let mut files: Vec<RootFile> = Vec::new();
    for entry in entries {
        // A failed entry read leaves the root's trigger set unknown; loading
        // what was read would install a registry missing that trigger.
        let entry = entry.map_err(|source| SchemaError::Io {
            path: root.to_path_buf(),
            source,
        })?;
        let path = entry.path();

        // Do not follow symlinks: check the entry's file type directly.
        let file_type = match entry.file_type() {
            Ok(ft) => ft,
            Err(e) => {
                return Err(SchemaError::Io {
                    path: path.clone(),
                    source: e,
                });
            }
        };
        if file_type.is_symlink() || !file_type.is_file() {
            continue;
        }

        // Only `.yaml` / `.yml`.
        let is_yaml = path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| YAML_EXTENSIONS.contains(&ext));
        if !is_yaml {
            continue;
        }

        // UTF-8 filename.
        let Some(name) = path
            .file_name()
            .and_then(|n| n.to_str())
            .map(str::to_string)
        else {
            continue;
        };

        files.push(RootFile { path, name });
    }

    // Sort by UTF-8 filename bytes.
    files.sort_by(|a, b| a.name.as_bytes().cmp(b.name.as_bytes()));

    // Detect case-fold collisions.
    check_case_fold_collision(root, &files)?;

    Ok(files)
}

/// Returns `Err` when two filenames in the same root collide after
/// case-folding, preventing divergent behavior on case-sensitive vs
/// case-insensitive filesystems.
fn check_case_fold_collision(
    root: &Path,
    files: &[RootFile],
) -> Result<(), SchemaError> {
    let mut seen_lowered: HashMap<String, String> = HashMap::new();
    for file in files {
        let lowered = file.name.to_lowercase();
        if let Some(existing) = seen_lowered.get(&lowered) {
            return Err(SchemaError::TriggerCaseFoldCollision {
                root: root.to_path_buf(),
                files: vec![existing.clone(), file.name.clone()],
            });
        }
        seen_lowered.insert(lowered, file.name.clone());
    }
    Ok(())
}

// ── Full scan ───────────────────────────────────────────────────────────────

/// Performs a full trigger-schema discovery scan for one document.
///
/// Computes the document's [`SchemaRoots`] from `context`, enumerates each
/// searched root, applies filename shadowing, classifies each unshadowed
/// file, loads trigger envelopes, and resolves every payload.
///
/// `context` must be the document's own context (its `cwd` is the document's
/// folder): the package and package-area roots are the document's, and the
/// registry judges `$path` patterns in it.
///
/// Loading is **transactional**: if any root folder cannot be inspected or
/// read, or any unshadowed opted-in trigger file is malformed or its payload
/// fails to resolve (missing file, cyclic reference, non-mergeable shape), no
/// registry is installed (an error is returned).
/// Payload resolution runs regardless of whether any current document matches
/// the trigger, so a registry is never installed with a known-bad payload.
pub fn scan(context: &FileResolutionContext) -> Result<TriggerRegistry, SchemaError> {
    scan_with_embedded(context, &[])
}

/// [`scan`], then each of `bundles` in order as a last-resort trigger
/// location (see [`TriggerRegistry::append_embedded_bundle`]).
///
/// A bundle document is skipped when a file of the same name was enumerated
/// in any searched root — a matching, non-matching, non-trigger, or malformed
/// one alike — or an earlier bundle loaded a trigger of that name. A
/// malformed claimed disk trigger stays a [`SchemaError::TriggerLoad`]; the
/// bundle never stands in for it.
///
/// ## Errors
///
/// Every [`scan`] error, plus those of
/// [`TriggerRegistry::append_embedded_bundle`].
pub fn scan_with_embedded(
    context: &FileResolutionContext,
    bundles: &[EmbeddedSchemaBundle],
) -> Result<TriggerRegistry, SchemaError> {
    scan_with_source(context, bundles, &mut FilesystemRoots)
}

/// [`scan_with_embedded`], reading each searched root's file list and file
/// text from `source` instead of the filesystem.
///
/// Everything else is unchanged: the roots come from
/// [`SchemaRoots::for_document`], and payloads resolve in `context` through
/// the filesystem. Given a `source` that returns what the filesystem holds,
/// the result equals [`scan_with_embedded`]'s.
///
/// ## Errors
///
/// Every [`scan_with_embedded`] error, and any error `source` returns.
pub fn scan_with_source(
    context: &FileResolutionContext,
    bundles: &[EmbeddedSchemaBundle],
    source: &mut dyn SchemaRootSource,
) -> Result<TriggerRegistry, SchemaError> {
    let roots = SchemaRoots::for_document(context)?;

    // Apply filename shadowing: the first root wins by filename.
    let mut first_by_name: HashMap<String, PathBuf> = HashMap::new();
    let mut unshadowed: Vec<(RootFile, &Path)> = Vec::new();
    let mut shadowed: Vec<ShadowedFile> = Vec::new();
    for root in roots.searched() {
        for path in source.root_files(&root.path)? {
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let file = RootFile { name: name.to_string(), path };
            if let Some(shadowing_path) = first_by_name.get(&file.name) {
                shadowed.push(ShadowedFile {
                    path: file.path.clone(),
                    shadowed_by: shadowing_path.clone(),
                });
            } else {
                first_by_name.insert(file.name.clone(), file.path.clone());
                unshadowed.push((file, &root.pattern_cwd));
            }
        }
    }

    // Classify and load each unshadowed file (transactional).
    let mut triggers: Vec<LoadedTrigger> = Vec::new();
    let mut trigger_paths: Vec<PathBuf> = Vec::new();
    for (file, pattern_cwd) in &unshadowed {
        let content = source.root_file_text(&file.path)?;
        match parse_trigger_envelope_from_str(&content) {
            Ok(None) => {
                // Not a trigger-schema — silently ignored.
            }
            Ok(Some(envelope)) => {
                trigger_paths.push(file.path.clone());
                triggers.push(LoadedTrigger {
                    source: TriggerSource::File(file.path.clone()),
                    envelope,
                    pattern_cwd: pattern_cwd.to_path_buf(),
                    bundle: None,
                });
            }
            Err(source) => {
                return Err(SchemaError::TriggerLoad {
                    path: file.path.clone(),
                    source: Box::new(source),
                });
            }
        }
    }

    // Resolve every payload (transactional). Payload failures — missing files,
    // cycles, non-mergeable shapes — are hard load errors at scan time,
    // independent of whether any current document matches the trigger.
    let mut payloads = Vec::with_capacity(triggers.len());
    for (trigger, path) in triggers.iter().zip(&trigger_paths) {
        let payload = super::assemble::resolve_trigger_payload(trigger, roots.search_paths(), context).map_err(
            |source| SchemaError::TriggerLoad {
                path: path.clone(),
                source: Box::new(source),
            },
        )?;
        payloads.push(payload);
    }

    let mut registry = TriggerRegistry {
        roots,
        triggers,
        payloads,
        shadowed,
        claimed_names: first_by_name.into_keys().collect(),
        context: context.clone(),
    };
    for bundle in bundles {
        registry.append_embedded_bundle(*bundle)?;
    }
    Ok(registry)
}



#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::schemas::errors::SchemaError;
    use std::fs;
    use std::path::PathBuf;
    use tempfile::TempDir;

    /// Creates a temp directory standing for a repository root; tests pass it
    /// to the context as the repository, so discovery never reaches a
    /// shared tempdir.
    fn repo_fixture() -> TempDir {
        let dir = TempDir::new().unwrap();
        fs::create_dir_all(dir.path().join(".git")).unwrap();
        dir
    }

    fn write_file(path: &Path, content: &str) {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(path, content).unwrap();
    }

    /// A minimal valid trigger envelope with a payload reference.
    const VALID_TRIGGER: &str =
        "kind: trigger-schema\nmatch:\n  prompt: string(required)\n$schema: payload-a.yaml\n";

    /// A second minimal trigger with a different match key.
    const VALID_TRIGGER_2: &str =
        "kind: trigger-schema\nmatch:\n  title: string(required)\n$schema: payload-b.yaml\n";

    /// A simple mergeable payload for test fixtures.
    const PAYLOAD_A: &str = "$schema:\n  model: string(required)\n";
    const PAYLOAD_B: &str = "$schema:\n  owner: string(required)\n";

    /// Writes a payload file (`payload-a.yaml` / `payload-b.yaml`) into `dir`.
    fn write_payloads(dir: &Path) {
        write_file(&dir.join("payload-a.yaml"), PAYLOAD_A);
        write_file(&dir.join("payload-b.yaml"), PAYLOAD_B);
    }

    /// The context of a document in `doc_dir` inside repository `root`,
    /// with no home directory and no environment, so no user-level root is
    /// searched.
    fn doc_context(doc_dir: &Path, root: &Path) -> biscuit_file::FileResolutionContext {
        biscuit_file::FileResolutionContext::from_snapshot(doc_dir, None, std::collections::HashMap::new())
            .with_repository_root(root)
    }

    /// Scans for a document at `doc` (its folder need not exist). A document
    /// under `{root}/pkg` belongs to the package `{root}/pkg`.
    fn scan_for(doc: &Path, root: &Path) -> Result<TriggerRegistry, SchemaError> {
        let mut context = doc_context(doc.parent().unwrap(), root);
        if doc.starts_with(root.join("pkg")) {
            context = context.with_package_root(root.join("pkg"));
        }
        scan(&context)
    }

    // ── File enumeration ────────────────────────────────────────────────

    #[cfg(any(unix, windows))]
    #[test]
    fn symlinked_tree_schemas_root_is_not_searched() {
        let repo = repo_fixture();
        let external = repo_fixture();
        let external_schemas = external.path().join("schemas");
        fs::create_dir_all(&external_schemas).unwrap();
        fs::write(external_schemas.join("t.trigger.yaml"), VALID_TRIGGER).unwrap();

        let schemas_link = repo.path().join("schemas");
        #[cfg(unix)]
        std::os::unix::fs::symlink(&external_schemas, &schemas_link).unwrap();
        #[cfg(windows)]
        if let Err(error) = std::os::windows::fs::symlink_dir(&external_schemas, &schemas_link) {
            if error.kind() == std::io::ErrorKind::PermissionDenied
                || error.raw_os_error() == Some(1314)
            {
                return;
            }
            panic!("failed to create directory symlink: {error}");
        }

        let registry = scan_for(&repo.path().join("doc.md"), repo.path()).unwrap();
        assert!(registry.roots.searched().is_empty(), "symlinked schema root must be excluded");
        assert!(registry.is_empty());
    }

    /// A schema root that exists but cannot be read fails the scan naming
    /// it, so no registry is installed without that root's triggers.
    #[cfg(unix)]
    #[test]
    fn an_unreadable_schema_root_fails_the_scan() {
        use std::os::unix::fs::PermissionsExt;

        struct Restore(PathBuf);
        impl Drop for Restore {
            fn drop(&mut self) {
                let _ = fs::set_permissions(&self.0, fs::Permissions::from_mode(0o755));
            }
        }

        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(schemas.join("a.yaml"), VALID_TRIGGER).unwrap();
        fs::set_permissions(&schemas, fs::Permissions::from_mode(0o000)).unwrap();
        let _restore = Restore(schemas.clone());
        match fs::read_dir(&schemas) {
            Err(error) if error.kind() == std::io::ErrorKind::PermissionDenied => {}
            other => {
                eprintln!("skipping: {} is still readable after chmod 000 ({other:?})", schemas.display());
                return;
            }
        }

        let result = scan_for(&repo.path().join("doc.md"), repo.path());
        assert!(
            matches!(&result, Err(SchemaError::Io { path, .. }) if *path == schemas),
            "{result:?}"
        );
    }

    #[test]
    fn enumerate_yaml_and_yml() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        fs::write(schemas.join("a.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("b.yml"), VALID_TRIGGER_2).unwrap();
        // Non-YAML file is ignored.
        fs::write(schemas.join("c.txt"), "not yaml").unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].name, "a.yaml");
        assert_eq!(files[1].name, "b.yml");
    }

    #[test]
    fn enumerate_sorted_by_utf8_bytes() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        // Byte order: M(77) < a(97) < z(122).
        fs::write(schemas.join("z.trigger.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("a.trigger.yaml"), VALID_TRIGGER_2).unwrap();
        fs::write(schemas.join("M.trigger.yaml"), VALID_TRIGGER).unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 3);
        assert_eq!(files[0].name, "M.trigger.yaml");
        assert_eq!(files[1].name, "a.trigger.yaml");
        assert_eq!(files[2].name, "z.trigger.yaml");
    }

    #[test]
    fn detect_case_fold_collision_directly() {
        // On macOS (case-insensitive filesystem) two files differing only by
        // case cannot physically coexist, so test the collision check logic
        // directly.
        let root = Path::new("/repo/schemas");
        let files = vec![
            RootFile {
                path: PathBuf::from("/repo/schemas/Foo.yaml"),
                name: "Foo.yaml".to_string(),
            },
            RootFile {
                path: PathBuf::from("/repo/schemas/foo.yaml"),
                name: "foo.yaml".to_string(),
            },
        ];
        let err = check_case_fold_collision(root, &files).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerCaseFoldCollision { .. }
        ));
    }

    #[test]
    fn detect_no_collision_when_names_differ() {
        let root = Path::new("/repo/schemas");
        let files = vec![
            RootFile {
                path: PathBuf::from("/repo/schemas/a.yaml"),
                name: "a.yaml".to_string(),
            },
            RootFile {
                path: PathBuf::from("/repo/schemas/b.yaml"),
                name: "b.yaml".to_string(),
            },
        ];
        assert!(check_case_fold_collision(root, &files).is_ok());
    }

    #[test]
    fn enumerate_excludes_symlinked_file() {
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(&schemas).unwrap();
        let target = repo.path().join("real.yaml");
        fs::write(&target, VALID_TRIGGER).unwrap();

        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&target, schemas.join("link.yaml")).unwrap();
        }

        // On non-Unix the symlink test is a no-op; just verify the real file
        // was found and the symlink was excluded.
        #[cfg(unix)]
        {
            let files = enumerate_root(&schemas).unwrap();
            assert_eq!(files.len(), 0, "symlinked file should be excluded");
        }
        #[cfg(not(unix))]
        {
            let _ = target;
        }
    }

    #[test]
    fn enumerate_excludes_subdirectories() {
        // `read_dir` lists only immediate children; subdirectories are never
        // descended into. This implicitly satisfies the "do not follow
        // directory symlinks" policy since symlinked subdirs are also
        // immediate-child entries that are not regular files.
        let repo = repo_fixture();
        let schemas = repo.path().join("schemas");
        fs::create_dir_all(schemas.join("subdir")).unwrap();
        fs::write(schemas.join("subdir/hidden.yaml"), VALID_TRIGGER).unwrap();
        fs::write(schemas.join("visible.yaml"), VALID_TRIGGER_2).unwrap();

        let files = enumerate_root(&schemas).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].name, "visible.yaml");
    }

    // ── Shadowing ────────────────────────────────────────────────────────

    #[test]
    fn shadowing_nearest_root_wins() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Same filename in both roots.
        fs::write(
            root.join("pkg/schemas/claudine.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/claudine.trigger.yaml"),
            VALID_TRIGGER_2,
        )
        .unwrap();
        // Payload for the winning (nearest) trigger.
        write_payloads(&root.join("pkg/schemas"));

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
        // The nearest root's file won.
        assert!(registry.triggers[0]
            .source
            .path()
            .unwrap()
            .ends_with("pkg/schemas/claudine.trigger.yaml"));
        assert_eq!(registry.shadowed.len(), 1);
        assert!(registry.shadowed[0]
            .path
            .ends_with("schemas/claudine.trigger.yaml"));
    }

    #[test]
    fn shadowing_different_names_coexist() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(
            root.join("pkg/schemas/claudine.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/inline.trigger.yaml"),
            VALID_TRIGGER_2,
        )
        .unwrap();
        // Each trigger's payload must be co-located; using distinct payload
        // filenames keeps shadowing empty.
        write_file(&root.join("pkg/schemas/payload-a.yaml"), PAYLOAD_A);
        write_file(&root.join("schemas/payload-b.yaml"), PAYLOAD_B);

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 2);
        assert!(registry.shadowed.is_empty());
    }

    // ── Discovery classification ────────────────────────────────────────

    #[test]
    fn non_trigger_yaml_silently_ignored() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // A normal schema file (no kind: trigger-schema).
        fs::write(
            root.join("schemas/normal.yaml"),
            "$schema:\n  title: string\n",
        )
        .unwrap();
        // A trigger file.
        fs::write(
            root.join("schemas/trigger.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
        assert!(registry.triggers[0]
            .source
            .path()
            .unwrap()
            .ends_with("trigger.trigger.yaml"));
    }

    #[test]
    fn malformed_trigger_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Claims the envelope but has no `match:`.
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan_for(&doc, root).unwrap_err();
        assert!(matches!(err, SchemaError::TriggerLoad { .. }));
    }

    #[test]
    fn malformed_trigger_atomic_no_registry() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // One valid, one malformed.
        fs::write(
            root.join("schemas/good.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\n",
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let result = scan_for(&doc, root);
        assert!(result.is_err(), "transactional: no registry on any failure");
    }

    #[test]
    fn vacuous_trigger_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(
            root.join("schemas/vacuous.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  maybe: string\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan_for(&doc, root).unwrap_err();
        assert!(matches!(
            err,
            SchemaError::TriggerLoad { ref source, .. }
                if matches!(**source, SchemaError::TriggerVacuousArm)
        ));
    }

    // ── Full scan integration ───────────────────────────────────────────

    #[test]
    fn scan_nested_roots_and_ordering() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("pkg/schemas")).unwrap();
        fs::create_dir_all(root.join("schemas")).unwrap();

        // Nearest root has two triggers; root-level has one (shadowed).
        write_file(
            &root.join("pkg/schemas/a.trigger.yaml"),
            VALID_TRIGGER,
        );
        write_file(
            &root.join("pkg/schemas/b.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_file(
            &root.join("schemas/a.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_payloads(&root.join("pkg/schemas"));

        let doc = root.join("pkg/sub/doc.md");
        let registry = scan_for(&doc, root).unwrap();
        // Two unshadowed triggers (a and b from nearest root).
        assert_eq!(registry.triggers.len(), 2);
        // Order: filename-lexicographic within nearest root.
        assert!(registry.triggers[0]
            .source
            .path()
            .unwrap()
            .ends_with("pkg/schemas/a.trigger.yaml"));
        assert!(registry.triggers[1]
            .source
            .path()
            .unwrap()
            .ends_with("pkg/schemas/b.trigger.yaml"));
        // Root-level a.trigger.yaml is shadowed.
        assert_eq!(registry.shadowed.len(), 1);
        // Roots searched package first.
        assert_eq!(registry.roots.search_paths().len(), 2);
        assert!(registry.roots.search_paths()[0].ends_with("pkg/schemas"));
    }

    #[test]
    fn scan_reads_another_repository_s_roots_never() {
        let repo = repo_fixture();
        let other = repo_fixture();
        fs::create_dir_all(repo.path().join("schemas")).unwrap();
        fs::write(
            repo.path().join("schemas/t.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();

        let doc = other.path().join("doc.md");
        let registry = scan_for(&doc, other.path()).unwrap();
        assert!(registry.is_empty());
        assert!(registry.roots.search_paths().is_empty());
    }

    #[test]
    fn scan_skips_in_between_schemas_folders() {
        let repo = repo_fixture();
        let root = repo.path();
        write_file(&root.join("docs/schemas/t.trigger.yaml"), VALID_TRIGGER);
        write_payloads(&root.join("docs/schemas"));

        let registry = scan_for(&root.join("docs/doc.md"), root).unwrap();
        assert!(registry.is_empty());
        assert!(registry.roots.search_paths().is_empty());
    }

    #[test]
    fn scan_no_schemas_dirs_empty() {
        let repo = repo_fixture();
        let doc = repo.path().join("doc.md");
        let registry = scan_for(&doc, repo.path()).unwrap();
        assert!(registry.is_empty());
    }

    #[test]
    fn scan_yml_and_yaml_mix() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        write_file(
            &root.join("schemas/a.trigger.yml"),
            VALID_TRIGGER,
        );
        write_file(
            &root.join("schemas/b.trigger.yaml"),
            VALID_TRIGGER_2,
        );
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 2);
    }

    #[test]
    fn scan_empty_schemas_dir_no_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();

        let doc = root.join("doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert!(registry.is_empty());
        assert_eq!(registry.roots.search_paths().len(), 1);
    }

    #[test]
    fn scan_non_yaml_yaml_files_ignored() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        fs::write(root.join("schemas/readme.md"), "# Schemas").unwrap();
        fs::write(root.join("schemas/config.json"), "{}").unwrap();
        fs::write(
            root.join("schemas/real.trigger.yaml"),
            VALID_TRIGGER,
        )
        .unwrap();
        write_payloads(&root.join("schemas"));

        let doc = root.join("doc.md");
        let registry = scan_for(&doc, root).unwrap();
        assert_eq!(registry.triggers.len(), 1);
    }

    // ── Payload resolution is transactional at scan time ───────────────

    #[test]
    fn unmatched_trigger_with_missing_payload_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // The trigger matches on `prompt`, which no document in this test
        // provides — but the payload file is absent, so scan must fail
        // regardless of whether any document would match.
        fs::write(
            root.join("schemas/lonely.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema: does-not-exist.yaml\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan_for(&doc, root).unwrap_err();
        assert!(
            matches!(err, SchemaError::TriggerLoad { .. }),
            "missing payload must be a hard load error: {err:?}"
        );
    }

    #[test]
    fn unmatched_trigger_with_non_mergeable_payload_is_hard_error() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // Inline root-union payload — not merge-compatible. The match key
        // (`prompt`) is absent from every document in this test.
        fs::write(
            root.join("schemas/union.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  prompt: string(required)\n\
             $schema:\n  - model: string(required)\n  - title: string(required)\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let err = scan_for(&doc, root).unwrap_err();
        match err {
            SchemaError::TriggerLoad { source, .. } => {
                assert!(
                    matches!(*source, SchemaError::TriggerPayloadNotMergeable { .. }),
                    "inner error must be TriggerPayloadNotMergeable: {source:?}"
                );
            }
            other => panic!("expected TriggerLoad wrapping TriggerPayloadNotMergeable, got {other:?}"),
        }
    }

    #[test]
    fn payload_resolution_failure_is_transactional() {
        let repo = repo_fixture();
        let root = repo.path();
        fs::create_dir_all(root.join("schemas")).unwrap();
        // One valid trigger with a proper payload.
        write_file(&root.join("schemas/good.trigger.yaml"), VALID_TRIGGER);
        write_payloads(&root.join("schemas"));
        // One trigger whose payload file does not exist.
        fs::write(
            root.join("schemas/bad.trigger.yaml"),
            "kind: trigger-schema\nmatch:\n  title: string(required)\n\
             $schema: missing.yaml\n",
        )
        .unwrap();

        let doc = root.join("doc.md");
        let result = scan_for(&doc, root);
        assert!(
            result.is_err(),
            "a single bad payload must fail the whole scan transactionally"
        );
    }

    // ── Error display ───────────────────────────────────────────────────

    #[test]
    fn case_fold_collision_display_includes_files() {
        let err = SchemaError::TriggerCaseFoldCollision {
            root: PathBuf::from("/repo/schemas"),
            files: vec!["Foo.yaml".into(), "foo.yaml".into()],
        };
        let msg = err.to_string();
        assert!(msg.contains("Foo.yaml"));
        assert!(msg.contains("foo.yaml"));
    }

    #[test]
    fn trigger_load_display_includes_path() {
        let err = SchemaError::TriggerLoad {
            path: PathBuf::from("/repo/schemas/bad.trigger.yaml"),
            source: Box::new(SchemaError::TriggerMatch {
                message: "missing match key".into(),
            }),
        };
        let msg = err.to_string();
        assert!(msg.contains("bad.trigger.yaml"));
        assert!(msg.contains("missing match key"));
    }
}

//! The trigger-registry cache: each document's trigger registry, prepared
//! once and served from memory until one of its inputs changes.
//!
//! Two layers, cached separately:
//!
//! - **Root contents** ([`RootContents`]): each schema-root folder's file
//!   list and each root file's text, xxHash, and classification (named
//!   schema, trigger schema, failed trigger, or unclassified). These depend
//!   on the files alone, so every document whose roots include a folder
//!   shares them. Whether a trigger's payload resolves depends on the
//!   document's context, so it is recorded on the document registry instead.
//! - **Document registries** ([`Scope`]): the registry discovery computes for
//!   one document, through
//!   [`scan_with_source`](darkmatter::markdown::schemas::triggers::scan_with_source)
//!   over the root contents. The result therefore equals `triggers::scan`
//!   for that document. It is bound to the document's context, because
//!   payloads and `$path` patterns resolve there; two documents with the same
//!   roots but different contexts never share one.
//!
//! [`TriggerRegistries::lookup`] only reads installed state: it never lists a
//! directory, reads, hashes, or resolves a payload. A registry that is
//! missing, stale, or prepared in another context
//! ([`TriggerRegistries::is_fresh`]) is prepared in three steps, so the
//! cache's lock is never held across preparation I/O:
//!
//! 1. [`TriggerRegistries::begin`] (under the lock) copies the root contents
//!    into a [`Preparation`];
//! 2. [`Preparation::run`] (no lock) scans the roots and reads the payloads;
//! 3. [`TriggerRegistries::install`] (under the lock) installs the result in
//!    one step, unless a preparation of the same document that began later
//!    has installed already. A change recorded while the preparation ran
//!    leaves the installed scope stale when it touches what the preparation
//!    read, and keeps the preparation's root reads out of the shared cache.
//!
//! A document registry is prepared again only after
//! [`TriggerRegistries::invalidate_paths`] or
//! [`TriggerRegistries::invalidate_all`] marks it stale, or when its context
//! changes. Each preparation records what it read: the searched and absent
//! root folders, every root file and payload dependency with its xxHash, the
//! folders holding the dependencies, and every candidate location payload
//! resolution searched (present or absent) through the selected file. A
//! changed path is re-read and compared with its recorded hash; an unchanged
//! file invalidates nothing. A creation or deletion in a recorded folder, at
//! a searched candidate, or of a folder above a candidate invalidates without
//! a hash. A failed scope is invalidated by any change, since its error may
//! name a file that does not exist yet, and keeps the candidates and folders
//! of the scope it replaced so their watches survive until it recovers.
//!
//! Failures follow the transition rule in [`TriggerRegistries::record_failure`]:
//! one record per failure scope (workspace boundary and document folder)
//! decides both the trigger-file diagnostic and the log level, so a repeated
//! failure is neither re-published nor re-logged at `warn`.

use std::collections::{BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::triggers::{
    SchemaRootSource, TriggerRegistry, enumerate_root_files, parse_trigger_envelope_from_str,
    scan_with_source,
};
use darkmatter::markdown::schemas::{
    SchemaError, SchemaRootKind, SchemaRootState, SchemaRoots, parse_standalone_schema_document,
};

use super::TriggerDiagnosticTransition;
use crate::work_counters;
use crate::workspace::watch::WatchedFolder;

/// What a schema-root file is, judged from its text alone.
///
/// A [`SchemaTrigger`](Self::SchemaTrigger) can still fail to load once its
/// payload is resolved in a document's context; [`TriggerRegistries::summary`]
/// combines this class with those outcomes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RootFileClass {
    /// A standalone SimplifiedSchema document, available by name.
    NamedSchema,
    /// A file claiming `kind: schema-trigger` whose envelope loads.
    SchemaTrigger,
    /// A file claiming `kind: schema-trigger` whose envelope does not load.
    FailedTrigger,
    /// Neither: ordinary or unparseable YAML that claims no envelope.
    Unclassified,
}

/// Classifies one root file's text. Passive: nothing is resolved or read.
fn classify(text: &str, path: &Path) -> RootFileClass {
    match parse_trigger_envelope_from_str(text) {
        Ok(Some(_)) => RootFileClass::SchemaTrigger,
        Err(_) => RootFileClass::FailedTrigger,
        Ok(None) => match parse_standalone_schema_document(text, path) {
            Ok(Some(_)) => RootFileClass::NamedSchema,
            Ok(None) | Err(_) => RootFileClass::Unclassified,
        },
    }
}

/// Distinct schema-root folders and files across the prepared document
/// registries, before per-document shadowing.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RegistrySummary {
    /// Distinct searched schema folders.
    pub roots: usize,
    /// Named schemas in folders serving a tree, `SCHEMAS_DIR`, or home role.
    /// A folder that serves both a scoped and a tree role counts here.
    pub tree_schemas: usize,
    /// Named schemas in folders serving only package or package-area roles.
    pub scoped_schemas: usize,
    /// Trigger schemas whose envelope loads and whose payload no prepared
    /// registry failed to resolve, in any folder.
    pub trigger_schemas: usize,
    /// Root files claiming the trigger envelope that failed to load: the
    /// envelope does not load, or a prepared registry failed resolving the
    /// payload. A trigger no preparation attempted (shadowed in every
    /// prepared registry, or after an earlier failing trigger) keeps its
    /// envelope's class.
    pub failed_triggers: usize,
    /// Root files that are neither named nor trigger schemas.
    pub unclassified: usize,
    /// Document registries whose last preparation failed.
    pub failed_scopes: usize,
}

/// A cached root file.
#[derive(Clone)]
struct RootFile {
    text: Arc<str>,
    hash: u64,
    class: RootFileClass,
}

/// The schema-root contents shared by every document registry, keyed by
/// [`normalized`] path.
#[derive(Clone, Default)]
struct RootContents {
    listings: HashMap<PathBuf, Vec<PathBuf>>,
    files: HashMap<PathBuf, RootFile>,
    /// Root files to compare with disk before reuse, after a change DMLS
    /// could not attribute to a path (a server rescan).
    unverified: HashSet<PathBuf>,
}

impl RootContents {
    /// Applies a change at `path`. A known file is re-read and compared; a
    /// changed or missing file is dropped, a missing one with its folder's
    /// listing. An unknown YAML file in a listed folder drops that listing.
    ///
    /// ## Returns
    ///
    /// The file's current hash when it is known and still readable.
    fn apply_change(&mut self, path: &Path) -> Option<u64> {
        let parent = path.parent().map(Path::to_path_buf);
        if let Some(file) = self.files.get(path) {
            let current = read_text(path).ok().map(|(_, hash)| hash);
            if current == Some(file.hash) {
                return current;
            }
            self.files.remove(path);
            if current.is_none()
                && let Some(parent) = &parent
            {
                self.listings.remove(parent);
            }
            return current;
        }
        if is_yaml(path)
            && let Some(parent) = &parent
        {
            self.listings.remove(parent);
        }
        None
    }

    /// Forgets every listing and marks every file for comparison with disk.
    fn invalidate_all(&mut self) {
        self.listings.clear();
        self.unverified.extend(self.files.keys().cloned());
    }

    /// Adopts what a preparation read into its copy of these contents. Only
    /// called when no change arrived since the copy was taken, so both
    /// describe the same disk state.
    fn adopt(&mut self, read: RootContents) {
        self.listings.extend(read.listings);
        self.files.extend(read.files);
        self.unverified.retain(|path| read.unverified.contains(path));
    }
}

impl RootContents {
    /// Root file `path`'s text, read and classified unless cached and
    /// verified. A re-read with an unchanged hash keeps the classification.
    fn load(&mut self, path: &Path) -> std::io::Result<Arc<str>> {
        let key = normalized(path);
        if !self.unverified.remove(&key)
            && let Some(file) = self.files.get(&key)
        {
            return Ok(Arc::clone(&file.text));
        }
        let (text, hash) = read_text(path)?;
        if let Some(file) = self.files.get(&key)
            && file.hash == hash
        {
            return Ok(Arc::clone(&file.text));
        }
        let class = classify(&text, path);
        let text: Arc<str> = text.into();
        self.files.insert(key, RootFile { text: Arc::clone(&text), hash, class });
        Ok(text)
    }
}

impl SchemaRootSource for RootContents {
    /// Lists `root` and classifies every file in it, including files a
    /// scan never reads (shadowed ones, or those after a failing trigger),
    /// so every enumerated file is counted. An unreadable file is left for
    /// the scan to report when it reads it.
    fn root_files(&mut self, root: &Path) -> Result<Vec<PathBuf>, SchemaError> {
        let key = normalized(root);
        if let Some(listing) = self.listings.get(&key) {
            return Ok(listing.clone());
        }
        work_counters::record_enumeration();
        let listing = enumerate_root_files(root)?;
        for path in &listing {
            let _ = self.load(path);
        }
        self.listings.insert(key, listing.clone());
        Ok(listing)
    }

    fn root_file_text(&mut self, path: &Path) -> Result<Arc<str>, SchemaError> {
        self.load(path).map_err(|source| SchemaError::Io {
            path: path.to_path_buf(),
            source,
        })
    }
}

/// Reads `path` as UTF-8 text and hashes it.
fn read_text(path: &Path) -> std::io::Result<(String, u64)> {
    let bytes = work_counters::read(path)?;
    let hash = work_counters::hash_bytes(&bytes);
    let text = String::from_utf8(bytes)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    Ok((text, hash))
}

fn is_yaml(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("yaml" | "yml"))
}

/// `path` with its nearest existing ancestor folder canonicalized, so a root
/// recorded through one spelling (`/var/…`) matches an event reported through
/// another (`/private/var/…`), even after the folder itself was deleted. The
/// last component is kept as written: a deleted or not-yet-created file has
/// no canonical form of its own.
fn normalized(path: &Path) -> PathBuf {
    let mut rest = Vec::new();
    let mut current = path;
    while let (Some(parent), Some(name)) = (current.parent(), current.file_name()) {
        rest.push(name);
        if let Ok(canonical) = biscuit_file::canonicalize_simplified(parent) {
            return rest.iter().rev().fold(canonical, |path, name| path.join(name));
        }
        current = parent;
    }
    path.to_path_buf()
}

/// A registry installed for a document, with the version the schema cache
/// keys on.
#[derive(Clone)]
pub struct InstalledRegistry {
    /// The registry discovery produced.
    pub registry: TriggerRegistry,
    /// Distinct for every installed registry, so a cached effective schema
    /// re-assembles when its registry is replaced.
    pub version: u64,
}

/// What a document registry's last preparation produced.
enum Outcome {
    Ready,
    /// A trigger failed to load; the scope serves its last-good registry.
    TriggerFailed {
        /// The [`normalized`] trigger file the error names, when it names one.
        trigger_file: Option<PathBuf>,
    },
    /// A schema root could not be read; the scope serves this error and
    /// never last-good data.
    RootFailed(Arc<SchemaError>),
}

/// What one document registry's preparation read, for invalidation.
#[derive(Default)]
struct Dependencies {
    /// Searched and absent root folders, each with the role it serves.
    roots: Vec<(PathBuf, SchemaRootKind)>,
    /// Absent root folders, where a created file adds a root.
    absent: Vec<PathBuf>,
    /// Root files and payload dependencies with their hash (`None` when
    /// unreadable at preparation).
    files: HashMap<PathBuf, Option<u64>>,
    /// Folders holding payload dependencies.
    folders: HashSet<PathBuf>,
    /// Every candidate location payload resolution searched, through the
    /// selected file ([`TriggerRegistry::searched_locations`]), present or
    /// absent. A change at a candidate, or at a folder above one, can change
    /// which file resolution selects.
    searched: Vec<PathBuf>,
    /// Absent root folders and absent searched-candidate folders, each with
    /// its nearest existing ancestor, from which the folder's creation is
    /// watched.
    absent_bases: HashMap<PathBuf, PathBuf>,
}

impl Dependencies {
    /// Records `candidates` (searched locations) and the absence of their
    /// folders. Reads folder metadata, so only a preparation calls it.
    fn record_searched<'a>(&mut self, candidates: impl IntoIterator<Item = &'a Path>) {
        for candidate in candidates {
            let key = normalized(candidate);
            if let Some(folder) = key.parent() {
                self.record_absence(folder);
            }
            self.searched.push(key);
        }
        self.searched.sort();
        self.searched.dedup();
    }

    /// Records [`normalized`] `folder`'s nearest existing ancestor when the
    /// folder does not exist.
    fn record_absence(&mut self, folder: &Path) {
        if self.absent_bases.contains_key(folder) || folder.is_dir() {
            return;
        }
        if let Some(base) = folder.ancestors().skip(1).find(|ancestor| ancestor.is_dir()) {
            self.absent_bases.insert(folder.to_path_buf(), base.to_path_buf());
        }
    }

    /// The folders a watch must cover: root folders, dependency folders, and
    /// the folders of searched candidates.
    fn watched_folders(&self) -> impl Iterator<Item = WatchedFolder> + '_ {
        self.roots
            .iter()
            .map(|(folder, _)| folder.as_path())
            .chain(self.absent.iter().map(PathBuf::as_path))
            .chain(self.folders.iter().map(PathBuf::as_path))
            .chain(self.searched.iter().filter_map(|candidate| candidate.parent()))
            .map(|folder| WatchedFolder {
                folder: folder.to_path_buf(),
                absent_base: self.absent_bases.get(folder).cloned(),
            })
    }
}

/// One document's registry.
struct Scope {
    /// The context the registry was prepared in; a lookup in any other
    /// context prepares it again.
    context: FileResolutionContext,
    outcome: Outcome,
    /// The last registry that loaded, served while a trigger failure holds
    /// and never while a root failure does.
    installed: Option<InstalledRegistry>,
    deps: Dependencies,
    stale: bool,
    /// The [`Preparation`] sequence number that installed this scope.
    seq: u64,
}

/// A change recorded while a preparation was running, replayed against its
/// result at [`TriggerRegistries::install`].
#[derive(Clone)]
enum Change {
    Path(PathBuf),
    All,
}

/// One document registry's preparation: what [`TriggerRegistries::begin`]
/// took from the cache, run by [`Preparation::run`] without the cache's lock.
pub struct Preparation {
    path: PathBuf,
    boundary: PathBuf,
    context: FileResolutionContext,
    /// A copy of the shared root contents; the scan fills it.
    roots: RootContents,
    /// The change epoch when the copy was taken.
    epoch: u64,
    seq: u64,
    /// The previous scope's searched candidates and dependency folders, kept
    /// when this preparation fails: the failing file may be one of them, and
    /// its repair must still be watched.
    carried: (Vec<PathBuf>, HashSet<PathBuf>),
}

/// A finished [`Preparation`], ready for [`TriggerRegistries::install`].
pub struct Prepared {
    path: PathBuf,
    boundary: PathBuf,
    context: FileResolutionContext,
    roots: RootContents,
    epoch: u64,
    seq: u64,
    result: Result<TriggerRegistry, SchemaError>,
    deps: Dependencies,
}

impl Prepared {
    /// Whether the registry failed to load.
    pub fn failed(&self) -> bool {
        self.result.is_err()
    }
}

impl Preparation {
    /// Scans the document's roots and reads its payload dependencies' hashes.
    /// Does all of a preparation's I/O, and touches no shared state.
    pub fn run(mut self) -> Prepared {
        let _scan = work_counters::record_registry_scan();
        let mut deps = Dependencies::default();
        let result = scan_with_source(&self.context, &[], &mut self.roots).and_then(fail_closed_on_findings);
        if let Ok(roots) = SchemaRoots::for_document(&self.context) {
            record_roots(&roots, &mut deps);
        }
        match &result {
            Ok(registry) => deps.record_searched(registry.searched_locations()),
            // A failed resolution reports no candidates; keep the previous
            // scope's, so recovery is watched where it was.
            Err(_) => {
                let (searched, folders) = std::mem::take(&mut self.carried);
                deps.record_searched(searched.iter().map(PathBuf::as_path));
                deps.folders.extend(folders);
            }
        }
        if let Ok(registry) = &result {
            for (file, hash) in roots_read(&self.roots, &deps) {
                deps.files.insert(file, Some(hash));
            }
            // Envelopes and payloads inside a root already carry the hash the
            // root contents recorded; only files elsewhere are read.
            for dependency in registry.dependencies() {
                let key = normalized(dependency);
                if let Some(folder) = key.parent() {
                    deps.folders.insert(folder.to_path_buf());
                }
                deps.files.entry(key).or_insert_with(|| {
                    work_counters::read(dependency)
                        .ok()
                        .map(|bytes| work_counters::hash_bytes(&bytes))
                });
            }
        }
        Prepared {
            path: self.path,
            boundary: self.boundary,
            context: self.context,
            roots: self.roots,
            epoch: self.epoch,
            seq: self.seq,
            result,
            deps,
        }
    }
}

/// A failure scope: the workspace boundary and the document's folder.
type FailureKey = (PathBuf, PathBuf);

/// The current failure of one failure scope.
#[derive(Clone)]
struct FailureRecord {
    /// The trigger file (or root folder) the error names.
    path: Option<PathBuf>,
    error: Arc<SchemaError>,
    /// Whether the error is a trigger-file load error, whose diagnostic is
    /// published on that file.
    owns_diagnostic: bool,
}

/// Every document's trigger registry, and the failure state shared by the
/// diagnostics and the log.
#[derive(Default)]
pub struct TriggerRegistries {
    roots: RootContents,
    /// Keyed by document path.
    scopes: HashMap<PathBuf, Scope>,
    failures: HashMap<FailureKey, FailureRecord>,
    transitions: Vec<TriggerDiagnosticTransition>,
    next_version: u64,
    /// Bumped by every invalidation.
    epoch: u64,
    /// The next [`Preparation`] sequence number.
    next_seq: u64,
    /// Running preparations: sequence number to the epoch they began at.
    in_flight: HashMap<u64, u64>,
    /// Invalidations since the oldest running preparation began, each with
    /// the epoch it produced.
    changes: Vec<(u64, Change)>,
}

impl TriggerRegistries {
    /// The installed registry for the document at `path`. A lookup only:
    /// it never prepares, so it does no I/O whatever the scope's state.
    ///
    /// ## Returns
    ///
    /// `Ok(None)` when no registry has loaded for this document yet (a cold
    /// trigger failure, or a document never prepared); the last-good
    /// registry while a trigger failure holds; the installed registry of a
    /// scope that has gone stale until its preparation installs.
    ///
    /// ## Errors
    ///
    /// A schema-root I/O error, which is never answered with last-good data.
    pub fn lookup(&self, path: &Path) -> Result<Option<InstalledRegistry>, Arc<SchemaError>> {
        let Some(scope) = self.scopes.get(path) else {
            return Ok(None);
        };
        match &scope.outcome {
            Outcome::RootFailed(error) => Err(Arc::clone(error)),
            Outcome::Ready | Outcome::TriggerFailed { .. } => Ok(scope.installed.clone()),
        }
    }

    /// Whether the document at `path` has a registry installed for `context`
    /// that no change has marked stale. A lookup only.
    pub fn is_fresh(&self, path: &Path, context: &FileResolutionContext) -> bool {
        self.scopes
            .get(path)
            .is_some_and(|scope| !scope.stale && scope.context == context.for_source(path))
    }

    /// Starts preparing the document's registry unless it
    /// [is fresh](Self::is_fresh). Cheap: it copies the cached root contents
    /// and reads nothing; [`Preparation::run`] does the I/O.
    pub fn begin(
        &mut self,
        path: &Path,
        boundary: &Path,
        context: &FileResolutionContext,
    ) -> Option<Preparation> {
        if self.is_fresh(path, context) {
            return None;
        }
        self.next_seq += 1;
        self.in_flight.insert(self.next_seq, self.epoch);
        let carried = self
            .scopes
            .get(path)
            .map(|scope| (scope.deps.searched.clone(), scope.deps.folders.clone()))
            .unwrap_or_default();
        Some(Preparation {
            path: path.to_path_buf(),
            boundary: boundary.to_path_buf(),
            context: context.for_source(path),
            roots: self.roots.clone(),
            epoch: self.epoch,
            seq: self.next_seq,
            carried,
        })
    }

    /// Installs a finished preparation in one step and records its failure
    /// or recovery.
    ///
    /// A result is dropped when a preparation of the same document that
    /// began later has already installed, so an older reading never
    /// replaces a newer one. A change recorded since the preparation began
    /// keeps its root reads out of the shared cache, and leaves the scope
    /// stale when the change touches anything the preparation read.
    ///
    /// ## Returns
    ///
    /// Whether the document's registry is fresh afterwards.
    pub fn install(&mut self, prepared: Prepared) -> bool {
        self.in_flight.remove(&prepared.seq);
        let changes: Vec<Change> = self
            .changes
            .iter()
            .filter(|(epoch, _)| *epoch > prepared.epoch)
            .map(|(_, change)| change.clone())
            .collect();
        match self.in_flight.values().min() {
            Some(oldest) => self.changes.retain(|(epoch, _)| epoch > oldest),
            None => self.changes.clear(),
        }
        let Prepared { path, boundary, context, roots, seq, result, deps, .. } = prepared;
        if self.scopes.get(&path).is_some_and(|scope| scope.seq > seq) {
            return self.is_fresh(&path, &context);
        }
        if changes.is_empty() {
            self.roots.adopt(roots);
        }
        let failure_key = (
            boundary.to_path_buf(),
            path.parent().unwrap_or(&path).to_path_buf(),
        );
        let previous = self.scopes.remove(&path);
        let mut installed = previous.and_then(|scope| scope.installed);
        let outcome = match result {
            Ok(registry) => {
                self.next_version += 1;
                installed = Some(InstalledRegistry { registry, version: self.next_version });
                self.record_recovery(&failure_key);
                Outcome::Ready
            }
            Err(error @ SchemaError::Io { .. }) => {
                let error = Arc::new(error);
                self.record_failure(&failure_key, &error, false);
                Outcome::RootFailed(error)
            }
            Err(error) => {
                let trigger_file = match &error {
                    SchemaError::TriggerLoad { path, .. } => Some(normalized(path)),
                    _ => None,
                };
                let error = Arc::new(error);
                self.record_failure(&failure_key, &error, installed.is_some());
                Outcome::TriggerFailed { trigger_file }
            }
        };
        let mut scope = Scope { context, outcome, installed, deps, stale: false, seq };
        // No hash comparison here (it would read under the lock): a change
        // to a file the preparation read is treated as a change.
        scope.stale = changes.iter().any(|change| match change {
            Change::All => true,
            Change::Path(changed) => scope_affected(&scope, changed, None),
        });
        let fresh = !scope.stale;
        self.scopes.insert(path, scope);
        fresh
    }

    /// Prepares the document's registry on the calling thread when it is
    /// not fresh: [`begin`](Self::begin), [`Preparation::run`], and
    /// [`install`](Self::install) in a row. For callers that own the cache
    /// outright; the server prepares through the three steps instead, so its
    /// lock is not held across the I/O.
    pub fn prepare(&mut self, path: &Path, boundary: &Path, context: &FileResolutionContext) {
        if let Some(preparation) = self.begin(path, boundary, context) {
            let prepared = preparation.run();
            self.install(prepared);
        }
    }

    /// Records an invalidation for the preparations running now.
    fn record_change(&mut self, change: Change) {
        self.epoch += 1;
        if !self.in_flight.is_empty() {
            self.changes.push((self.epoch, change));
        }
    }

    /// Records a failed preparation for `key` and logs it once.
    ///
    /// The comparison with the recorded failure is the one source of truth
    /// for "this failure is new": a different file or error text publishes
    /// the trigger-file diagnostic (moving it off a previously failing file)
    /// and logs at `warn`; the same file and text logs at `debug` only.
    fn record_failure(&mut self, key: &FailureKey, error: &Arc<SchemaError>, has_last_good: bool) {
        let owns_diagnostic = matches!(**error, SchemaError::TriggerLoad { .. });
        let failure = FailureRecord {
            path: failing_path(error),
            error: Arc::clone(error),
            owns_diagnostic,
        };
        let previous = self.failures.insert(key.clone(), failure.clone());
        let is_new = previous.as_ref().is_none_or(|old| {
            old.path != failure.path || old.error.to_string() != failure.error.to_string()
        });
        if let Some(old) = previous.as_ref().filter(|old| old.owns_diagnostic && old.path != failure.path)
            && let Some(path) = &old.path
        {
            self.transitions.push(TriggerDiagnosticTransition { path: path.clone(), error: None });
        }
        if is_new
            && owns_diagnostic
            && let Some(path) = &failure.path
        {
            self.transitions.push(TriggerDiagnosticTransition {
                path: path.clone(),
                error: Some(Arc::clone(&failure.error)),
            });
        }
        log_failure(key, &failure, has_last_good, is_new);
    }

    /// Clears `key`'s recorded failure after a successful preparation.
    fn record_recovery(&mut self, key: &FailureKey) {
        let Some(previous) = self.failures.remove(key) else {
            return;
        };
        if previous.owns_diagnostic
            && let Some(path) = &previous.path
        {
            self.transitions.push(TriggerDiagnosticTransition { path: path.clone(), error: None });
        }
        tracing::info!(
            scope = %key.1.display(),
            trigger_file = %display_path(previous.path.as_deref()),
            "trigger registry recovered"
        );
    }

    /// Marks stale every registry that a change at one of `paths` affects,
    /// after comparing each known file with its recorded hash.
    pub fn invalidate_paths(&mut self, paths: &[PathBuf]) {
        for path in paths {
            let path = normalized(path);
            self.record_change(Change::Path(path.clone()));
            let current = self.roots.apply_change(&path);
            let mut dependency_hash: Option<Option<u64>> = current.map(Some);
            for scope in self.scopes.values_mut() {
                if scope.stale {
                    continue;
                }
                scope.stale |= scope_affected(scope, &path, Some(&mut dependency_hash));
            }
        }
    }

    /// Marks every registry stale and every root file for comparison with
    /// disk: a server rescan, a configuration change, or a package-manifest
    /// change, any of which can change every scope's inputs or roots.
    pub fn invalidate_all(&mut self) {
        self.record_change(Change::All);
        self.roots.invalidate_all();
        for scope in self.scopes.values_mut() {
            scope.stale = true;
        }
    }

    /// The current load error a failure scope attributes to trigger file
    /// `path`.
    pub fn load_error_for(&self, path: &Path) -> Option<Arc<SchemaError>> {
        self.failures
            .values()
            .find(|failure| failure.owns_diagnostic && failure.path.as_deref() == Some(path))
            .map(|failure| Arc::clone(&failure.error))
    }

    /// Takes trigger-file diagnostic changes since the last call.
    pub fn take_transitions(&mut self) -> Vec<TriggerDiagnosticTransition> {
        std::mem::take(&mut self.transitions)
    }

    /// Every folder the prepared registries depend on: searched and absent
    /// root folders, the folders holding payload dependencies, and the
    /// folders of every searched payload candidate, each absent one with the
    /// nearest existing ancestor its preparation observed. A lookup only.
    pub fn watch_folders(&self) -> BTreeSet<WatchedFolder> {
        self.scopes.values().flat_map(|scope| scope.deps.watched_folders()).collect()
    }

    /// Counts the distinct root folders and files the prepared registries
    /// searched. Each file counts once, under its passive class, except that
    /// a trigger schema whose load failed in a prepared registry counts as a
    /// failed trigger.
    pub fn summary(&self) -> RegistrySummary {
        let mut tree_role: HashMap<&Path, bool> = HashMap::new();
        for scope in self.scopes.values() {
            for (folder, kind) in &scope.deps.roots {
                let tree = !matches!(kind, SchemaRootKind::Package | SchemaRootKind::PackageArea);
                *tree_role.entry(folder.as_path()).or_default() |= tree;
            }
        }
        let mut summary = RegistrySummary {
            roots: tree_role.len(),
            failed_scopes: self
                .scopes
                .values()
                .filter(|scope| !matches!(scope.outcome, Outcome::Ready))
                .count(),
            ..RegistrySummary::default()
        };
        // Failed in any prepared registry counts as failed, even where another
        // context loads it: the payload resolves per context, and counting the
        // file as loaded would hide a failure the log has reported.
        let failed_loads: HashSet<&Path> = self
            .scopes
            .values()
            .filter_map(|scope| match &scope.outcome {
                Outcome::TriggerFailed { trigger_file } => trigger_file.as_deref(),
                Outcome::Ready | Outcome::RootFailed(_) => None,
            })
            .collect();
        for (path, file) in &self.roots.files {
            let Some(tree) = path.parent().and_then(|parent| tree_role.get(parent)) else {
                continue;
            };
            match file.class {
                RootFileClass::SchemaTrigger if failed_loads.contains(path.as_path()) => {
                    summary.failed_triggers += 1;
                }
                RootFileClass::NamedSchema if *tree => summary.tree_schemas += 1,
                RootFileClass::NamedSchema => summary.scoped_schemas += 1,
                RootFileClass::SchemaTrigger => summary.trigger_schemas += 1,
                RootFileClass::FailedTrigger => summary.failed_triggers += 1,
                RootFileClass::Unclassified => summary.unclassified += 1,
            }
        }
        summary
    }
}

/// Records `roots`'s searched and absent folders.
fn record_roots(roots: &SchemaRoots, deps: &mut Dependencies) {
    for root in roots.searched() {
        deps.roots.push((normalized(&root.path), root.kind));
    }
    for entry in roots.entries() {
        if let SchemaRootState::Absent(folder) = &entry.state {
            let folder = normalized(folder);
            deps.record_absence(&folder);
            deps.absent.push(folder);
        }
    }
}

/// The cached root files inside `deps`'s searched folders, with hashes.
fn roots_read(roots: &RootContents, deps: &Dependencies) -> Vec<(PathBuf, u64)> {
    let folders: HashSet<&Path> = deps.roots.iter().map(|(folder, _)| folder.as_path()).collect();
    roots
        .files
        .iter()
        .filter(|(path, _)| path.parent().is_some_and(|parent| folders.contains(parent)))
        .map(|(path, file)| (path.clone(), file.hash))
        .collect()
}

/// Whether a change at [`normalized`] `path` affects `scope`. A failed
/// scope is affected by any change. A file the scope read is affected when
/// its current hash differs from the recorded one: `current` caches that
/// hash across scopes, and `None` skips the comparison (the file counts as
/// changed).
fn scope_affected(scope: &Scope, path: &Path, current: Option<&mut Option<Option<u64>>>) -> bool {
    if !matches!(scope.outcome, Outcome::Ready) {
        return true;
    }
    if let Some(recorded) = scope.deps.files.get(path) {
        let Some(current) = current else {
            return true;
        };
        let current = *current.get_or_insert_with(|| {
            work_counters::read(path).ok().map(|bytes| work_counters::hash_bytes(&bytes))
        });
        return current != *recorded;
    }
    // A candidate created, deleted, or replaced, or a folder above one
    // (a client may report only the directory of a folder-and-file creation).
    if scope.deps.searched.iter().any(|candidate| candidate.starts_with(path)) {
        return true;
    }
    let Some(parent) = path.parent() else {
        return false;
    };
    let in_root = scope.deps.roots.iter().any(|(folder, _)| folder == parent)
        || scope.deps.absent.iter().any(|folder| folder == parent);
    (in_root && is_yaml(path)) || scope.deps.folders.contains(parent)
}

/// Fails a scan that skipped an invalid trigger as the first skipped
/// trigger's load error, so the scope keeps failing closed with its
/// last-good registry and trigger-file diagnostic.
///
/// Discovery now recovers per trigger; this keeps DMLS's whole-scope failure
/// contract until DMLS installs a scan's valid subset and publishes its
/// findings as warnings itself.
fn fail_closed_on_findings(mut registry: TriggerRegistry) -> Result<TriggerRegistry, SchemaError> {
    if registry.findings.is_empty() {
        return Ok(registry);
    }
    let finding = registry.findings.swap_remove(0);
    let path = finding.source().path().map(Path::to_path_buf);
    let cause = Arc::try_unwrap(finding.into_cause())
        .unwrap_or_else(|shared| SchemaError::TriggerMatch { message: shared.to_string() });
    Err(match path {
        Some(path) => SchemaError::TriggerLoad { path, source: Box::new(cause) },
        None => cause,
    })
}

/// The file or folder a preparation error names.
fn failing_path(error: &SchemaError) -> Option<PathBuf> {
    match error {
        SchemaError::TriggerLoad { path, .. } | SchemaError::Io { path, .. } => Some(path.clone()),
        SchemaError::TriggerCaseFoldCollision { root, .. } => Some(root.clone()),
        _ => None,
    }
}

/// Logs a failed preparation at `warn` when it is new and `debug` when it
/// repeats the recorded failure.
fn log_failure(key: &FailureKey, failure: &FailureRecord, has_last_good: bool, is_new: bool) {
    let scope = key.1.display().to_string();
    let trigger_file = display_path(failure.path.as_deref());
    let error = &failure.error;
    match (has_last_good, is_new) {
        (true, true) => tracing::warn!(
            %scope, %trigger_file, %error, has_last_good,
            "retaining last-good trigger registry"
        ),
        (true, false) => tracing::debug!(
            %scope, %trigger_file, %error, has_last_good,
            "retaining last-good trigger registry"
        ),
        (false, true) => tracing::warn!(%scope, %trigger_file, %error, "trigger registry unavailable"),
        (false, false) => tracing::debug!(%scope, %trigger_file, %error, "trigger registry unavailable"),
    }
}

/// A log field for an optional path: empty when there is none.
fn display_path(path: Option<&Path>) -> String {
    path.map(|path| path.display().to_string()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::collections::HashMap;

    use darkmatter::markdown::schemas::triggers::scan;

    use crate::log_capture::CapturedLogs;
    use crate::work_counters::{WorkCounts, work_counts};

    fn trigger(key: &str, payload: &str) -> String {
        format!("kind: schema-trigger\nmatch:\n  {key}: string(required)\n$schema: {payload}\n")
    }

    fn payload(property: &str) -> String {
        format!("$schema:\n  {property}: string(required)\n")
    }

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A repository with a package area (`area/`) holding a package
    /// (`area/pkg/`), a `SCHEMAS_DIR` folder and a home outside it, and a
    /// trigger in each of the five roots.
    struct Fixture {
        _dir: tempfile::TempDir,
        repo: PathBuf,
        home: PathBuf,
        schemas_dir: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
            let repo = root.join("repo");
            std::fs::create_dir_all(repo.join(".git")).unwrap();
            let home = root.join("home");
            let schemas_dir = root.join("env-schemas");
            write(&repo.join("schemas/tree.trigger.yaml"), &trigger("title", "tree-payload.yaml"));
            write(&repo.join("schemas/tree-payload.yaml"), &payload("tree_model"));
            write(&repo.join("schemas/notes.yaml"), "just: data\n");
            write(&repo.join("area/schemas/area.trigger.yaml"), &trigger("area", "area-payload.yaml"));
            write(&repo.join("area/schemas/area-payload.yaml"), &payload("area_model"));
            write(&repo.join("area/pkg/schemas/pkg.trigger.yaml"), &trigger("pkg", "pkg-payload.yaml"));
            write(&repo.join("area/pkg/schemas/pkg-payload.yaml"), &payload("pkg_model"));
            // Not a trigger, yet it shadows the tree root's trigger of this name.
            write(&repo.join("area/pkg/schemas/tree.trigger.yaml"), "ordinary: yaml\n");
            write(&schemas_dir.join("env.trigger.yaml"), &trigger("env", "env-payload.yaml"));
            write(&schemas_dir.join("env-payload.yaml"), &payload("env_model"));
            write(&home.join("schemas/home.trigger.yaml"), &trigger("home", "home-payload.yaml"));
            write(&home.join("schemas/home-payload.yaml"), &payload("home_model"));
            Self { _dir: dir, repo, home, schemas_dir }
        }

        /// The context of `doc`'s folder: the tree root is the repository,
        /// and a document under `area/` (and `area/pkg/`) belongs to that
        /// package area (and package).
        fn context(&self, doc: &Path) -> FileResolutionContext {
            let env = HashMap::from([(
                "SCHEMAS_DIR".to_string(),
                self.schemas_dir.to_string_lossy().into_owned(),
            )]);
            let mut context = FileResolutionContext::from_snapshot(
                doc.parent().unwrap(),
                Some(self.home.clone()),
                env,
            )
            .with_repository_root(&self.repo);
            if doc.starts_with(self.repo.join("area")) {
                context = context.with_package_area(self.repo.join("area"));
            }
            if doc.starts_with(self.repo.join("area/pkg")) {
                context = context.with_package_root(self.repo.join("area/pkg"));
            }
            context
        }

        fn tree_doc(&self) -> PathBuf {
            self.repo.join("doc.md")
        }

        fn area_doc(&self) -> PathBuf {
            self.repo.join("area/doc.md")
        }

        fn pkg_doc(&self) -> PathBuf {
            self.repo.join("area/pkg/doc.md")
        }

        /// Prepares `doc`'s registry when it is not fresh, then looks it up.
        fn lookup(&self, registries: &mut TriggerRegistries, doc: &Path) -> Result<Option<InstalledRegistry>, Arc<SchemaError>> {
            registries.prepare(doc, &self.repo, &self.context(doc));
            registries.lookup(doc)
        }

        fn ready(&self, registries: &mut TriggerRegistries, doc: &Path) -> InstalledRegistry {
            self.lookup(registries, doc).expect("roots readable").expect("registry loaded")
        }
    }

    fn names(registry: &TriggerRegistry) -> Vec<String> {
        registry
            .triggers
            .iter()
            .filter_map(|trigger| trigger.source.file_name().map(str::to_string))
            .collect()
    }

    #[test]
    fn per_document_registries_equal_direct_discovery() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let expected = [
            (fixture.tree_doc(), vec!["tree.trigger.yaml", "env.trigger.yaml", "home.trigger.yaml"]),
            (
                fixture.area_doc(),
                vec!["area.trigger.yaml", "tree.trigger.yaml", "env.trigger.yaml", "home.trigger.yaml"],
            ),
            // The package's ordinary `tree.trigger.yaml` shadows the tree
            // root's trigger of that name.
            (
                fixture.pkg_doc(),
                vec!["pkg.trigger.yaml", "area.trigger.yaml", "env.trigger.yaml", "home.trigger.yaml"],
            ),
        ];
        for (doc, triggers) in expected {
            let installed = fixture.ready(&mut registries, &doc);
            let direct = scan(&fixture.context(&doc).for_source(&doc)).unwrap();
            assert_eq!(installed.registry, direct, "{}", doc.display());
            assert_eq!(names(&installed.registry), triggers, "{}", doc.display());
        }
    }

    #[test]
    fn repeated_ready_lookups_do_no_io() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let docs = [fixture.tree_doc(), fixture.area_doc(), fixture.pkg_doc()];
        let first: Vec<u64> = docs.iter().map(|doc| fixture.ready(&mut registries, doc).version).collect();

        let before = work_counts();
        for _ in 0..3 {
            for (doc, version) in docs.iter().zip(&first) {
                assert_eq!(fixture.ready(&mut registries, doc).version, *version);
            }
        }
        assert_eq!(work_counts().since(before), WorkCounts::default());
    }

    #[test]
    fn a_lookup_never_prepares() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let before = work_counts();
        assert!(registries.lookup(&doc).unwrap().is_none(), "never prepared");
        assert!(!registries.is_fresh(&doc, &fixture.context(&doc)));
        assert_eq!(work_counts().since(before), WorkCounts::default());

        let installed = fixture.ready(&mut registries, &doc);
        registries.invalidate_all();
        let before = work_counts();
        // A stale scope serves its installed registry until a preparation
        // installs, and looking it up reads nothing.
        assert_eq!(registries.lookup(&doc).unwrap().unwrap().version, installed.version);
        assert!(!registries.is_fresh(&doc, &fixture.context(&doc)));
        assert_eq!(work_counts().since(before), WorkCounts::default());
    }

    #[test]
    fn a_preparation_runs_apart_from_the_cache_and_installs_in_one_step() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let context = fixture.context(&doc);
        let before = work_counts();
        let preparation = registries.begin(&doc, &fixture.repo, &context).expect("not fresh");
        assert_eq!(work_counts().since(before), WorkCounts::default(), "beginning reads nothing");

        // The cache stays usable while the preparation runs.
        let prepared = preparation.run();
        assert!(registries.lookup(&doc).unwrap().is_none(), "nothing installed before `install`");
        assert!(registries.install(prepared));
        let installed = registries.lookup(&doc).unwrap().unwrap();
        assert_eq!(installed.registry, scan(&context.for_source(&doc)).unwrap());
        assert!(registries.begin(&doc, &fixture.repo, &context).is_none(), "fresh");
    }

    #[test]
    fn an_older_preparation_never_replaces_a_newer_one() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let context = fixture.context(&doc);
        let older = registries.begin(&doc, &fixture.repo, &context).unwrap().run();
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        write(&envelope, &trigger("headline", "tree-payload.yaml"));
        registries.invalidate_paths(std::slice::from_ref(&envelope));
        let newer = registries.begin(&doc, &fixture.repo, &context).unwrap().run();

        assert!(registries.install(newer));
        let current = registries.lookup(&doc).unwrap().unwrap();
        registries.install(older);
        assert_eq!(registries.lookup(&doc).unwrap().unwrap().version, current.version, "older result dropped");
        assert_eq!(current.registry, scan(&context.for_source(&doc)).unwrap());
    }

    #[test]
    fn a_change_during_a_preparation_leaves_its_result_stale() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let context = fixture.context(&doc);
        let preparation = registries.begin(&doc, &fixture.repo, &context).unwrap();
        let prepared = preparation.run();
        // The envelope changes after the preparation read it.
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        write(&envelope, &trigger("headline", "tree-payload.yaml"));
        registries.invalidate_paths(std::slice::from_ref(&envelope));

        assert!(!registries.install(prepared), "installed, but stale");
        assert!(registries.lookup(&doc).unwrap().is_some());
        let reloaded = fixture.ready(&mut registries, &doc);
        assert_eq!(reloaded.registry, scan(&context.for_source(&doc)).unwrap(), "the next preparation reads the change");

        // An unrelated change leaves a running preparation's result fresh.
        let other = fixture.tree_doc().with_file_name("other.md");
        let preparation = registries.begin(&other, &fixture.repo, &fixture.context(&other)).unwrap();
        let prepared = preparation.run();
        registries.invalidate_paths(&[fixture.repo.join("unrelated/notes.txt")]);
        assert!(registries.install(prepared));
    }

    #[test]
    fn root_contents_are_shared_across_document_registries() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        fixture.ready(&mut registries, &fixture.area_doc());
        let before = work_counts();
        // A second document in the same folder shares the cached root
        // contents; only its own scan (payload resolution) runs.
        fixture.ready(&mut registries, &fixture.repo.join("area/other.md"));
        let work = work_counts().since(before);
        assert_eq!((work.registry_scans, work.enumerations, work.reads), (1, 0, 0), "{work:?}");
    }

    #[test]
    fn watched_changes_to_an_envelope_an_external_payload_and_a_new_root_file_update_the_registry() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        let outside = fixture.repo.join("shared/outside-payload.yaml");
        write(&outside, &payload("outside_model"));
        write(&envelope, &trigger("title", "../shared/outside-payload.yaml"));
        let initial = fixture.ready(&mut registries, &doc);
        assert!(initial.registry.dependencies().any(|path| path.ends_with("shared/outside-payload.yaml")));

        // The envelope.
        write(&envelope, &trigger("headline", "../shared/outside-payload.yaml"));
        registries.invalidate_paths(std::slice::from_ref(&envelope));
        let edited = fixture.ready(&mut registries, &doc);
        assert_ne!(edited.version, initial.version);
        assert_eq!(edited.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());

        // A payload outside every root.
        write(&outside, &payload("other_model"));
        registries.invalidate_paths(std::slice::from_ref(&outside));
        let repaid = fixture.ready(&mut registries, &doc);
        assert_ne!(repaid.version, edited.version);
        assert_eq!(repaid.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());

        // A new file in a root.
        let created = fixture.repo.join("schemas/added.trigger.yaml");
        write(&created, &trigger("added", "tree-payload.yaml"));
        registries.invalidate_paths(std::slice::from_ref(&created));
        let added = fixture.ready(&mut registries, &doc);
        assert_eq!(names(&added.registry)[0], "added.trigger.yaml");
        assert_eq!(added.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
    }

    #[test]
    fn an_event_for_an_unchanged_file_does_not_rescan() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let initial = fixture.ready(&mut registries, &doc);
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        let payload_file = fixture.repo.join("schemas/tree-payload.yaml");
        // Rewritten with the same bytes: a save that changed nothing.
        write(&envelope, &trigger("title", "tree-payload.yaml"));
        write(&payload_file, &payload("tree_model"));

        let before = work_counts();
        registries.invalidate_paths(&[envelope, payload_file]);
        let after = fixture.ready(&mut registries, &doc);
        let work = work_counts().since(before);
        assert_eq!(after.version, initial.version);
        assert_eq!((work.registry_scans, work.enumerations), (0, 0), "{work:?}");
    }

    #[test]
    fn deleting_a_trigger_removes_it_and_creating_a_missing_payload_recovers() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        fixture.ready(&mut registries, &doc);

        std::fs::remove_file(&envelope).unwrap();
        registries.invalidate_paths(std::slice::from_ref(&envelope));
        assert_eq!(
            names(&fixture.ready(&mut registries, &doc).registry),
            ["env.trigger.yaml", "home.trigger.yaml"]
        );

        // A trigger whose payload does not exist yet fails cold: no registry
        // is claimed for a fresh document.
        let missing = fixture.repo.join("payloads/late.yaml");
        write(&envelope, &trigger("title", "../payloads/late.yaml"));
        registries.invalidate_paths(std::slice::from_ref(&envelope));
        let fresh_doc = fixture.repo.join("fresh.md");
        assert!(fixture.lookup(&mut registries, &fresh_doc).unwrap().is_none());

        write(&missing, &payload("late_model"));
        registries.invalidate_paths(std::slice::from_ref(&missing));
        let recovered = fixture.ready(&mut registries, &fresh_doc);
        assert_eq!(names(&recovered.registry)[0], "tree.trigger.yaml");
        assert!(registries.take_transitions().iter().any(|transition| transition.error.is_none()));
    }

    #[test]
    fn external_home_and_environment_roots_recover() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        fixture.ready(&mut registries, &doc);
        let env_envelope = fixture.schemas_dir.join("env.trigger.yaml");
        let home_envelope = fixture.home.join("schemas/home.trigger.yaml");
        let mut previous = fixture.ready(&mut registries, &doc).version;
        for (envelope, key) in [(&env_envelope, "env"), (&home_envelope, "home")] {
            write(envelope, "kind: schema-trigger\nmatch: {}\n");
            registries.invalidate_paths(std::slice::from_ref(envelope));
            assert_eq!(fixture.ready(&mut registries, &doc).version, previous, "last-good retained");
            assert!(registries.load_error_for(envelope).is_some(), "{}", envelope.display());

            write(envelope, &trigger(key, &format!("{key}-payload.yaml")));
            registries.invalidate_paths(std::slice::from_ref(envelope));
            let recovered = fixture.ready(&mut registries, &doc);
            assert_ne!(recovered.version, previous);
            assert_eq!(recovered.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
            assert!(registries.load_error_for(envelope).is_none());
            previous = recovered.version;
        }
    }

    #[test]
    fn a_non_yaml_example_change_reloads_its_trigger() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let example = fixture.repo.join("examples/today.json");
        let example_text = |returns: &str| {
            format!("{{\"kind\": \"example\", \"invocation\": \"{{{{ ctx.today }}}}\", \"returns\": \"{returns}\", \"description\": \"d\"}}\n")
        };
        write(&example, &example_text("2024-12-25"));
        write(
            &fixture.repo.join("schemas/tree-payload.yaml"),
            "$schema:\n  today: \"date(example(../examples/today.json))\"\n",
        );
        let initial = fixture.ready(&mut registries, &doc);
        assert!(
            initial.registry.dependencies().any(|path| path.ends_with("examples/today.json")),
            "the example is a payload dependency"
        );

        write(&example, &example_text("2025-01-01"));
        registries.invalidate_paths(std::slice::from_ref(&example));
        let reloaded = fixture.ready(&mut registries, &doc);
        assert_ne!(reloaded.version, initial.version);
        assert_eq!(reloaded.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
    }

    #[test]
    fn last_good_is_retained_only_where_the_malformed_file_is_unshadowed() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let tree = fixture.ready(&mut registries, &fixture.tree_doc());
        let pkg = fixture.ready(&mut registries, &fixture.pkg_doc());
        let envelope = fixture.repo.join("schemas/tree.trigger.yaml");
        write(&envelope, "kind: schema-trigger\nmatch: {}\n");
        registries.invalidate_paths(std::slice::from_ref(&envelope));

        // The tree document serves its last-good registry and reports the
        // failure; the package document, where the file is shadowed, loads.
        let retained = fixture.ready(&mut registries, &fixture.tree_doc());
        assert_eq!(retained.version, tree.version);
        let skipped = scan(&fixture.context(&fixture.tree_doc()).for_source(fixture.tree_doc())).unwrap();
        assert_eq!(skipped.findings.len(), 1, "discovery skips the malformed file");
        let reloaded = fixture.ready(&mut registries, &fixture.pkg_doc());
        assert_ne!(reloaded.version, pkg.version);
        assert_eq!(
            reloaded.registry,
            scan(&fixture.context(&fixture.pkg_doc()).for_source(fixture.pkg_doc())).unwrap()
        );
        assert_eq!(registries.summary().failed_scopes, 1);
        assert!(registries.load_error_for(&envelope).is_some());
    }

    #[test]
    fn a_root_io_error_fails_closed_and_recovers() {
        let fixture = Fixture::new();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.area_doc();
        fixture.ready(&mut registries, &doc);
        // Text that is not UTF-8 cannot be read as a root file: an I/O
        // failure, never an unclassified file.
        let unreadable = fixture.repo.join("area/schemas/binary.yaml");
        std::fs::write(&unreadable, [0xff, 0xfe, 0x00]).unwrap();
        registries.invalidate_paths(std::slice::from_ref(&unreadable));
        let failed = fixture.lookup(&mut registries, &doc);
        assert!(
            matches!(failed.as_ref().map_err(|error| &**error), Err(SchemaError::Io { path, .. }) if *path == unreadable),
            "no last-good registry and no less-local root: {:?}",
            failed.map(|registry| registry.map(|installed| names(&installed.registry)))
        );
        // An unrelated healthy scope still loads.
        assert!(fixture.lookup(&mut registries, &fixture.tree_doc()).unwrap().is_some());

        std::fs::remove_file(&unreadable).unwrap();
        registries.invalidate_paths(std::slice::from_ref(&unreadable));
        let recovered = fixture.ready(&mut registries, &doc);
        assert_eq!(recovered.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
    }

    #[test]
    fn scoped_contexts_keep_distinct_payload_and_pattern_resolution() {
        let fixture = Fixture::new();
        // One `SCHEMAS_DIR` trigger whose bare payload name each package
        // supplies itself.
        write(&fixture.schemas_dir.join("env.trigger.yaml"), &trigger("env", "scoped-payload.yaml"));
        let pkg_b = fixture.repo.join("area/pkg-b");
        write(&fixture.repo.join("area/pkg/schemas/scoped-payload.yaml"), &payload("a_model"));
        write(&pkg_b.join("schemas/scoped-payload.yaml"), &payload("b_model"));
        write(&pkg_b.join("schemas/pkg.trigger.yaml"), &trigger("pkg", "scoped-payload.yaml"));
        let mut registries = TriggerRegistries::default();
        let doc_a = fixture.pkg_doc();
        let doc_b = pkg_b.join("doc.md");
        let context_b = fixture.context(&doc_b).with_package_root(&pkg_b);

        let a = fixture.ready(&mut registries, &doc_a).registry;
        registries.prepare(&doc_b, &fixture.repo, &context_b);
        let b = registries.lookup(&doc_b).unwrap().unwrap().registry;
        assert_eq!(b, scan(&context_b.for_source(&doc_b)).unwrap());
        let env_payload = |registry: &TriggerRegistry| {
            registry
                .dependencies()
                .filter(|path| path.ends_with("scoped-payload.yaml"))
                .map(Path::to_path_buf)
                .collect::<Vec<_>>()
        };
        assert!(env_payload(&a).iter().all(|path| path.starts_with(fixture.repo.join("area/pkg/"))), "{:?}", env_payload(&a));
        assert!(env_payload(&b).iter().all(|path| path.starts_with(&pkg_b)), "{:?}", env_payload(&b));
        assert!(!env_payload(&a).is_empty() && !env_payload(&b).is_empty());
        let pattern_cwd = |registry: &TriggerRegistry| {
            registry
                .triggers
                .iter()
                .find(|trigger| trigger.source.file_name() == Some("pkg.trigger.yaml"))
                .map(|trigger| trigger.pattern_cwd.clone())
        };
        assert_eq!(pattern_cwd(&a), Some(fixture.repo.join("area/pkg")));
        assert_eq!(pattern_cwd(&b), Some(pkg_b));
    }

    #[test]
    fn repeated_identical_failures_warn_once_and_recovery_logs_info() {
        let fixture = Fixture::new();
        let doc = fixture.tree_doc();
        // Joined per component: the log shows the path, and a `/` inside one
        // joined segment displays as written on Windows.
        let envelope = fixture.repo.join("schemas").join("tree.trigger.yaml");
        let mut registries = TriggerRegistries::default();
        let ((), logs) = CapturedLogs::scoped(|| {
            fixture.ready(&mut registries, &doc);
            write(&envelope, "kind: schema-trigger\nmatch: {}\n");
            registries.invalidate_paths(std::slice::from_ref(&envelope));
            fixture.ready(&mut registries, &doc);
            assert_eq!(registries.take_transitions().len(), 1);
            // The same failure again: neither re-published nor re-warned.
            for _ in 0..3 {
                registries.invalidate_all();
                fixture.ready(&mut registries, &doc);
            }
            assert!(registries.take_transitions().is_empty());
            // A different error.
            write(&envelope, "kind: schema-trigger\nmatch: []\n");
            registries.invalidate_paths(std::slice::from_ref(&envelope));
            fixture.ready(&mut registries, &doc);
            // Recovery.
            write(&envelope, &trigger("title", "tree-payload.yaml"));
            registries.invalidate_paths(std::slice::from_ref(&envelope));
            fixture.ready(&mut registries, &doc);
        });
        let warnings = logs.matching("WARN");
        assert_eq!(warnings.len(), 2, "{:#?}", logs.lines());
        for warning in &warnings {
            assert!(warning.contains("retaining last-good trigger registry"), "{warning}");
            assert!(warning.contains("has_last_good=true"), "{warning}");
            assert!(warning.contains(&format!("scope={}", fixture.repo.display())), "{warning}");
            assert!(warning.contains(&format!("trigger_file={}", envelope.display())), "{warning}");
            assert!(warning.contains("error="), "{warning}");
        }
        assert_ne!(warnings[0], warnings[1], "the second warning carries the different error");
        let repeats = logs.matching("DEBUG");
        assert_eq!(
            repeats.iter().filter(|line| line.contains("retaining last-good trigger registry")).count(),
            3,
            "{repeats:#?}"
        );
        let recovered = logs.matching("trigger registry recovered");
        assert_eq!(recovered.len(), 1, "{:#?}", logs.lines());
        assert!(recovered[0].contains("INFO"), "{}", recovered[0]);
        assert!(recovered[0].contains(&format!("trigger_file={}", envelope.display())), "{}", recovered[0]);
    }

    #[test]
    fn a_cold_failure_logs_unavailable_once() {
        let fixture = Fixture::new();
        let doc = fixture.tree_doc();
        write(&fixture.repo.join("schemas/tree.trigger.yaml"), "kind: schema-trigger\nmatch: {}\n");
        let mut registries = TriggerRegistries::default();
        let ((), logs) = CapturedLogs::scoped(|| {
            for _ in 0..3 {
                assert!(fixture.lookup(&mut registries, &doc).unwrap().is_none());
                registries.invalidate_all();
            }
        });
        let unavailable = logs.matching("trigger registry unavailable");
        assert_eq!(unavailable.len(), 3, "{:#?}", logs.lines());
        assert!(unavailable[0].contains("WARN"), "{}", unavailable[0]);
        assert!(unavailable[1..].iter().all(|line| line.contains("DEBUG")), "{unavailable:#?}");
        assert!(logs.matching("retaining last-good").is_empty(), "a cold failure claims no last-good registry");
    }

    #[test]
    fn root_files_are_classified_and_counted_once_per_folder() {
        let fixture = Fixture::new();
        write(&fixture.repo.join("schemas/broken.trigger.yaml"), "kind: schema-trigger\nmatch: {}\n");
        let mut registries = TriggerRegistries::default();
        for doc in [fixture.tree_doc(), fixture.area_doc(), fixture.pkg_doc()] {
            let _ = fixture.lookup(&mut registries, &doc);
        }
        // Roots: tree, SCHEMAS_DIR, home (tree role); area and package
        // (scoped). The package's ordinary `tree.trigger.yaml` and the tree
        // root's `notes.yaml` are unclassified. Shadowed files that no scan
        // read are not counted.
        assert_eq!(
            registries.summary(),
            RegistrySummary {
                roots: 5,
                tree_schemas: 3,
                scoped_schemas: 2,
                trigger_schemas: 5,
                failed_triggers: 1,
                unclassified: 2,
                failed_scopes: 3,
            }
        );
    }

    #[test]
    fn a_payload_failure_counts_its_trigger_as_failed_once_across_contexts() {
        let fixture = Fixture::new();
        // One `SCHEMAS_DIR` trigger whose bare payload name the first package
        // supplies and the second does not.
        write(&fixture.schemas_dir.join("env.trigger.yaml"), &trigger("env", "scoped-payload.yaml"));
        write(&fixture.repo.join("area/pkg/schemas/scoped-payload.yaml"), &payload("a_model"));
        let pkg_b = fixture.repo.join("area/pkg-b");
        write(&pkg_b.join("schemas/notes.yaml"), "just: data\n");
        let mut registries = TriggerRegistries::default();
        fixture.ready(&mut registries, &fixture.pkg_doc());
        let loaded = registries.summary();
        assert_eq!(loaded.failed_scopes, 0, "{loaded:?}");

        for doc in [pkg_b.join("doc.md"), pkg_b.join("other.md")] {
            let context = fixture.context(&doc).with_package_root(&pkg_b);
            registries.prepare(&doc, &fixture.repo, &context);
            assert!(registries.lookup(&doc).unwrap().is_none());
        }
        let failed = registries.summary();
        assert_eq!(
            (failed.trigger_schemas, failed.failed_triggers, failed.failed_scopes),
            (loaded.trigger_schemas - 1, loaded.failed_triggers + 1, 2),
            "loaded: {loaded:?}, failed: {failed:?}"
        );
    }

    #[test]
    fn a_shadowed_trigger_counts_as_failed_only_once_a_preparation_reaches_its_payload() {
        let fixture = Fixture::new();
        std::fs::remove_file(fixture.repo.join("schemas/tree-payload.yaml")).unwrap();
        let mut registries = TriggerRegistries::default();
        // The package's ordinary `tree.trigger.yaml` shadows the tree root's
        // trigger, so the package document never resolves its payload.
        fixture.ready(&mut registries, &fixture.pkg_doc());
        let shadowed = registries.summary();
        assert_eq!((shadowed.failed_triggers, shadowed.failed_scopes), (0, 0), "{shadowed:?}");

        assert!(fixture.lookup(&mut registries, &fixture.tree_doc()).unwrap().is_none());
        let reached = registries.summary();
        assert_eq!(
            (reached.trigger_schemas, reached.failed_triggers, reached.failed_scopes),
            (shadowed.trigger_schemas - 1, 1, 1),
            "shadowed: {shadowed:?}, reached: {reached:?}"
        );
    }

    // ── Searched payload candidates ─────────────────────────────────────────

    const SHARED_FIELDS: &str = "$schema:\n  Fields: string\n";
    const NEARER_FIELDS: &str = "$schema:\n  Fields: number\n";

    /// The tree payload imports `Fields@shared/types.yaml`, which resolves in
    /// the authoring folder first (`schemas/shared/types.yaml`, absent with
    /// its folder) and then the repository root, where it is found.
    fn shared_import_fixture() -> Fixture {
        let fixture = Fixture::new();
        write(&fixture.repo.join("schemas/tree-payload.yaml"), "$schema:\n  tree_model: Fields@shared/types.yaml\n");
        write(&fixture.repo.join("shared/types.yaml"), SHARED_FIELDS);
        fixture
    }

    fn selects(installed: &InstalledRegistry, file: &Path) -> bool {
        installed.registry.dependencies().any(|dependency| dependency == file)
    }

    #[test]
    fn a_nearer_candidate_created_in_an_absent_folder_invalidates_a_ready_registry() {
        let fixture = shared_import_fixture();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let shared = fixture.repo.join("shared/types.yaml");
        let nearer_folder = fixture.repo.join("schemas/shared");
        let nearer = nearer_folder.join("types.yaml");
        let initial = fixture.ready(&mut registries, &doc);
        assert!(selects(&initial, &shared));
        assert!(
            registries.watch_folders().contains(&WatchedFolder {
                folder: nearer_folder.clone(),
                absent_base: Some(fixture.repo.join("schemas")),
            }),
            "the absent candidate folder is watched from its nearest existing ancestor: {:?}",
            registries.watch_folders()
        );

        write(&nearer, NEARER_FIELDS);
        registries.invalidate_paths(std::slice::from_ref(&nearer));
        assert!(!registries.is_fresh(&doc, &fixture.context(&doc)), "the created candidate affects the scope");
        let shadowed = fixture.ready(&mut registries, &doc);
        assert_ne!(shadowed.version, initial.version);
        assert!(selects(&shadowed, &nearer) && !selects(&shadowed, &shared));
        assert_eq!(shadowed.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
        assert!(
            registries.watch_folders().contains(&WatchedFolder::present(nearer_folder.clone())),
            "coverage advances to the created folder: {:?}",
            registries.watch_folders()
        );

        // Removal falls back to the repository-root file.
        std::fs::remove_file(&nearer).unwrap();
        registries.invalidate_paths(std::slice::from_ref(&nearer));
        let fallback = fixture.ready(&mut registries, &doc);
        assert!(selects(&fallback, &shared) && !selects(&fallback, &nearer));
        assert_eq!(fallback.registry, scan(&fixture.context(&doc).for_source(&doc)).unwrap());
    }

    #[test]
    fn a_directory_event_above_a_candidate_invalidates_a_ready_registry() {
        // `schemas/shared/` exists but holds no dependency, so no root or
        // dependency-folder rule covers an event inside it; only the
        // searched candidate `schemas/shared/deep/types.yaml` does.
        let fixture = Fixture::new();
        write(&fixture.repo.join("schemas/tree-payload.yaml"), "$schema:\n  tree_model: Fields@shared/deep/types.yaml\n");
        write(&fixture.repo.join("shared/deep/types.yaml"), SHARED_FIELDS);
        write(&fixture.repo.join("schemas/shared/readme.md"), "unrelated\n");
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let nearer_folder = fixture.repo.join("schemas/shared/deep");
        fixture.ready(&mut registries, &doc);
        assert!(registries.watch_folders().contains(&WatchedFolder {
            folder: nearer_folder.clone(),
            absent_base: Some(fixture.repo.join("schemas/shared")),
        }));

        // The client reports only the directory of a folder-and-file creation.
        write(&nearer_folder.join("types.yaml"), NEARER_FIELDS);
        registries.invalidate_paths(std::slice::from_ref(&nearer_folder));
        let shadowed = fixture.ready(&mut registries, &doc);
        assert!(selects(&shadowed, &nearer_folder.join("types.yaml")));
        assert!(registries.watch_folders().contains(&WatchedFolder::present(nearer_folder)));
    }

    #[test]
    fn unrelated_events_keep_a_ready_registry_fresh_and_do_no_work() {
        let fixture = shared_import_fixture();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let initial = fixture.ready(&mut registries, &doc);
        let unrelated = [
            fixture.repo.join("elsewhere/shared/types.yaml"),
            fixture.repo.join("schemas-old/shared"),
            fixture.repo.join("notes/todo.yaml"),
        ];
        for path in &unrelated {
            write(&path.join("file.yaml"), "a: b\n");
        }

        let before = work_counts();
        registries.invalidate_paths(&unrelated);
        assert!(registries.is_fresh(&doc, &fixture.context(&doc)));
        assert_eq!(fixture.ready(&mut registries, &doc).version, initial.version);
        assert_eq!(work_counts().since(before), WorkCounts::default());
    }

    #[test]
    fn a_candidate_created_during_a_preparation_leaves_its_result_stale() {
        let fixture = shared_import_fixture();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let context = fixture.context(&doc);
        let nearer = fixture.repo.join("schemas/shared/types.yaml");
        fixture.ready(&mut registries, &doc);
        registries.invalidate_all();
        let prepared = registries.begin(&doc, &fixture.repo, &context).unwrap().run();
        // The candidate appears after the preparation searched for it.
        write(&nearer, NEARER_FIELDS);
        registries.invalidate_paths(std::slice::from_ref(&nearer));

        assert!(!registries.install(prepared), "installed, but stale");
        assert!(selects(&fixture.ready(&mut registries, &doc), &nearer));
    }

    #[test]
    fn a_malformed_nearer_candidate_keeps_last_good_and_its_watch_until_repaired() {
        let fixture = shared_import_fixture();
        let mut registries = TriggerRegistries::default();
        let doc = fixture.tree_doc();
        let nearer_folder = fixture.repo.join("schemas/shared");
        let nearer = nearer_folder.join("types.yaml");
        let initial = fixture.ready(&mut registries, &doc);

        write(&nearer, "$schema:\n  Fields: [unclosed\n");
        registries.invalidate_paths(std::slice::from_ref(&nearer));
        assert_eq!(
            scan(&fixture.context(&doc).for_source(&doc)).unwrap().findings.len(),
            1,
            "the nearer file fails to load, so discovery skips its trigger"
        );
        assert_eq!(fixture.ready(&mut registries, &doc).version, initial.version, "last-good retained");
        assert_eq!(registries.summary().failed_scopes, 1);
        // The failed resolution reports no candidates, yet the folder holding
        // the failing file stays watched, now as present.
        assert!(
            registries.watch_folders().contains(&WatchedFolder::present(nearer_folder.clone())),
            "{:?}",
            registries.watch_folders()
        );
        // A second failure keeps the watch too.
        registries.invalidate_paths(std::slice::from_ref(&nearer));
        fixture.ready(&mut registries, &doc);
        assert!(registries.watch_folders().contains(&WatchedFolder::present(nearer_folder.clone())));

        write(&nearer, NEARER_FIELDS);
        registries.invalidate_paths(std::slice::from_ref(&nearer));
        let repaired = fixture.ready(&mut registries, &doc);
        assert_ne!(repaired.version, initial.version);
        assert!(selects(&repaired, &nearer));
        assert_eq!(registries.summary().failed_scopes, 0);
    }
}

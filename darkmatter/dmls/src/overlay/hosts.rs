//! Registered hosts per configuration, and the one host view each document
//! is analyzed under.
//!
//! [`HostRegistrations`] holds the registered hosts of every configuration
//! the open documents' contexts project (a home and a repository), loaded
//! passively and prepared off the dispatch thread like the trigger registries
//! ([`super::registry`]). [`HostView`] is a document's
//! [`select`](darkmatter::config::registration::select) result with
//! everything its providers read from it: the lifecycle host, the host
//! global catalogs, the host function sets, and the warnings selection
//! produced. It is computed once per schema preparation and carried on the
//! overlay, so no provider rescans a schema root or a registration.
//!
//! User-facing guide: `darkmatter/docs/topics/host-registration.md`
//! ("DMLS").

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::{FileReference, FileResolutionContext};
use darkmatter::config::load::{CONFIG_MIGRATION_CODE, CONFIG_MISSING_CODE, ConfigRevision};
use darkmatter::config::migration_receipt::content_hash;
use darkmatter::config::registration::{
    HostClaim, HostSelection, LegacyHost, LoadedRegistrations, RegisteredHosts, RegistrationDiagnostic,
    manifest_member_candidates, select,
};
use darkmatter::lifecycle::host_profile::lifecycle_artifact_path;
use darkmatter::lifecycle::md_host;
use darkmatter::lifecycle::registry::KeyResolution;
use darkmatter::markdown::compose::expression::HostFunctions;
use darkmatter::markdown::compose::expression::host::facts_artifact_path;
use darkmatter::markdown::schemas::catalog::{CatalogError, GlobalCatalog, HostFunctionCatalogError};
use serde_json::Value;

use crate::overlay::event_views::LifecycleHost;
use crate::overlay::registry::normalized;
use crate::work_counters;
use crate::workspace::watch::WatchedFolder;

/// The name of the configuration file in a home and a repository root.
const CONFIG_FILE_NAME: &str = ".darkmatter.yaml";

/// The configuration a context projects: the registrations of two contexts
/// with equal keys are the same.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
struct HostsKey {
    home: Option<PathBuf>,
    repository: Option<PathBuf>,
    /// The tree root, which registered triggers read `$path` patterns from.
    base: PathBuf,
    /// The context's environment, sorted, which a `register` reference may
    /// name.
    env: Vec<(String, String)>,
}

impl HostsKey {
    fn of(context: &FileResolutionContext) -> Self {
        let mut env: Vec<(String, String)> =
            context.env().iter().map(|(name, value)| (name.clone(), value.clone())).collect();
        env.sort();
        Self {
            home: context.home_dir().map(Path::to_path_buf),
            repository: context.repository_root().map(Path::to_path_buf),
            base: context.base_dir().to_path_buf(),
            env,
        }
    }
}

/// One configuration's installed registrations.
#[derive(Clone, Debug)]
pub struct InstalledHosts {
    /// The registrations, grouped for selection. Empty when nothing is
    /// registered or the configuration could not be loaded.
    pub hosts: Arc<RegisteredHosts>,
    /// Distinct for every install whose result differs from the one it
    /// replaced; a reload that finds the same profiles and warnings keeps
    /// the version, so no document's schema reassembles.
    pub version: u64,
    /// Why the configuration could not be loaded, when it could not. The
    /// configuration's own diagnostics report it; registration then
    /// contributes nothing.
    pub load_error: Option<Arc<str>>,
}

impl InstalledHosts {
    /// The configuration-level warnings: every refused registration and
    /// every same-host conflict ([`RegisteredHosts::diagnostics`]).
    pub fn diagnostics(&self) -> Vec<RegistrationDiagnostic> {
        self.hosts.diagnostics()
    }

    /// Whether `other` loaded the same profiles, warnings, and failure.
    fn same_result(&self, other: &Self) -> bool {
        let profiles = |installed: &Self| -> Vec<(PathBuf, String)> {
            installed
                .hosts
                .loaded()
                .profiles
                .iter()
                .map(|profile| (profile.manifest().path.clone(), profile.revision().to_string()))
                .collect()
        };
        profiles(self) == profiles(other)
            && self.hosts.loaded().diagnostics == other.hosts.loaded().diagnostics
            && self.load_error == other.load_error
    }
}

/// What one configuration's load read, for invalidation.
#[derive(Default, Clone, Debug)]
struct HostDependencies {
    /// Every file the load read or would have read, with the BLAKE3 hash of
    /// its bytes (`None` when absent or unreadable).
    files: BTreeMap<PathBuf, Option<String>>,
    /// Manifest and member folders, where a new entry can change the load
    /// (a missing member created, a manifest moved in).
    folders: BTreeSet<PathBuf>,
    /// Each absent watched folder's nearest existing ancestor.
    absent_bases: BTreeMap<PathBuf, PathBuf>,
    /// The repository's `HEAD` and the branch ref it names, with the BLAKE3
    /// hash of each (`None` when absent). The repository configuration's
    /// permissions are read as committed at `HEAD` (the commit rule), so a
    /// commit or checkout can change what the load accepts. Watched by path,
    /// never by folder: the Git directory changes on every Git command.
    heads: BTreeMap<PathBuf, Option<String>>,
}

impl HostDependencies {
    /// Records `path` with the hash of its bytes now, unless it is
    /// recorded already.
    fn record_file(&mut self, path: &Path) {
        if let std::collections::btree_map::Entry::Vacant(entry) = self.files.entry(normalized(path)) {
            entry.insert(work_counters::read(path).ok().map(|bytes| content_hash(&bytes)));
        }
    }

    fn record_folder_of(&mut self, path: &Path) {
        if let Some(folder) = normalized(path).parent() {
            self.folders.insert(folder.to_path_buf());
        }
    }

    fn record_heads(&mut self, repository: &Path) {
        for path in crate::slot_jobs::head_files(repository) {
            let hash = work_counters::read(&path).ok().map(|bytes| content_hash(&bytes));
            self.heads.insert(normalized(&path), hash);
        }
    }

    fn record_absences(&mut self) {
        let folders: BTreeSet<PathBuf> =
            self.files.keys().filter_map(|file| file.parent().map(Path::to_path_buf)).chain(self.folders.iter().cloned()).collect();
        for folder in folders {
            if folder.is_dir() {
                continue;
            }
            if let Some(base) = folder.ancestors().skip(1).find(|ancestor| ancestor.is_dir()) {
                self.absent_bases.insert(folder.clone(), base.to_path_buf());
            }
        }
    }

    /// Whether a change at `path` can change the load: a recorded file whose
    /// bytes now hash differently, a new entry in a manifest or member
    /// folder, or a directory above a recorded file.
    fn affected_by(&self, path: &Path) -> bool {
        if let Some(recorded) = self.files.get(path).or_else(|| self.heads.get(path)) {
            let current = work_counters::read(path).ok().map(|bytes| content_hash(&bytes));
            return current != *recorded;
        }
        path.parent().is_some_and(|folder| self.folders.contains(folder))
            || self.files.keys().any(|file| file.starts_with(path))
            || self.folders.iter().any(|folder| folder.starts_with(path))
    }

    /// Whether `path` is something this load read or searched, without
    /// comparing content: an in-flight preparation's result is stale when a
    /// change touched it.
    fn touches(&self, path: &Path) -> bool {
        self.files.contains_key(path)
            || self.heads.contains_key(path)
            || path.parent().is_some_and(|folder| self.folders.contains(folder))
            || self.files.keys().any(|file| file.starts_with(path))
    }

    fn watched_folders(&self) -> impl Iterator<Item = WatchedFolder> + '_ {
        self.files
            .keys()
            .filter_map(|file| file.parent())
            .chain(self.folders.iter().map(PathBuf::as_path))
            .map(|folder| WatchedFolder {
                folder: folder.to_path_buf(),
                absent_base: self.absent_bases.get(folder).cloned(),
            })
    }
}

/// One configuration's registrations.
struct Scope {
    installed: InstalledHosts,
    deps: HostDependencies,
    notices: Vec<ConfigNotice>,
    stale: bool,
    /// The [`HostPreparation`] sequence number that installed this scope.
    seq: u64,
}

/// A change recorded while a preparation was running.
#[derive(Clone)]
enum Change {
    Path(PathBuf),
    All,
}

/// One configuration's load, taken from the cache by
/// [`HostRegistrations::begin`] and run by [`HostPreparation::run`] without
/// the cache's lock.
pub struct HostPreparation {
    key: HostsKey,
    context: FileResolutionContext,
    epoch: u64,
    seq: u64,
}

/// A finished [`HostPreparation`], ready for [`HostRegistrations::install`].
pub struct PreparedHosts {
    key: HostsKey,
    epoch: u64,
    seq: u64,
    hosts: RegisteredHosts,
    load_error: Option<Arc<str>>,
    deps: HostDependencies,
    notices: Vec<ConfigNotice>,
}

/// A session-level notice of a passive configuration load: the missing user
/// file (`dm.config.missing`) or a legacy policy file the next `md` or
/// Claudine run would migrate (`dm.config.migration`). DMLS only reports
/// these; it never creates or migrates a file.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConfigNotice {
    /// `dm.config.missing` or `dm.config.migration`.
    pub code: &'static str,
    /// The file the notice is about.
    pub file: PathBuf,
    /// The library's message, naming the file and what creates or
    /// migrates it.
    pub message: String,
}

impl HostPreparation {
    /// Loads the configuration passively and its registrations, and hashes
    /// everything they read. Does all of a preparation's I/O, and touches no
    /// shared state: it never migrates, writes, prompts, or fetches.
    pub fn run(self) -> PreparedHosts {
        let mut deps = HostDependencies::default();
        let mut notices = Vec::new();
        let loaded = darkmatter::markdown::schemas::slots::session::load_passively(&self.context);
        if let Ok(snapshot) = &loaded {
            // The load hashed what it read; the committed revision's hash is
            // not the working file's, and `HEAD` stands for it instead.
            for dependency in snapshot.dependencies().iter().filter(|dependency| dependency.revision != ConfigRevision::RepositoryHead) {
                deps.files.entry(normalized(&dependency.path)).or_insert_with(|| dependency.content_hash.clone());
            }
        }
        for folder in [self.key.home.as_deref(), self.key.repository.as_deref()].into_iter().flatten() {
            deps.record_file(&folder.join(CONFIG_FILE_NAME));
        }
        if let Some(repository) = self.key.repository.as_deref() {
            deps.record_heads(repository);
        }
        let (hosts, load_error) = match loaded {
            Ok(snapshot) => {
                // Only an absent user file at a known home is reported as
                // missing; a repository without a configuration is normal.
                let missing = snapshot.user_config_missing();
                notices = snapshot
                    .diagnostics()
                    .iter()
                    .filter(|notice| match notice.code {
                        CONFIG_MISSING_CODE => missing == Some(notice.file.as_path()),
                        code => code == CONFIG_MIGRATION_CODE,
                    })
                    .map(|notice| ConfigNotice { code: notice.code, file: notice.file.clone(), message: notice.message.clone() })
                    .collect();
                (RegisteredHosts::load(&snapshot, &self.context), None)
            }
            Err(error) => {
                tracing::debug!(%error, "configuration not loaded; no host is registered for it");
                (RegisteredHosts::new(Arc::new(LoadedRegistrations::default())), Some(Arc::from(error.to_string())))
            }
        };
        for profile in &hosts.loaded().profiles {
            deps.record_folder_of(&profile.manifest().path);
            for dependency in profile.dependencies() {
                deps.files.insert(normalized(&dependency.path), Some(dependency.content_hash.clone()));
                deps.record_folder_of(&dependency.path);
            }
        }
        for diagnostic in &hosts.loaded().diagnostics {
            // A manifest that cannot be found names its configuration file;
            // the paths its entry would resolve to are where it may appear.
            for candidate in entry_candidates(&diagnostic.entry.reference, &diagnostic.entry.declared_in, &self.context) {
                deps.record_file(&candidate);
                deps.record_folder_of(&candidate);
                // A member that is missing or broken is where its repair or
                // creation will appear.
                for member in manifest_member_candidates(&candidate, &self.context) {
                    deps.record_file(&member);
                    deps.record_folder_of(&member);
                }
            }
            let declaring = diagnostic.file == diagnostic.entry.declared_in;
            deps.record_file(&diagnostic.file);
            if !declaring {
                deps.record_folder_of(&diagnostic.file);
            }
        }
        deps.record_absences();
        PreparedHosts { key: self.key, epoch: self.epoch, seq: self.seq, hosts, load_error, deps, notices }
    }
}

/// Every location `reference`, declared in `declared_in`, is probed at.
fn entry_candidates(reference: &str, declared_in: &Path, context: &FileResolutionContext) -> Vec<PathBuf> {
    let Ok(parsed) = FileReference::new(reference) else {
        return Vec::new();
    };
    parsed.resolve_detailed(&context.for_source(declared_in)).probed_paths().collect()
}

/// Every configuration's registered hosts.
#[derive(Default)]
pub struct HostRegistrations {
    scopes: HashMap<HostsKey, Scope>,
    next_version: u64,
    /// Bumped by every invalidation.
    epoch: u64,
    next_seq: u64,
    /// Running preparations: sequence number to the epoch they began at.
    in_flight: HashMap<u64, u64>,
    /// Invalidations since the oldest running preparation began.
    changes: Vec<(u64, Change)>,
}

impl HostRegistrations {
    /// The installed registrations of `context`'s configuration; `None`
    /// before its first preparation. A lookup only: a stale scope serves its
    /// registrations until its preparation installs.
    pub fn lookup(&self, context: &FileResolutionContext) -> Option<InstalledHosts> {
        self.scopes.get(&HostsKey::of(context)).map(|scope| scope.installed.clone())
    }

    /// Whether `context`'s configuration is installed and no change has
    /// marked it stale.
    pub fn is_fresh(&self, context: &FileResolutionContext) -> bool {
        self.scopes.get(&HostsKey::of(context)).is_some_and(|scope| !scope.stale)
    }

    /// Starts loading `context`'s configuration unless it is fresh. Reads
    /// nothing.
    pub fn begin(&mut self, context: &FileResolutionContext) -> Option<HostPreparation> {
        if self.is_fresh(context) {
            return None;
        }
        self.next_seq += 1;
        self.in_flight.insert(self.next_seq, self.epoch);
        Some(HostPreparation { key: HostsKey::of(context), context: context.clone(), epoch: self.epoch, seq: self.next_seq })
    }

    /// Installs a finished preparation. An older preparation never replaces
    /// a newer one, and a change recorded while it ran that touches what it
    /// read leaves the scope stale. A result equal to the installed one
    /// keeps its version.
    ///
    /// ## Returns
    ///
    /// Whether the configuration's registrations are fresh afterwards.
    pub fn install(&mut self, prepared: PreparedHosts) -> bool {
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
        let PreparedHosts { key, seq, hosts, load_error, deps, notices, .. } = prepared;
        if let Some(scope) = self.scopes.get(&key)
            && scope.seq > seq
        {
            return !scope.stale;
        }
        let mut installed = InstalledHosts { hosts: Arc::new(hosts), version: 0, load_error };
        match self.scopes.get(&key) {
            Some(previous) if previous.installed.same_result(&installed) => installed = previous.installed.clone(),
            _ => {
                self.next_version += 1;
                installed.version = self.next_version;
            }
        }
        let stale = changes.iter().any(|change| match change {
            Change::All => true,
            Change::Path(path) => deps.touches(path),
        });
        self.scopes.insert(key, Scope { installed, deps, notices, stale, seq });
        !stale
    }

    /// Prepares `context`'s configuration on the calling thread unless it is
    /// fresh, for a caller that owns the cache outright.
    pub fn prepare(&mut self, context: &FileResolutionContext) {
        if let Some(preparation) = self.begin(context) {
            let prepared = preparation.run();
            self.install(prepared);
        }
    }

    fn record_change(&mut self, change: Change) {
        self.epoch += 1;
        if !self.in_flight.is_empty() {
            self.changes.push((self.epoch, change));
        }
    }

    /// Marks stale every configuration a change at one of `paths` can
    /// affect. A recorded file is re-read and compared with its recorded
    /// hash first, so an unchanged file invalidates nothing.
    pub fn invalidate_paths(&mut self, paths: &[PathBuf]) {
        for path in paths {
            let path = normalized(path);
            self.record_change(Change::Path(path.clone()));
            for scope in self.scopes.values_mut() {
                scope.stale |= !scope.stale && scope.deps.affected_by(&path);
            }
        }
    }

    /// Marks every configuration stale.
    pub fn invalidate_all(&mut self) {
        self.record_change(Change::All);
        for scope in self.scopes.values_mut() {
            scope.stale = true;
        }
    }

    /// Every folder a configuration's load depends on: the configuration
    /// files' folders, each manifest's, member's, and import's folder, and
    /// the folders a refused entry's manifest was searched in, each absent
    /// one with the nearest existing ancestor its preparation observed. A
    /// lookup only.
    pub fn watch_folders(&self) -> BTreeSet<WatchedFolder> {
        self.scopes.values().flat_map(|scope| scope.deps.watched_folders()).collect()
    }

    /// Advances with every install whose result differs from the one it
    /// replaced (each new [`InstalledHosts::version`]).
    pub fn generation(&self) -> u64 {
        self.next_version
    }

    /// Every file a configuration's load read or searched for.
    pub fn dependencies(&self) -> BTreeSet<PathBuf> {
        self.scopes.values().flat_map(|scope| scope.deps.files.keys().cloned()).collect()
    }

    /// Each repository's `HEAD` and the branch ref it names, watched by
    /// path (see [`HostDependencies::heads`]).
    pub fn head_paths(&self) -> BTreeSet<PathBuf> {
        self.scopes.values().flat_map(|scope| scope.deps.heads.keys().cloned()).collect()
    }

    /// The session-level notices of every installed configuration, each
    /// distinct notice once.
    pub fn notices(&self) -> BTreeSet<ConfigNotice> {
        self.scopes.values().flat_map(|scope| scope.notices.iter().cloned()).collect()
    }

    /// The configuration-level warnings of every installed configuration,
    /// each distinct record once. Each is published on the file it names
    /// (the declaring configuration, the manifest, or the member), never on
    /// the documents the configuration serves.
    pub fn diagnostics(&self) -> Vec<RegistrationDiagnostic> {
        let mut all: Vec<RegistrationDiagnostic> = Vec::new();
        for scope in self.scopes.values() {
            for diagnostic in scope.installed.diagnostics() {
                if !all.contains(&diagnostic) {
                    all.push(diagnostic);
                }
            }
        }
        all
    }
}

/// The host one document is analyzed as, with what its providers read.
#[derive(Clone)]
pub struct HostView {
    selection: HostSelection,
    lifecycle: LifecycleHost,
    catalogs: Vec<GlobalCatalog>,
    functions: Vec<HostFunctions>,
    diagnostics: Vec<RegistrationDiagnostic>,
    unknown_events: Vec<UnknownHostEvent>,
}

/// A root key that is a host-only lifecycle event of a registered host that
/// did not identify the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnknownHostEvent {
    /// The root key.
    pub key: String,
    /// The registered host whose event it is.
    pub host: String,
}

impl std::fmt::Debug for HostView {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("HostView")
            .field("selection", &self.selection)
            .field("diagnostics", &self.diagnostics)
            .field("unknown_events", &self.unknown_events)
            .finish_non_exhaustive()
    }
}

impl HostView {
    /// No host: the `md` lifecycle, no host globals or functions.
    pub fn md() -> Self {
        Self::of(HostSelection::Md, Vec::new())
    }

    /// The view `select` gives a document with `frontmatter` at `path`,
    /// judged in its own `context`, under `installed` (the configuration's
    /// registrations, when prepared). Only the document's own selection
    /// warnings are part of the view; the configuration's are published on
    /// the files they name ([`HostRegistrations::diagnostics`]).
    ///
    /// Reads the legacy host catalogs in the document's schema roots only
    /// when no registered host identifies the document.
    pub fn select(
        installed: Option<&InstalledHosts>,
        frontmatter: &Value,
        path: &Path,
        context: &FileResolutionContext,
    ) -> Self {
        let hosts = installed.map(|installed| installed.hosts.as_ref());
        let selected = select(hosts, frontmatter, Some(path), context);
        let mut view = Self::of(selected.selection, selected.diagnostics);
        if let Some(hosts) = hosts {
            view.unknown_events = unknown_host_events(&view, hosts, frontmatter);
        }
        view
    }

    /// The legacy view of `context`'s schema roots alone, with no
    /// registration excluded: only for a caller with no installed
    /// registrations to select under (a provider test building its own
    /// [`DocumentContext`](crate::providers::DocumentContext)). A served
    /// document's view always comes from [`select`](Self::select).
    pub fn unregistered(context: &FileResolutionContext) -> Self {
        let legacy = LegacyHost::discover(context, None);
        if legacy.is_empty() {
            Self::md()
        } else {
            Self::of(HostSelection::Legacy(Arc::new(legacy)), Vec::new())
        }
    }

    fn of(selection: HostSelection, diagnostics: Vec<RegistrationDiagnostic>) -> Self {
        let (lifecycle, catalogs, functions) = match &selection {
            HostSelection::Registered(profile) => (
                LifecycleHost::of(Some(Arc::clone(profile.lifecycle())), Vec::new()),
                vec![GlobalCatalog::clone(profile.globals())],
                profile.functions().iter().map(|set| set.functions.clone()).collect(),
            ),
            HostSelection::Legacy(legacy) => (
                LifecycleHost::of(legacy.lifecycle().cloned(), legacy.incomplete().to_vec()),
                legacy.globals().iter().map(|(_, catalog)| GlobalCatalog::clone(catalog)).collect(),
                legacy.functions().iter().map(|(_, functions)| functions.clone()).collect(),
            ),
            HostSelection::Md | HostSelection::Ambiguous(_) => (LifecycleHost::md(), Vec::new(), Vec::new()),
        };
        Self { selection, lifecycle, catalogs, functions, diagnostics, unknown_events: Vec::new() }
    }

    #[allow(missing_docs)]
    pub fn selection(&self) -> &HostSelection {
        &self.selection
    }

    /// The lifecycle host the document's stacks are parsed and analyzed
    /// under.
    pub fn lifecycle(&self) -> &LifecycleHost {
        &self.lifecycle
    }

    /// The host global catalogs, in root order for a legacy host.
    pub fn catalogs(&self) -> &[GlobalCatalog] {
        &self.catalogs
    }

    /// The legacy host catalogs that did not load.
    pub fn catalog_errors(&self) -> &[CatalogError] {
        match &self.selection {
            HostSelection::Legacy(legacy) => legacy.catalog_errors(),
            _ => &[],
        }
    }

    /// The analysis-only host function sets; an earlier set wins a name.
    pub fn functions(&self) -> &[HostFunctions] {
        &self.functions
    }

    /// The legacy function catalogs that did not load.
    pub fn function_errors(&self) -> &[HostFunctionCatalogError] {
        match &self.selection {
            HostSelection::Legacy(legacy) => legacy.function_errors(),
            _ => &[],
        }
    }

    /// The selection's warnings: `dm.host.ambiguous`, and a conflicting
    /// claim's `dm.host.registration` for a document the conflicting host
    /// identifies.
    pub fn diagnostics(&self) -> &[RegistrationDiagnostic] {
        &self.diagnostics
    }

    /// The root keys that are host-only events of a registered host that
    /// did not identify the document.
    pub fn unknown_events(&self) -> &[UnknownHostEvent] {
        &self.unknown_events
    }

    /// The files a legacy view read, whose content a cached view depends on,
    /// and the artifact locations beside them, whose presence it depends on.
    pub(crate) fn legacy_inputs(&self) -> (Vec<PathBuf>, Vec<PathBuf>) {
        let HostSelection::Legacy(legacy) = &self.selection else {
            return (Vec::new(), Vec::new());
        };
        let catalogs: Vec<PathBuf> = legacy
            .globals()
            .iter()
            .map(|(path, _)| path.clone())
            .chain(legacy.catalog_errors().iter().map(|error| PathBuf::from(&error.catalog)))
            .collect();
        let functions: Vec<PathBuf> = legacy
            .functions()
            .iter()
            .map(|(path, _)| path.clone())
            .chain(legacy.function_errors().iter().map(|error| error.path.clone()))
            .collect();
        let probes =
            catalogs.iter().map(|path| lifecycle_artifact_path(path)).chain(functions.iter().map(|path| facts_artifact_path(path))).collect();
        (catalogs.into_iter().chain(functions).collect(), probes)
    }
}

/// The root keys of `frontmatter` that the document's own lifecycle treats
/// as data but an agreed registered host's lifecycle registers as one of its
/// own events (an event `md` does not have under that name). Only a
/// document no registered host identified has any.
fn unknown_host_events(view: &HostView, hosts: &RegisteredHosts, frontmatter: &Value) -> Vec<UnknownHostEvent> {
    if !matches!(view.selection, HostSelection::Md | HostSelection::Legacy(_)) {
        return Vec::new();
    }
    let Some(map) = frontmatter.as_object() else {
        return Vec::new();
    };
    let md = md_host::registry();
    let mut found = Vec::new();
    for key in map.keys() {
        if !matches!(view.lifecycle.registry().resolve_key(key), KeyResolution::Data)
            || !matches!(md.resolve_key(key), KeyResolution::Data)
        {
            continue;
        }
        for claim in hosts.claims() {
            let HostClaim::Agreed { profile, .. } = claim else { continue };
            if matches!(profile.lifecycle().registry().resolve_key(key), KeyResolution::Event(_)) {
                found.push(UnknownHostEvent { key: key.clone(), host: profile.host().to_string() });
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A temporary home and working directory, with no repository.
    struct Fixture {
        _dir: tempfile::TempDir,
        root: PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let dir = tempfile::tempdir().unwrap();
            let root = biscuit_file::canonicalize_simplified(dir.path()).unwrap();
            std::fs::create_dir_all(root.join("home")).unwrap();
            std::fs::create_dir_all(root.join("work")).unwrap();
            Self { _dir: dir, root }
        }

        fn context(&self) -> FileResolutionContext {
            FileResolutionContext::from_snapshot(self.root.join("work"), Some(self.root.join("home")), HashMap::new())
        }

        fn config(&self) -> PathBuf {
            self.root.join("home").join(CONFIG_FILE_NAME)
        }

        fn version(&self, registrations: &mut HostRegistrations) -> u64 {
            registrations.prepare(&self.context());
            registrations.lookup(&self.context()).expect("installed").version
        }
    }

    #[test]
    fn an_unchanged_reload_keeps_the_version_and_a_changed_configuration_replaces_it() {
        let fixture = Fixture::new();
        let mut registrations = HostRegistrations::default();
        let first = fixture.version(&mut registrations);

        std::fs::write(fixture.config(), "register: []\n").unwrap();
        registrations.invalidate_paths(&[fixture.config()]);
        assert!(!registrations.is_fresh(&fixture.context()), "the configuration file changed");
        let again = fixture.version(&mut registrations);
        assert_eq!(again, first, "the same (empty) result keeps its version");

        registrations.invalidate_paths(&[fixture.config()]);
        assert!(registrations.is_fresh(&fixture.context()), "an unchanged file invalidates nothing");

        std::fs::write(fixture.config(), "register:\n  - ./missing/darkmatter-host.yaml\n").unwrap();
        registrations.invalidate_all();
        let refused = fixture.version(&mut registrations);
        assert_ne!(refused, first);
        let diagnostics = registrations.diagnostics();
        assert_eq!(diagnostics.len(), 1, "{diagnostics:#?}");
        assert_eq!(diagnostics[0].code, darkmatter::config::registration::HOST_REGISTRATION_CODE);
    }

    #[test]
    fn a_missing_manifest_is_watched_where_it_would_appear() {
        let fixture = Fixture::new();
        std::fs::write(fixture.config(), "register:\n  - ./missing/darkmatter-host.yaml\n").unwrap();
        let mut registrations = HostRegistrations::default();
        registrations.prepare(&fixture.context());
        let manifest = fixture.root.join("home/missing/darkmatter-host.yaml");
        assert!(registrations.dependencies().contains(&manifest), "{:?}", registrations.dependencies());
        let watched = registrations.watch_folders();
        let folder = fixture.root.join("home/missing");
        assert!(
            watched.contains(&WatchedFolder { folder: folder.clone(), absent_base: Some(fixture.root.join("home")) }),
            "{watched:?}"
        );

        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(&manifest, "kind: darkmatter-host\n").unwrap();
        registrations.invalidate_paths(&[manifest]);
        assert!(!registrations.is_fresh(&fixture.context()), "the manifest appeared");
    }

    /// The repository configuration is read as committed at `HEAD`, so a
    /// commit (a rewritten branch ref) or a checkout (a rewritten `HEAD`)
    /// reloads the registrations; an unchanged ref does not.
    #[test]
    fn a_commit_or_checkout_reloads_the_registrations() {
        let fixture = Fixture::new();
        let repo = fixture.root.join("work");
        std::fs::create_dir_all(repo.join(".git/refs/heads")).unwrap();
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/main\n").unwrap();
        let branch = repo.join(".git/refs/heads/main");
        std::fs::write(&branch, "1111111111111111111111111111111111111111\n").unwrap();
        let context = fixture.context().with_repository_root(&repo);
        let mut registrations = HostRegistrations::default();
        registrations.prepare(&context);
        assert_eq!(registrations.head_paths(), BTreeSet::from([repo.join(".git/HEAD"), branch.clone()]));

        registrations.invalidate_paths(std::slice::from_ref(&branch));
        assert!(registrations.is_fresh(&context), "an unchanged ref invalidates nothing");
        std::fs::write(&branch, "2222222222222222222222222222222222222222\n").unwrap();
        registrations.invalidate_paths(std::slice::from_ref(&branch));
        assert!(!registrations.is_fresh(&context), "a commit moved the branch");

        registrations.prepare(&context);
        std::fs::write(repo.join(".git/HEAD"), "ref: refs/heads/other\n").unwrap();
        registrations.invalidate_paths(&[repo.join(".git/HEAD")]);
        assert!(!registrations.is_fresh(&context), "a checkout moved `HEAD`");
        registrations.prepare(&context);
        assert!(registrations.head_paths().contains(&repo.join(".git/refs/heads/other")));
    }

    /// The user file's absence is one session notice naming `md init`; a
    /// repository without a configuration reports nothing, and writing the
    /// user file withdraws the notice. Nothing is written by the load.
    #[test]
    fn a_missing_user_configuration_is_a_notice_and_nothing_is_written() {
        let fixture = Fixture::new();
        let mut registrations = HostRegistrations::default();
        registrations.prepare(&fixture.context());
        let notices = registrations.notices();
        assert_eq!(notices.len(), 1, "{notices:#?}");
        let notice = notices.first().unwrap();
        assert_eq!(notice.code, CONFIG_MISSING_CODE);
        assert_eq!(notice.file, fixture.config());
        assert!(notice.message.contains("md init"), "{}", notice.message);
        assert!(!fixture.config().exists(), "a passive load creates nothing");

        std::fs::write(fixture.config(), "{}\n").unwrap();
        registrations.invalidate_paths(&[fixture.config()]);
        registrations.prepare(&fixture.context());
        assert!(registrations.notices().is_empty(), "{:#?}", registrations.notices());
    }

    /// A legacy policy file awaiting migration is reported as a notice and
    /// left exactly as it was: the passive load migrates nothing.
    #[test]
    fn a_pending_legacy_migration_is_reported_and_never_performed() {
        let fixture = Fixture::new();
        std::fs::write(fixture.config(), "{}\n").unwrap();
        let legacy = fixture.root.join("home").join(darkmatter::config::migration::LEGACY_WHITELIST_FILE_NAME);
        std::fs::write(&legacy, "ls\n").unwrap();
        let mut registrations = HostRegistrations::default();
        registrations.prepare(&fixture.context());
        let notices = registrations.notices();
        assert!(
            notices.iter().any(|notice| notice.code == CONFIG_MIGRATION_CODE && notice.file == legacy),
            "{notices:#?}"
        );
        assert_eq!(std::fs::read_to_string(&legacy).unwrap(), "ls\n");
        assert_eq!(std::fs::read_to_string(fixture.config()).unwrap(), "{}\n");
    }

    #[test]
    fn a_change_during_a_preparation_leaves_it_stale_and_an_older_one_never_wins() {
        let fixture = Fixture::new();
        let mut registrations = HostRegistrations::default();
        let older = registrations.begin(&fixture.context()).expect("cold").run();

        std::fs::write(fixture.config(), "# edited\n").unwrap();
        let during = registrations.begin(&fixture.context()).expect("still cold");
        registrations.invalidate_paths(&[fixture.config()]);
        assert!(!registrations.install(during.run()), "the change touched what it read");

        let newer = registrations.begin(&fixture.context()).expect("stale").run();
        assert!(registrations.install(newer));
        let installed = registrations.lookup(&fixture.context()).unwrap().version;
        assert!(registrations.install(older), "a dropped older result leaves the newer one fresh");
        assert_eq!(registrations.lookup(&fixture.context()).unwrap().version, installed);
    }
}

//! The prepared contract of each open YAML document: everything judging the
//! document needs that reads files, prepared on the registry worker beside
//! the document's kind registry and served from memory until an input
//! changes.
//!
//! A [`PreparedYaml`] holds the selected schema with its compiled validators
//! ([`YamlContract`]), or the error selecting or preparing it; the facts of
//! a fallback `$schema` reference ([`SchemaReferenceFacts`]); for a document
//! whose kind checks resolve its own imports (`schema`, `yaml-schema`), its
//! whole judgment; for a `kind: schema` / `kind: yaml-schema` document, the
//! resolution of its named types; and for a root-`$schema` contract, the
//! files its named types import. Building the document's overlay only looks it up and
//! judges the current data against it, passively.
//!
//! An entry is keyed on what it was prepared from: the document's
//! declaration (its authored `kind` and root `$schema`,
//! [`LoadedYamlDocument::declaration_key`]), its context, and its kind
//! registry's version; and, only when its preparation depends on the whole
//! text (the `schema` / `yaml-schema` kinds, whose checks read files), the
//! text. A data
//! edit of any other document therefore keeps the key, and the contract
//! judges the new data with no file read, resolution, or compilation.
//!
//! The three steps mirror [`super::registry`]'s, so the overlay lock is never
//! held across the I/O:
//!
//! ```text
//! begin (lock: key, kinds, buffers) ─▶ run (no lock: resolve, compile, hash)
//!     ─▶ install (lock: newest wins; a change meanwhile leaves it stale)
//! ```
//!
//! An entry goes stale when a file it read changes (a watched event, a save,
//! or an open buffer's edit, compared by xxHash), when a source appears or
//! disappears at a location it probed, on any change while its preparation
//! failed (its error may name a file that does not exist yet), and on a
//! rescan. A stale entry keeps serving its key until it is prepared again;
//! an entry under another key is never served, so a changed `kind` or
//! `$schema` never shows an earlier selection's results, and a failure always
//! replaces the success before it.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use biscuit_file::FileResolutionContext;
use biscuit_hash::xx_hash_bytes;
use darkmatter::markdown::compose::file_resolution_context_identity;
use darkmatter::markdown::schemas::{DarkmatterSchemas, KindRegistry, LoadedYamlDocument, Route, YamlContract, YamlValidation};
use simplified_schema::resolve::buffers::{self, SourceBuffers};
use simplified_schema::resolve::observed;
use simplified_schema::{SchemaError, parse_selected_standalone_schema_document};

use super::references::SchemaReferenceFacts;
use crate::context::DocumentResolution;
use super::registry::InstalledKinds;
use super::yaml_document::{hash_files, present, source_hash};
use super::{FrontmatterAst, NamedTypeCatalog, OverlayCache, StandaloneNamedTypes, error_dependencies};
use crate::work_counters;
use crate::workspace::watch::WatchedFolder;

/// What one YAML document's preparation produced.
pub(super) struct PreparedYaml {
    /// Distinct for every installed preparation.
    pub(super) version: u64,
    /// The selected schema; `Ok(None)` when none is selected.
    pub(super) contract: Result<Option<YamlContract>, Arc<SchemaError>>,
    /// The whole judgment, for a document whose kind checks read files; it
    /// judged the text the entry's key names.
    pub(super) validation: Option<Arc<YamlValidation>>,
    pub(super) schema_references: Arc<SchemaReferenceFacts>,
    /// The named-type resolution of a `kind: schema` / `kind: yaml-schema`
    /// document (whose key names its text).
    pub(super) standalone: Option<StandaloneNamedTypes>,
    /// The files a root-`$schema` contract's named types import; empty on
    /// the kind route.
    pub(super) named_types: Arc<NamedTypeCatalog>,
}

/// One document's installed preparation and what it read.
struct Entry {
    key: u64,
    prepared: Arc<PreparedYaml>,
    /// Every source read, with its xxHash (its open buffer's, else the
    /// file's).
    deps: Vec<(PathBuf, u64)>,
    /// Every location probed, with whether a source was there.
    probes: Vec<(PathBuf, bool)>,
    /// The folders a watch must cover for `deps` and `probes`.
    folders: Vec<WatchedFolder>,
    failed: bool,
    stale: bool,
    seq: u64,
}

/// A change recorded while a preparation ran, replayed at install.
#[derive(Clone)]
enum Change {
    Path(PathBuf),
    All,
}

/// Every open YAML document's prepared contract.
#[derive(Default)]
pub(super) struct YamlContracts {
    /// Keyed by document path.
    entries: HashMap<PathBuf, Entry>,
    /// The text-derived part of each document's key, memoized by the text's
    /// xxHash: `None` when the text does not load.
    declarations: HashMap<PathBuf, (u64, Option<u64>)>,
    epoch: u64,
    next_seq: u64,
    next_version: u64,
    /// Running preparations: sequence number to the epoch they began at.
    in_flight: HashMap<u64, u64>,
    changes: Vec<(u64, Change)>,
}

/// A preparation [`YamlContracts::begin`] started, run by [`Self::run`]
/// without the overlay lock.
pub(super) struct ContractJob {
    path: PathBuf,
    text: Arc<str>,
    key: u64,
    context: FileResolutionContext,
    kinds: Option<InstalledKinds>,
    workspace_roots: Vec<PathBuf>,
    sources: SourceBuffers,
    epoch: u64,
    seq: u64,
}

/// A finished [`ContractJob`], for [`YamlContracts::install`].
pub(super) struct PreparedJob {
    path: PathBuf,
    key: u64,
    epoch: u64,
    seq: u64,
    contract: Result<Option<YamlContract>, Arc<SchemaError>>,
    validation: Option<Arc<YamlValidation>>,
    schema_references: Arc<SchemaReferenceFacts>,
    standalone: Option<StandaloneNamedTypes>,
    named_types: Arc<NamedTypeCatalog>,
    deps: Vec<(PathBuf, u64)>,
    probes: Vec<(PathBuf, bool)>,
    folders: Vec<WatchedFolder>,
}

impl YamlContracts {
    /// The key a preparation of `text`, the YAML document at `path`, is
    /// installed under in `context` (of `generation`), or `None` when there
    /// is nothing to prepare: the document does not load.
    pub(super) fn key(
        &mut self,
        text: &str,
        path: &Path,
        context: &FileResolutionContext,
        generation: u64,
        kinds: Option<&InstalledKinds>,
    ) -> Option<u64> {
        let text_hash = xx_hash_bytes(text.as_bytes());
        let declaration = match self.declarations.get(path) {
            Some((hash, declaration)) if *hash == text_hash => *declaration,
            _ => {
                let declaration = declaration_part(text, path, text_hash);
                self.declarations.insert(path.to_path_buf(), (text_hash, declaration));
                declaration
            }
        }?;
        let mut bytes = Vec::with_capacity(32);
        bytes.extend_from_slice(&declaration.to_le_bytes());
        bytes.extend_from_slice(&file_resolution_context_identity(context).to_le_bytes());
        bytes.extend_from_slice(&generation.to_le_bytes());
        bytes.extend_from_slice(&kinds.map_or(0, |kinds| kinds.version).to_le_bytes());
        Some(xx_hash_bytes(&bytes))
    }

    /// The preparation installed for `path` under `key`, stale or not.
    /// How many sources the installed preparation of `path` read.
    #[cfg(test)]
    pub(super) fn dependency_count(&self, path: &Path) -> Option<usize> {
        self.entries.get(path).map(|entry| entry.deps.len())
    }

    pub(super) fn lookup(&self, path: &Path, key: u64) -> Option<&Arc<PreparedYaml>> {
        self.entries.get(path).filter(|entry| entry.key == key).map(|entry| &entry.prepared)
    }

    /// Whether `path` has a preparation installed under `key` that no change
    /// has marked stale. A lookup only.
    pub(super) fn is_fresh(&self, path: &Path, key: u64) -> bool {
        self.entries.get(path).is_some_and(|entry| entry.key == key && !entry.stale)
    }

    /// Starts preparing `path` under `key`, unless it is fresh. Cheap: the
    /// [`ContractJob`] copies its inputs and reads nothing.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin(
        &mut self,
        path: &Path,
        text: Arc<str>,
        key: u64,
        context: &FileResolutionContext,
        kinds: Option<InstalledKinds>,
        workspace_roots: &[PathBuf],
        sources: &SourceBuffers,
    ) -> Option<ContractJob> {
        if self.is_fresh(path, key) {
            return None;
        }
        self.next_seq += 1;
        self.in_flight.insert(self.next_seq, self.epoch);
        Some(ContractJob {
            path: path.to_path_buf(),
            text,
            key,
            context: context.clone(),
            kinds,
            workspace_roots: workspace_roots.to_vec(),
            sources: sources.clone(),
            epoch: self.epoch,
            seq: self.next_seq,
        })
    }

    /// Installs a finished preparation, unless one that began later already
    /// has. A change recorded while it ran that touches what it read leaves
    /// it stale (without re-reading: a touched file counts as changed).
    ///
    /// ## Returns
    ///
    /// Whether the document's preparation is fresh afterwards.
    pub(super) fn install(&mut self, job: PreparedJob) -> bool {
        self.in_flight.remove(&job.seq);
        let changes: Vec<Change> =
            self.changes.iter().filter(|(epoch, _)| *epoch > job.epoch).map(|(_, change)| change.clone()).collect();
        match self.in_flight.values().min() {
            Some(oldest) => self.changes.retain(|(epoch, _)| epoch > oldest),
            None => self.changes.clear(),
        }
        if self.entries.get(&job.path).is_some_and(|entry| entry.seq > job.seq) {
            return self.is_fresh(&job.path, job.key);
        }
        self.next_version += 1;
        let failed = job.contract.is_err();
        let mut entry = Entry {
            key: job.key,
            prepared: Arc::new(PreparedYaml {
                version: self.next_version,
                contract: job.contract,
                validation: job.validation,
                schema_references: job.schema_references,
                standalone: job.standalone,
                named_types: job.named_types,
            }),
            deps: job.deps,
            probes: job.probes,
            folders: job.folders,
            failed,
            stale: false,
            seq: job.seq,
        };
        entry.stale = changes.iter().any(|change| match change {
            Change::All => true,
            Change::Path(path) => affected(&entry, path, None),
        });
        let fresh = !entry.stale;
        self.entries.insert(job.path, entry);
        fresh
    }

    fn record_change(&mut self, change: Change) {
        self.epoch += 1;
        if !self.in_flight.is_empty() {
            self.changes.push((self.epoch, change));
        }
    }

    /// Marks stale every preparation a change at one of `paths` (a watched
    /// event or a save) affects, comparing each file read with its recorded
    /// hash first, through the open buffers in `sources`.
    pub(super) fn invalidate_paths(&mut self, paths: &[PathBuf], sources: &SourceBuffers) {
        for path in paths {
            let path = &SourceBuffers::key(path);
            self.record_change(Change::Path(path.clone()));
            for entry in self.entries.values_mut() {
                entry.stale |= affected(entry, path, Some(sources));
            }
        }
    }

    /// [`Self::invalidate_paths`] for an open buffer's text at `path`
    /// changing (or the buffer closing), as `sources` now holds it.
    pub(super) fn set_buffer(&mut self, path: &Path, sources: &SourceBuffers) {
        let path = SourceBuffers::key(path);
        self.record_change(Change::Path(path.clone()));
        for entry in self.entries.values_mut() {
            let read = entry.deps.iter().any(|(dep, _)| *dep == path);
            let probed = entry.probes.iter().any(|(probe, _)| *probe == path);
            entry.stale |= (read || probed) && affected(entry, &path, Some(sources));
        }
    }

    /// Marks every preparation stale: a rescan, a configuration change, or a
    /// package-manifest change.
    pub(super) fn invalidate_all(&mut self) {
        self.record_change(Change::All);
        for entry in self.entries.values_mut() {
            entry.stale = true;
        }
    }

    /// Drops a closed document's preparation.
    pub(super) fn forget(&mut self, path: &Path) {
        self.entries.remove(path);
        self.declarations.remove(path);
    }

    /// The folders the installed preparations read from or probed.
    pub(super) fn watch_folders(&self) -> impl Iterator<Item = WatchedFolder> + '_ {
        self.entries.values().flat_map(|entry| entry.folders.iter().cloned())
    }
}

impl OverlayCache {
    /// The named-type resolution prepared with the contract of `text`, the
    /// standalone schema YAML document at `path`; empty until one is
    /// prepared for this text and context. A lookup only.
    pub(super) fn prepared_standalone(
        &mut self,
        path: &Path,
        text: &str,
        workspace_roots: &[PathBuf],
        resolution: &DocumentResolution,
    ) -> StandaloneNamedTypes {
        let Ok(context) = resolution.context() else {
            return StandaloneNamedTypes::default();
        };
        let kinds = super::schema::trigger_boundary(path, workspace_roots).and_then(|_| self.triggers.lookup_kinds(path));
        self.contracts
            .key(text, path, context, resolution.generation(), kinds.as_ref())
            .and_then(|key| self.contracts.lookup(path, key))
            .and_then(|prepared| prepared.standalone.clone())
            .unwrap_or_default()
    }
}

/// The text-derived part of a document's key: its declaration, and its text
/// when its preparation reads the text as a whole. `None` when the text does
/// not load.
fn declaration_part(text: &str, path: &Path, text_hash: u64) -> Option<u64> {
    let loaded = LoadedYamlDocument::load(text, path).ok()?;
    // The kinds whose checks read files are exactly the schema-document
    // kinds whose named types are resolved here.
    let whole_text = loaded.kind_checks_read_files();
    let mut bytes = loaded.declaration_key().to_le_bytes().to_vec();
    if whole_text {
        bytes.extend_from_slice(&text_hash.to_le_bytes());
    }
    Some(xx_hash_bytes(&bytes))
}

/// Whether a change at `path` affects `entry`. A failed preparation is
/// affected by any change. A file it read is affected when its hash now
/// differs (`sources` compares through the open buffers; `None` skips the
/// comparison, so the file counts as changed); a probed location when a
/// source appeared or disappeared there, or a folder above it changed.
fn affected(entry: &Entry, path: &Path, sources: Option<&SourceBuffers>) -> bool {
    if entry.failed {
        return true;
    }
    if let Some((_, recorded)) = entry.deps.iter().find(|(dep, _)| dep == path) {
        return sources.is_none_or(|sources| source_hash(path, sources) != Some(*recorded));
    }
    entry.probes.iter().any(|(probe, existed)| {
        probe.starts_with(path) && sources.is_none_or(|sources| probe != path || present(probe, sources) != *existed)
    })
}

impl ContractJob {
    /// Prepares the document: selects and compiles its contract, selects
    /// its `$schema` references, runs file-reading kind checks, resolves a
    /// standalone schema's named types, and hashes everything read. All of a
    /// preparation's I/O, through the open buffers, touching no shared
    /// state.
    pub(super) fn run(self) -> PreparedJob {
        work_counters::record_yaml_preparation();
        let Self { path, text, key, context, kinds, workspace_roots, sources, epoch, seq } = self;
        let (prepared, observed) = buffers::with(&sources, || {
            observed::collect(|| prepare(&path, &text, &context, kinds, &workspace_roots))
        });
        let Prepared { contract, validation, schema_references, standalone, named_types, mut deps } = prepared;
        deps.extend(observed.read);
        let mut probes = schema_references.probes();
        for (probe, existed) in &mut probes {
            *existed |= sources.get(probe).is_some();
        }
        let mut seen: std::collections::HashSet<PathBuf> = probes.iter().map(|(probe, _)| probe.clone()).collect();
        for probe in observed.searched {
            if seen.insert(probe.clone()) {
                let existed = present(&probe, &sources);
                probes.push((probe, existed));
            }
        }
        // Compared with event and buffer paths in one spelling.
        let deps: Vec<PathBuf> = deps.iter().map(|dep| SourceBuffers::key(dep)).collect();
        let probes: Vec<(PathBuf, bool)> =
            probes.into_iter().map(|(probe, existed)| (SourceBuffers::key(&probe), existed)).collect();
        let deps = hash_files(deps.iter(), &sources);
        let folders = watched_folders(deps.iter().map(|(dep, _)| dep).chain(probes.iter().map(|(probe, _)| probe)));
        PreparedJob {
            path,
            key,
            epoch,
            seq,
            contract,
            validation,
            schema_references,
            standalone,
            named_types,
            deps,
            probes,
            folders,
        }
    }
}

/// What [`prepare`] produced, before its reads are hashed.
struct Prepared {
    contract: Result<Option<YamlContract>, Arc<SchemaError>>,
    validation: Option<Arc<YamlValidation>>,
    schema_references: Arc<SchemaReferenceFacts>,
    standalone: Option<StandaloneNamedTypes>,
    named_types: Arc<NamedTypeCatalog>,
    deps: Vec<PathBuf>,
}

fn prepare(
    path: &Path,
    text: &str,
    context: &FileResolutionContext,
    kinds: Option<InstalledKinds>,
    workspace_roots: &[PathBuf],
) -> Prepared {
    let mut prepared = Prepared {
        contract: Ok(None),
        validation: None,
        schema_references: Arc::default(),
        standalone: None,
        named_types: Arc::default(),
        deps: Vec::new(),
    };
    let Ok(loaded) = LoadedYamlDocument::load(text, path) else {
        return prepared;
    };
    let outer_context = context;
    let context = context.for_source(path);
    let registry = kinds.map_or_else(KindRegistry::shipped, |kinds| kinds.registry);
    let schema_roots = registry.roots().search_paths().to_vec();
    let schemas = DarkmatterSchemas::new(context.clone()).with_kind_registry(registry);
    let contract = schemas.prepare_yaml_contract(&loaded).map_err(Arc::new);
    let kind_route = match &contract {
        Ok(Some(contract)) => matches!(contract.route(), Route::Kind(_)),
        Err(error) => matches!(**error, SchemaError::KindSelection { .. }),
        Ok(None) => false,
    };
    match &contract {
        // A definition's own files are the registry's to track.
        Ok(Some(contract)) if *contract.route() == Route::RootSchema => {
            prepared.deps.extend(contract.effective().dependencies().iter().cloned());
        }
        Err(error) => prepared.deps.extend(error_dependencies(error)),
        _ => {}
    }
    if let Ok(Some(contract)) = &contract
        && contract.checks_read_files()
    {
        prepared.validation = Some(Arc::new(contract.validate(&loaded)));
    }
    // A `kind: schema` / `kind: yaml-schema` document is a schema document,
    // read as its selected definition accepted it (as the overlay reads it);
    // the named types its own declarations reference are resolved here.
    if loaded.kind_checks_read_files() {
        let accepted = match &contract {
            Ok(Some(contract)) => contract.effective().data_copy(loaded.json()).into_owned(),
            _ => loaded.json().clone(),
        };
        if let Ok(Some(model)) = parse_selected_standalone_schema_document(text, path, &accepted) {
            let (resolved, deps) = OverlayCache::resolve_standalone(&model, workspace_roots, outer_context);
            prepared.deps.extend(deps);
            prepared.standalone = Some(resolved);
        }
    }
    // A root `$schema` that is not data is the document's own contract:
    // the files its named types import serve its authoring, as a Markdown
    // document's inline `$schema` imports do.
    if !kind_route
        && let Some(schema) = loaded.json().get("$schema")
        && let Ok(schema) = serde_yaml_ng::to_value(schema)
    {
        let base_dir = path.parent().unwrap_or(path);
        let catalog = NamedTypeCatalog::build_tolerant(&schema, base_dir, &schema_roots, &context);
        prepared.deps.extend(catalog.dependencies().cloned());
        prepared.named_types = Arc::new(catalog);
    }
    if let Some(ast) = FrontmatterAst::parse_yaml_document(text, kind_route).ast
        && ast.schema_entry().is_some()
    {
        let assembled = matches!(contract, Ok(Some(_)));
        let facts = SchemaReferenceFacts::select_in(&ast, text, path, Ok(&schema_roots), &context, assembled);
        prepared.deps.extend(facts.selected().cloned());
        prepared.schema_references = Arc::new(facts);
    }
    prepared.contract = contract;
    prepared
}

/// The folder of each of `paths`, watched as present, or from its nearest
/// existing ancestor when it does not exist.
fn watched_folders<'a>(paths: impl Iterator<Item = &'a PathBuf>) -> Vec<WatchedFolder> {
    let mut seen = std::collections::HashSet::new();
    paths
        .filter_map(|path| path.parent())
        .filter(|folder| seen.insert(folder.to_path_buf()))
        .map(|folder| WatchedFolder {
            folder: folder.to_path_buf(),
            absent_base: (!folder.is_dir())
                .then(|| folder.ancestors().skip(1).find(|ancestor| ancestor.is_dir()).map(Path::to_path_buf))
                .flatten(),
        })
        .collect()
}

//! Where facts come from, and the fallback ladder that keeps every request
//! answerable when the store is not.
//!
//! [`FactsSource`] has two backends: [`StoreFacts`] (one root's SQLite store)
//! and [`MemoryFacts`] (a map, the fallback). [`RootFacts`] owns one root's
//! backend and never fails: a store failure switches it to memory for the
//! rest of the process, with one `warn` per process.
//!
//! A request reads through [`FactsSource::view`]: its candidates, facts, or
//! headings together with every row hash, all from one state of the facts. A
//! store takes them inside one read transaction, so a commit by another
//! process cannot land between the hashes and the rows they describe.
//!
//! Every log line here carries the `workspace` field of the server that
//! opened the root.
//!
//! | Failure | Behavior |
//! | --- | --- |
//! | corrupt, not a database, format mismatch, root mismatch (at open) | take the exclusive lock, delete, and recreate once; if the lock is held or the delete fails, memory |
//! | read-only, full, cannot open, busy beyond the timeout, WAL switch still busy | memory |
//! | any failure after open (a write mid-fill, a row that does not decode) | memory, keeping the store's rows when all of them can still be read, plus the facts being written |
//! | `DMLS_CACHE_DIR=off`, no cache directory | memory |

use std::collections::{BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::clock::Clock;
use crate::graph::HeadingFact;

use super::facts::FileFacts;
use super::failure::StoreFailure;
use super::keys::target_keys;
use super::location::{StoreLocation, lock_path, remove_database_files, store_path};
use super::lock::StoreLock;
use super::store::{IndexStore, OpenReport, OpenSettings, UpsertReport};

/// A backend of stored facts for one root. Every method may fail only for a
/// store; [`MemoryFacts`] never does.
pub trait FactsSource: Send + std::fmt::Debug {
    /// Every file with facts and its row hash.
    fn hashes(&mut self) -> Result<HashMap<PathBuf, u64>, StoreFailure>;
    /// The facts of every path in `paths` that has them, in `paths` order.
    fn load(&mut self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure>;
    /// Stores `facts`, replacing by path, skipping matching hashes.
    fn upsert(&mut self, facts: &[FileFacts]) -> Result<UpsertReport, StoreFailure>;
    /// Drops the facts of files missing on disk; returns how many.
    fn remove_missing(&mut self) -> Result<usize, StoreFailure>;
    /// Drops the facts of those `paths` missing on disk; returns how many.
    fn remove_paths(&mut self, paths: &[PathBuf]) -> Result<usize, StoreFailure>;
    /// Files holding a candidate key of `target`, sorted by path.
    fn candidates_for(&mut self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure>;
    /// Every file with its headings, sorted by path.
    fn scan_headings(&mut self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure>;
    /// One request's reads, all from one state of the facts: no write lands
    /// between the row hashes and the reads they vouch for.
    fn view(&mut self, read: &ViewRead<'_>) -> Result<View, StoreFailure> {
        read_view(self, read)
    }
}

/// What one request reads through [`FactsSource::view`].
#[derive(Debug, Clone, Copy)]
pub enum ViewRead<'a> {
    /// The files in `listed` holding a candidate key of one of `targets`.
    Candidates { targets: &'a [PathBuf], listed: &'a BTreeSet<PathBuf> },
    /// Those candidates, and the facts of each of them and of `also`.
    Documents { targets: &'a [PathBuf], listed: &'a BTreeSet<PathBuf>, also: &'a [PathBuf] },
    /// Every file's headings.
    Headings,
}

impl ViewRead<'_> {
    /// The targets whose candidates are read; none for headings.
    pub fn targets(&self) -> &[PathBuf] {
        match self {
            Self::Candidates { targets, .. } | Self::Documents { targets, .. } => targets,
            Self::Headings => &[],
        }
    }
}

/// The result of a [`ViewRead`], with every row's hash from the same state,
/// so a caller can tell which of the rows it read it may trust.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct View {
    /// Every file with a row, and its row hash.
    pub hashes: HashMap<PathBuf, u64>,
    /// Sorted, once per path; empty for [`ViewRead::Headings`].
    pub candidates: Vec<PathBuf>,
    /// Set for [`ViewRead::Documents`] only.
    pub facts: Vec<FileFacts>,
    /// Sorted by path; set for [`ViewRead::Headings`] only.
    pub headings: Vec<(PathBuf, Vec<HeadingFact>)>,
}

/// Performs `read` through `source`'s individual reads. It is consistent
/// only when `source` cannot change between them (memory, or a store inside
/// a read transaction).
fn read_view<S: FactsSource + ?Sized>(source: &mut S, read: &ViewRead<'_>) -> Result<View, StoreFailure> {
    let mut view = View { hashes: source.hashes()?, ..View::default() };
    match *read {
        ViewRead::Headings => view.headings = source.scan_headings()?,
        ViewRead::Candidates { targets, listed } | ViewRead::Documents { targets, listed, .. } => {
            let mut candidates = BTreeSet::new();
            for target in targets {
                candidates.extend(source.candidates_for(target)?.into_iter().filter(|path| listed.contains(path)));
            }
            if let ViewRead::Documents { also, .. } = *read {
                let mut paths = candidates.clone();
                paths.extend(also.iter().cloned());
                view.facts = source.load(&paths.into_iter().collect::<Vec<_>>())?;
            }
            view.candidates = candidates.into_iter().collect();
        }
    }
    Ok(view)
}

/// One root's SQLite store as a [`FactsSource`].
#[derive(Debug)]
pub struct StoreFacts {
    store: IndexStore,
}

/// How [`StoreFacts::open`] obtained its store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoreOpened {
    pub report: OpenReport,
    /// The failure that made open delete and recreate the store, if any.
    pub recreated: Option<StoreFailure>,
}

impl StoreFacts {
    /// Opens `root`'s store at `path`. A corrupt store, or one recorded for
    /// another root, is deleted under the exclusive lock and recreated once.
    ///
    /// ## Errors
    ///
    /// The open failure when it is not repairable, [`StoreFailure::InUse`]
    /// when repair could not take the exclusive lock, or the second failure
    /// after a recreate.
    pub fn open(path: &Path, root: &Path, clock: Arc<dyn Clock>) -> Result<(Self, StoreOpened), StoreFailure> {
        Self::open_with(path, root, clock, OpenSettings::default())
    }

    pub(crate) fn open_with(
        path: &Path,
        root: &Path,
        clock: Arc<dyn Clock>,
        settings: OpenSettings,
    ) -> Result<(Self, StoreOpened), StoreFailure> {
        match IndexStore::open_with(path, root, clock.clone(), settings) {
            Ok((store, report)) => Ok((Self { store }, StoreOpened { report, recreated: None })),
            Err(failure @ (StoreFailure::Corrupt(_) | StoreFailure::RootMismatch { .. })) => {
                delete_for_repair(path)?;
                let (store, report) = IndexStore::open_with(path, root, clock, settings)?;
                Ok((Self { store }, StoreOpened { report, recreated: Some(failure) }))
            }
            Err(other) => Err(other),
        }
    }

    pub fn store(&self) -> &IndexStore {
        &self.store
    }
}

/// Deletes a store's database files under its exclusive lock, releasing the
/// lock before returning so the recreate can take its shared one.
fn delete_for_repair(path: &Path) -> Result<(), StoreFailure> {
    let Some(_exclusive) = StoreLock::try_exclusive(&lock_path(path))? else {
        return Err(StoreFailure::InUse);
    };
    if remove_database_files(path)? { Ok(()) } else { Err(StoreFailure::InUse) }
}

impl FactsSource for StoreFacts {
    fn hashes(&mut self) -> Result<HashMap<PathBuf, u64>, StoreFailure> {
        self.store.hashes()
    }
    fn load(&mut self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure> {
        self.store.load(paths)
    }
    fn upsert(&mut self, facts: &[FileFacts]) -> Result<UpsertReport, StoreFailure> {
        self.store.upsert_batch(facts)
    }
    fn remove_missing(&mut self) -> Result<usize, StoreFailure> {
        self.store.remove_missing()
    }
    fn remove_paths(&mut self, paths: &[PathBuf]) -> Result<usize, StoreFailure> {
        self.store.remove_paths(paths)
    }
    fn candidates_for(&mut self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure> {
        self.store.candidates_for(target)
    }
    fn scan_headings(&mut self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure> {
        self.store.scan_headings()
    }
    fn view(&mut self, read: &ViewRead<'_>) -> Result<View, StoreFailure> {
        self.store.begin_read()?;
        let view = read_view(self, read);
        let ended = self.store.end_read();
        let view = view?;
        ended?;
        Ok(view)
    }
}

/// Facts held in memory: the fallback backend. It holds facts, not parse
/// trees (about 16 MiB for a 5,700-file repository).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MemoryFacts(pub HashMap<PathBuf, FileFacts>);

impl MemoryFacts {
    fn put(&mut self, facts: &[FileFacts]) -> UpsertReport {
        let mut report = UpsertReport::default();
        for file in facts {
            if self.0.get(&file.path).is_some_and(|held| held.hash == file.hash) {
                report.skipped += 1;
            } else {
                self.0.insert(file.path.clone(), file.clone());
                report.written += 1;
            }
        }
        report
    }

    fn hash_map(&self) -> HashMap<PathBuf, u64> {
        self.0.iter().map(|(path, file)| (path.clone(), file.hash)).collect()
    }

    fn get_all(&self, paths: &[PathBuf]) -> Vec<FileFacts> {
        paths.iter().filter_map(|path| self.0.get(path).cloned()).collect()
    }

    fn drop_missing(&mut self) -> usize {
        let before = self.0.len();
        self.0.retain(|path, _| !super::store::is_missing(path));
        before - self.0.len()
    }

    fn drop_paths(&mut self, paths: &[PathBuf]) -> usize {
        paths
            .iter()
            .filter(|path| super::store::is_missing(path) && self.0.remove(*path).is_some())
            .count()
    }

    fn candidates(&self, target: &Path) -> Vec<PathBuf> {
        let keys = target_keys(target);
        let mut out: Vec<PathBuf> = self
            .0
            .values()
            .filter(|file| keys.iter().any(|key| file.candidate_keys.contains(key)))
            .map(|file| file.path.clone())
            .collect();
        out.sort();
        out
    }

    fn headings(&self) -> Vec<(PathBuf, Vec<HeadingFact>)> {
        let mut out: Vec<_> = self.0.values().map(|file| (file.path.clone(), file.headings.clone())).collect();
        out.sort_by(|left, right| left.0.cmp(&right.0));
        out
    }
}

impl FactsSource for MemoryFacts {
    fn hashes(&mut self) -> Result<HashMap<PathBuf, u64>, StoreFailure> {
        Ok(self.hash_map())
    }
    fn load(&mut self, paths: &[PathBuf]) -> Result<Vec<FileFacts>, StoreFailure> {
        Ok(self.get_all(paths))
    }
    fn upsert(&mut self, facts: &[FileFacts]) -> Result<UpsertReport, StoreFailure> {
        Ok(self.put(facts))
    }
    fn remove_missing(&mut self) -> Result<usize, StoreFailure> {
        Ok(self.drop_missing())
    }
    fn remove_paths(&mut self, paths: &[PathBuf]) -> Result<usize, StoreFailure> {
        Ok(self.drop_paths(paths))
    }
    fn candidates_for(&mut self, target: &Path) -> Result<Vec<PathBuf>, StoreFailure> {
        Ok(self.candidates(target))
    }
    fn scan_headings(&mut self) -> Result<Vec<(PathBuf, Vec<HeadingFact>)>, StoreFailure> {
        Ok(self.headings())
    }
}

/// Set once this process has logged its memory-fallback `warn`.
static FALLBACK_WARNED: AtomicBool = AtomicBool::new(false);

/// Whether this process has fallen back to memory for any root.
pub fn fallback_warned() -> bool {
    FALLBACK_WARNED.load(Ordering::SeqCst)
}

fn warn_fallback(reason: &str, detail: &str, path: Option<&Path>, workspace: &str) {
    if FALLBACK_WARNED.swap(true, Ordering::SeqCst) {
        tracing::debug!(%reason, %detail, %workspace, "index store unavailable, already using memory");
        return;
    }
    let path = path.map_or_else(String::new, |path| path.display().to_string());
    tracing::warn!(%reason, %detail, path = %path, %workspace, "index store unavailable, using memory");
}

/// How [`RootFacts::open`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RootOpened {
    Store(StoreOpened),
    /// Memory facts, for this reason.
    Memory(String),
}

/// One root's facts: its store, or memory once the store has failed.
///
/// Infallible by design: a cache problem never fails a request. After a
/// switch to memory the held facts can be incomplete (only what could be
/// salvaged), so [`RootFacts::take_switched`] tells the caller to revalidate
/// before trusting a candidate set.
#[derive(Debug)]
pub struct RootFacts {
    store_path: Option<PathBuf>,
    primary: Option<Box<dyn FactsSource>>,
    memory: MemoryFacts,
    switched: bool,
    /// The `workspace` field of this root's log lines.
    workspace: String,
}

impl RootFacts {
    /// Opens `root`'s facts at `location` for the server whose log lines
    /// carry `workspace`; never fails.
    pub fn open(root: &Path, location: &StoreLocation, clock: Arc<dyn Clock>, workspace: &str) -> (Self, RootOpened) {
        Self::open_with(root, location, clock, OpenSettings::default(), workspace)
    }

    pub(crate) fn open_with(
        root: &Path,
        location: &StoreLocation,
        clock: Arc<dyn Clock>,
        settings: OpenSettings,
        workspace: &str,
    ) -> (Self, RootOpened) {
        let dir = match location {
            StoreLocation::Directory(dir) => dir,
            StoreLocation::Disabled => return Self::memory_because("disabled", "DMLS_CACHE_DIR=off", None, workspace),
            StoreLocation::Unavailable(detail) => return Self::memory_because("unavailable", detail, None, workspace),
        };
        let path = store_path(dir, root);
        match StoreFacts::open_with(&path, root, clock, settings) {
            Ok((store, opened)) => {
                if let Some(failure) = &opened.recreated {
                    tracing::warn!(path = %path.display(), reason = %failure.reason(), %workspace, "index store recreated");
                }
                let created = opened.report.created;
                tracing::info!(
                    path = %path.display(),
                    created,
                    facts_version = super::FACTS_VERSION,
                    rows = (!created).then_some(opened.report.rows),
                    %workspace,
                    "index store opened"
                );
                let facts = Self::with_source(Box::new(store), Some(path), workspace);
                (facts, RootOpened::Store(opened))
            }
            Err(failure) => Self::memory_because(failure.reason(), &failure.to_string(), Some(&path), workspace),
        }
    }

    fn memory_because(reason: &str, detail: &str, path: Option<&Path>, workspace: &str) -> (Self, RootOpened) {
        warn_fallback(reason, detail, path, workspace);
        let facts = Self {
            store_path: path.map(Path::to_path_buf),
            primary: None,
            memory: MemoryFacts::default(),
            switched: false,
            workspace: workspace.to_string(),
        };
        (facts, RootOpened::Memory(detail.to_string()))
    }

    /// Facts backed by `primary` until it fails.
    pub(crate) fn with_source(primary: Box<dyn FactsSource>, store_path: Option<PathBuf>, workspace: &str) -> Self {
        Self { store_path, primary: Some(primary), memory: MemoryFacts::default(), switched: false, workspace: workspace.to_string() }
    }

    /// Whether facts now come from memory.
    pub fn in_memory(&self) -> bool {
        self.primary.is_none()
    }

    /// Whether the backend switched to memory since the last call. Memory
    /// then holds only what was salvaged, so the caller revalidates.
    pub fn take_switched(&mut self) -> bool {
        std::mem::take(&mut self.switched)
    }

    pub fn hashes(&mut self) -> HashMap<PathBuf, u64> {
        if let Some(primary) = self.primary.as_mut() {
            match primary.hashes() {
                Ok(hashes) => return hashes,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.hash_map()
    }

    pub fn load(&mut self, paths: &[PathBuf]) -> Vec<FileFacts> {
        if let Some(primary) = self.primary.as_mut() {
            match primary.load(paths) {
                Ok(facts) => return facts,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.get_all(paths)
    }

    /// Stores `facts`. On a store failure the facts are kept in memory.
    pub fn upsert(&mut self, facts: &[FileFacts]) -> UpsertReport {
        if let Some(primary) = self.primary.as_mut() {
            match primary.upsert(facts) {
                Ok(report) => return report,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.put(facts)
    }

    pub fn remove_missing(&mut self) -> usize {
        if let Some(primary) = self.primary.as_mut() {
            match primary.remove_missing() {
                Ok(removed) => return removed,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.drop_missing()
    }

    pub fn remove_paths(&mut self, paths: &[PathBuf]) -> usize {
        if let Some(primary) = self.primary.as_mut() {
            match primary.remove_paths(paths) {
                Ok(removed) => return removed,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.drop_paths(paths)
    }

    pub fn candidates_for(&mut self, target: &Path) -> Vec<PathBuf> {
        if let Some(primary) = self.primary.as_mut() {
            match primary.candidates_for(target) {
                Ok(candidates) => return candidates,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.candidates(target)
    }

    pub fn scan_headings(&mut self) -> Vec<(PathBuf, Vec<HeadingFact>)> {
        if let Some(primary) = self.primary.as_mut() {
            match primary.scan_headings() {
                Ok(headings) => return headings,
                Err(failure) => self.fall_back(failure),
            }
        }
        self.memory.headings()
    }

    /// One request's reads from one state of the facts; see
    /// [`FactsSource::view`].
    pub fn view(&mut self, read: &ViewRead<'_>) -> View {
        if let Some(primary) = self.primary.as_mut() {
            match primary.view(read) {
                Ok(view) => return view,
                Err(failure) => self.fall_back(failure),
            }
        }
        // Memory never fails.
        read_view(&mut self.memory, read).unwrap_or_default()
    }

    /// Switches to memory for the rest of the process, salvaging the failed
    /// store's rows when all of them can still be read; one row that does
    /// not decode leaves memory without any.
    fn fall_back(&mut self, failure: StoreFailure) {
        let Some(mut primary) = self.primary.take() else { return };
        warn_fallback(failure.reason(), &failure.to_string(), self.store_path.as_deref(), &self.workspace);
        let salvaged = primary.hashes().and_then(|hashes| {
            let paths: Vec<PathBuf> = hashes.into_keys().collect();
            primary.load(&paths)
        });
        match salvaged {
            Ok(facts) => {
                self.memory.put(&facts);
            }
            Err(error) => tracing::debug!(%error, workspace = %self.workspace, "index store rows could not be salvaged into memory"),
        }
        self.switched = true;
    }
}

#[cfg(test)]
#[path = "source_tests.rs"]
mod tests;

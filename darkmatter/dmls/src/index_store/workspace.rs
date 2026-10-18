//! Every root's facts for one workspace, kept equal to the files on disk.
//!
//! [`WorkspaceFacts`] owns one [`RootFacts`] per effective workspace root and
//! remembers the hash of every file it has read in this process. Three
//! operations keep the rows current:
//!
//! - [`WorkspaceFacts::revalidate`], the first-need pass: read and hash every
//!   listed Markdown file in parallel, re-parse only a file whose hash differs
//!   from its row (or that has none), write those rows in batches, and delete
//!   the rows of paths missing on disk. There is no size-and-mtime shortcut:
//!   a stale row that misses a new link is never a candidate.
//! - [`WorkspaceFacts::refresh`], the same work for a few paths: a live
//!   update, a fallback-mode save, or a rename re-reading its referrers.
//! - [`WorkspaceFacts::request_documents`], the read side: stored facts with
//!   every open buffer's parse in place of its row. A buffer never writes a
//!   row; rows hold disk state only.
//!
//! Requests read through [`WorkspaceFacts::request_graph`] (references,
//! heading rename, file rename: a small graph over the candidates of the
//! targets) and [`WorkspaceFacts::symbol_documents`] (workspace symbols: the
//! stored headings, no graph). Both de-duplicate a file under nested roots.
//!
//! **Ready** ([`WorkspaceFacts::is_ready`]) means a revalidation completed and
//! since then every listed file has a row whose hash matches bytes read in
//! this process. A store failure that switches a root to memory withdraws it,
//! because memory then holds only the rows that could be salvaged.
//!
//! A file under two nested roots has a row in each root's store.

use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::SystemTime;

use crossbeam_channel::unbounded;

use crate::clock::Clock;
use crate::graph::{DocumentContexts, DocumentIndex, HeadingFact, WorkspaceGraph, index_document};
use crate::providers::symbols::SymbolDocument;
use crate::wiki::WikiUniverse;
use crate::workspace::startup::ProgressReporter;

use super::facts::FileFacts;
use super::location::{STALE_STORE_AGE, StoreLocation, sweep_stale_stores};
use super::source::{RootFacts, RootOpened};
use super::store::BATCH_SIZE;

/// Set once this process has swept its store directory.
static SWEPT: AtomicBool = AtomicBool::new(false);

/// What a [`WorkspaceFacts::revalidate`] or [`WorkspaceFacts::refresh`] did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PassReport {
    /// Files read and hashed.
    pub read: usize,
    /// Files whose rows already matched their bytes in every owning root.
    pub unchanged: usize,
    /// Files whose facts were recomputed (parsed, or taken from a current
    /// parse the caller supplied) and written.
    pub reparsed: usize,
    /// Rows deleted for paths missing on disk.
    pub removed: usize,
    /// Files that exist but could not be read, or whose changed bytes are not
    /// UTF-8, with the error. No row is written for them.
    pub failed: Vec<(PathBuf, String)>,
    /// Requested files that are missing on disk.
    pub missing: Vec<PathBuf>,
    /// The hash of each file's bytes as read by this pass.
    pub hashes: Vec<(PathBuf, u64)>,
    /// Whether the pass stopped early because `cancel` was set. The rows it
    /// wrote are correct, but it read only some files.
    pub cancelled: bool,
    /// Whether a root switched to memory during the pass. Memory then holds
    /// only the rows that could be salvaged, so the caller revalidates.
    pub switched: bool,
}

/// Every effective workspace root's facts; see the [module docs](self).
#[derive(Debug)]
pub struct WorkspaceFacts {
    roots: Vec<(PathBuf, RootFacts)>,
    /// The hash of each file's bytes as last read in this process.
    verified: HashMap<PathBuf, u64>,
    ready: bool,
}

impl WorkspaceFacts {
    /// Opens the facts of every root at `location`. The first open in a
    /// process also sweeps the store directory of stores untouched for
    /// [`STALE_STORE_AGE`], after this workspace's own stores are open and
    /// therefore locked and fresh. Never fails: a root whose store cannot be
    /// used keeps its facts in memory.
    pub fn open(roots: &[PathBuf], location: &StoreLocation, clock: Arc<dyn Clock>) -> Self {
        let roots = roots
            .iter()
            .map(|root| {
                let (facts, opened) = RootFacts::open(root, location, Arc::clone(&clock));
                if let RootOpened::Store(opened) = &opened {
                    tracing::debug!(root = %root.display(), created = opened.report.created, rows = opened.report.rows, "index store open");
                }
                (root.clone(), facts)
            })
            .collect();
        if let StoreLocation::Directory(dir) = location
            && !SWEPT.swap(true, Ordering::SeqCst)
        {
            match sweep_stale_stores(dir, STALE_STORE_AGE, SystemTime::now()) {
                Ok(report) => tracing::debug!(?report, "index store sweep"),
                Err(error) => tracing::debug!(%error, dir = %dir.display(), "index store sweep failed"),
            }
        }
        Self::from_roots(roots)
    }

    /// Facts over already-opened roots.
    pub(crate) fn from_roots(roots: Vec<(PathBuf, RootFacts)>) -> Self {
        Self { roots, verified: HashMap::new(), ready: false }
    }

    /// The effective roots these facts were opened for, in open order.
    pub fn roots(&self) -> impl Iterator<Item = &Path> {
        self.roots.iter().map(|(root, _)| root.as_path())
    }

    /// Whether facts are ready for `listed` under `roots`: a revalidation
    /// completed for the same roots, nothing has withdrawn it since, and every
    /// listed file has been read in this process.
    pub fn is_ready(&self, listed: &BTreeSet<PathBuf>, roots: &[PathBuf]) -> bool {
        self.ready && self.roots().eq(roots.iter().map(PathBuf::as_path)) && listed.iter().all(|path| self.verified.contains_key(path))
    }

    /// Records that a revalidation and every change received while it ran
    /// have been applied. The caller decides this; a pass alone cannot know
    /// about changes that arrived meanwhile.
    pub fn mark_ready(&mut self) {
        self.ready = true;
    }

    /// Withdraws ready, so the next request revalidates.
    pub fn withdraw_ready(&mut self) {
        self.ready = false;
    }

    /// The hash of `path`'s bytes as last read in this process.
    pub fn verified_hash(&self, path: &Path) -> Option<u64> {
        self.verified.get(path).copied()
    }

    /// The first-need pass over `files`, the listed Markdown files; see the
    /// [module docs](self). Also deletes every row whose path is missing on
    /// disk, in every root.
    ///
    /// `known` holds parses the caller already has (a parse is used only
    /// when its hash equals the bytes this pass read). `cancel` is checked
    /// between files and between write batches. Progress is reported per
    /// file, without `begin` or `end`.
    pub fn revalidate(
        &mut self,
        files: &[PathBuf],
        known: &BTreeMap<PathBuf, DocumentIndex>,
        progress: &dyn ProgressReporter,
        cancel: &AtomicBool,
    ) -> PassReport {
        let stored: Vec<HashMap<PathBuf, u64>> = self.roots.iter_mut().map(|(_, facts)| facts.hashes()).collect();
        let mut report = self.pass(files, stored, known, progress, cancel);
        if !report.cancelled {
            for (_, facts) in &mut self.roots {
                report.removed += facts.remove_missing();
            }
        }
        report.switched |= self.take_switched();
        report
    }

    /// Re-reads and re-hashes `paths`, re-parsing each one whose bytes no
    /// longer match its row and deleting the row of each one missing on disk.
    /// Does not change whether facts are ready, unless a root switches to
    /// memory.
    pub fn refresh(&mut self, paths: &[PathBuf], known: &BTreeMap<PathBuf, DocumentIndex>) -> PassReport {
        // Only these rows' hashes, not every row's.
        let stored: Vec<HashMap<PathBuf, u64>> = self
            .roots
            .iter_mut()
            .map(|(root, facts)| {
                let owned: Vec<PathBuf> = paths.iter().filter(|path| path.starts_with(&*root)).cloned().collect();
                if owned.is_empty() {
                    return HashMap::new();
                }
                facts.load(&owned).into_iter().map(|file| (file.path, file.hash)).collect()
            })
            .collect();
        let mut report = self.pass(paths, stored, known, &crate::workspace::startup::SilentProgress, &AtomicBool::new(false));
        report.switched |= self.take_switched();
        report
    }

    /// The documents a request graph is built over: the stored facts of each
    /// of `paths`, with every buffer in `buffers` in place of its row (or
    /// added, when it has none). Nothing is written.
    ///
    /// A root that fails while reading switches to memory and withdraws
    /// ready; the request still gets every fact memory holds.
    pub fn request_documents<'a>(
        &mut self,
        paths: &[PathBuf],
        buffers: impl IntoIterator<Item = &'a DocumentIndex>,
    ) -> BTreeMap<PathBuf, DocumentIndex> {
        let mut documents = BTreeMap::new();
        let mut wanted: Vec<PathBuf> = paths.to_vec();
        for (root, facts) in &mut self.roots {
            let owned: Vec<PathBuf> = wanted.iter().filter(|path| path.starts_with(&*root)).cloned().collect();
            if owned.is_empty() {
                continue;
            }
            for file in facts.load(&owned) {
                documents.insert(file.path.clone(), file.into_index());
            }
            wanted.retain(|path| !documents.contains_key(path));
        }
        if self.take_switched() {
            self.ready = false;
        }
        for buffer in buffers {
            documents.insert(buffer.path.clone(), buffer.clone());
        }
        documents
    }

    /// Every listed file that holds a candidate key of one of `targets`, in
    /// any root, sorted and once per path (a file under nested roots has a
    /// row in each). Candidates only narrow: a request graph decides which
    /// of them really reference a target.
    ///
    /// A row of a file outside `listed` (written by another process with a
    /// different file list) is never a candidate.
    pub fn candidates_for(&mut self, targets: &[PathBuf], listed: &BTreeSet<PathBuf>) -> Vec<PathBuf> {
        let mut candidates = BTreeSet::new();
        for (_, facts) in &mut self.roots {
            for target in targets {
                candidates.extend(facts.candidates_for(target).into_iter().filter(|path| listed.contains(path)));
            }
        }
        self.take_switched();
        candidates.into_iter().collect()
    }

    /// The graph a references, rename, or file-rename request about
    /// `targets` is answered from: the stored facts of every candidate
    /// referrer and of each listed target, with every document in `overlay`
    /// in place of its row (or added), resolved against `universe`.
    ///
    /// `overlay` holds every open buffer, plus any target outside the file
    /// list the caller has parsed. `universe` is the whole file list's wiki
    /// universe, so wiki ambiguity and rename spellings are judged as a graph
    /// over every file would judge them. The graph is the request's alone;
    /// nothing is kept.
    pub fn request_graph<'a>(
        &mut self,
        targets: &[PathBuf],
        listed: &BTreeSet<PathBuf>,
        overlay: impl IntoIterator<Item = &'a DocumentIndex>,
        universe: Arc<WikiUniverse>,
        contexts: &dyn DocumentContexts,
    ) -> WorkspaceGraph {
        let mut paths: BTreeSet<PathBuf> = self.candidates_for(targets, listed).into_iter().collect();
        paths.extend(targets.iter().filter(|target| listed.contains(*target)).cloned());
        let paths: Vec<PathBuf> = paths.into_iter().collect();
        let documents = self.request_documents(&paths, overlay);
        WorkspaceGraph::build_with_roots(&documents, 0, universe, contexts)
    }

    /// The documents workspace symbols read: every listed file's stored
    /// headings, once per path, with each document in `overlay` (every open
    /// buffer) in place of its row (or added), sorted by path.
    pub fn symbol_documents<'a>(
        &mut self,
        listed: &BTreeSet<PathBuf>,
        overlay: impl IntoIterator<Item = &'a DocumentIndex>,
    ) -> Vec<SymbolDocument> {
        let mut documents: BTreeMap<PathBuf, Vec<HeadingFact>> = BTreeMap::new();
        for (_, facts) in &mut self.roots {
            for (path, headings) in facts.scan_headings() {
                if listed.contains(&path) {
                    documents.entry(path).or_insert(headings);
                }
            }
        }
        self.take_switched();
        for buffer in overlay {
            documents.insert(buffer.path.clone(), buffer.headings.clone());
        }
        documents.into_iter().collect()
    }

    /// Whether any root switched to memory since the last check; withdraws
    /// ready when one did.
    fn take_switched(&mut self) -> bool {
        let mut switched = false;
        for (_, facts) in &mut self.roots {
            switched |= facts.take_switched();
        }
        if switched {
            self.ready = false;
        }
        switched
    }

    /// The indices of the roots that contain `path`.
    fn owners(&self, path: &Path) -> Vec<usize> {
        self.roots.iter().enumerate().filter(|(_, (root, _))| path.starts_with(root)).map(|(index, _)| index).collect()
    }

    /// Reads `files` and writes the rows that differ from `stored`, each
    /// root's row hashes for (at least) those files.
    fn pass(
        &mut self,
        files: &[PathBuf],
        stored: Vec<HashMap<PathBuf, u64>>,
        known: &BTreeMap<PathBuf, DocumentIndex>,
        progress: &dyn ProgressReporter,
        cancel: &AtomicBool,
    ) -> PassReport {
        let jobs: Vec<Job> = files.iter().map(|path| Job { path: path.clone(), owners: self.owners(path) }).collect();
        let (outcomes, cancelled) = read_all(&jobs, &stored, known, progress, cancel);

        let mut report = PassReport { cancelled, ..PassReport::default() };
        let mut writes: Vec<Vec<FileFacts>> = vec![Vec::new(); self.roots.len()];
        let mut missing_by_root: Vec<Vec<PathBuf>> = vec![Vec::new(); self.roots.len()];
        for (job, outcome) in jobs.iter().zip(outcomes) {
            match outcome {
                Outcome::Skipped => {}
                Outcome::Missing => {
                    self.verified.remove(&job.path);
                    for &owner in &job.owners {
                        missing_by_root[owner].push(job.path.clone());
                    }
                    report.missing.push(job.path.clone());
                }
                Outcome::Failed(error) => {
                    self.verified.remove(&job.path);
                    report.read += 1;
                    report.failed.push((job.path.clone(), error));
                }
                Outcome::Read { hash, facts } => {
                    report.read += 1;
                    report.hashes.push((job.path.clone(), hash));
                    self.verified.insert(job.path.clone(), hash);
                    let Some(facts) = facts else {
                        report.unchanged += 1;
                        continue;
                    };
                    report.reparsed += 1;
                    for &owner in &job.owners {
                        if stored[owner].get(&job.path) != Some(&hash) {
                            writes[owner].push(facts.clone());
                        }
                    }
                }
            }
        }

        'roots: for (index, (_, facts)) in self.roots.iter_mut().enumerate() {
            for batch in writes[index].chunks(BATCH_SIZE) {
                if cancel.load(Ordering::SeqCst) {
                    report.cancelled = true;
                    break 'roots;
                }
                facts.upsert(batch);
            }
            if !missing_by_root[index].is_empty() {
                report.removed += facts.remove_paths(&missing_by_root[index]);
            }
        }
        report
    }
}

/// One file a pass reads, and the roots whose stores hold its row.
struct Job {
    path: PathBuf,
    owners: Vec<usize>,
}

enum Outcome {
    /// Not read: the pass was cancelled first.
    Skipped,
    Missing,
    Failed(String),
    /// Read and hashed; `facts` is set when some owning root's row differs.
    Read { hash: u64, facts: Option<FileFacts> },
}

/// Reads, hashes, and (where a row differs) parses every job across a worker
/// pool, returning one outcome per job in job order and whether `cancel` was
/// seen.
fn read_all(
    jobs: &[Job],
    stored: &[HashMap<PathBuf, u64>],
    known: &BTreeMap<PathBuf, DocumentIndex>,
    progress: &dyn ProgressReporter,
    cancel: &AtomicBool,
) -> (Vec<Outcome>, bool) {
    let total = jobs.len();
    let mut outcomes: Vec<Outcome> = (0..total).map(|_| Outcome::Skipped).collect();
    if total == 0 {
        return (outcomes, cancel.load(Ordering::SeqCst));
    }
    let workers = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1).min(total).max(1);
    let (work_tx, work_rx) = unbounded::<usize>();
    for index in 0..total {
        let _ = work_tx.send(index);
    }
    drop(work_tx);
    let (result_tx, result_rx) = unbounded::<(usize, Outcome)>();
    std::thread::scope(|scope| {
        for _ in 0..workers {
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();
            scope.spawn(move || {
                while let Ok(index) = work_rx.recv() {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let _ = result_tx.send((index, read_one(&jobs[index], stored, known)));
                }
            });
        }
        drop(work_rx);
        drop(result_tx);
        let mut done = 0;
        while let Ok((index, outcome)) = result_rx.recv() {
            outcomes[index] = outcome;
            done += 1;
            progress.report(done, total);
        }
    });
    (outcomes, cancel.load(Ordering::SeqCst))
}

fn read_one(job: &Job, stored: &[HashMap<PathBuf, u64>], known: &BTreeMap<PathBuf, DocumentIndex>) -> Outcome {
    let bytes = match crate::work_counters::read(&job.path) {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Outcome::Missing,
        Err(error) => return Outcome::Failed(error.to_string()),
    };
    let hash = crate::work_counters::hash_bytes(&bytes);
    let current = job.owners.iter().all(|&owner| stored[owner].get(&job.path) == Some(&hash));
    if current {
        return Outcome::Read { hash, facts: None };
    }
    if let Some(parse) = known.get(&job.path).filter(|parse| parse.content_hash == hash) {
        return Outcome::Read { hash, facts: Some(FileFacts::from_index(parse)) };
    }
    match String::from_utf8(bytes) {
        Ok(source) => Outcome::Read { hash, facts: Some(FileFacts::from_index(&index_document(&job.path, &source))) },
        Err(error) => Outcome::Failed(error.utf8_error().to_string()),
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;

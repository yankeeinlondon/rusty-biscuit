//! Workspace listing and full-index parsing on a small worker pool.
//!
//! Server startup only lists files ([`list_files`]); Markdown is parsed when
//! a feature needs it. A full-index build hands the list to a fixed pool of
//! worker threads (crossbeam channels, no async runtime — AD-3) that read and
//! parse in parallel ([`parse_full_index_round`], and
//! [`collect_indices_from_files`] for the bench); results assemble into one
//! immutable snapshot. Progress is reported through the
//! [`ProgressReporter`] seam so the protocol loop can drive
//! `window/workDoneProgress` while tests use a silent collector.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;

use crate::config::WorkspaceConfig;
use crate::graph::{
    DocumentContexts, DocumentIndex, IndexTimings, WorkspaceIndex, index_document_timed,
};

use super::discover::{DiscoveryReport, discover_workspace};

/// Aggregate per-stage indexing timings, summed across the worker pool.
///
/// Because indexing is parallel these are aggregate CPU time per stage, not
/// disjoint wall-clock slices (the stages overlap across workers). The bench
/// harness reports them as the R-6 `read`/`hash`/`frontmatter`/`parse_markdown`
/// /`directives` stages so a bench run can attribute full-index cost.
#[derive(Debug, Default, Clone, Copy)]
pub struct IndexStageTimings {
    /// Reading file contents from disk.
    pub read: Duration,
    /// Content hashing (xxHash identity).
    pub hash: Duration,
    /// Frontmatter block extraction + span-aware AST read.
    pub frontmatter: Duration,
    /// Markdown parsing (headings, links, images, wiki links).
    pub parse_markdown: Duration,
    /// Darkmatter DSL scanning (directives, `{{ }}` interpolation).
    pub directives: Duration,
}

impl IndexStageTimings {
    fn accumulate(&mut self, read: Duration, index: IndexTimings) {
        self.read += read;
        self.hash += index.hash;
        self.frontmatter += index.frontmatter;
        self.parse_markdown += index.parse_markdown;
        self.directives += index.directives;
    }
}

/// A sink for indexing progress. Implementations drive
/// `window/workDoneProgress` (protocol loop) or collect for tests.
pub trait ProgressReporter: Send + Sync {
    /// Emitted once before discovery, so the client sees activity within the
    /// first-notification budget.
    fn begin(&self, title: &str);
    /// Emitted as files finish (`done` of `total`).
    fn report(&self, done: usize, total: usize);
    /// Emitted once when indexing completes.
    fn end(&self);
}

/// A progress reporter that does nothing (bench mode, tests without a client).
#[derive(Debug, Default)]
pub struct SilentProgress;

impl ProgressReporter for SilentProgress {
    fn begin(&self, _title: &str) {}
    fn report(&self, _done: usize, _total: usize) {}
    fn end(&self) {}
}

/// Discovers and indexes a workspace, returning the assembled index.
///
/// Reads files from disk (a workspace scan is not a passive LSP request, so
/// disk I/O is expected); it never composes, executes shell, or fetches, so no
/// child process is spawned.
pub fn index_workspace(
    roots: &[PathBuf],
    config: &WorkspaceConfig,
    reporter: &dyn ProgressReporter,
    contexts: Arc<dyn DocumentContexts>,
) -> WorkspaceIndex {
    WorkspaceIndex::from_indices(collect_indices(roots, config, reporter), contexts)
}

/// Indexes an already-discovered file list, returning the per-document map and
/// aggregate per-stage timings.
///
/// The bench harness uses this so discovery runs exactly once (it walks the
/// workspace itself, then hands the file list here) rather than discovering a
/// second time inside indexing.
pub fn collect_indices_from_files(
    files: &[PathBuf],
    reporter: &dyn ProgressReporter,
) -> (BTreeMap<PathBuf, DocumentIndex>, IndexStageTimings) {
    reporter.begin("Indexing workspace");
    let out = parallel_index(files, reporter, &AtomicBool::new(false));
    reporter.end();
    (out.parsed, out.stages)
}

/// What one full-index build round read and parsed.
#[derive(Debug, Default)]
pub struct FullIndexRound {
    /// The files that were read and parsed.
    pub parsed: BTreeMap<PathBuf, DocumentIndex>,
    /// The files that could not be read as UTF-8 text, with the error. Any
    /// entry fails the build; see [`WorkspaceIndex::finish_full_build`].
    pub failed: Vec<(PathBuf, String)>,
    /// Whether the round stopped early because its build was cancelled; its
    /// results are then incomplete and must be discarded.
    pub cancelled: bool,
}

/// Reads and parses `files` for a full-index build, reporting each finished
/// file to `reporter` (without `begin`/`end`, which span the whole build).
///
/// `cancel` is checked between files, so a cancelled build stops promptly
/// without aborting a parse midway.
pub fn parse_full_index_round(
    files: &[PathBuf],
    reporter: &dyn ProgressReporter,
    cancel: &AtomicBool,
) -> FullIndexRound {
    let out = parallel_index(files, reporter, cancel);
    FullIndexRound {
        parsed: out.parsed,
        failed: out.failed,
        cancelled: cancel.load(Ordering::SeqCst),
    }
}

/// Discovers and parses a workspace into a per-document index map, without
/// assembling the graph ([`index_workspace`] assembles it).
pub fn collect_indices(
    roots: &[PathBuf],
    config: &WorkspaceConfig,
    reporter: &dyn ProgressReporter,
) -> BTreeMap<PathBuf, DocumentIndex> {
    reporter.begin("Indexing workspace");
    let report = list_files(roots, config);
    let out = parallel_index(&report.files, reporter, &AtomicBool::new(false));
    reporter.end();
    out.parsed
}

/// Lists a workspace's files without reading them, logging each duplicate
/// physical file at `warn` and an incomplete walk at `warn`.
pub fn list_files(roots: &[PathBuf], config: &WorkspaceConfig) -> DiscoveryReport {
    let report = discover_workspace(roots, config);
    for duplicate in &report.duplicates {
        tracing::warn!("skipping duplicate physical file {}", duplicate.display());
    }
    if !report.is_complete() {
        tracing::warn!(
            errors = report.errors.len(),
            first_error = report.errors.first().map(String::as_str).unwrap_or_default(),
            "workspace discovery incomplete"
        );
    }
    report
}

/// What [`parallel_index`] produced.
struct ParallelIndex {
    parsed: BTreeMap<PathBuf, DocumentIndex>,
    failed: Vec<(PathBuf, String)>,
    stages: IndexStageTimings,
}

/// Reads and parses `files` across a worker pool, reporting progress and
/// accumulating per-stage timings. Workers stop taking files once `cancel`
/// is set.
fn parallel_index(
    files: &[PathBuf],
    reporter: &dyn ProgressReporter,
    cancel: &AtomicBool,
) -> ParallelIndex {
    let total = files.len();
    let mut out = ParallelIndex {
        parsed: BTreeMap::new(),
        failed: Vec::new(),
        stages: IndexStageTimings::default(),
    };
    if total == 0 {
        reporter.report(0, 0);
        return out;
    }

    let worker_count = worker_count(total);
    let (work_tx, work_rx) = unbounded::<PathBuf>();
    for file in files {
        let _ = work_tx.send(file.clone());
    }
    drop(work_tx);
    type IndexResult = Result<(DocumentIndex, Duration, IndexTimings), String>;
    let (result_tx, result_rx) = unbounded::<(PathBuf, IndexResult)>();

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();
            scope.spawn(move || {
                while let Ok(path) = work_rx.recv() {
                    if cancel.load(Ordering::SeqCst) {
                        break;
                    }
                    let index = read_and_index(&path);
                    let _ = result_tx.send((path, index));
                }
            });
        }
        // Drop the loop thread's copies so `recv` terminates once every worker
        // finishes.
        drop(work_rx);
        drop(result_tx);

        let mut done = 0;
        while let Ok((path, result)) = result_rx.recv() {
            match result {
                Ok((index, read, timings)) => {
                    out.parsed.insert(path, index);
                    out.stages.accumulate(read, timings);
                }
                Err(error) => {
                    tracing::debug!("unreadable {}: {error}", path.display());
                    out.failed.push((path, error));
                }
            }
            done += 1;
            reporter.report(done, total);
        }
    });

    out
}

/// Reads one file and indexes it, capturing the read and per-stage parse
/// timings; the read error when it is unreadable or not valid UTF-8.
fn read_and_index(path: &Path) -> Result<(DocumentIndex, Duration, IndexTimings), String> {
    let read_start = Instant::now();
    let source = crate::work_counters::read_to_string(path).map_err(|error| error.to_string())?;
    let read = read_start.elapsed();
    let (index, timings) = index_document_timed(path, &source);
    Ok((index, read, timings))
}

/// Worker count: at least one, at most the available parallelism, never more
/// than the file count.
fn worker_count(files: usize) -> usize {
    let cores = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    cores.min(files).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;
    use std::sync::atomic::AtomicUsize;

    fn write(root: &Path, rel: &str, body: &str) {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, body).unwrap();
    }

    #[derive(Default)]
    struct RecordingProgress {
        begins: AtomicUsize,
        ends: AtomicUsize,
        last: Mutex<(usize, usize)>,
    }

    impl ProgressReporter for RecordingProgress {
        fn begin(&self, _title: &str) {
            self.begins.fetch_add(1, Ordering::SeqCst);
        }
        fn report(&self, done: usize, total: usize) {
            *self.last.lock().unwrap() = (done, total);
        }
        fn end(&self) {
            self.ends.fetch_add(1, Ordering::SeqCst);
        }
    }

    #[test]
    fn test_index_workspace_builds_graph_and_reports_progress() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "a.md", "# A\n\n[to b](b.md)\n");
        write(root, "b.md", "# B\n");
        write(root, "ignore.txt", "no");

        let progress = RecordingProgress::default();
        let index = index_workspace(
            &[root.to_path_buf()],
            &WorkspaceConfig::default(),
            &progress,
            Arc::new(crate::graph::NoContexts),
        );

        assert_eq!(index.len(), 2);
        assert_eq!(index.snapshot().document_count(), 2);
        assert_eq!(progress.begins.load(Ordering::SeqCst), 1);
        assert_eq!(progress.ends.load(Ordering::SeqCst), 1);
        assert_eq!(*progress.last.lock().unwrap(), (2, 2));
    }

    #[test]
    fn test_empty_workspace_indexes_cleanly() {
        let temp = tempfile::tempdir().unwrap();
        let index = index_workspace(
            &[temp.path().to_path_buf()],
            &WorkspaceConfig::default(),
            &SilentProgress,
            Arc::new(crate::graph::NoContexts),
        );
        assert!(index.is_empty());
    }

    #[test]
    fn test_a_full_index_round_reports_unreadable_files() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        write(root, "good.md", "# Good\n");
        std::fs::write(root.join("binary.md"), [0xff, 0xfe, 0x00]).unwrap();
        let files = vec![root.join("good.md"), root.join("binary.md"), root.join("missing.md")];

        let progress = RecordingProgress::default();
        let round = parse_full_index_round(&files, &progress, &AtomicBool::new(false));
        assert!(!round.cancelled);
        assert_eq!(round.parsed.keys().cloned().collect::<Vec<_>>(), vec![root.join("good.md")]);
        let mut failed: Vec<PathBuf> = round.failed.iter().map(|(path, _)| path.clone()).collect();
        failed.sort();
        assert_eq!(failed, vec![root.join("binary.md"), root.join("missing.md")]);
        // A round reports each file, but `begin`/`end` belong to the build.
        assert_eq!(*progress.last.lock().unwrap(), (3, 3));
        assert_eq!(progress.begins.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_a_cancelled_round_reads_nothing_more() {
        let temp = tempfile::tempdir().unwrap();
        write(temp.path(), "a.md", "# A\n");
        let before = crate::work_counters::work_counts();
        let round = parse_full_index_round(
            &[temp.path().join("a.md")],
            &SilentProgress,
            &AtomicBool::new(true),
        );
        assert!(round.cancelled);
        assert!(round.parsed.is_empty() && round.failed.is_empty());
        assert_eq!(crate::work_counters::work_counts().since(before).reads, 0);
    }

    #[test]
    fn test_worker_count_bounds() {
        assert_eq!(worker_count(0).max(1), 1);
        assert!(worker_count(1) >= 1);
        assert!(worker_count(1) <= 1);
    }
}

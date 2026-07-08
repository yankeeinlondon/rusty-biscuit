//! Startup workspace indexing on a small worker pool.
//!
//! Discovery hands a file list to a fixed pool of worker threads (crossbeam
//! channels, no async runtime — AD-3) that read and parse in parallel; results
//! assemble into one immutable snapshot. Progress is reported through the
//! [`ProgressReporter`] seam so the protocol loop can drive
//! `window/workDoneProgress` while tests use a silent collector. The first
//! progress signal is emitted *before* the walk so the ≤250 ms
//! first-notification budget (R-6) does not wait on discovery.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use crossbeam_channel::unbounded;

use crate::config::WorkspaceConfig;
use crate::graph::{DocumentIndex, IndexTimings, WorkspaceIndex, index_document_timed};

use super::discover::{DiscoveryReport, discover_workspace};

/// Aggregate per-stage indexing timings, summed across the worker pool.
///
/// Because indexing is parallel these are aggregate CPU time per stage, not
/// disjoint wall-clock slices (the stages overlap across workers). The bench
/// harness reports them as the R-6 `read`/`hash`/`frontmatter`/`parse_markdown`
/// /`directives` stages so a bench run can attribute cold-start cost.
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
) -> WorkspaceIndex {
    WorkspaceIndex::from_indices(collect_indices(roots, config, reporter))
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
    let out = parallel_index(files, reporter);
    reporter.end();
    out
}

/// Discovers and parses a workspace into a per-document index map, without
/// assembling the graph.
///
/// The protocol loop uses this so it can *merge* the disk-derived documents
/// under any already-open buffers (open buffers are authoritative) before a
/// single snapshot build.
pub fn collect_indices(
    roots: &[PathBuf],
    config: &WorkspaceConfig,
    reporter: &dyn ProgressReporter,
) -> BTreeMap<PathBuf, DocumentIndex> {
    reporter.begin("Indexing workspace");
    let report = discover_workspace(roots, config);
    for duplicate in &report.duplicates {
        tracing::warn!("skipping duplicate physical file {}", duplicate.display());
    }
    let (indices, _timings) = parallel_index(&report.files, reporter);
    reporter.end();
    indices
}

/// Discovers a workspace without indexing (for callers that only need the file
/// list, e.g. the rescan fallback).
pub fn discover(roots: &[PathBuf], config: &WorkspaceConfig) -> DiscoveryReport {
    discover_workspace(roots, config)
}

/// Reads and parses `files` across a worker pool, reporting progress and
/// accumulating per-stage timings.
fn parallel_index(
    files: &[PathBuf],
    reporter: &dyn ProgressReporter,
) -> (BTreeMap<PathBuf, DocumentIndex>, IndexStageTimings) {
    let total = files.len();
    let mut map: BTreeMap<PathBuf, DocumentIndex> = BTreeMap::new();
    let mut stages = IndexStageTimings::default();
    if total == 0 {
        reporter.report(0, 0);
        return (map, stages);
    }

    let worker_count = worker_count(total);
    let (work_tx, work_rx) = unbounded::<PathBuf>();
    for file in files {
        let _ = work_tx.send(file.clone());
    }
    drop(work_tx);
    type IndexResult = Option<(DocumentIndex, Duration, IndexTimings)>;
    let (result_tx, result_rx) = unbounded::<(PathBuf, IndexResult)>();

    std::thread::scope(|scope| {
        for _ in 0..worker_count {
            let work_rx = work_rx.clone();
            let result_tx = result_tx.clone();
            scope.spawn(move || {
                while let Ok(path) = work_rx.recv() {
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
            if let Some((index, read, timings)) = result {
                map.insert(path, index);
                stages.accumulate(read, timings);
            }
            done += 1;
            reporter.report(done, total);
        }
    });

    (map, stages)
}

/// Reads one file and indexes it, capturing the read and per-stage parse
/// timings; `None` when unreadable or not valid UTF-8.
fn read_and_index(path: &Path) -> Option<(DocumentIndex, Duration, IndexTimings)> {
    let read_start = Instant::now();
    let source = match std::fs::read_to_string(path) {
        Ok(source) => source,
        Err(error) => {
            tracing::debug!("skipping unreadable {}: {error}", path.display());
            return None;
        }
    };
    let read = read_start.elapsed();
    let (index, timings) = index_document_timed(path, &source);
    Some((index, read, timings))
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
    use std::sync::atomic::{AtomicUsize, Ordering};

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
        let index = index_workspace(&[root.to_path_buf()], &WorkspaceConfig::default(), &progress);

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
        );
        assert!(index.is_empty());
    }

    #[test]
    fn test_worker_count_bounds() {
        assert_eq!(worker_count(0).max(1), 1);
        assert!(worker_count(1) >= 1);
        assert!(worker_count(1) <= 1);
    }
}

use super::docs::{self, MarkdownMeta};
use super::file_types::{
    self, FileClassification, FileInventory, FileScanScope, MAX_FILES, should_skip_directory_name,
};
use super::repo;
use super::repo::manifest_index::{ManifestIndex, is_fixture_manifest, is_generated_manifest};
use crate::performance;
use crate::performance::counters;
use ignore::{DirEntry, WalkBuilder, WalkState};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use std::time::Instant;
use tracing::Level;

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct SharedWalkOptions {
    pub(crate) collect_manifests: bool,
    pub(crate) collect_inventory: bool,
    pub(crate) collect_docs: bool,
    pub(crate) collect_nested_markers: bool,
}

impl SharedWalkOptions {
    /// The evidence set a full repository observation needs.
    pub(crate) fn full_repo() -> Self {
        Self {
            collect_manifests: true,
            collect_inventory: true,
            collect_docs: false,
            collect_nested_markers: true,
        }
    }

    /// Whether any observer other than the file inventory is still active.
    ///
    /// Governs whether the walk may quit at inventory saturation: the
    /// inventory cap bounds classification, never manifest, marker, or
    /// document discovery.
    fn has_observer_beyond_inventory(&self) -> bool {
        self.collect_manifests || self.collect_docs || self.collect_nested_markers
    }
}

/// One request's compact evidence about a directory tree.
///
/// ## Notes
///
/// Retains only what a consumer declared it needs. Directory entries and file
/// bodies are deliberately absent: the walk observes each entry once, extracts
/// the evidence, and drops it. There is no process-global cache — the view's
/// lifetime is the request's.
#[derive(Debug, Clone)]
pub(crate) struct FilesystemSystemView {
    pub(crate) root: PathBuf,
    pub(crate) manifest_index: Option<ManifestIndex>,
    /// Directories holding a package manifest, exactly as observed.
    ///
    /// Distinct from `manifest_index`, which drops generated and fixture
    /// manifests because they are not discovery boundaries. Membership globs
    /// resolve a boundary by marker presence alone and have never applied that
    /// exclusion, so they need the unfiltered set.
    pub(crate) manifest_dirs: Option<Vec<PathBuf>>,
    /// Observed nested-workspace marker files, including `*.sln` solutions.
    pub(crate) nested_markers: Option<Vec<PathBuf>>,
    pub(crate) inventory: Option<FileInventory>,
    pub(crate) docs: Option<Vec<MarkdownMeta>>,
}

#[derive(Default)]
struct SharedWalkAccumulator {
    manifest_paths: Vec<PathBuf>,
    manifest_dirs: Vec<PathBuf>,
    nested_markers: Vec<PathBuf>,
    classifications: Vec<FileClassification>,
    docs: Vec<MarkdownMeta>,
}

/// Per-worker state for one `build_parallel` thread.
///
/// ## Notes
///
/// Constructed on the spawning thread and dropped on the worker thread, which
/// is what makes both the result append and the performance flush correct: a
/// worker's classifications and its counters land in the request's output at
/// the same moment, so a report can never show entries a worker's results
/// omitted.
struct WorkerBuffers {
    shared: Arc<Mutex<SharedWalkAccumulator>>,
    collector: performance::WorkerCollector,
    manifests: Vec<PathBuf>,
    manifest_dirs: Vec<PathBuf>,
    nested_markers: Vec<PathBuf>,
    classifications: Vec<FileClassification>,
    docs: Vec<MarkdownMeta>,
}

impl WorkerBuffers {
    fn new(shared: Arc<Mutex<SharedWalkAccumulator>>) -> Self {
        Self {
            shared,
            collector: performance::WorkerCollector::inherit(),
            manifests: Vec::new(),
            manifest_dirs: Vec::new(),
            nested_markers: Vec::new(),
            classifications: Vec::new(),
            docs: Vec::new(),
        }
    }
}

impl Drop for WorkerBuffers {
    fn drop(&mut self) {
        let mut shared = self
            .shared
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        shared.manifest_paths.append(&mut self.manifests);
        shared.manifest_dirs.append(&mut self.manifest_dirs);
        shared.nested_markers.append(&mut self.nested_markers);
        shared.classifications.append(&mut self.classifications);
        shared.docs.append(&mut self.docs);
    }
}

pub(crate) fn build_filesystem_system_view(
    root: &Path,
    options: SharedWalkOptions,
) -> FilesystemSystemView {
    let started = Instant::now();
    let shared = Arc::new(Mutex::new(SharedWalkAccumulator::default()));
    let scanned_files = Arc::new(AtomicUsize::new(0));
    performance::increment_counter(counters::FS_WALK_STARTS, 1);

    WalkBuilder::new(root)
        .hidden(false)
        .git_ignore(true)
        .git_global(true)
        .git_exclude(true)
        .filter_entry(|entry| !is_ignored_directory(entry))
        .build_parallel()
        .run(|| {
            let shared = Arc::clone(&shared);
            let scanned_files = Arc::clone(&scanned_files);
            let scan_root = root.to_path_buf();
            let mut worker = WorkerBuffers::new(shared);
            Box::new(move |result| {
                // The first callback is the earliest point that runs on this
                // worker's own thread, which is where the collector must live.
                worker.collector.activate();
                performance::increment_counter(counters::FS_WALK_ENTRIES, 1);

                let Ok(entry) = result else {
                    return WalkState::Continue;
                };

                if !entry
                    .file_type()
                    .is_some_and(|file_type| file_type.is_file())
                {
                    return WalkState::Continue;
                }

                process_file_entry(
                    entry.path(),
                    &scan_root,
                    &options,
                    &mut worker,
                    &scanned_files,
                )
            })
        });

    let mut accumulated = shared
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    accumulated
        .classifications
        .sort_by(|a, b| a.path.cmp(&b.path));
    accumulated.docs.sort_by(|a, b| a.relative.cmp(&b.relative));
    accumulated.manifest_paths.sort();
    accumulated.nested_markers.sort();
    // Several manifests can share a directory (`Cargo.toml` beside
    // `package.json`), so the parents collected per entry are deduped here
    // rather than at every query.
    accumulated.manifest_dirs.sort();
    accumulated.manifest_dirs.dedup();

    performance::record_logged_stage("filesystem.shared_walk", started.elapsed(), Level::DEBUG);

    // See `scan_inventory_parallel`: exceeding the cap is what proves a file
    // went unrepresented; a tree of exactly `MAX_FILES` files is complete.
    let truncated = scanned_files.load(Ordering::Relaxed) > MAX_FILES;

    let inventory = options.collect_inventory.then(|| FileInventory {
        scope: FileScanScope {
            root: root.to_path_buf(),
            exclude_roots: Vec::new(),
        },
        total_files_scanned: accumulated.classifications.len(),
        classifications: Arc::new(std::mem::take(&mut accumulated.classifications)),
        truncated,
        limit: truncated.then_some(MAX_FILES),
    });

    let docs = options.collect_docs.then(|| {
        let docs = std::mem::take(&mut accumulated.docs);
        if docs.is_empty() { None } else { Some(docs) }
    });

    let manifest_index = options.collect_manifests.then(|| {
        ManifestIndex::from_manifest_paths(std::mem::take(&mut accumulated.manifest_paths))
    });
    let manifest_dirs = options
        .collect_manifests
        .then(|| std::mem::take(&mut accumulated.manifest_dirs));
    let nested_markers = options
        .collect_nested_markers
        .then(|| std::mem::take(&mut accumulated.nested_markers));

    FilesystemSystemView {
        root: root.to_path_buf(),
        manifest_index,
        manifest_dirs,
        nested_markers,
        inventory,
        docs: docs.flatten(),
    }
}

/// Observe one file entry for every active observer.
///
/// ## Notes
///
/// Returns [`WalkState::Quit`] once the inventory cap is reached **only** when
/// inventory is the sole active observer. When manifests or docs are also being
/// collected the walk must continue — quitting would silently truncate the
/// manifest index and the document list, turning an inventory bound into a
/// wrong repository structure. Classification and its counters stop either way.
fn process_file_entry(
    path: &Path,
    root: &Path,
    options: &SharedWalkOptions,
    worker: &mut WorkerBuffers,
    scanned_files: &Arc<AtomicUsize>,
) -> WalkState {
    if options.collect_inventory {
        let index = scanned_files.fetch_add(1, Ordering::Relaxed);
        if index < MAX_FILES {
            let entry_timer = performance::StageTimer::start("filesystem.file_inventory.walk.entry");
            performance::increment_counter(counters::FS_INVENTORY_ACCEPTED, 1);
            worker
                .classifications
                .push(file_types::classify_file(root, path));
            entry_timer.finish();
        } else {
            performance::increment_counter(counters::FS_INVENTORY_SATURATED, 1);
            if !options.has_observer_beyond_inventory() {
                return WalkState::Quit;
            }
        }
    }

    if options.collect_docs
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        let doc_timer = performance::StageTimer::start("filesystem.shared_walk.docs");
        performance::increment_counter(counters::FS_DOCS_PARSED, 1);
        if let Some(doc) = docs::parse_markdown_meta(path, root, &[]) {
            worker.docs.push(doc);
        }
        doc_timer.finish();
    }

    if options.collect_manifests && is_manifest_path(path) {
        // The directory is evidence regardless of the manifest's content: a
        // membership glob resolves a boundary by marker presence, where the
        // index below is a discovery set and drops generated/fixture manifests.
        if let Some(parent) = path.parent() {
            worker.manifest_dirs.push(parent.to_path_buf());
        }
        // Both predicates read the manifest, so they stay on the worker rather
        // than moving to index construction, which is serial.
        if !is_generated_manifest(path) && !is_fixture_manifest(path) {
            worker.manifests.push(path.to_path_buf());
        }
    }

    if options.collect_nested_markers && repo::nested::is_nested_marker_path(path) {
        worker.nested_markers.push(path.to_path_buf());
    }

    WalkState::Continue
}

fn is_manifest_path(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|value| value.to_str()),
        Some("Cargo.toml" | "package.json" | "pyproject.toml" | "go.mod")
    )
}

fn is_ignored_directory(entry: &DirEntry) -> bool {
    if !entry
        .file_type()
        .is_some_and(|file_type| file_type.is_dir())
    {
        return false;
    }

    entry
        .file_name()
        .to_str()
        .is_some_and(should_skip_directory_name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::performance::testing;
    use std::fs;
    use tempfile::TempDir;

    /// Files written per directory by [`fixture`].
    const FILES_PER_DIR: usize = 12;
    /// Directories written by [`fixture`].
    const DIR_COUNT: usize = 8;

    /// A tree wide enough that `build_parallel` uses more than one worker, so
    /// the accounting assertions actually exercise cross-worker aggregation
    /// rather than a single-threaded degenerate case.
    fn fixture() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        for d in 0..DIR_COUNT {
            let sub = dir.path().join(format!("pkg{d:02}"));
            fs::create_dir_all(&sub).expect("create dir");
            for f in 0..FILES_PER_DIR {
                fs::write(sub.join(format!("file_{f}.rs")), b"pub fn f() {}\n").expect("write");
            }
        }
        dir
    }

    fn inventory_options() -> SharedWalkOptions {
        SharedWalkOptions {
            collect_inventory: true,
            ..Default::default()
        }
    }

    /// Manifests written by [`saturating_fixture`], one per directory.
    const SATURATING_MANIFESTS: usize = 6;

    /// A tree with more files than the inventory cap, plus a manifest in every
    /// directory.
    ///
    /// The manifests are what make the combined-walk assertion meaningful:
    /// whichever ones happen to be enumerated after saturation are exactly the
    /// evidence a walk that quit at the cap would lose.
    fn saturating_fixture() -> TempDir {
        let dir = TempDir::new().expect("tempdir");
        let per_dir = (MAX_FILES / SATURATING_MANIFESTS) + 200;
        for d in 0..SATURATING_MANIFESTS {
            let sub = dir.path().join(format!("pkg{d:02}"));
            fs::create_dir_all(&sub).expect("create dir");
            fs::write(
                sub.join("Cargo.toml"),
                format!("[package]\nname = \"pkg{d:02}\"\n").as_bytes(),
            )
            .expect("write manifest");
            for f in 0..per_dir {
                fs::write(sub.join(format!("file_{f}.rs")), b"pub fn f() {}\n").expect("write");
            }
        }
        dir
    }

    #[test]
    fn inventory_only_walk_stops_at_the_cap_and_reports_truncation() {
        let dir = saturating_fixture();
        let (view, counts) =
            testing::measure(|| build_filesystem_system_view(dir.path(), inventory_options()));
        let inventory = view.inventory.expect("inventory requested");

        assert!(inventory.truncated, "a tree above the cap must report truncation");
        assert_eq!(inventory.limit, Some(MAX_FILES));
        assert_eq!(
            inventory.total_files_scanned, MAX_FILES,
            "the represented count is the cap, not the tree's file count"
        );
        assert_eq!(inventory.classifications.len(), MAX_FILES);
        assert_eq!(
            counts.get(counters::FS_INVENTORY_ACCEPTED),
            MAX_FILES as u64,
            "classification must stop at the cap"
        );
        assert!(
            counts.get(counters::FS_INVENTORY_SATURATED) > 0,
            "the over-cap counter and the truncated flag must never disagree"
        );

        // The subset is unspecified across runs, so assert the properties that
        // *are* promised rather than an exact set of paths.
        let mut sorted = inventory.classifications.to_vec();
        sorted.sort_by(|a, b| a.path.cmp(&b.path));
        assert_eq!(
            sorted.iter().map(|c| &c.path).collect::<Vec<_>>(),
            inventory.classifications.iter().map(|c| &c.path).collect::<Vec<_>>(),
            "classifications must be sorted by path"
        );
        assert!(
            inventory
                .classifications
                .iter()
                .all(|c| c.path.is_relative() && c.path.file_name().is_some()),
            "every retained path must remain a valid relative native path"
        );
    }

    #[test]
    fn combined_walk_keeps_going_past_saturation_for_its_other_observers() {
        let dir = saturating_fixture();
        let options = SharedWalkOptions {
            collect_manifests: true,
            collect_inventory: true,
            ..Default::default()
        };
        let (view, counts) = testing::measure(|| build_filesystem_system_view(dir.path(), options));

        assert!(
            view.inventory.expect("inventory requested").truncated,
            "the fixture must actually saturate, or this test proves nothing"
        );
        assert_eq!(
            counts.get(counters::FS_INVENTORY_ACCEPTED),
            MAX_FILES as u64,
            "classification still stops at the cap"
        );

        // The real assertion: quitting at the cap would silently drop whichever
        // manifests sort after it, turning an inventory bound into a wrong
        // repository structure.
        let manifests = view.manifest_index.expect("manifests requested");
        assert_eq!(
            manifests.entries.len(),
            SATURATING_MANIFESTS,
            "every manifest must be found even though the inventory saturated"
        );
    }

    #[test]
    fn parallel_walk_counts_every_accepted_file_exactly_once() {
        let dir = fixture();
        let (view, counts) =
            testing::measure(|| build_filesystem_system_view(dir.path(), inventory_options()));

        let classified = view.inventory.expect("inventory requested").total_files_scanned;
        assert_eq!(
            classified,
            DIR_COUNT * FILES_PER_DIR,
            "fixture should classify every file it wrote"
        );
        // The counter is the whole point of this phase: if any worker's buffer
        // were dropped when its thread parked, this would read short; if a
        // worker flushed twice it would read long.
        assert_eq!(
            counts.get(counters::FS_INVENTORY_ACCEPTED),
            classified as u64,
            "accepted-classification counter must equal the classifications in the result; \
             counters were {:?}",
            counts.all()
        );
    }

    #[test]
    fn parallel_walk_visits_at_least_every_file_it_accepts() {
        let dir = fixture();
        let (_view, counts) =
            testing::measure(|| build_filesystem_system_view(dir.path(), inventory_options()));

        let entries = counts.get(counters::FS_WALK_ENTRIES);
        let accepted = counts.get(counters::FS_INVENTORY_ACCEPTED);
        // Entries include directories and the root, so the walk necessarily
        // sees more than it accepts — but never fewer.
        assert!(
            entries >= accepted,
            "walk visited {entries} entries but accepted {accepted}"
        );
        assert!(
            entries >= (DIR_COUNT * FILES_PER_DIR + DIR_COUNT) as u64,
            "walk should visit every file and directory; visited {entries}"
        );
        assert_eq!(counts.get(counters::FS_WALK_STARTS), 1, "one shared walk per request");
    }

    #[test]
    fn parallel_and_serial_walks_agree_on_accepted_work() {
        let dir = fixture();
        let (parallel, parallel_counts) =
            testing::measure(|| build_filesystem_system_view(dir.path(), inventory_options()));
        let (serial, serial_counts) = testing::measure(|| {
            super::super::file_types::scan_file_inventory(dir.path()).expect("scan")
        });

        assert_eq!(
            parallel.inventory.expect("inventory").total_files_scanned,
            serial.total_files_scanned,
            "both walkers see the same tree"
        );
        assert_eq!(
            parallel_counts.get(counters::FS_INVENTORY_ACCEPTED),
            serial_counts.get(counters::FS_INVENTORY_ACCEPTED),
            "parallel accounting must match the independently-walked total"
        );
    }

    #[test]
    fn worker_counts_do_not_leak_into_a_later_request() {
        let dir = fixture();
        let (_view, first) =
            testing::measure(|| build_filesystem_system_view(dir.path(), inventory_options()));
        assert!(first.get(counters::FS_INVENTORY_ACCEPTED) > 0, "first request did work");

        // Walker threads park rather than exit, so a worker that failed to
        // drain would surface its stale buffer here — attributed to a request
        // that walked nothing.
        let ((), second) = testing::measure(|| {});
        assert_eq!(
            second.get(counters::FS_INVENTORY_ACCEPTED),
            0,
            "a request that walked nothing must record no walk work; counters were {:?}",
            second.all()
        );
        assert_eq!(second.get(counters::FS_WALK_ENTRIES), 0);
    }

    #[test]
    fn uncollected_walk_records_nothing_into_a_later_request() {
        let dir = fixture();

        // The default path: no collector installed anywhere.
        let view = performance::without_any_collector(|| {
            assert!(!performance::is_collecting(), "no collector should be active here");
            build_filesystem_system_view(dir.path(), inventory_options())
        });
        assert_eq!(
            view.inventory.expect("inventory").total_files_scanned,
            DIR_COUNT * FILES_PER_DIR,
            "disabling collection must not change results"
        );

        // Nothing the uncollected walk did may reappear once collection starts.
        let ((), counts) = testing::measure(|| {});
        assert_eq!(counts.get(counters::FS_INVENTORY_ACCEPTED), 0);
        assert_eq!(counts.get(counters::FS_WALK_ENTRIES), 0);
        assert!(
            !counts.recorded_any_stage(),
            "the disabled path must record no stage timings; stages leaked from an \
             uncollected walk would mean per-entry clocks are still running"
        );
    }
}

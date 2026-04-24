use super::docs::{self, MarkdownMeta};
use super::file_types::{
    self, FileClassification, FileInventory, FileScanScope, MAX_FILES, should_skip_directory_name,
};
use super::repo::detection::{is_fixture_manifest, is_generated_manifest};
use super::repo::types::ManifestIndex;
use crate::performance;
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
}

#[derive(Debug, Clone)]
pub(crate) struct FilesystemSystemView {
    pub(crate) root: PathBuf,
    pub(crate) manifest_index: Option<ManifestIndex>,
    pub(crate) inventory: Option<FileInventory>,
    pub(crate) docs: Option<Vec<MarkdownMeta>>,
}

#[derive(Default)]
struct SharedWalkAccumulator {
    manifest_paths: Vec<PathBuf>,
    classifications: Vec<FileClassification>,
    docs: Vec<MarkdownMeta>,
}

struct WorkerBuffers {
    shared: Arc<Mutex<SharedWalkAccumulator>>,
    manifests: Vec<PathBuf>,
    classifications: Vec<FileClassification>,
    docs: Vec<MarkdownMeta>,
}

impl WorkerBuffers {
    fn new(shared: Arc<Mutex<SharedWalkAccumulator>>) -> Self {
        Self {
            shared,
            manifests: Vec::new(),
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

    performance::record_logged_stage("filesystem.shared_walk", started.elapsed(), Level::DEBUG);

    let inventory = options.collect_inventory.then(|| FileInventory {
        scope: FileScanScope {
            root: root.to_path_buf(),
            exclude_roots: Vec::new(),
        },
        total_files_scanned: accumulated.classifications.len(),
        classifications: Arc::new(std::mem::take(&mut accumulated.classifications)),
    });

    let docs = options.collect_docs.then(|| {
        let docs = std::mem::take(&mut accumulated.docs);
        if docs.is_empty() { None } else { Some(docs) }
    });

    let manifest_index = options.collect_manifests.then(|| {
        ManifestIndex::from_manifest_paths(std::mem::take(&mut accumulated.manifest_paths))
    });

    FilesystemSystemView {
        root: root.to_path_buf(),
        manifest_index,
        inventory,
        docs: docs.flatten(),
    }
}

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
            let callback_started = Instant::now();
            performance::increment_counter("filesystem.file_inventory.files_scanned", 1);
            worker
                .classifications
                .push(file_types::classify_file(root, path));
            performance::record_stage(
                "filesystem.file_inventory.walk.entry",
                callback_started.elapsed(),
            );
        }
    }

    if options.collect_docs
        && path
            .extension()
            .and_then(|value| value.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        let doc_started = Instant::now();
        if let Some(doc) = docs::parse_markdown_meta(path, root, &[]) {
            worker.docs.push(doc);
        }
        performance::record_stage("filesystem.shared_walk.docs", doc_started.elapsed());
    }

    if options.collect_manifests
        && is_manifest_path(path)
        && !is_generated_manifest(path)
        && !is_fixture_manifest(path)
    {
        worker.manifests.push(path.to_path_buf());
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

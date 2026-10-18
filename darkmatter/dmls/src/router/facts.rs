//! The router's side of the index store: where the stores live, how disk
//! changes reach the rows, and the rename freshness rule.
//!
//! The facts open on the first build round, never at startup, and each
//! round's store pass runs on that round's thread
//! ([`crate::index_build::FactsPlan`]). On the dispatch thread:
//!
//! | Facts state | A changed Markdown file |
//! | --- | --- |
//! | not ready, no build | only the file list changes; the next build reads everything |
//! | a build is running | recorded; the next round re-reads it |
//! | ready, client watching | re-read, re-hashed, and its row upserted or deleted at once |
//! | ready, fallback watch mode | held until the next save, which re-reads the saved file and these |
//!
//! A changed file is one a watched event, a relist, or a rescan reported, or
//! an open document that closed (its disk bytes become authoritative again).
//!
//! References, heading rename, and file rename are answered from a graph
//! built for the request over the store's candidates
//! ([`ServerState::request_graph`]); workspace symbols read the stored
//! headings ([`ServerState::symbol_documents`]).

use std::collections::{BTreeMap, BTreeSet};
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use darkmatter::markdown::compose::RequestSnapshot;

use crate::graph::{DocumentIndex, WorkspaceGraph, WorkspaceIndex};
use crate::providers::symbols::SymbolDocument;
use crate::index_store::{PassReport, StoreLocation};
use crate::workspace::discover::effective_roots;
use crate::workspace::watch::WatchMode;

use super::ServerState;

/// The environment variable that moves the store directory, or disables
/// the stores with `off`.
const CACHE_DIR_VARIABLE: &str = "DMLS_CACHE_DIR";

/// The request snapshot's inputs to the store location, captured once at
/// startup so no later code reads the process.
#[derive(Debug, Clone)]
pub(super) struct StoreInputs {
    cache_dir_override: Option<String>,
    user_cache_dir: Option<PathBuf>,
}

impl StoreInputs {
    pub(super) fn from_snapshot(snapshot: &RequestSnapshot) -> Self {
        let env = snapshot.env();
        Self {
            cache_dir_override: env.get(CACHE_DIR_VARIABLE).cloned(),
            user_cache_dir: biscuit_file::user_cache_dir_from(|name| env.get(name).map(OsString::from), snapshot.home()),
        }
    }

    /// Prepares the store directory (creating it when missing).
    pub(super) fn location(&self) -> StoreLocation {
        StoreLocation::prepare(self.cache_dir_override.as_deref().map(OsStr::new), self.user_cache_dir.clone())
    }
}

/// The dispatch thread's view of the facts.
#[derive(Debug, Default)]
pub(super) struct FactsEvents {
    /// `Some` while a build runs: the Markdown paths that changed since its
    /// current round began.
    touched: Option<BTreeSet<PathBuf>>,
    /// Fallback watch mode, once ready: changed paths not yet re-read.
    pending: BTreeSet<PathBuf>,
}

impl FactsEvents {
    /// Starts recording changes for a build round.
    pub(super) fn begin_round(&mut self) {
        self.touched = Some(BTreeSet::new());
    }

    /// Ends a round, returning what changed while it read.
    pub(super) fn finish_round(&mut self) -> BTreeSet<PathBuf> {
        self.touched.take().unwrap_or_default()
    }

    /// Stops recording (the build ended without a next round).
    pub(super) fn end_build(&mut self) {
        self.touched = None;
    }
}

impl ServerState {
    /// The effective workspace roots, one store each.
    pub(super) fn store_roots(&self) -> Vec<PathBuf> {
        effective_roots(&self.roots, &self.config.effective().workspace)
    }

    /// Whether the facts are ready for the current file list. Always false
    /// while a build runs.
    pub(super) fn facts_ready(&self) -> bool {
        if self.facts_events.borrow().touched.is_some() {
            return false;
        }
        let roots = self.store_roots();
        let index = self.index.lock().expect("index lock poisoned");
        let facts = self.facts.lock().expect("facts lock poisoned");
        facts.as_ref().is_some_and(|facts| facts.is_ready(index.listed(), &roots))
    }

    /// Records Markdown paths whose disk state may have changed (see the
    /// [module docs](self)).
    pub(super) fn facts_changed(&self, paths: impl IntoIterator<Item = PathBuf>) {
        let paths: Vec<PathBuf> = paths.into_iter().filter(|path| crate::graph::is_markdown_path(path)).collect();
        if paths.is_empty() {
            return;
        }
        {
            let mut events = self.facts_events.borrow_mut();
            if let Some(touched) = events.touched.as_mut() {
                touched.extend(paths);
                return;
            }
        }
        if !self.facts_ready() {
            return;
        }
        match self.watch_mode {
            WatchMode::ClientWatched => {
                self.refresh_facts(&paths);
            }
            WatchMode::ServerRescan => self.facts_events.borrow_mut().pending.extend(paths),
        }
    }

    /// A fallback-mode save, once ready: re-reads the saved file and every
    /// file with a held change, and nothing else.
    pub(super) fn facts_saved(&self, saved: &Path) {
        if self.watch_mode != WatchMode::ServerRescan || !self.facts_ready() {
            return;
        }
        let mut paths: Vec<PathBuf> = std::mem::take(&mut self.facts_events.borrow_mut().pending).into_iter().collect();
        if crate::graph::is_markdown_path(saved) && !paths.iter().any(|path| path == saved) {
            paths.push(saved.to_path_buf());
        }
        self.refresh_facts(&paths);
    }

    /// Re-reads `paths` into the facts.
    pub(super) fn refresh_facts(&self, paths: &[PathBuf]) {
        // Rows exist only for listed files; a path that left the list is
        // re-read only to delete its row when it is gone from disk.
        let (paths, known) = {
            let index = self.index.lock().expect("index lock poisoned");
            let paths: Vec<PathBuf> = paths
                .iter()
                .filter(|path| index.listed().contains(*path) || std::fs::symlink_metadata(path).is_err())
                .cloned()
                .collect();
            let known = known_parses(&index, &paths);
            (paths, known)
        };
        if paths.is_empty() {
            return;
        }
        let report = {
            let mut facts = self.facts.lock().expect("facts lock poisoned");
            let Some(facts) = facts.as_mut() else {
                return;
            };
            facts.refresh(&paths, &known)
        };
        log_refresh(&report);
    }

    /// The rename freshness rule: before a rename of `targets` (the
    /// renamed heading's document, or the renamed files) is computed,
    /// re-reads each closed target and each closed candidate referrer, so a
    /// referrer whose bytes changed since it was last read is re-parsed and
    /// its edit lands on its current text. It runs before the request graph
    /// is built: an edit computed from a stale parse is dropped, not
    /// corrected, when the old text is no longer at its span.
    pub(super) fn refresh_rename_referrers(&self, targets: &[PathBuf]) {
        let candidates = {
            let index = self.index.lock().expect("index lock poisoned");
            let mut facts = self.facts.lock().expect("facts lock poisoned");
            match facts.as_mut() {
                Some(facts) => facts.candidates_for(targets, index.listed()),
                None => Vec::new(),
            }
        };
        let mut paths: BTreeSet<PathBuf> = candidates.into_iter().collect();
        paths.extend(targets.iter().cloned());
        let paths: Vec<PathBuf> = paths
            .into_iter()
            .filter(|path| crate::workspace::file_path_to_uri(path).is_none_or(|uri| !self.documents.is_open(&uri)))
            .collect();
        self.refresh_facts(&paths);
    }

    /// The graph a references, heading-rename, or file-rename request about
    /// `targets` is answered from (see
    /// [`WorkspaceFacts::request_graph`](crate::index_store::WorkspaceFacts::request_graph)):
    /// the candidates' stored facts, every open buffer, and each target the
    /// workspace index parsed outside the file list, resolved against the
    /// live snapshot's wiki universe through the shared repository contexts.
    pub(super) fn request_graph(&self, targets: &[PathBuf]) -> WorkspaceGraph {
        let snapshot = self.snapshot.load();
        let universe = snapshot.shared_wiki_universe();
        let index = self.index.lock().expect("index lock poisoned");
        let unlisted_targets = targets
            .iter()
            .filter(|target| !index.listed().contains(*target))
            .filter_map(|target| index.disk_parse(target));
        let overlay: Vec<&DocumentIndex> = index.open_buffers().chain(unlisted_targets).collect();
        let mut facts = self.facts.lock().expect("facts lock poisoned");
        match facts.as_mut() {
            Some(facts) => facts.request_graph(targets, index.listed(), overlay, universe, self.contexts.as_ref()),
            // Facts are open whenever these requests proceed; without them
            // the request sees only what the editor holds.
            None => {
                let documents: BTreeMap<PathBuf, DocumentIndex> =
                    overlay.into_iter().map(|document| (document.path.clone(), document.clone())).collect();
                WorkspaceGraph::build_with_roots(&documents, 0, universe, self.contexts.as_ref())
            }
        }
    }

    /// The documents workspace symbols read: every listed file's stored
    /// headings with every open buffer in place of its row.
    pub(super) fn symbol_documents(&self) -> Vec<SymbolDocument> {
        let index = self.index.lock().expect("index lock poisoned");
        let mut facts = self.facts.lock().expect("facts lock poisoned");
        match facts.as_mut() {
            Some(facts) => facts.symbol_documents(index.listed(), index.open_buffers()),
            None => {
                let mut documents: Vec<SymbolDocument> =
                    index.open_buffers().map(|buffer| (buffer.path.clone(), buffer.headings.clone())).collect();
                documents.sort_by(|left, right| left.0.cmp(&right.0));
                documents
            }
        }
    }
}

/// The parses of `paths` the workspace index holds, disk or open buffer,
/// which a pass reuses only when a parse's hash matches the bytes it reads.
fn known_parses(index: &WorkspaceIndex, paths: &[PathBuf]) -> BTreeMap<PathBuf, DocumentIndex> {
    let mut known: BTreeMap<PathBuf, DocumentIndex> = paths
        .iter()
        .filter_map(|path| index.disk_parse(path).map(|parse| (path.clone(), parse.clone())))
        .collect();
    for buffer in index.open_buffers().filter(|buffer| paths.contains(&buffer.path)) {
        known.insert(buffer.path.clone(), buffer.clone());
    }
    known
}

fn log_refresh(report: &PassReport) {
    tracing::debug!(
        read = report.read,
        reparsed = report.reparsed,
        removed = report.removed,
        failed = report.failed.len(),
        switched = report.switched,
        "index store refreshed"
    );
}

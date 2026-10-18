//! `dmls --bench-index <dir>`: the index store benchmark.
//!
//! It measures what the first workspace-wide request costs, in two halves:
//!
//! - **cold** ([`bench_cold`]): no store exists. List the files, open (create)
//!   the store, and fill it: read, hash, parse, and write every listed
//!   Markdown file.
//! - **warm** ([`bench_warm`]): the cold half's store is present and no file
//!   changed, as after an editor restart. List the files, open the store,
//!   revalidate (read and hash every file, parse none), then answer a
//!   references request, a heading rename, and a workspace-symbols request
//!   the way the server does: a small graph over the store's candidates, and
//!   the stored headings.
//!
//! The warm half reports the first references latency with and without the
//! file-list walk, and the resident memory serving those requests adds: the
//! process's peak resident set after the requests minus its peak after the
//! file-list walk. A peak counts every allocation the cold half made, so the
//! binary runs the warm half in a fresh process; [`bench_index`] runs both in
//! one process and its warm memory figure includes the cold half's.
//!
//! The store directory is the caller's; the binary uses a temporary one, so
//! a bench never reads or writes the editor's stores.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::capabilities::ClientProfile;
use crate::clock::SystemClock;
use crate::config::ConfigState;
use crate::context::RepositoryContexts;
use crate::graph::{DocumentIndex, WorkspaceGraph};
use crate::index_store::{PassReport, StoreLocation, WorkspaceFacts, store_path};
use crate::providers::{DocumentContext, ProviderRegistry};
use crate::source_map::{PositionEncoding, SourceMap};
use crate::wiki::WikiUniverse;
use crate::workspace::discover::effective_roots;
use crate::workspace::startup::{SilentProgress, list_files};
use crate::workspace::{DocumentStore, file_path_to_uri, resolve_wiki_roots};

/// The `workspace` field of the store's log lines during a bench.
const BENCH_WORKSPACE: &str = "bench";

/// What one store pass did.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct PassCounts {
    /// Files read and hashed.
    pub read: usize,
    /// Files whose rows already matched their bytes.
    pub unchanged: usize,
    /// Files parsed and written.
    pub reparsed: usize,
    /// Files that could not be read.
    pub failed: usize,
}

impl From<&PassReport> for PassCounts {
    fn from(report: &PassReport) -> Self {
        Self { read: report.read, unchanged: report.unchanged, reparsed: report.reparsed, failed: report.failed.len() }
    }
}

/// The cold half: no store, then a filled one.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColdReport {
    /// The file-list walk.
    pub list_ms: f64,
    /// Opening (creating) every root's store.
    pub open_ms: f64,
    /// The fill: read, hash, parse, and write every listed file.
    pub fill_ms: f64,
    pub fill: PassCounts,
    /// The store files' size on disk after the fill, when it is a file.
    pub store_bytes: Option<u64>,
    /// The process's peak resident set, when the platform reports it.
    pub peak_rss_bytes: Option<u64>,
}

/// The warm half: the store present and every file unchanged.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WarmReport {
    /// The file-list walk.
    pub list_ms: f64,
    /// Opening every root's existing store.
    pub open_ms: f64,
    /// The revalidation: read and hash every listed file.
    pub revalidate_ms: f64,
    pub revalidate: PassCounts,
    /// The document the requests are about, and the heading under the
    /// cursor (its first heading), when it has one.
    pub target: PathBuf,
    pub heading: Option<String>,
    /// The candidate referrers of `target`.
    pub candidates: usize,
    /// A references request on the heading (or the document start): the
    /// request graph and the provider.
    pub references_ms: f64,
    pub references: usize,
    /// A heading rename of the heading: re-reading the candidates, the
    /// request graph, and the provider. `None` without a heading.
    pub rename_ms: Option<f64>,
    /// A workspace-symbols request for the heading's title (or the empty
    /// query).
    pub symbols_ms: f64,
    pub symbols: usize,
    /// `open_ms + revalidate_ms + references_ms`: the first references
    /// answer after a restart, excluding the file-list walk.
    pub first_references_ms: f64,
    /// `first_references_ms + list_ms`.
    pub first_references_with_list_ms: f64,
    /// The process's peak resident set after the file-list walk and after
    /// the requests, and their difference, when the platform reports it.
    pub peak_rss_after_list_bytes: Option<u64>,
    pub peak_rss_bytes: Option<u64>,
    pub added_rss_bytes: Option<u64>,
}

/// The `--bench-index` report.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchReport {
    /// The directory that was indexed.
    pub root: PathBuf,
    /// Markdown files listed.
    pub files: usize,
    pub cold: ColdReport,
    pub warm: WarmReport,
}

impl BenchReport {
    /// Serializes the report as pretty JSON (the `--json` shape).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).expect("BenchReport serializes")
    }

    /// A compact human summary.
    pub fn to_summary(&self) -> String {
        let (cold, warm) = (&self.cold, &self.warm);
        format!(
            "index store bench over {files} Markdown files in {root}\n  \
             cold: list {cold_list:.1} ms · open {cold_open:.1} ms · fill {fill:.1} ms \
             ({reparsed} parsed) · store {store} · peak RSS {cold_rss}\n  \
             warm: list {warm_list:.1} ms · open {warm_open:.1} ms · revalidate {revalidate:.1} ms \
             ({unchanged} unchanged, {warm_reparsed} parsed)\n  \
             first references {first:.1} ms ({with_list:.1} ms with the list walk) · \
             {references} locations over {candidates} candidates\n  \
             requests: references {references_ms:.1} ms · rename {rename} · symbols {symbols_ms:.1} ms \
             ({symbols} symbols) · added RSS {added}",
            files = self.files,
            root = self.root.display(),
            cold_list = cold.list_ms,
            cold_open = cold.open_ms,
            fill = cold.fill_ms,
            reparsed = cold.fill.reparsed,
            store = mib(cold.store_bytes),
            cold_rss = mib(cold.peak_rss_bytes),
            warm_list = warm.list_ms,
            warm_open = warm.open_ms,
            revalidate = warm.revalidate_ms,
            unchanged = warm.revalidate.unchanged,
            warm_reparsed = warm.revalidate.reparsed,
            first = warm.first_references_ms,
            with_list = warm.first_references_with_list_ms,
            references = warm.references,
            candidates = warm.candidates,
            references_ms = warm.references_ms,
            rename = warm.rename_ms.map_or_else(|| "n/a".to_string(), |ms| format!("{ms:.1} ms")),
            symbols_ms = warm.symbols_ms,
            symbols = warm.symbols,
            added = mib(warm.added_rss_bytes),
        )
    }
}

/// Both halves in this process; see the [module docs](self) for why the
/// binary runs the warm half in a fresh one.
///
/// ## Errors
///
/// As [`bench_warm`].
pub fn bench_index(
    root: &Path,
    contexts: &RepositoryContexts,
    store_dir: &Path,
    target: Option<&Path>,
) -> Result<BenchReport, String> {
    let (files, cold) = bench_cold(root, store_dir);
    let warm = bench_warm(root, contexts, store_dir, target)?;
    Ok(BenchReport { root: root.to_path_buf(), files, cold, warm })
}

/// The cold half over `root`, filling a store in `store_dir`, which should
/// hold no store for `root`.
///
/// ## Returns
///
/// The number of listed Markdown files, and the report.
pub fn bench_cold(root: &Path, store_dir: &Path) -> (usize, ColdReport) {
    let workspace = Workspace::list(root);
    let location = StoreLocation::prepare(Some(store_dir.as_os_str()), None);
    let started = Instant::now();
    let mut facts = WorkspaceFacts::open(&workspace.store_roots, &location, Arc::new(SystemClock), BENCH_WORKSPACE);
    let open_ms = elapsed_ms(started);
    let started = Instant::now();
    let fill = facts.revalidate(&workspace.files, &Default::default(), &SilentProgress, &AtomicBool::new(false));
    let fill_ms = elapsed_ms(started);
    drop(facts);
    let store_bytes = match &location {
        StoreLocation::Directory(dir) => workspace
            .store_roots
            .iter()
            .map(|root| std::fs::metadata(store_path(dir, root)).map(|metadata| metadata.len()).ok())
            .sum(),
        _ => None,
    };
    let report = ColdReport {
        list_ms: workspace.list_ms,
        open_ms,
        fill_ms,
        fill: PassCounts::from(&fill),
        store_bytes,
        peak_rss_bytes: peak_rss_bytes(),
    };
    (workspace.files.len(), report)
}

/// The warm half over `root`, reading the store [`bench_cold`] filled in
/// `store_dir`. The requests are about `target` (default: `root`'s
/// `README.md`, else its first listed file).
///
/// ## Errors
///
/// When `root` lists no Markdown file, or the target cannot be read as
/// UTF-8 text.
pub fn bench_warm(
    root: &Path,
    contexts: &RepositoryContexts,
    store_dir: &Path,
    target: Option<&Path>,
) -> Result<WarmReport, String> {
    let workspace = Workspace::list(root);
    let peak_rss_after_list_bytes = peak_rss_bytes();
    let location = StoreLocation::prepare(Some(store_dir.as_os_str()), None);
    let started = Instant::now();
    let mut facts = WorkspaceFacts::open(&workspace.store_roots, &location, Arc::new(SystemClock), BENCH_WORKSPACE);
    let open_ms = elapsed_ms(started);
    let started = Instant::now();
    let revalidate = facts.revalidate(&workspace.files, &Default::default(), &SilentProgress, &AtomicBool::new(false));
    let revalidate_ms = elapsed_ms(started);
    facts.mark_ready();

    let target = workspace.target(target).ok_or_else(|| format!("{} lists no Markdown file", root.display()))?;
    let text = std::fs::read_to_string(&target).map_err(|error| format!("cannot read {}: {error}", target.display()))?;
    let document = crate::graph::index_document(&target, &text);
    let heading = document.headings.first();
    let offset = heading.map_or(0, |heading| heading.span.start);
    let session = Session::new(&workspace, &target, &text, contexts);

    // The server's references path: candidates, a request graph with the
    // target's buffer in place of its row, then the provider.
    let started = Instant::now();
    let graph = session.request_graph(&mut facts, &workspace, &document);
    let references = session.references(&graph, offset);
    let references_ms = elapsed_ms(started);
    let candidates = facts.candidates_for(std::slice::from_ref(&target), &workspace.listed).len();

    // The server's heading-rename path: re-read the candidates first.
    let rename_ms = heading.map(|heading| {
        let started = Instant::now();
        let referrers = facts.candidates_for(std::slice::from_ref(&target), &workspace.listed);
        facts.refresh(&referrers, &Default::default());
        let graph = session.request_graph(&mut facts, &workspace, &document);
        let _ = session.rename(&graph, heading.title_span.start, &format!("{} renamed", heading.title));
        elapsed_ms(started)
    });

    let query = heading.map_or("", |heading| heading.title.as_str());
    let started = Instant::now();
    let documents = facts.symbol_documents(&workspace.listed, [&document]);
    let symbols = session.registry.workspace_symbols(query, &documents).len();
    let symbols_ms = elapsed_ms(started);

    let peak = peak_rss_bytes();
    let first_references_ms = open_ms + revalidate_ms + references_ms;
    Ok(WarmReport {
        list_ms: workspace.list_ms,
        open_ms,
        revalidate_ms,
        revalidate: PassCounts::from(&revalidate),
        target: target.clone(),
        heading: heading.map(|heading| heading.title.clone()),
        candidates,
        references_ms,
        references,
        rename_ms,
        symbols_ms,
        symbols,
        first_references_ms,
        first_references_with_list_ms: first_references_ms + workspace.list_ms,
        peak_rss_after_list_bytes,
        peak_rss_bytes: peak,
        added_rss_bytes: peak.zip(peak_rss_after_list_bytes).map(|(peak, before)| peak.saturating_sub(before)),
    })
}

/// The listed workspace a bench half runs over.
struct Workspace {
    roots: Vec<PathBuf>,
    store_roots: Vec<PathBuf>,
    config: ConfigState,
    files: Vec<PathBuf>,
    listed: std::collections::BTreeSet<PathBuf>,
    list_ms: f64,
}

impl Workspace {
    fn list(root: &Path) -> Self {
        let roots = vec![biscuit_file::canonicalize_simplified(root).unwrap_or_else(|_| root.to_path_buf())];
        let config = ConfigState::load(roots.clone(), None);
        let started = Instant::now();
        let report = list_files(&roots, &config.effective().workspace);
        let list_ms = elapsed_ms(started);
        let store_roots = effective_roots(&roots, &config.effective().workspace);
        let listed = report.files.iter().cloned().collect();
        Self { roots, store_roots, config, files: report.files, listed, list_ms }
    }

    /// The requested target, made absolute; by default `README.md`, else
    /// the first listed file (`None` when nothing is listed).
    fn target(&self, target: Option<&Path>) -> Option<PathBuf> {
        if let Some(target) = target {
            let target = if target.is_absolute() { target.to_path_buf() } else { self.roots[0].join(target) };
            // A target that does not exist fails at the read, naming the path.
            return Some(biscuit_file::canonicalize_simplified(&target).unwrap_or(target));
        }
        let readme = self.roots[0].join("README.md");
        if self.listed.contains(&readme) {
            return Some(readme);
        }
        self.files.first().cloned()
    }
}

/// What a request needs beyond the store: the target as an open document.
struct Session<'a> {
    uri: lsp_types::Uri,
    target: &'a Path,
    text: &'a str,
    source_map: SourceMap,
    contexts: &'a RepositoryContexts,
    config: &'a ConfigState,
    roots: &'a [PathBuf],
    universe: Arc<WikiUniverse>,
    profile: ClientProfile,
    documents: DocumentStore,
    registry: ProviderRegistry,
}

impl<'a> Session<'a> {
    fn new(workspace: &'a Workspace, target: &'a Path, text: &'a str, contexts: &'a RepositoryContexts) -> Self {
        let uri = file_path_to_uri(target).expect("the bench target is an absolute path");
        let source_map = SourceMap::new(uri.clone(), 1, PositionEncoding::Utf16, Arc::from(text));
        let wiki_roots = resolve_wiki_roots(&workspace.roots, workspace.config.effective().wiki.wiki_root.as_deref());
        Self {
            uri,
            target,
            text,
            source_map,
            contexts,
            config: &workspace.config,
            roots: &workspace.roots,
            universe: Arc::new(WikiUniverse::new(workspace.files.iter().cloned(), &wiki_roots)),
            profile: ClientProfile::from_initialize(&Default::default(), PositionEncoding::Utf16),
            documents: DocumentStore::new(PositionEncoding::Utf16),
            registry: ProviderRegistry::with_substrate(),
        }
    }

    fn request_graph(&self, facts: &mut WorkspaceFacts, workspace: &Workspace, document: &DocumentIndex) -> WorkspaceGraph {
        let targets = [self.target.to_path_buf()];
        facts.request_graph(&targets, &workspace.listed, [document], Arc::clone(&self.universe), self.contexts)
    }

    fn references(&self, graph: &WorkspaceGraph, offset: usize) -> usize {
        let resolution = self.contexts.for_document(self.target);
        let ctx = DocumentContext {
            uri: &self.uri,
            path: self.target,
            text: self.text,
            source_map: &self.source_map,
            graph,
            doc_id: graph.document_id(self.target),
            config: self.config.effective(),
            profile: &self.profile,
            overlay: None,
            resolution: &resolution,
        };
        self.registry.references(&ctx, offset, false).len()
    }

    fn rename(&self, graph: &WorkspaceGraph, offset: usize, new_name: &str) -> bool {
        let Some(position) = self.source_map.byte_to_lsp(offset) else {
            return false;
        };
        let env = crate::providers::rename::RenameEnv {
            documents: &self.documents,
            graph,
            config: self.config.effective(),
            profile: &self.profile,
            roots: self.roots,
            encoding: PositionEncoding::Utf16,
        };
        crate::providers::rename::rename(&env, &self.uri, position, new_name).is_ok()
    }
}

fn elapsed_ms(start: Instant) -> f64 {
    start.elapsed().as_secs_f64() * 1000.0
}

fn mib(bytes: Option<u64>) -> String {
    bytes.map_or_else(|| "n/a".to_string(), |bytes| format!("{:.1} MiB", bytes as f64 / (1024.0 * 1024.0)))
}

/// Peak resident set size in bytes.
///
/// Uses `getrusage` on Unix (`ru_maxrss` — KiB on Linux, bytes on macOS);
/// returns `None` where unavailable (Windows here — a later pass can add the
/// `GetProcessMemoryInfo` path).
pub fn peak_rss_bytes() -> Option<u64> {
    #[cfg(unix)]
    {
        use std::mem::MaybeUninit;
        // SAFETY: `getrusage` fills the provided `rusage` struct and returns 0
        // on success; we only read `ru_maxrss` on success.
        let mut usage = MaybeUninit::<libc_rusage::rusage>::zeroed();
        let result = unsafe { libc_rusage::getrusage(libc_rusage::RUSAGE_SELF, usage.as_mut_ptr()) };
        if result != 0 {
            return None;
        }
        let usage = unsafe { usage.assume_init() };
        let max_rss = usage.ru_maxrss as u64;
        // Linux reports KiB; macOS reports bytes.
        if cfg!(target_os = "macos") {
            Some(max_rss)
        } else {
            Some(max_rss * 1024)
        }
    }
    #[cfg(not(unix))]
    {
        None
    }
}

/// Minimal `getrusage` FFI so the bench can read peak RSS without pulling in
/// the whole `libc` crate as a dependency. Unix-only.
#[cfg(unix)]
mod libc_rusage {
    #![allow(non_camel_case_types)]

    pub const RUSAGE_SELF: i32 = 0;

    // The leading fields we read (`ru_maxrss`) are stable across Unix ABIs;
    // the tail is padded generously so the struct is at least as large as the
    // platform's real `rusage`.
    #[repr(C)]
    pub struct rusage {
        pub ru_utime: timeval,
        pub ru_stime: timeval,
        pub ru_maxrss: core::ffi::c_long,
        _tail: [core::ffi::c_long; 16],
    }

    #[repr(C)]
    pub struct timeval {
        pub tv_sec: core::ffi::c_long,
        pub tv_usec: core::ffi::c_long,
    }

    unsafe extern "C" {
        pub fn getrusage(who: i32, usage: *mut rusage) -> i32;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_peak_rss_is_positive_where_available() {
        if let Some(bytes) = peak_rss_bytes() {
            assert!(bytes > 0);
        }
    }
}

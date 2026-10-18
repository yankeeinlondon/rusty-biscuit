//! The on-demand index build: revalidating the index store for the requests
//! that need every Markdown file, and the requests waiting on it.
//!
//! Find references, heading rename, `workspace/willRenameFiles`, and
//! workspace symbols are answered from the index store
//! ([`crate::index_store::WorkspaceFacts`]), which needs every listed
//! Markdown file read once in this process. The first such request starts one
//! background build; every request that needs the store while it builds waits
//! on the same build. The router owns this state on the dispatch thread,
//! which keeps answering edits, cancellation, and shutdown while the build's
//! workers read.
//!
//! ```text
//! NotStarted ──request──▶ Building ──every round reconciled──▶ Ready
//!     ▲                    │ │ ▲                                  │
//!     └─last waiter cancels┘ │ └──request, the store not ready────┘
//!                            │
//!                            └──a listed file unreadable──▶ Failed ──request──▶ Building
//! ```
//!
//! A build runs in rounds. A round runs one store pass on a background
//! thread ([`FactsPlan`]): the first round revalidates every listed file,
//! and each later round re-reads only the files that changed while the
//! previous one read. The build becomes ready when a round finishes with
//! nothing changed meanwhile. A ready build is answered from only while the
//! store is ready (every listed file read in this process, no root switched
//! to memory since); otherwise the next request starts a build again, and in
//! fallback watch mode every rename does.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use lsp_server::Request;
use lsp_types::request::{References, Rename, Request as _, WillRenameFiles, WorkspaceSymbolRequest};

use crate::index_store::PassReport;
use crate::request_timing::RequestTimings;
use crate::workspace::startup::ProgressReporter;

/// The request that started an index build, named in the
/// `index store revalidated` log line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Trigger {
    /// `textDocument/references`.
    References,
    /// `textDocument/rename`.
    Rename,
    /// `workspace/willRenameFiles`.
    FileRename,
    /// `workspace/symbol`.
    WorkspaceSymbols,
}

impl Trigger {
    /// The trigger for a request method that needs the index store; `None`
    /// for every other method (including `textDocument/prepareRename`, which
    /// checks only the local rename target).
    pub fn for_method(method: &str) -> Option<Self> {
        match method {
            References::METHOD => Some(Self::References),
            Rename::METHOD => Some(Self::Rename),
            WillRenameFiles::METHOD => Some(Self::FileRename),
            WorkspaceSymbolRequest::METHOD => Some(Self::WorkspaceSymbols),
            _ => None,
        }
    }

    /// The log field value.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::References => "references",
            Self::Rename => "rename",
            Self::FileRename => "file-rename",
            Self::WorkspaceSymbols => "workspace-symbols",
        }
    }
}

/// A request waiting for a prerequisite (the index build, or a trigger
/// registry; see [`crate::registry_jobs`]), with the timing record started
/// when it was dequeued, so its completion event covers the wait.
pub(crate) struct Waiter {
    pub request: Request,
    pub timings: RequestTimings,
}

/// A build in flight.
pub(crate) struct Build {
    /// Distinguishes this build's round results from a cancelled build's.
    pub id: u64,
    pub trigger: Trigger,
    pub started: Instant,
    /// Set when the build is abandoned; workers check it between files.
    pub cancel: Arc<AtomicBool>,
    /// The build's `window/workDoneProgress`, spanning every round.
    pub progress: Arc<dyn ProgressReporter>,
    pub waiters: Vec<Waiter>,
    /// Whether the build waits for the startup file list before its first
    /// round.
    pub awaiting_list: bool,
    /// What the next round's store pass reads.
    pub facts: FactsPlan,
    /// What the build's rounds did so far.
    pub totals: PassTotals,
}

/// The files a build round's store pass reads (see
/// [`crate::index_store::WorkspaceFacts`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FactsPlan {
    /// Every listed Markdown file, deleting rows for missing paths: a
    /// build's first round, and any round after a store switched to memory.
    Full,
    /// Only these paths: the Markdown files that changed while the previous
    /// round read.
    Paths(Vec<std::path::PathBuf>),
}

/// The counts of a build's store passes, summed over its rounds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct PassTotals {
    pub unchanged: usize,
    pub reparsed: usize,
    pub removed: usize,
}

impl PassTotals {
    pub fn add(&mut self, report: &PassReport) {
        self.unchanged += report.unchanged;
        self.reparsed += report.reparsed;
        self.removed += report.removed;
    }
}

impl Drop for Build {
    /// A build dropped before it finished (cancelled, or the server
    /// stopped) stops its workers.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// The index build's state.
#[derive(Default)]
pub(crate) enum IndexBuildState {
    /// No build has run, or the last one was cancelled.
    #[default]
    NotStarted,
    Building(Build),
    /// Built; the store stays current file by file from here on. Requests
    /// are answered at once only while the store is also ready; otherwise
    /// they start a build again.
    Ready,
    /// The last build could not read a listed file; the next request that
    /// needs the store retries.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_backlink_and_symbol_requests_need_the_index_store() {
        let triggers = [
            ("textDocument/references", Some("references")),
            ("textDocument/rename", Some("rename")),
            ("workspace/willRenameFiles", Some("file-rename")),
            ("workspace/symbol", Some("workspace-symbols")),
            ("textDocument/prepareRename", None),
            ("textDocument/hover", None),
        ];
        for (method, expected) in triggers {
            assert_eq!(Trigger::for_method(method).map(Trigger::as_str), expected, "{method}");
        }
    }

    #[test]
    fn test_dropping_a_build_cancels_its_workers() {
        let cancel = Arc::new(AtomicBool::new(false));
        let build = Build {
            id: 1,
            trigger: Trigger::References,
            started: Instant::now(),
            cancel: Arc::clone(&cancel),
            progress: Arc::new(crate::workspace::startup::SilentProgress),
            waiters: Vec::new(),
            awaiting_list: false,
            facts: FactsPlan::Full,
            totals: PassTotals::default(),
        };
        drop(build);
        assert!(cancel.load(Ordering::SeqCst));
    }
}

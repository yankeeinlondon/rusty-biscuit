//! The on-demand full index: its build states and the requests waiting on it.
//!
//! Find references, heading rename, `workspace/willRenameFiles`, and
//! workspace symbols are answered from the index store
//! ([`crate::index_store::WorkspaceFacts`]), which needs every Markdown
//! document read once in this process; each build round fills it. The first such
//! request starts one background build; every request that needs the index
//! while it builds waits on the same build. The router owns this state on
//! the dispatch thread, which keeps answering edits, cancellation, and
//! shutdown while the build's workers read.
//!
//! ```text
//! NotStarted ──request──▶ Building ──every round reconciled──▶ Ready
//!     ▲                    │ │ ▲                                  │
//!     └─last waiter cancels┘ │ └──request, a listed file unparsed─┘
//!                            │
//!                            └──a listed file unreadable──▶ Failed ──request──▶ Building
//! ```
//!
//! A ready index is answered from only while every listed document has a
//! current parse ([`WorkspaceIndex::is_full`]). A listed file that becomes
//! unreadable afterwards withdraws that, and the next request that needs the
//! index runs a build round over the unparsed files: it fails, or retries
//! after a repair, exactly as a first build does.
//!
//! A build runs in rounds ([`WorkspaceIndex::begin_full_build`],
//! [`WorkspaceIndex::finish_full_build`]): a round reads the unparsed listed
//! files with the index lock released, and the dispatch thread then adopts
//! only what nothing changed meanwhile. Each round also runs an index-store
//! pass ([`FactsPlan`]; see [`crate::index_store::WorkspaceFacts`]), and a
//! build becomes ready when a round leaves nothing unparsed and no Markdown
//! file changed while its store pass read. A ready index whose store is not
//! ready, and in fallback watch mode every rename, starts a build again.
//!
//! [`WorkspaceIndex::begin_full_build`]: crate::graph::WorkspaceIndex::begin_full_build
//! [`WorkspaceIndex::finish_full_build`]: crate::graph::WorkspaceIndex::finish_full_build
//! [`WorkspaceIndex::is_full`]: crate::graph::WorkspaceIndex::is_full

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use lsp_server::Request;
use lsp_types::request::{References, Rename, Request as _, WillRenameFiles, WorkspaceSymbolRequest};

use crate::request_timing::RequestTimings;
use crate::workspace::startup::ProgressReporter;

/// The request that started a full-index build, named in the
/// `full index built` log line.
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
    /// The trigger for a request method that needs the full index; `None`
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

/// A request waiting for a prerequisite (the full index, or a trigger
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
    /// What the next round's index-store pass reads.
    pub facts: FactsPlan,
}

/// The files a build round's index-store pass reads (see
/// [`crate::index_store::WorkspaceFacts`]).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum FactsPlan {
    /// Every listed Markdown file, deleting rows for missing paths: a
    /// build's first round, and any round after a store switched to memory.
    Full,
    /// Only these paths: the Markdown files that changed while the previous
    /// round read. Empty when only the full index needs another round.
    Paths(Vec<std::path::PathBuf>),
}

impl Drop for Build {
    /// A build dropped before it finished (cancelled, or the server
    /// stopped) stops its workers.
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

/// The full index's build state.
#[derive(Default)]
pub(crate) enum FullIndexState {
    /// No build has run, or the last one was cancelled.
    #[default]
    NotStarted,
    Building(Build),
    /// Built; the index stays current file by file from here on. Requests
    /// are answered at once only while it is also
    /// [full](crate::graph::WorkspaceIndex::is_full); otherwise they start a
    /// build again.
    Ready,
    /// The last build could not read a listed file; the next request that
    /// needs the index retries.
    Failed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_only_backlink_and_symbol_requests_need_the_full_index() {
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
        };
        drop(build);
        assert!(cancel.load(Ordering::SeqCst));
    }
}

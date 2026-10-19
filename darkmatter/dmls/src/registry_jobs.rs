//! Trigger-registry preparation off the dispatch thread: the jobs the
//! registry worker runs, and the requests and diagnostics refreshes waiting
//! on them.
//!
//! The dispatch thread only looks registries up
//! ([`OverlayState::registry_demand`](crate::overlay::OverlayState::registry_demand)).
//! A document whose registry is missing, stale, or prepared in another
//! context gets one [`RegistryJob::Document`]; every schema-dependent request
//! for it waits on that job, and its diagnostics refresh runs when the job's
//! result is installed. A refresh that marked the workspace scopes stale
//! queues one [`RegistryJob::Workspace`]. The worker prepares each job
//! without holding the overlay lock across its I/O and reports back through
//! the dispatch loop, which keeps handling edits, cancellation, and
//! shutdown meanwhile.
//!
//! ```text
//! not fresh ──need──▶ Pending (job queued) ──installed, fresh──▶ waiters answered,
//!                         ▲                                        diagnostics published
//!                         └──installed, but changed meanwhile──────┘ (queued again)
//! ```

use std::collections::HashMap;
use std::path::PathBuf;

use crossbeam_channel::Sender;
use lsp_server::RequestId;
use lsp_types::Uri;

use crate::index_build::Waiter;
use crate::overlay::RegistryNeed;
use crate::router::StartupEvent;

/// Which preparation a [`RegistryJob`] (or startup) ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryJobKind {
    /// Startup's preparation of the workspace scopes.
    Startup,
    /// The workspace scopes again, after a refresh marked them stale.
    Workspace,
    /// One document's registry.
    Document,
}

/// Work for the registry worker.
pub(crate) enum RegistryJob {
    /// Prepare every stale workspace scope.
    Workspace,
    /// Prepare one document's registry.
    Document(Box<RegistryNeed>),
}

impl RegistryJob {
    pub fn kind(&self) -> RegistryJobKind {
        match self {
            Self::Workspace => RegistryJobKind::Workspace,
            Self::Document(_) => RegistryJobKind::Document,
        }
    }
}

/// What waits on one document's queued preparation.
#[derive(Default)]
pub(crate) struct Pending {
    /// Requests answered once the registry is installed.
    pub waiters: Vec<Waiter>,
    /// The open document whose diagnostics refresh was held for it.
    pub refresh: Option<Uri>,
}

/// The document preparations queued or running, keyed by document path, and
/// the worker's job queue. One job serves every need for a document until
/// its result is installed.
pub(crate) struct PendingRegistries {
    jobs: Sender<RegistryJob>,
    pending: HashMap<PathBuf, Pending>,
    /// Jobs queued or running.
    outstanding: usize,
    /// The [`StartupHooks::observer`](crate::StartupHooks::observer), told
    /// of every change to `outstanding`.
    observer: Option<Sender<StartupEvent>>,
}

impl PendingRegistries {
    pub fn new(jobs: Sender<RegistryJob>, observer: Option<Sender<StartupEvent>>) -> Self {
        Self { jobs, pending: HashMap::new(), outstanding: 0, observer }
    }

    /// Queues a preparation of the workspace scopes.
    pub fn schedule_workspace(&mut self) {
        self.send(RegistryJob::Workspace);
    }

    fn send(&mut self, job: RegistryJob) {
        if self.jobs.send(job).is_ok() {
            self.outstanding += 1;
            self.report();
        }
    }

    /// Records that a job's result was handled.
    pub fn finished(&mut self) {
        self.outstanding = self.outstanding.saturating_sub(1);
        self.report();
    }

    fn report(&self) {
        if let Some(observer) = &self.observer {
            let _ = observer.send(StartupEvent::RegistryJobsOutstanding(self.outstanding));
        }
    }

    /// Records that `refresh` (a held diagnostics refresh) or `waiter` (a
    /// request) waits on the registry `need` names, queuing its preparation
    /// unless one is already queued or running.
    pub fn wait_for(&mut self, need: RegistryNeed, refresh: Option<Uri>, waiter: Option<Waiter>) {
        if !self.pending.contains_key(&need.path) {
            self.pending.insert(need.path.clone(), Pending::default());
            self.send(RegistryJob::Document(Box::new(need.clone())));
        }
        let pending = self.pending.get_mut(&need.path).expect("inserted above");
        if refresh.is_some() {
            pending.refresh = refresh;
        }
        pending.waiters.extend(waiter);
    }

    /// Removes what waited on the document at `path`, whose job finished.
    pub fn take(&mut self, path: &PathBuf) -> Option<Pending> {
        self.pending.remove(path)
    }

    /// Removes the request `id` from whatever it waits on. The job keeps
    /// running: other requests or a held refresh may need it, and its
    /// result is cached either way.
    pub fn cancel(&mut self, id: &RequestId) -> Option<Waiter> {
        self.pending.values_mut().find_map(|pending| {
            let position = pending.waiters.iter().position(|waiter| waiter.request.id == *id)?;
            Some(pending.waiters.remove(position))
        })
    }

    /// Removes every waiting request, at shutdown.
    pub fn drain_waiters(&mut self) -> Vec<Waiter> {
        self.pending.values_mut().flat_map(|pending| std::mem::take(&mut pending.waiters)).collect()
    }
}

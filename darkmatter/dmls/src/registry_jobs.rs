//! Trigger-registry preparation off the dispatch thread: the jobs the
//! registry worker runs, the queue it takes them from, and the requests and
//! diagnostics refreshes waiting on them.
//!
//! The dispatch thread only looks registries up
//! ([`OverlayState::registry_demand`](crate::overlay::OverlayState::registry_demand)).
//! A document whose registry is missing, stale, or prepared in another
//! context gets one [`RegistryJob::Document`]; every schema-dependent request
//! for it waits on that job, and its diagnostics refresh runs when the job's
//! result is installed. A refresh that marked the workspace scopes stale
//! schedules a [`RegistryJob::Workspace`] pass. The worker prepares each job
//! without holding the overlay lock across its I/O and reports back through
//! the dispatch loop, which keeps handling edits, cancellation, and
//! shutdown meanwhile.
//!
//! ```text
//! not fresh ──need──▶ Pending (job queued) ──installed, fresh──▶ waiters answered,
//!                         ▲                                        diagnostics published
//!                         └──installed, but changed meanwhile──────┘ (queued again)
//! ```
//!
//! The [`RegistryQueue`] has two lanes. Document jobs, which requests wait
//! on, always run before a workspace pass starts, and a running pass runs
//! any document job that arrives before each of its probes, so a request
//! waits for at most the probe in progress. The workspace lane is one slot:
//!
//! ```text
//! Idle ──schedule──▶ Queued ──worker starts the pass──▶ Running ──pass ends──▶ Idle
//!                      ▲                                   │
//!                      │                               schedule
//!                      │                                   ▼
//!                      └──────────pass ends─────────── RequestedAgain
//! ```
//!
//! Any number of schedules while a pass is queued, or while one runs, add
//! nothing beyond one pass, and a follow-up pass reads the workspace scopes
//! when it starts, so it covers every change scheduled before then.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};

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

/// Where the workspace lane of a [`RegistryQueue`] is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WorkspaceSlot {
    Idle,
    Queued,
    Running,
    /// Running, with one follow-up pass scheduled.
    RequestedAgain,
}

struct Lanes {
    documents: VecDeque<RegistryNeed>,
    workspace: WorkspaceSlot,
    closed: bool,
}

/// The registry worker's job queue, shared by the dispatch thread (which
/// schedules) and the worker (which takes jobs and moves the workspace slot
/// on). Both sides change the slot under one lock, so a schedule and the
/// end of a pass never race into a lost or doubled pass.
pub(crate) struct RegistryQueue {
    lanes: Mutex<Lanes>,
    ready: Condvar,
}

impl RegistryQueue {
    pub fn new() -> Self {
        Self {
            lanes: Mutex::new(Lanes {
                documents: VecDeque::new(),
                workspace: WorkspaceSlot::Idle,
                closed: false,
            }),
            ready: Condvar::new(),
        }
    }

    fn lanes(&self) -> std::sync::MutexGuard<'_, Lanes> {
        self.lanes.lock().expect("registry queue lock poisoned")
    }

    /// Queues a document job.
    ///
    /// ## Returns
    ///
    /// Whether it was queued: `false` once the queue is closed.
    fn push_document(&self, need: RegistryNeed) -> bool {
        let mut lanes = self.lanes();
        if lanes.closed {
            return false;
        }
        lanes.documents.push_back(need);
        self.ready.notify_one();
        true
    }

    /// Schedules a workspace pass.
    ///
    /// ## Returns
    ///
    /// Whether this added a pass that will run: `false` when one is already
    /// queued, a follow-up is already scheduled, or the queue is closed.
    fn schedule_workspace(&self) -> bool {
        let mut lanes = self.lanes();
        if lanes.closed {
            return false;
        }
        match lanes.workspace {
            WorkspaceSlot::Idle => {
                lanes.workspace = WorkspaceSlot::Queued;
                self.ready.notify_one();
                true
            }
            WorkspaceSlot::Running => {
                lanes.workspace = WorkspaceSlot::RequestedAgain;
                true
            }
            WorkspaceSlot::Queued | WorkspaceSlot::RequestedAgain => false,
        }
    }

    /// The worker's next job, waiting for one: a queued document first, else
    /// a queued workspace pass, which this marks running.
    ///
    /// ## Returns
    ///
    /// `None` once the queue is closed.
    pub fn next(&self) -> Option<RegistryJob> {
        let mut lanes = self.lanes();
        loop {
            if lanes.closed {
                return None;
            }
            if let Some(need) = lanes.documents.pop_front() {
                return Some(RegistryJob::Document(Box::new(need)));
            }
            if lanes.workspace == WorkspaceSlot::Queued {
                lanes.workspace = WorkspaceSlot::Running;
                return Some(RegistryJob::Workspace);
            }
            lanes = self.ready.wait(lanes).expect("registry queue lock poisoned");
        }
    }

    /// The next queued document job, without waiting, for a running pass to
    /// run before its next probe.
    pub fn next_document(&self) -> Option<RegistryNeed> {
        let mut lanes = self.lanes();
        if lanes.closed {
            return None;
        }
        lanes.documents.pop_front()
    }

    /// Ends the running workspace pass, queuing the follow-up a schedule
    /// during it asked for. Called before the pass's completion is reported,
    /// so a schedule the dispatch thread makes after this always adds a pass.
    pub fn finish_workspace(&self) {
        let mut lanes = self.lanes();
        lanes.workspace = match lanes.workspace {
            WorkspaceSlot::RequestedAgain => WorkspaceSlot::Queued,
            _ => WorkspaceSlot::Idle,
        };
    }

    /// Stops the queue: the worker takes no further job.
    fn close(&self) {
        self.lanes().closed = true;
        self.ready.notify_all();
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
    queue: Arc<RegistryQueue>,
    pending: HashMap<PathBuf, Pending>,
    /// Jobs that will report a completion: each queued document job, plus
    /// the workspace pass queued or running and its scheduled follow-up.
    outstanding: usize,
    /// The [`StartupHooks::observer`](crate::StartupHooks::observer), told
    /// of every change to `outstanding`.
    observer: Option<Sender<StartupEvent>>,
}

impl PendingRegistries {
    pub fn new(queue: Arc<RegistryQueue>, observer: Option<Sender<StartupEvent>>) -> Self {
        Self { queue, pending: HashMap::new(), outstanding: 0, observer }
    }

    /// Schedules a pass over the workspace scopes, merged with one already
    /// queued, or run once more after the one running.
    pub fn schedule_workspace(&mut self) {
        if self.queue.schedule_workspace() {
            self.added();
        }
    }

    fn added(&mut self) {
        self.outstanding += 1;
        self.report();
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
            if self.queue.push_document(need.clone()) {
                self.added();
            }
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

impl Drop for PendingRegistries {
    /// Stops the worker once the server state is gone.
    fn drop(&mut self) {
        self.queue.close();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use biscuit_file::FileResolutionContext;

    fn need(name: &str) -> RegistryNeed {
        let folder = std::env::temp_dir().join("dmls-registry-queue");
        RegistryNeed {
            path: folder.join(name),
            context: FileResolutionContext::from_snapshot(&folder, None, HashMap::new()),
        }
    }

    /// The worker's next job without waiting: `None` when nothing is queued.
    fn take(queue: &RegistryQueue) -> Option<String> {
        let lanes = queue.lanes();
        let idle = lanes.documents.is_empty() && lanes.workspace != WorkspaceSlot::Queued;
        drop(lanes);
        if idle {
            return None;
        }
        Some(match queue.next().expect("queue open") {
            RegistryJob::Workspace => "workspace".to_string(),
            RegistryJob::Document(need) => need.path.file_name().unwrap().to_string_lossy().into_owned(),
        })
    }

    #[test]
    fn a_document_queued_after_two_workspace_schedules_runs_first_and_one_pass_follows() {
        let queue = RegistryQueue::new();
        assert!(queue.schedule_workspace(), "the first schedule adds a pass");
        assert!(!queue.schedule_workspace(), "a second schedule merges into the queued pass");
        assert!(queue.push_document(need("doc.md")));

        assert_eq!(take(&queue).as_deref(), Some("doc.md"));
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        queue.finish_workspace();
        assert_eq!(take(&queue), None, "exactly one pass ran");
    }

    #[test]
    fn schedules_during_a_running_pass_add_exactly_one_follow_up() {
        let queue = RegistryQueue::new();
        queue.schedule_workspace();
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        assert_eq!(take(&queue), None, "a running pass is not queued again by starting");

        assert!(queue.schedule_workspace(), "a schedule during the pass adds one follow-up");
        assert!(!queue.schedule_workspace(), "later schedules merge into it");
        assert!(!queue.schedule_workspace());
        assert_eq!(take(&queue), None, "the follow-up waits for the running pass");

        queue.finish_workspace();
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        queue.finish_workspace();
        assert_eq!(take(&queue), None, "one follow-up only");
    }

    #[test]
    fn a_schedule_after_a_pass_ends_but_before_its_completion_is_handled_adds_one_pass() {
        let queue = RegistryQueue::new();
        queue.schedule_workspace();
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        // The worker ends the pass, then reports; the dispatch thread
        // schedules before it handles that report.
        queue.finish_workspace();
        assert!(queue.schedule_workspace(), "the ended pass cannot absorb a later schedule");
        assert!(!queue.schedule_workspace());
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        queue.finish_workspace();
        assert_eq!(take(&queue), None);
    }

    #[test]
    fn a_running_pass_takes_the_documents_queued_meanwhile_in_order() {
        let queue = RegistryQueue::new();
        queue.schedule_workspace();
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        assert!(queue.next_document().is_none(), "nothing waits yet");

        queue.push_document(need("a.md"));
        queue.push_document(need("b.md"));
        let names: Vec<PathBuf> =
            std::iter::from_fn(|| queue.next_document()).map(|need| need.path).collect();
        assert_eq!(names, vec![need("a.md").path, need("b.md").path]);
        queue.finish_workspace();
        assert_eq!(take(&queue), None);
    }

    #[test]
    fn a_closed_queue_takes_and_hands_out_nothing() {
        let queue = Arc::new(RegistryQueue::new());
        let waiting = {
            let queue = Arc::clone(&queue);
            std::thread::spawn(move || queue.next().is_none())
        };
        queue.close();
        assert!(waiting.join().unwrap(), "a waiting worker wakes and stops");
        assert!(!queue.push_document(need("doc.md")));
        assert!(!queue.schedule_workspace());
        assert!(queue.next_document().is_none());
        assert!(queue.next().is_none());
    }
}

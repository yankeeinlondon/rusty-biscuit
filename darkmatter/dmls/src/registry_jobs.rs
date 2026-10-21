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
//! The [`RegistryQueue`] has three lanes: documents, startup phases, and
//! one workspace slot. Document jobs, which requests wait on, always run
//! before a pass starts, and a running pass (a startup phase or a workspace
//! pass) runs any document job that arrives before each of its probes, so a
//! request waits for at most the probe in progress. The worker runs one job
//! at a time, so no two of its preparations overlap. The workspace lane is
//! one slot:
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
//!
//! Startup runs as two [`StartupPhase`]s on the same worker, coordinated by
//! the dispatch thread through completion messages ([`PendingRegistries::begin_startup`]):
//!
//! ```text
//! server start ──▶ Initial queued ─┐
//!                                  ├─ both done ──▶ Discovered queued (only when discovery
//! file discovery (own thread) ─────┘                 found new repositories) ──▶ ready
//! ```
//!
//! Discovery's file list is installed as soon as it arrives; only the
//! `schema registry ready` line waits for the last phase. Startup counts as
//! one outstanding job from server start until then, so an observer never
//! sees zero between the phases.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::{Arc, Condvar, Mutex};
use std::thread::ThreadId;
use std::time::Duration;

use crossbeam_channel::Sender;
use lsp_server::RequestId;
use lsp_types::Uri;

use crate::index_build::Waiter;
use crate::overlay::{RegistryNeed, RegistrySummary};
use crate::router::StartupEvent;

/// Which preparation a [`RegistryJob`] ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RegistryJobKind {
    /// Startup's preparation of the workspace scopes (both phases; reported
    /// once, after the last).
    Startup,
    /// The workspace scopes again, after a refresh marked them stale.
    Workspace,
    /// One document's registry.
    Document,
}

/// One of startup's two preparation passes over the workspace scopes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    /// The scopes known without a listing: each workspace folder and the
    /// packages and package areas inside it.
    Initial,
    /// The scopes of the repositories file discovery found below the
    /// folders. Runs only when discovery changed the repository set.
    Discovered,
}

/// Work for the registry worker.
pub(crate) enum RegistryJob {
    /// Prepare every stale workspace scope.
    Workspace,
    /// Prepare every stale workspace scope for a startup phase.
    Startup(StartupPhase),
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
    startup: VecDeque<StartupPhase>,
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
                startup: VecDeque::new(),
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

    /// Queues a startup phase.
    ///
    /// ## Returns
    ///
    /// Whether it was queued: `false` once the queue is closed.
    fn push_startup(&self, phase: StartupPhase) -> bool {
        let mut lanes = self.lanes();
        if lanes.closed {
            return false;
        }
        lanes.startup.push_back(phase);
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

    /// The worker's next job, waiting for one: a queued document first, then
    /// a queued startup phase, else a queued workspace pass, which this
    /// marks running.
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
            if let Some(phase) = lanes.startup.pop_front() {
                return Some(RegistryJob::Startup(phase));
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

/// Where startup's preparation is, between server start and the
/// `schema registry ready` line.
#[derive(Default)]
struct StartupProgress {
    /// The initial phase finished.
    initial_done: bool,
    /// File discovery's result arrived: whether it changed the repository
    /// set.
    discovered: Option<bool>,
    /// The discovered-scopes phase was queued.
    discovered_queued: bool,
    /// The discovered-scopes phase finished.
    discovered_done: bool,
    /// The phases' preparation time so far, without queue waits, gate
    /// waits, or interrupting document jobs.
    preparing: Duration,
    /// The registry summary at the end of the last finished phase.
    summary: RegistrySummary,
    /// The thread the last finished phase ran on.
    thread: Option<ThreadId>,
}

/// Startup's preparation finished: what its one registry line reports.
pub(crate) struct StartupReady {
    pub summary: RegistrySummary,
    pub preparing: Duration,
    /// The worker thread that ran the phases.
    pub thread: ThreadId,
}

/// The document preparations queued or running, keyed by document path, and
/// the worker's job queue. One job serves every need for a document until
/// its result is installed.
pub(crate) struct PendingRegistries {
    queue: Arc<RegistryQueue>,
    pending: HashMap<PathBuf, Pending>,
    /// Jobs that will report a completion: each queued document job, the
    /// workspace pass queued or running and its scheduled follow-up, and
    /// startup (one job for both phases, until it is ready).
    outstanding: usize,
    /// `Some` from [`begin_startup`](Self::begin_startup) until startup is
    /// ready or the server shuts down.
    startup: Option<StartupProgress>,
    /// The [`StartupHooks::observer`](crate::StartupHooks::observer), told
    /// of every change to `outstanding`.
    observer: Option<Sender<StartupEvent>>,
}

impl PendingRegistries {
    pub fn new(queue: Arc<RegistryQueue>, observer: Option<Sender<StartupEvent>>) -> Self {
        Self { queue, pending: HashMap::new(), outstanding: 0, startup: None, observer }
    }

    /// Queues startup's initial phase. Startup counts as one outstanding
    /// job until [`startup_phase_finished`](Self::startup_phase_finished) or
    /// [`startup_discovered`](Self::startup_discovered) reports it ready.
    pub fn begin_startup(&mut self) {
        if self.queue.push_startup(StartupPhase::Initial) {
            self.startup = Some(StartupProgress::default());
            self.added();
        }
    }

    /// Whether startup's preparation is still under way.
    pub fn starting(&self) -> bool {
        self.startup.is_some()
    }

    /// Records a finished startup phase, `preparing` long, after which the
    /// prepared registries summarize as `summary`.
    ///
    /// ## Returns
    ///
    /// Startup's result when this was its last phase. The caller reports it
    /// and then calls [`finished`](Self::finished).
    pub fn startup_phase_finished(
        &mut self,
        phase: StartupPhase,
        preparing: Duration,
        summary: RegistrySummary,
        thread: ThreadId,
    ) -> Option<StartupReady> {
        let progress = self.startup.as_mut()?;
        match phase {
            StartupPhase::Initial => progress.initial_done = true,
            StartupPhase::Discovered => progress.discovered_done = true,
        }
        progress.preparing += preparing;
        progress.summary = summary;
        progress.thread = Some(thread);
        self.advance_startup()
    }

    /// Records file discovery's result: whether it changed the repository
    /// set, which then needs the discovered-scopes phase.
    ///
    /// ## Returns
    ///
    /// Startup's result when nothing else remains, as for
    /// [`startup_phase_finished`](Self::startup_phase_finished).
    pub fn startup_discovered(&mut self, repositories_changed: bool) -> Option<StartupReady> {
        let progress = self.startup.as_mut()?;
        progress.discovered = Some(repositories_changed);
        self.advance_startup()
    }

    /// Queues the discovered-scopes phase once both the initial phase and
    /// discovery are done, or ends startup when no phase remains.
    fn advance_startup(&mut self) -> Option<StartupReady> {
        let progress = self.startup.as_mut()?;
        let (true, Some(changed)) = (progress.initial_done, progress.discovered) else {
            return None;
        };
        if changed && !progress.discovered_done {
            if !progress.discovered_queued {
                progress.discovered_queued = self.queue.push_startup(StartupPhase::Discovered);
            }
            if progress.discovered_queued {
                return None;
            }
        }
        let progress = self.startup.take()?;
        Some(StartupReady {
            summary: progress.summary,
            preparing: progress.preparing,
            thread: progress.thread?,
        })
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

    /// Stops at shutdown: the worker takes no further job, startup is no
    /// longer awaited, nothing counts as outstanding, and every waiting
    /// request is handed back to be answered once.
    pub fn shut_down(&mut self) -> Vec<Waiter> {
        self.queue.close();
        self.startup = None;
        if self.outstanding > 0 {
            self.outstanding = 0;
            self.report();
        }
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
        let idle = lanes.documents.is_empty()
            && lanes.startup.is_empty()
            && lanes.workspace != WorkspaceSlot::Queued;
        drop(lanes);
        if idle {
            return None;
        }
        Some(match queue.next().expect("queue open") {
            RegistryJob::Workspace => "workspace".to_string(),
            RegistryJob::Startup(phase) => format!("{phase:?}"),
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

    #[test]
    fn documents_run_before_a_startup_phase_and_a_startup_phase_before_a_workspace_pass() {
        let queue = RegistryQueue::new();
        queue.schedule_workspace();
        assert!(queue.push_startup(StartupPhase::Initial));
        queue.push_document(need("doc.md"));

        assert_eq!(take(&queue).as_deref(), Some("doc.md"));
        assert_eq!(take(&queue).as_deref(), Some("Initial"));
        assert_eq!(take(&queue).as_deref(), Some("workspace"));
        queue.finish_workspace();
        assert_eq!(take(&queue), None);
    }

    fn pending() -> (Arc<RegistryQueue>, PendingRegistries) {
        let queue = Arc::new(RegistryQueue::new());
        (Arc::clone(&queue), PendingRegistries::new(queue, None))
    }

    fn finish_phase(pending: &mut PendingRegistries, phase: StartupPhase, ms: u64) -> Option<StartupReady> {
        let summary = RegistrySummary { roots: ms as usize, ..RegistrySummary::default() };
        pending.startup_phase_finished(phase, Duration::from_millis(ms), summary, std::thread::current().id())
    }

    #[test]
    fn the_discovered_phase_waits_for_both_the_initial_phase_and_discovery() {
        for discovery_first in [false, true] {
            let (queue, mut pending) = pending();
            pending.begin_startup();
            assert_eq!(take(&queue).as_deref(), Some("Initial"));
            assert_eq!(pending.outstanding, 1);

            let first = if discovery_first {
                pending.startup_discovered(true)
            } else {
                finish_phase(&mut pending, StartupPhase::Initial, 10)
            };
            assert!(first.is_none());
            assert_eq!(take(&queue), None, "the second phase waits for the other half");
            let second = if discovery_first {
                finish_phase(&mut pending, StartupPhase::Initial, 10)
            } else {
                pending.startup_discovered(true)
            };
            assert!(second.is_none(), "a discovered phase still remains");
            assert_eq!(take(&queue).as_deref(), Some("Discovered"));
            assert!(pending.starting());
            assert_eq!(pending.outstanding, 1, "startup stays outstanding between its phases");

            let ready = finish_phase(&mut pending, StartupPhase::Discovered, 5).expect("startup is ready");
            assert_eq!(ready.preparing, Duration::from_millis(15), "both phases' preparation time");
            assert_eq!(ready.summary.roots, 5, "the summary after the last phase");
            assert!(!pending.starting());
            assert_eq!(take(&queue), None, "no third phase");
        }
    }

    #[test]
    fn discovery_that_changes_no_repository_ends_startup_after_the_initial_phase() {
        let (queue, mut pending) = pending();
        pending.begin_startup();
        assert_eq!(take(&queue).as_deref(), Some("Initial"));
        assert!(finish_phase(&mut pending, StartupPhase::Initial, 7).is_none(), "discovery is still running");
        let ready = pending.startup_discovered(false).expect("nothing else remains");
        assert_eq!(ready.preparing, Duration::from_millis(7));
        assert_eq!(take(&queue), None, "no discovered phase");
        pending.finished();
        assert_eq!(pending.outstanding, 0);
    }

    #[test]
    fn shutdown_during_startup_releases_it_and_takes_no_further_phase() {
        let (queue, mut pending) = pending();
        pending.begin_startup();
        assert_eq!(take(&queue).as_deref(), Some("Initial"));
        assert!(pending.shut_down().is_empty());
        assert!(!pending.starting());
        assert_eq!(pending.outstanding, 0, "startup is no longer awaited");
        assert!(finish_phase(&mut pending, StartupPhase::Initial, 1).is_none());
        assert!(pending.startup_discovered(true).is_none());
        assert!(queue.next().is_none(), "the worker stops");
    }
}

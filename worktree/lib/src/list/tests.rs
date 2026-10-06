//! The listing pipeline against real repositories: what overlaps what, the
//! accept-or-regather decision, and what the library never touches (the
//! process's current directory).

use std::path::Path;
use std::process::Command;

mod pipeline;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git").current_dir(repo).args(args).status().expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

/// Test seam that proves what overlaps what in [`gather`](super::gather).
///
/// The pipeline reports each local gather's start (`arrive`) and end
/// (`finished`), and the end of the remote wait (`remote_finished`). Without
/// an installed seam these do nothing, so every other test runs the pipeline
/// unchanged. What `arrive` does depends on the [`Mode`]; every wait is
/// bounded, so a pipeline that does not overlap fails instead of hanging.
pub(super) mod overlap {
    use std::sync::{Arc, Condvar, Mutex, MutexGuard};
    use std::time::Duration;

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::list) enum Gather {
        List,
        Graph,
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    pub(in crate::list) enum Mode {
        /// Each local gather waits for the other to start: both start only
        /// when neither has to finish first.
        Rendezvous,
        /// Only records; a stub worker waits on the events.
        Observe,
        /// The list gather starts only once the remote wait has returned, so
        /// it certainly outlasts the wait.
        HoldListUntilRemote,
    }

    #[derive(Default)]
    struct Events {
        list: bool,
        graph: bool,
        list_saw_graph: bool,
        graph_saw_list: bool,
        list_done: bool,
        graph_done: bool,
        remote_done: bool,
        list_saw_remote_done: bool,
    }

    struct Seam {
        mode: Mode,
        events: Mutex<Events>,
        changed: Condvar,
    }

    impl Seam {
        fn lock(&self) -> MutexGuard<'_, Events> {
            self.events.lock().unwrap_or_else(|e| e.into_inner())
        }

        /// Records with `record`, then waits (bounded) while `pending` holds.
        fn update_and_wait(&self, record: impl FnOnce(&mut Events), pending: impl Fn(&Events) -> bool) -> MutexGuard<'_, Events> {
            let mut events = self.lock();
            record(&mut events);
            self.changed.notify_all();
            self.changed
                .wait_timeout_while(events, WAIT, |events| pending(events))
                .unwrap_or_else(|e| e.into_inner())
                .0
        }
    }

    /// Only a failing (non-overlapping) pipeline waits this long; an
    /// overlapping one is released as soon as the awaited event happens.
    const WAIT: Duration = Duration::from_secs(10);

    static INSTALLED: Mutex<Option<Arc<Seam>>> = Mutex::new(None);

    fn installed() -> Option<Arc<Seam>> {
        INSTALLED.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    pub(in crate::list) fn arrive(side: Gather) {
        let Some(seam) = installed() else {
            return;
        };
        let record = |events: &mut Events| match side {
            Gather::List => events.list = true,
            Gather::Graph => events.graph = true,
        };
        match (seam.mode, side) {
            (Mode::Rendezvous, _) => {
                let mut events = seam.update_and_wait(record, |events| match side {
                    Gather::List => !events.graph,
                    Gather::Graph => !events.list,
                });
                match side {
                    Gather::List => events.list_saw_graph = events.graph,
                    Gather::Graph => events.graph_saw_list = events.list,
                }
            }
            (Mode::HoldListUntilRemote, Gather::List) => {
                let mut events = seam.update_and_wait(record, |events| !events.remote_done);
                events.list_saw_remote_done = events.remote_done;
            }
            (Mode::Observe | Mode::HoldListUntilRemote, _) => drop(seam.update_and_wait(record, |_| false)),
        }
    }

    pub(in crate::list) fn finished(side: Gather) {
        if let Some(seam) = installed() {
            let mut events = seam.lock();
            match side {
                Gather::List => events.list_done = true,
                Gather::Graph => events.graph_done = true,
            }
            seam.changed.notify_all();
        }
    }

    pub(in crate::list) fn remote_finished() {
        if let Some(seam) = installed() {
            seam.lock().remote_done = true;
            seam.changed.notify_all();
        }
    }

    /// For a stub worker holding its outcome: waits (bounded) until `done`
    /// holds; `false` on timeout or without an installed seam.
    fn await_events(done: impl Fn(&Events) -> bool) -> bool {
        let Some(seam) = installed() else {
            return false;
        };
        let events = seam.lock();
        let (events, _) = seam
            .changed
            .wait_timeout_while(events, WAIT, |events| !done(events))
            .unwrap_or_else(|e| e.into_inner());
        done(&events)
    }

    /// Both local gathers have started.
    pub(in crate::list) fn await_both_started() -> bool {
        await_events(|events| events.list && events.graph)
    }

    /// Both local gathers have finished (a gather that does not run, such as
    /// the graph on a non-image path, never finishes).
    pub(in crate::list) fn await_both_finished() -> bool {
        await_events(|events| events.list_done && events.graph_done)
    }

    /// Uninstalls on drop, so a failed assertion cannot leak the seam into a
    /// later test in the same process.
    pub(in crate::list) struct Installed(Arc<Seam>);

    impl Installed {
        pub(in crate::list) fn new() -> Self {
            Self::with_mode(Mode::Rendezvous)
        }

        pub(in crate::list) fn with_mode(mode: Mode) -> Self {
            let seam = Arc::new(Seam { mode, events: Mutex::default(), changed: Condvar::new() });
            *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = Some(Arc::clone(&seam));
            Installed(seam)
        }

        /// `(list gather saw the graph start, graph gather saw the list start)`.
        pub(in crate::list) fn outcome(&self) -> (bool, bool) {
            let events = self.0.lock();
            (events.list_saw_graph, events.graph_saw_list)
        }

        /// `(list gather started, graph gather started)`.
        pub(in crate::list) fn started(&self) -> (bool, bool) {
            let events = self.0.lock();
            (events.list, events.graph)
        }

        /// The list gather started only after the remote wait returned.
        pub(in crate::list) fn list_started_after_the_wait(&self) -> bool {
            self.0.lock().list_saw_remote_done
        }
    }

    impl Drop for Installed {
        fn drop(&mut self) {
            *INSTALLED.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

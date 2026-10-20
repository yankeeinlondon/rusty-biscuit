//! The process tree a research run owns: the sequence command's process and
//! every descendant observed while it, or an observed ancestor, was alive.
//!
//! Each observation records the process's start time with its PID, so a PID
//! the OS has since reused for an unrelated process is never signaled.
//! Observations accumulate across samples, which keeps a descendant
//! addressable after its parent exits and it is reparented away from the tree.
//! A descendant that both appears and loses its observed parent between two
//! samples is not addressable; on Unix that is a double fork followed by
//! `setsid`, completed within one sampling interval.

use std::collections::HashMap;
use std::process::Child;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessStatus, ProcessesToUpdate, Signal, System};

/// How often a settlement re-reads the process table while it waits.
const SETTLE_POLL: Duration = Duration::from_millis(25);

#[derive(Clone, Copy)]
struct Observed {
    start_time: u64,
    depth: usize,
}

pub(super) struct OwnedTree {
    root: Pid,
    /// Cleared once the direct child is reaped: from then on its PID may name
    /// an unrelated process and is never used as a root again.
    root_live: bool,
    observed: HashMap<Pid, Observed>,
    system: System,
}

impl OwnedTree {
    pub(super) fn new(child: &Child) -> Self {
        Self { root: Pid::from_u32(child.id()), root_live: true, observed: HashMap::new(), system: System::new() }
    }

    /// The direct child has been reaped.
    pub(super) fn root_reaped(&mut self) {
        self.root_live = false;
    }

    /// Records every current descendant of the direct child (while it is
    /// unreaped) and of each observed process that is still the same
    /// incarnation.
    pub(super) fn sample(&mut self) {
        refresh(&mut self.system);
        let mut children: HashMap<Pid, Vec<(Pid, u64)>> = HashMap::new();
        for (pid, process) in self.system.processes() {
            if process.thread_kind().is_some() || process.status() == ProcessStatus::Zombie {
                continue;
            }
            if let Some(parent) = process.parent() {
                children.entry(parent).or_default().push((*pid, process.start_time()));
            }
        }
        let mut frontier: Vec<(Pid, u64, usize)> = Vec::new();
        if self.root_live
            && let Some(root) = self.system.process(self.root)
        {
            frontier.push((self.root, root.start_time(), 0));
        }
        for (pid, observed) in &self.observed {
            if alive(&self.system, *pid, observed.start_time) {
                frontier.push((*pid, observed.start_time, observed.depth));
            }
        }
        while let Some((parent, parent_start, depth)) = frontier.pop() {
            for &(pid, start_time) in children.get(&parent).map(Vec::as_slice).unwrap_or_default() {
                // Windows keeps a stale parent PID after the parent exits, so a
                // process older than its "parent" belongs to an earlier holder
                // of that PID, not to this tree.
                if pid == self.root || start_time < parent_start || self.observed.contains_key(&pid) {
                    continue;
                }
                self.observed.insert(pid, Observed { start_time, depth: depth + 1 });
                frontier.push((pid, start_time, depth + 1));
            }
        }
    }

    /// Requests graceful shutdown of the unreaped direct child (Unix
    /// `SIGINT`, the sequence's Ctrl-C path). `false` when the platform has no
    /// such request or it could not be sent.
    pub(super) fn interrupt_root(&mut self) -> bool {
        if !self.root_live {
            return false;
        }
        self.system.refresh_processes_specifics(ProcessesToUpdate::Some(&[self.root]), true, kind());
        self.system.process(self.root).and_then(|process| process.kill_with(Signal::Interrupt)).unwrap_or(false)
    }

    /// Kills every observed process that is still the same incarnation,
    /// deepest first, after a final sample. The direct child is the caller's
    /// to kill through its [`Child`] handle.
    pub(super) fn kill_observed(&mut self) {
        self.sample();
        let mut targets: Vec<(Pid, Observed)> = self.observed.iter().map(|(pid, observed)| (*pid, *observed)).collect();
        targets.sort_by_key(|(_, observed)| std::cmp::Reverse(observed.depth));
        for (pid, observed) in targets {
            if let Some(process) = self.system.process(pid)
                && process.start_time() == observed.start_time
                && process.status() != ProcessStatus::Zombie
            {
                process.kill();
            }
        }
    }

    /// Waits until no observed incarnation is alive, or `limit` passes.
    /// Returns how many remain.
    pub(super) fn wait_gone(&mut self, limit: Duration) -> usize {
        let until = Instant::now() + limit;
        loop {
            refresh(&mut self.system);
            let remaining =
                self.observed.iter().filter(|(pid, observed)| alive(&self.system, **pid, observed.start_time)).count();
            if remaining == 0 || Instant::now() >= until {
                return remaining;
            }
            std::thread::sleep(SETTLE_POLL);
        }
    }
}

fn kind() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing().without_tasks()
}

fn refresh(system: &mut System) {
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, kind());
}

/// Whether `pid` is still the incarnation that started at `start_time`. A
/// zombie has finished; only its reaping is outstanding.
fn alive(system: &System, pid: Pid, start_time: u64) -> bool {
    system
        .process(pid)
        .is_some_and(|process| process.start_time() == start_time && process.status() != ProcessStatus::Zombie)
}

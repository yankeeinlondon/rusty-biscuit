//! Tool processes a controlled provider started outside its process group.
//!
//! The wrapper spawns every provider as a process-group leader and signals
//! that group when the run ends. A provider that gives its tools groups of
//! their own — Pi's `bash` tool does — leaves them out of that signal's reach,
//! and once the provider dies they are re-parented, so its tree can no longer
//! be walked either. A provider's exit, abort acknowledgment, or idle state
//! therefore proves nothing about its tools.
//!
//! [`DescendantWatch`] records every descendant it sees while the provider
//! runs, as pid plus start time, and at teardown terminates each recorded
//! process that is still that same process — never a process that merely
//! reused a pid. Recording is by periodic scan, so a process that starts and
//! escapes between two scans is not seen.
//!
//! Unix only. On Windows the wait scope's Job Object already terminates every
//! descendant that did not break away.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

use super::super::TickerCancel;

/// How often the provider's tree is scanned.
const SCAN_INTERVAL: Duration = Duration::from_millis(250);

/// Recorded descendants: pid → start time.
type Seen = Arc<Mutex<BTreeMap<u32, u64>>>;

pub(super) struct DescendantWatch {
    root: u32,
    cancel: TickerCancel,
    handle: JoinHandle<()>,
    seen: Seen,
}

impl DescendantWatch {
    /// Starts recording the descendants of `root`.
    pub(super) fn start(root: u32) -> Self {
        let cancel = TickerCancel::new();
        let seen: Seen = Arc::default();
        let handle = {
            let (cancel, seen) = (cancel.clone(), Arc::clone(&seen));
            std::thread::spawn(move || {
                let mut system = System::new();
                loop {
                    record(&mut system, root, &seen);
                    if cancel.sleep(SCAN_INTERVAL) {
                        return;
                    }
                }
            })
        };
        Self { root, cancel, handle, seen }
    }

    /// Stops recording and terminates every recorded descendant still
    /// running: `SIGTERM`, then `SIGKILL` for any left after `grace`.
    /// Returns how many were still running.
    pub(super) fn reap(self, grace: Duration) -> usize {
        self.cancel.cancel();
        let _ = self.handle.join();
        let mut system = System::new();
        // One last look catches a tree that grew since the previous scan, as
        // long as the provider is still alive to be walked.
        record(&mut system, self.root, &self.seen);
        let seen = std::mem::take(&mut *self.seen.lock().unwrap_or_else(|poison| poison.into_inner()));
        let survivors: Vec<(u32, u64)> = seen.into_iter().filter(|&(pid, start)| same_process(pid, start)).collect();
        for &(pid, _) in &survivors {
            signal(pid, libc::SIGTERM);
        }
        let deadline = Instant::now() + grace;
        let mut remaining = survivors.clone();
        while !remaining.is_empty() && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
            remaining.retain(|&(pid, start)| same_process(pid, start));
        }
        for (pid, start) in remaining {
            if same_process(pid, start) {
                signal(pid, libc::SIGKILL);
            }
        }
        survivors.len()
    }
}

/// Adds every current descendant of `root` to `seen`.
fn record(system: &mut System, root: u32, seen: &Seen) {
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::nothing());
    let mut frontier = vec![Pid::from_u32(root)];
    let mut found = Vec::new();
    while let Some(parent) = frontier.pop() {
        for (pid, process) in system.processes() {
            // Linux lists threads as processes whose parent is their owner;
            // signalling one would signal that whole process.
            if process.thread_kind().is_none() && process.parent() == Some(parent) {
                frontier.push(*pid);
                found.push((pid.as_u32(), process.start_time()));
            }
        }
    }
    let mut seen = seen.lock().unwrap_or_else(|poison| poison.into_inner());
    seen.extend(found);
}

/// Whether `pid` is still the process that started at `start`.
fn same_process(pid: u32, start: u64) -> bool {
    crate::cli_utils::process_start(pid) == Some(start)
}

fn signal(pid: u32, signal: i32) {
    if let Ok(pid) = i32::try_from(pid) {
        // SAFETY: `kill(2)` has no memory-safety preconditions; `pid` was
        // just re-verified as the recorded descendant.
        unsafe {
            libc::kill(pid, signal);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::process::{Command, Stdio};

    use super::*;

    /// A shell that starts a `sleep` in a process group of its own (`set -m`
    /// gives each background job one), the shape Pi's `bash` tool produces.
    /// Killing the shell alone leaves the sleep running; the watch finds and
    /// terminates it.
    #[test]
    fn a_descendant_outside_the_group_is_terminated_after_its_parent_dies() {
        let mut shell = Command::new("/bin/sh")
            .args(["-c", "set -m; /bin/sleep 30 & echo $!; wait"])
            .stdout(Stdio::piped())
            .spawn()
            .expect("spawn fixture shell");
        let mut line = String::new();
        std::io::BufRead::read_line(&mut std::io::BufReader::new(shell.stdout.take().unwrap()), &mut line).unwrap();
        let sleeper: u32 = line.trim().parse().expect("sleeper pid");
        let start = crate::cli_utils::process_start(sleeper).expect("sleeper is running");

        let watch = DescendantWatch::start(shell.id());
        let deadline = Instant::now() + Duration::from_secs(5);
        while !watch.seen.lock().unwrap().contains_key(&sleeper) {
            assert!(Instant::now() < deadline, "the sleeper was never recorded");
            std::thread::sleep(Duration::from_millis(20));
        }
        shell.kill().unwrap();
        shell.wait().unwrap();
        assert!(same_process(sleeper, start), "the sleeper outlives its parent on its own");

        assert_eq!(watch.reap(Duration::from_secs(2)), 1);
        let deadline = Instant::now() + Duration::from_secs(5);
        while same_process(sleeper, start) {
            assert!(Instant::now() < deadline, "the sleeper was not terminated");
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    #[test]
    fn a_recorded_pid_now_naming_another_process_is_left_alone() {
        let mut bystander = Guard(Command::new("/bin/sleep").arg("30").spawn().expect("spawn bystander"));
        let root = Guard(Command::new("/bin/sleep").arg("30").spawn().expect("spawn root"));
        let watch = DescendantWatch::start(root.0.id());
        // The bystander's pid recorded with a start time it never had: a pid
        // that was reused. It is never signalled.
        watch.seen.lock().unwrap().insert(bystander.0.id(), 1);
        assert_eq!(watch.reap(Duration::from_millis(10)), 0);
        assert!(bystander.0.try_wait().unwrap().is_none(), "an unrelated process survives");
    }

    #[test]
    fn threads_are_never_recorded_as_descendants() {
        let root = Guard(Command::new("/bin/sleep").arg("30").spawn().expect("spawn root"));
        let watch = DescendantWatch::start(std::process::id());
        std::thread::sleep(Duration::from_millis(300));
        let seen: Vec<u32> = watch.seen.lock().unwrap().keys().copied().collect();
        assert!(seen.contains(&root.0.id()), "the real child process is recorded: {seen:?}");
        assert!(!seen.contains(&std::process::id()), "the process itself is not its own descendant");
        // Only real children were recorded, so dropping the watch without
        // reaping leaves this test process (and its threads) alone.
        watch.cancel.cancel();
    }

    /// Kills and reaps the child when dropped, so a failed assertion cannot
    /// leak it.
    struct Guard(std::process::Child);

    impl Drop for Guard {
        fn drop(&mut self) {
            let _ = self.0.kill();
            let _ = self.0.wait();
        }
    }
}

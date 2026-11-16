//! Unix process-tree ownership: a process group plus recorded descendants.
//!
//! The child leads its own process group (or session), so its PID is the
//! group ID and a signal to the negative PID reaches every member still in
//! the group. A descendant that calls `setsid` or `setpgid` leaves the group;
//! it stays addressable only if a sample recorded it while it was still
//! linked to the tree by parentage. One that forks, leaves the group, and is
//! reparented entirely between two samples escapes.
//!
//! Signaling a group is not atomic with a concurrent `fork` on every Unix:
//! macOS can let a child forked while the signal goes out join the group
//! without receiving it. [`kill_group_hard`] repeats the kill there.

use std::collections::{HashMap, HashSet, VecDeque};
use std::io;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command};
use std::time::{Duration, Instant};

use sysinfo::{ProcessRefreshKind, ProcessesToUpdate, System};

use super::{CleanupScan, DESCENDANT_SAMPLE_INTERVAL, TreeConfig, TreeIsolation, TreeTermination};

/// How often graceful termination checks whether the root has exited.
const GRACE_POLL: Duration = Duration::from_millis(20);

/// The most a hard group kill spends re-killing members forked while the
/// previous kill went out. A bound, not a pace: the loop ends as soon as the
/// group has no live member, normally within a millisecond.
#[cfg(not(any(target_os = "linux", target_os = "android")))]
const GROUP_REAP_LIMIT: Duration = Duration::from_millis(250);

/// One process-table entry, as the owner needs it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Observed {
    pub(super) pid: u32,
    pub(super) parent: Option<u32>,
    pub(super) start_time: u64,
}

/// The OS operations the owner performs, injectable so tests can count scans
/// and script signal results without real processes.
pub(super) trait Host {
    fn now(&self) -> Instant;
    /// Re-reads the process table; the one expensive operation.
    fn refresh(&mut self);
    /// The table as of the last [`Host::refresh`].
    fn snapshot(&self) -> Vec<Observed>;
    fn kill_pid(&mut self, pid: u32, signal: i32) -> io::Result<()>;
    fn kill_group(&mut self, pgid: i32, signal: i32) -> io::Result<()>;
}

/// The real process table and `kill(2)`.
pub(super) struct SystemHost {
    /// Created on the first refresh, so attaching allocates nothing.
    system: Option<System>,
}

impl Host for SystemHost {
    fn now(&self) -> Instant {
        Instant::now()
    }

    fn refresh(&mut self) {
        self.system.get_or_insert_with(System::new).refresh_processes_specifics(
            ProcessesToUpdate::All,
            true,
            ProcessRefreshKind::nothing(),
        );
    }

    fn snapshot(&self) -> Vec<Observed> {
        let Some(system) = &self.system else {
            return Vec::new();
        };
        system
            .processes()
            .iter()
            .map(|(pid, process)| Observed {
                pid: pid.as_u32(),
                parent: process.parent().map(|parent| parent.as_u32()),
                start_time: process.start_time(),
            })
            .collect()
    }

    fn kill_pid(&mut self, pid: u32, signal: i32) -> io::Result<()> {
        let pid = libc::pid_t::try_from(pid)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "PID does not fit pid_t"))?;
        // SAFETY: `kill` has no memory-safety preconditions. Every caller
        // passes a positive PID validated against a recorded start time.
        if unsafe { libc::kill(pid, signal) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }

    fn kill_group(&mut self, pgid: i32, signal: i32) -> io::Result<()> {
        // SAFETY: a negative PID addresses exactly one process group. `attach`
        // verified that group is the owned child's own.
        if unsafe { libc::kill(-pgid, signal) } == 0 {
            Ok(())
        } else {
            Err(io::Error::last_os_error())
        }
    }
}

pub(super) fn configure(command: &mut Command, config: TreeConfig) {
    // Exactly one mode: `setsid` fails for a process that already leads a
    // group, so `process_group(0)` must never precede it.
    match config.isolation {
        TreeIsolation::ProcessGroup => {
            command.process_group(0);
        }
        TreeIsolation::Session => {
            // SAFETY: `setsid` is async-signal-safe, allocates nothing, and
            // touches no state shared with the parent.
            unsafe {
                command.pre_exec(|| {
                    if libc::setsid() == -1 {
                        Err(io::Error::last_os_error())
                    } else {
                        Ok(())
                    }
                });
            }
        }
    }
}

/// Checks that `pid` leads its own process group.
///
/// A child that already exited is accepted: its group ID was its PID by
/// construction, and macOS answers `getpgid` with `ESRCH` for an
/// exited-but-unreaped process (Linux still reports the zombie's group), so a
/// command as fast as `true` would otherwise fail by timing alone.
pub(super) fn verify_root(pid: i32, getpgid: impl Fn(i32) -> io::Result<i32>) -> io::Result<()> {
    if pid <= 0 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "child process ID does not fit a process-group ID",
        ));
    }
    match getpgid(pid) {
        Ok(pgid) if pgid == pid => Ok(()),
        Ok(pgid) => Err(io::Error::other(format!(
            "child {pid} is in foreign process group {pgid}"
        ))),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(()),
        Err(error) => Err(error),
    }
}

fn system_getpgid(pid: i32) -> io::Result<i32> {
    // SAFETY: `getpgid` has no memory-safety preconditions.
    let pgid = unsafe { libc::getpgid(pid) };
    if pgid < 0 {
        Err(io::Error::last_os_error())
    } else {
        Ok(pgid)
    }
}

pub(super) struct Core<H: Host = SystemHost> {
    host: H,
    /// The direct child's PID, which is also the group ID.
    root: u32,
    /// The root's start time, learned from the first sample. `None` until
    /// then, and the root is then never used to find descendants by
    /// parentage: after it is reaped its PID could name another process.
    root_start: Option<u64>,
    /// Descendants recorded while linked to the tree, with the start time that
    /// identifies each incarnation. Retaining them is what keeps a `setsid`
    /// descendant addressable after it is reparented away from the tree.
    observed: HashMap<u32, u64>,
    next_sample: Instant,
    armed: bool,
}

impl Core<SystemHost> {
    pub(super) fn attach(child: &Child) -> io::Result<Self> {
        let pid = i32::try_from(child.id()).map_err(|_| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                "child process ID does not fit a process-group ID",
            )
        })?;
        verify_root(pid, system_getpgid)?;
        Ok(Self::with_host(SystemHost { system: None }, child.id()))
    }
}

impl<H: Host> Core<H> {
    /// Owns `root` through `host` without touching the process table.
    pub(super) fn with_host(host: H, root: u32) -> Self {
        let next_sample = host.now() + DESCENDANT_SAMPLE_INTERVAL;
        Self {
            host,
            root,
            root_start: None,
            observed: HashMap::new(),
            next_sample,
            armed: true,
        }
    }

    pub(super) fn root(&self) -> u32 {
        self.root
    }

    pub(super) fn sample(&mut self) {
        let now = self.host.now();
        if !self.armed || now < self.next_sample {
            return;
        }
        self.next_sample = now + DESCENDANT_SAMPLE_INTERVAL;
        self.host.refresh();
        let table = Table::new(self.host.snapshot());

        // While the caller is still in its wait loop the root is unreaped, so
        // the entry under its PID is the root itself.
        if self.root_start.is_none() {
            self.root_start = table.start_of(self.root);
        }
        let roots = self.validated_roots(&table);
        for (pid, start_time) in table.descendants(&roots) {
            self.observed.insert(pid, start_time);
        }

        // Compiled out of every non-test build. See `super::sample_hook`.
        #[cfg(test)]
        super::sample_hook::after_sample();
    }

    pub(super) fn terminate(&mut self, scan: CleanupScan) -> io::Result<()> {
        if !self.armed {
            return Ok(());
        }
        if scan == CleanupScan::Always || !self.observed.is_empty() {
            let targets = self.scan_targets();
            // Deepest first, so a parent cannot replace a killed child.
            for target in targets.into_iter().rev() {
                let _ = self.host.kill_pid(target, libc::SIGKILL);
            }
            self.observed.clear();
        }
        let pgid = i32::try_from(self.root).map_err(|_| {
            io::Error::new(io::ErrorKind::InvalidInput, "root PID does not fit pid_t")
        })?;
        kill_group_hard(&mut self.host, pgid)?;
        self.armed = false;
        Ok(())
    }

    pub(super) fn terminate_gracefully(
        &mut self,
        grace: Duration,
        child: &mut Child,
    ) -> TreeTermination {
        let mut status = None;
        if self.armed {
            match child.try_wait() {
                Ok(Some(exited)) => status = Some(Ok(exited)),
                Ok(None) | Err(_) => status = self.ask_to_exit(grace, child),
            }
        }

        // Unconditional, even when the root already exited: the descendants
        // this exists for are precisely the ones that outlive it.
        //
        // Group-ID reuse window, accepted: once the root is reaped above, its
        // PID stays reserved only while some member still carries it as a
        // group ID. If every member has exited and the kernel hands that ID to
        // a new process group between the reap and this kill, the kill reaches
        // that unrelated group. The window is only the instructions between
        // `try_wait` returning and the signal; closing it would need an exit
        // check that does not reap (`waitid` with `WNOWAIT`), which
        // `std::process::Child` does not offer. When the root was not reaped,
        // its zombie or live process pins the ID and there is no window.
        let cleanup = self.terminate(CleanupScan::Always);
        if cleanup.is_err() {
            let _ = child.kill();
        }
        let status = status.unwrap_or_else(|| child.wait());
        TreeTermination { status, cleanup }
    }

    /// Sends `SIGTERM` to the group and the validated recorded descendants,
    /// then gives the root up to `grace` to exit. Returns the root's status if
    /// it exited in time.
    fn ask_to_exit(
        &mut self,
        grace: Duration,
        child: &mut Child,
    ) -> Option<io::Result<std::process::ExitStatus>> {
        let targets = self.scan_targets();
        if let Ok(pgid) = i32::try_from(self.root) {
            let _ = self.host.kill_group(pgid, libc::SIGTERM);
        }
        for target in targets {
            let _ = self.host.kill_pid(target, libc::SIGTERM);
        }
        let deadline = self.host.now() + grace;
        loop {
            match child.try_wait() {
                Ok(Some(exited)) => return Some(Ok(exited)),
                Ok(None) if self.host.now() < deadline => std::thread::sleep(GRACE_POLL),
                Ok(None) | Err(_) => return None,
            }
        }
    }

    pub(super) fn release(mut self) -> io::Result<()> {
        self.armed = false;
        self.observed.clear();
        Ok(())
    }

    /// One refresh, then every live process this owner may signal: the
    /// descendants of the validated roots plus the validated recorded PIDs.
    /// Parents precede their descendants.
    fn scan_targets(&mut self) -> Vec<u32> {
        self.host.refresh();
        let table = Table::new(self.host.snapshot());
        let roots = self.validated_roots(&table);
        let mut targets: Vec<u32> = roots.iter().copied().filter(|pid| *pid != self.root).collect();
        for (pid, _) in table.descendants(&roots) {
            if !targets.contains(&pid) {
                targets.push(pid);
            }
        }
        targets
    }

    /// The root, if its recorded start time matches the table, and every
    /// recorded descendant whose start time matches. A recorded PID that is
    /// absent or now names another process is forgotten, so a recycled PID
    /// can neither be signaled nor lend its children to the walk.
    fn validated_roots(&mut self, table: &Table) -> Vec<u32> {
        self.observed
            .retain(|pid, start_time| table.start_of(*pid) == Some(*start_time));
        let mut roots = Vec::with_capacity(self.observed.len() + 1);
        if self.root_start.is_some() && table.start_of(self.root) == self.root_start {
            roots.push(self.root);
        }
        roots.extend(self.observed.keys().copied());
        roots
    }
}

impl<H: Host> Drop for Core<H> {
    /// Best-effort hard stop when neither terminated nor released. Never
    /// waits and never panics; the caller still reaps the direct child.
    fn drop(&mut self) {
        if self.armed {
            let _ = self.terminate(CleanupScan::WhenObserved);
        }
    }
}

/// `SIGKILL`s the group. An already empty group is success.
///
/// Off Linux, a successful kill is repeated until the group answers with an
/// error or [`GROUP_REAP_LIMIT`] passes: macOS does not make `fork` atomic
/// with a signal sent to the forking process's group, so a child forked while
/// the kill went out can join the group without the signal (about 3 in 100
/// kills of `sh -c 'echo started; sleep 300'`, sent on reading `started`, left
/// `sleep` running). The group then answers `ESRCH` when empty, or `EPERM` on
/// macOS when only zombies are left. Linux delivers the signal to a
/// concurrently forked child, and answers `0` for a group holding only an
/// unreaped zombie, so it skips the loop.
fn kill_group_hard<H: Host>(host: &mut H, pgid: i32) -> io::Result<()> {
    let delivered = classify_group_kill(host.kill_group(pgid, libc::SIGKILL))?;
    #[cfg(not(any(target_os = "linux", target_os = "android")))]
    if delivered {
        let stop = host.now() + GROUP_REAP_LIMIT;
        while host.kill_group(pgid, libc::SIGKILL).is_ok() && host.now() < stop {
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    #[cfg(any(target_os = "linux", target_os = "android"))]
    let _ = delivered;
    Ok(())
}

/// `Ok(true)` when the signal reached a member, `Ok(false)` when the group is
/// already gone, otherwise the error.
fn classify_group_kill(result: io::Result<()>) -> io::Result<bool> {
    match result {
        Ok(()) => Ok(true),
        Err(error) if error.raw_os_error() == Some(libc::ESRCH) => Ok(false),
        #[cfg(not(any(target_os = "linux", target_os = "android")))]
        Err(error) if error.raw_os_error() == Some(libc::EPERM) => Ok(false),
        Err(error) => Err(error),
    }
}

/// A process-table snapshot indexed for identity checks and parent walks.
struct Table {
    starts: HashMap<u32, u64>,
    children: HashMap<u32, Vec<(u32, u64)>>,
}

impl Table {
    fn new(entries: Vec<Observed>) -> Self {
        let mut starts = HashMap::with_capacity(entries.len());
        let mut children: HashMap<u32, Vec<(u32, u64)>> = HashMap::new();
        for entry in entries {
            starts.insert(entry.pid, entry.start_time);
            if let Some(parent) = entry.parent {
                children
                    .entry(parent)
                    .or_default()
                    .push((entry.pid, entry.start_time));
            }
        }
        Self { starts, children }
    }

    fn start_of(&self, pid: u32) -> Option<u64> {
        self.starts.get(&pid).copied()
    }

    /// Every process below `roots` by parentage, parents before children.
    fn descendants(&self, roots: &[u32]) -> Vec<(u32, u64)> {
        let mut seen: HashSet<u32> = roots.iter().copied().collect();
        let mut queue: VecDeque<u32> = roots.iter().copied().collect();
        let mut found = Vec::new();
        while let Some(parent) = queue.pop_front() {
            for &(pid, start_time) in self.children.get(&parent).into_iter().flatten() {
                if seen.insert(pid) {
                    found.push((pid, start_time));
                    queue.push_back(pid);
                }
            }
        }
        found
    }
}

#[cfg(test)]
mod tests;

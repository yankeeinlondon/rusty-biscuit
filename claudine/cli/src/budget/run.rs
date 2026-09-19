//! The live side of a budgeted run: the monotonic clock, the heartbeat
//! thread, and the process-wide hooks the launch path calls.
//!
//! One Claudine process runs at most one budgeted sequence, and the launch
//! funnel it must guard (`execute_attempt_phase`) is reached through step
//! bodies, lifecycle retries/resumes/proxies, and parallel group tasks. The
//! active run is therefore installed process-wide rather than threaded through
//! every one of those signatures. With nothing installed every hook is a no-op.

use std::cell::Cell;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use chrono::Utc;

use super::error::BudgetError;
use super::model::{CloseOutcome, Ledger, LedgerState, OrphanOutcome};
use super::store::LedgerFile;

static ACTIVE: Mutex<Option<Arc<BudgetRun>>> = Mutex::new(None);

thread_local! {
    /// The invocation this thread admitted and has not settled. Admission,
    /// spawn, and settlement of one attempt all happen on the attempt's
    /// thread, which keeps parallel group tasks apart.
    static CURRENT_INVOCATION: Cell<Option<u64>> = const { Cell::new(None) };
}

struct Live {
    ledger: Ledger,
    file: LedgerFile,
    last_fold: Instant,
}

impl Live {
    /// Milliseconds since the last fold. The fold point advances by exactly
    /// the charged amount, so truncation never drops time.
    fn take_elapsed(&mut self) -> u64 {
        let ms = u64::try_from(self.last_fold.elapsed().as_millis()).unwrap_or(u64::MAX);
        self.last_fold += Duration::from_millis(ms);
        ms
    }

    fn persist(&self) -> Result<(), BudgetError> {
        self.file.write(&self.ledger)
    }

    fn persist_or_warn(&self) {
        if let Err(error) = self.persist() {
            tracing::warn!(%error, "failed to persist the budget ledger; retrying at the next heartbeat");
        }
    }
}

/// An open budgeted run holding its ledger lock.
pub(crate) struct BudgetRun {
    live: Mutex<Live>,
    stop_flag: Mutex<Option<Arc<AtomicBool>>>,
    shutdown: Mutex<Option<Sender<()>>>,
    ticker: Mutex<Option<JoinHandle<()>>>,
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Lock a ledger and, when its previous runner died, account for the crash.
///
/// Returns the orphan outcome when recovery ran.
pub(crate) fn acquire_recovered(
    path: &Path,
) -> Result<(LedgerFile, Ledger, Option<OrphanOutcome>), BudgetError> {
    let (file, mut ledger) = LedgerFile::acquire(path)?;
    if ledger.state != LedgerState::Active {
        return Ok((file, ledger, None));
    }
    // We hold the lock, so the runner that marked this ledger active is gone.
    let orphan = terminate_orphans(&ledger);
    ledger.recover_crash(&orphan, Utc::now());
    file.write(&ledger)?;
    Ok((file, ledger, Some(orphan)))
}

impl BudgetRun {
    /// Open a run: recover a crashed predecessor, take the shared exclusive
    /// lock, and move the ledger to `Active`.
    ///
    /// ## Errors
    ///
    /// Any refusal (locked, suspended, interrupted, exhausted, invalid) is
    /// persisted first, so the ledger records why no run started.
    pub(crate) fn open(path: &Path) -> Result<Arc<Self>, BudgetError> {
        let (mut file, mut ledger, _) = acquire_recovered(path)?;
        file.lock_exclusive(&ledger)?;
        let opened = ledger.open(Utc::now());
        file.write(&ledger)?;
        opened?;
        let interval = Duration::from_millis(ledger.heartbeat_ms);
        let run = Arc::new(Self {
            live: Mutex::new(Live {
                ledger,
                file,
                last_fold: Instant::now(),
            }),
            stop_flag: Mutex::new(None),
            shutdown: Mutex::new(None),
            ticker: Mutex::new(None),
        });
        let (sender, receiver) = mpsc::channel::<()>();
        let ticking = Arc::clone(&run);
        let handle = std::thread::Builder::new()
            .name("claudine-budget-heartbeat".to_string())
            .spawn(move || {
                while let Err(RecvTimeoutError::Timeout) = receiver.recv_timeout(interval) {
                    ticking.tick();
                }
            })
            .map_err(|source| BudgetError::Io {
                path: path.to_path_buf(),
                source,
            })?;
        *lock(&run.shutdown) = Some(sender);
        *lock(&run.ticker) = Some(handle);
        Ok(run)
    }

    fn tick(&self) {
        let mut live = lock(&self.live);
        let elapsed = live.take_elapsed();
        live.ledger.heartbeat(elapsed, Utc::now());
        let stop = live.ledger.is_time_exhausted();
        if stop {
            live.ledger.note_exhaustion(Utc::now());
        }
        live.persist_or_warn();
        drop(live);
        if stop {
            self.trigger_stop();
        }
    }

    fn trigger_stop(&self) {
        if let Some(flag) = lock(&self.stop_flag).as_ref() {
            flag.store(true, Ordering::SeqCst);
        }
    }

    /// Stop the heartbeat and settle the ledger into its resting state.
    pub(crate) fn close(&self, outcome: &CloseOutcome) -> Result<Ledger, BudgetError> {
        if let Some(sender) = lock(&self.shutdown).take() {
            drop(sender);
        }
        if let Some(handle) = lock(&self.ticker).take() {
            let _ = handle.join();
        }
        let mut live = lock(&self.live);
        let elapsed = live.take_elapsed();
        live.ledger.close(outcome, elapsed, Utc::now());
        live.persist()?;
        Ok(live.ledger.clone())
    }

    pub(crate) fn exhaustion_noted(&self) -> bool {
        lock(&self.live).ledger.exhaustion_noted()
    }
}

/// Install `run` as this process's active budget.
pub(crate) fn install(run: &Arc<BudgetRun>) {
    *lock(&ACTIVE) = Some(Arc::clone(run));
}

pub(crate) fn uninstall() {
    *lock(&ACTIVE) = None;
}

fn active() -> Option<Arc<BudgetRun>> {
    lock(&ACTIVE).clone()
}

/// Give the active run the flag that stops sequence steps and shell tasks.
///
/// Exhaustion sets it, so running shell tasks terminate cooperatively and no
/// later step starts.
pub(crate) fn attach_stop_flag(flag: &Arc<AtomicBool>) {
    let Some(run) = active() else { return };
    *lock(&run.stop_flag) = Some(Arc::clone(flag));
    if run.exhaustion_noted() {
        run.trigger_stop();
    }
}

/// Record the sequence step being entered; stops the run when exhausted.
pub(crate) fn enter_stage(name: &str) {
    let Some(run) = active() else { return };
    let mut live = lock(&run.live);
    let elapsed = live.take_elapsed();
    let exhausted = live.ledger.enter_stage(name, elapsed, Utc::now());
    live.persist_or_warn();
    drop(live);
    if exhausted {
        run.trigger_stop();
    }
}

/// Debit one invocation before an agent is spawned.
///
/// Returns the remaining active allowance, which the caller applies as the
/// launch's deadline, or `None` when no budget is active. The allowance is
/// rounded up to whole seconds because the wall-clock timeout it feeds has
/// one-second resolution; the overrun is charged like any other time.
///
/// ## Errors
///
/// Refuses when either limit is reached, or when the debit could not be
/// persisted: a launch whose charge is not on disk must not start.
pub(crate) fn admit_launch(attempt: u32) -> Result<Option<Duration>, BudgetError> {
    let Some(run) = active() else { return Ok(None) };
    let mut live = lock(&run.live);
    let elapsed = live.take_elapsed();
    match live.ledger.admit(attempt, elapsed, Utc::now()) {
        Ok(admission) => {
            live.persist()?;
            CURRENT_INVOCATION.with(|cell| cell.set(Some(admission.invocation)));
            Ok(Some(Duration::from_secs(admission.remaining_ms.div_ceil(1000))))
        }
        Err(refusal) => {
            live.persist_or_warn();
            drop(live);
            run.trigger_stop();
            Err(refusal)
        }
    }
}

/// Attach the spawned child's identity to this thread's admitted launch.
pub(crate) fn record_child(pid: u32) {
    let Some(run) = active() else { return };
    let Some(invocation) = CURRENT_INVOCATION.with(Cell::get) else {
        return;
    };
    let start = process_start(pid);
    let mut live = lock(&run.live);
    live.ledger.record_child(invocation, pid, start, Utc::now());
    live.persist_or_warn();
}

/// Settle this thread's admitted launch, if any.
pub(crate) fn settle_launch() {
    let Some(run) = active() else { return };
    let Some(invocation) = CURRENT_INVOCATION.with(|cell| cell.take()) else {
        return;
    };
    let mut live = lock(&run.live);
    let elapsed = live.take_elapsed();
    let exhausted = live.ledger.settle(invocation, elapsed, Utc::now());
    live.persist_or_warn();
    drop(live);
    if exhausted {
        run.trigger_stop();
    }
}

/// Cap an automatic wait (retry backoff) to the remaining active allowance.
///
/// The wait itself is charged by the clock; a capped wait ends exactly at
/// exhaustion, so the next admission refuses instead of sleeping past it.
pub(crate) fn cap_wait(delay: Duration) -> Duration {
    let Some(run) = active() else { return delay };
    let mut live = lock(&run.live);
    let elapsed = live.take_elapsed();
    live.ledger.heartbeat(elapsed, Utc::now());
    delay.min(Duration::from_millis(live.ledger.remaining().active_ms))
}

/// Whether the active run stopped because its budget ran out.
pub(crate) fn stopped_by_exhaustion() -> bool {
    active().is_some_and(|run| run.exhaustion_noted())
}

/// Start time (seconds since the Unix epoch) of a live process.
fn process_start(pid: u32) -> Option<u64> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    system.process(pid).map(sysinfo::Process::start_time)
}

/// Terminate every in-flight launch a dead runner left behind, signaling a
/// PID only when its start time still matches the recorded one.
fn terminate_orphans(ledger: &Ledger) -> OrphanOutcome {
    let rank = |outcome: &OrphanOutcome| match outcome {
        OrphanOutcome::NoneRecorded => 0,
        OrphanOutcome::NotRunning => 1,
        OrphanOutcome::IdentityMismatch => 2,
        OrphanOutcome::Terminated => 3,
        OrphanOutcome::TerminateFailed(_) => 4,
    };
    ledger
        .in_flight
        .iter()
        .map(|entry| match (entry.pid, entry.process_start) {
            (None, _) => OrphanOutcome::NoneRecorded,
            (Some(_), None) => OrphanOutcome::IdentityMismatch,
            (Some(pid), Some(recorded)) => match process_start(pid) {
                None => OrphanOutcome::NotRunning,
                Some(actual) if actual != recorded => OrphanOutcome::IdentityMismatch,
                Some(_) => match kill_tree(pid) {
                    Ok(()) => OrphanOutcome::Terminated,
                    Err(error) => OrphanOutcome::TerminateFailed(error),
                },
            },
        })
        .max_by_key(rank)
        .unwrap_or(OrphanOutcome::NoneRecorded)
}

/// Kill the orphan's process group. Claudine spawns captured agents as group
/// leaders; a descendant that called `setsid` has left the group and escapes.
#[cfg(unix)]
fn kill_tree(pid: u32) -> std::io::Result<()> {
    let pid = i32::try_from(pid).map_err(std::io::Error::other)?;
    // SAFETY: `kill` has no memory-safety preconditions.
    if unsafe { libc::kill(-pid, libc::SIGKILL) } == 0 {
        return Ok(());
    }
    // SAFETY: as above.
    if unsafe { libc::kill(pid, libc::SIGKILL) } == 0 {
        return Ok(());
    }
    Err(std::io::Error::last_os_error())
}

/// On Windows the Job Object dies with its wrapper and takes the tree with
/// it, so a surviving verified PID escaped the job; terminate it directly.
#[cfg(windows)]
fn kill_tree(pid: u32) -> std::io::Result<()> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};

    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        true,
        ProcessRefreshKind::nothing(),
    );
    match system.process(pid) {
        Some(process) if process.kill() => Ok(()),
        Some(_) => Err(std::io::Error::other("TerminateProcess failed")),
        None => Ok(()),
    }
}

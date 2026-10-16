//! The Messenger-owned runner: one research run at a time.
//!
//! [`execute`] takes the fleet lock for a whole execution, marks the run
//! launched, runs its sequence command from the repository root, and records
//! what it observed ([`RunnerOutcome`]) in `run.json`: `finished` leaves the
//! status `check-run` decided; `interrupted` and `stopped` rest the run on
//! `interrupted`/`failed` with a stop reason. The elapsed-time limit from the
//! run's [`RunLimits`] is enforced here: a sequence that overruns it is
//! stopped.
//!
//! The run owns the sequence command's process tree, and the fleet lock is
//! released only once that tree has settled. Stopping the sequence (at the
//! elapsed-time limit, or when it can no longer be waited on) first asks it to
//! shut down as Ctrl-C would, where the platform has such a request (Unix
//! `SIGINT`, so Claudine cancels its tasks and runs their cleanup), then kills
//! whatever of the tree outlives a grace period. Descendants are found by
//! sampling the process table while the sequence runs; see `owned_tree` for
//! the one shape that sampling cannot address. A sequence that exits on its
//! own has settled its own tasks, so nothing is killed after a normal exit.
//!
//! Each prepared prompt reserves a durable agent admission through
//! [`admit_agent`] before launch, enforcing the invocation cap across resumes.
//! The OS releases the lock when the runner dies, so a crashed
//! runner leaves a launched `active` record that is stale and must be
//! recovered explicitly (`cleanup::recover`), never silently.

use std::process::Command;
use std::time::{Duration, Instant};

use super::RefreshError;
use super::owned_tree::OwnedTree;
use super::prepare;
use super::state::{RunRecord, RunStatus, RunnerOutcome, StateArea, fleet_lock_path};
use crate::research::load::Loader;
use crate::research::publish::fsutil::{LockError, PublicationLock};

/// How often the runner polls a running sequence for the elapsed-time limit.
const POLL: Duration = Duration::from_millis(250);

/// Polls between descendant samples while the sequence runs.
const SAMPLE_EVERY: u32 = 4;

/// How long a stopped sequence has to finish its own cleanup before the rest
/// of its tree is killed.
pub const STOP_GRACE: Duration = Duration::from_secs(10);

/// How long the runner waits for killed processes to disappear.
const KILL_SETTLE: Duration = Duration::from_secs(5);

/// How often a stopping sequence is polled during its grace period.
const GRACE_POLL: Duration = Duration::from_millis(25);

/// Runs a prepared run's sequence command under the fleet lock and records
/// the outcome in its run record. The command is `claudine sequence --yolo
/// [extra…] <run_dir>/run.md` from the repository root; `extra` carries
/// caller arguments such as a provider flag.
///
/// ## Errors
///
/// [`RefreshError::WrongStatus`] unless the run is active;
/// [`RefreshError::RunnerLive`] while another runner holds the fleet lock;
/// [`RefreshError::NotRepositoryTopLevel`] as for [`prepare::prepare`].
pub fn execute(loader: &Loader, run_id: &str, extra: &[String]) -> Result<RunRecord, RefreshError> {
    let record = StateArea::new(loader.workspace()).load(run_id)?;
    let run_dir = format!("{}/{}/{}", super::state::RUNS, record.platform_id, record.run_id);
    let mut command = ["claudine", "sequence", "--yolo"].map(str::to_string).to_vec();
    command.extend(extra.iter().cloned());
    command.push(format!("{run_dir}/run.md"));
    execute_with(loader, run_id, &command)
}

/// [`execute`] with an explicit command, run from the repository root with
/// inherited standard streams. The fleet-lock protection, process-tree
/// settlement, and outcome recording are identical.
///
/// ## Errors
///
/// As [`execute`].
pub fn execute_with(loader: &Loader, run_id: &str, command: &[String]) -> Result<RunRecord, RefreshError> {
    execute_with_grace(loader, run_id, command, STOP_GRACE)
}

/// [`execute_with`] with a shorter stop grace period, so tests need not wait
/// out [`STOP_GRACE`].
#[doc(hidden)]
pub fn execute_with_grace(
    loader: &Loader,
    run_id: &str,
    command: &[String],
    grace: Duration,
) -> Result<RunRecord, RefreshError> {
    prepare::require_top_level(loader.workspace().repo_root())?;
    let state = StateArea::new(loader.workspace());
    let mut record = state.load(run_id)?;
    if record.status != RunStatus::Active {
        return Err(RefreshError::WrongStatus {
            run_id: record.run_id.to_string(),
            status: record.status,
            action: "running",
            needs: "an active run",
        });
    }
    let _lock = acquire(&state)?;
    prepare::ensure_admission_hooks(&state.run_dir(record.platform_id, &record.run_id), run_id)?;
    record.launched = true;
    state.save(&record)?;
    // `_lock` outlives `run`, which returns only once the owned tree has settled.
    let settled = run(loader.workspace().repo_root(), command, record.limits.max_seconds, grace);
    // The sequence's own `check-run` steps may have decided the run while it
    // ran; re-read before recording the runner's observation so it is applied
    // to what the run last wrote, not to the snapshot taken at start.
    let mut record = state.load(run_id)?;
    record.runner = Some(match &settled {
        Settled::Finished => RunnerOutcome::Finished,
        Settled::Interrupted { .. } => RunnerOutcome::Interrupted,
        Settled::Stopped { .. } => RunnerOutcome::Stopped,
    });
    match settled {
        Settled::Finished => {}
        Settled::Interrupted { reason } => {
            record.status = RunStatus::Interrupted;
            record.stop_reason = Some(reason);
        }
        Settled::Stopped { reason } => {
            // A failed check already rests the run with its own reason.
            if record.status == RunStatus::Active {
                record.status = RunStatus::Failed;
                record.stop_reason = Some(reason);
            }
        }
    }
    state.save(&record)?;
    Ok(record)
}

/// Reserves one agent launch before a prepared prompt starts. The count is
/// durable across resumptions; a refused admission leaves the record intact.
/// The preparation lock serializes read/update/write with resume and other
/// admissions, independently of the runner's whole-execution fleet lock.
///
/// ## Errors
///
/// Refuses non-active records, a spent invocation limit, or a busy preparation
/// lock. Record read/write failures are returned without launching an agent.
pub fn admit_agent(loader: &Loader, run_id: &str) -> Result<RunRecord, RefreshError> {
    let state = StateArea::new(loader.workspace());
    let path = state.prepare_lock();
    let _lock = PublicationLock::try_acquire(&path).map_err(|error| match error {
        LockError::Held => RefreshError::PrepareBusy { path: path.display().to_string() },
        LockError::Io(source) => super::state::io_err("lock agent admission")(source).into(),
    })?;
    let mut record = state.load(run_id)?;
    if record.status != RunStatus::Active {
        return Err(RefreshError::WrongStatus {
            run_id: run_id.to_string(), status: record.status,
            action: "admitting an agent", needs: "an active run",
        });
    }
    if record.invocations >= record.limits.max_invocations {
        return Err(RefreshError::InvocationLimit { run_id: run_id.to_string(), max: record.limits.max_invocations });
    }
    record.invocations += 1;
    state.save(&record)?;
    Ok(record)
}

/// The fleet lock, held for the whole execution. The loser is refused rather
/// than queued, so two sequences never share one research state area.
fn acquire(state: &StateArea) -> Result<PublicationLock, RefreshError> {
    PublicationLock::try_acquire(&state.fleet_lock()).map_err(|error| match error {
        LockError::Held => RefreshError::RunnerLive { path: fleet_lock_path() },
        LockError::Io(source) => super::state::io_err(format!("lock {}", fleet_lock_path()))(source).into(),
    })
}

/// How the sequence command settled, with the stop reason Messenger records.
enum Settled {
    Finished,
    Interrupted { reason: String },
    Stopped { reason: String },
}

/// Runs `command` from `root`, stopping it at the elapsed-time limit; a
/// stopped command returns only once its owned process tree has settled. The Windows
/// console-control exit value is treated as an interrupt alongside `130`,
/// matching the documented Ctrl-C exits of the sequence's children.
fn run(root: &std::path::Path, command: &[String], max_seconds: u64, grace: Duration) -> Settled {
    let (program, args) = command.split_first().expect("a command");
    let mut child = match Command::new(program).args(args).current_dir(root).spawn() {
        Ok(child) => child,
        Err(error) => {
            return Settled::Stopped { reason: format!("the sequence command could not start: {error}") };
        }
    };
    let mut tree = OwnedTree::new(&child);
    let deadline = Instant::now() + Duration::from_secs(max_seconds);
    let mut polls: u32 = 0;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {}
            Err(error) => {
                stop(&mut child, &mut tree, grace);
                return Settled::Stopped { reason: format!("the sequence could not be waited on: {error}") };
            }
        }
        if Instant::now() >= deadline {
            stop(&mut child, &mut tree, grace);
            return Settled::Stopped { reason: format!("the sequence exceeded the {max_seconds}s elapsed-time limit") };
        }
        polls += 1;
        if polls.is_multiple_of(SAMPLE_EVERY) {
            tree.sample();
        }
        std::thread::sleep(POLL);
    };
    // A sequence that exits on its own has already settled its tasks;
    // whatever it left running was detached on purpose (Playa's audio
    // worker finishing a lifecycle sound) and is not killed.
    match status.code() {
        Some(0) => Settled::Finished,
        Some(130 | -1073741510) => Settled::Interrupted { reason: "the sequence was interrupted".to_string() },
        Some(code) => Settled::Stopped { reason: format!("the sequence stopped before its checks passed (exit {code})") },
        None => Settled::Interrupted { reason: "the sequence was terminated by a signal".to_string() },
    }
}

/// Stops a running sequence and settles its tree: a graceful request and up
/// to `grace` for the sequence to exit, then a kill of every process of the
/// tree that remains. The tree is sampled before the request, while the
/// sequence is certainly alive to name its descendants.
fn stop(child: &mut std::process::Child, tree: &mut OwnedTree, grace: Duration) {
    tree.sample();
    if !grace.is_zero() && tree.interrupt_root() {
        let until = Instant::now() + grace;
        while Instant::now() < until {
            match child.try_wait() {
                Ok(Some(_)) => {
                    tree.root_reaped();
                    break;
                }
                Ok(None) => std::thread::sleep(GRACE_POLL),
                Err(_) => break,
            }
        }
    }
    tree.kill_observed();
    let _ = child.kill();
    let _ = child.wait();
    tree.root_reaped();
    settle(tree);
}

fn settle(tree: &mut OwnedTree) {
    let remaining = tree.wait_gone(KILL_SETTLE);
    if remaining > 0 {
        tracing::warn!(remaining, "research run processes survived settlement");
    }
}

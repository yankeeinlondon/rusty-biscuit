//! Spawn and readiness pre-flight for a child driven by a retained-stdin
//! control session (see [`super::super::control`]).
//!
//! The child's stdout and stderr are drained by forwarding threads from the
//! moment it starts. The stdout forwarder hands every line to the session
//! before passing it on, so the session sees its responses even while the
//! semantic parser does not exist yet; lines that arrive before readiness wait
//! in the channel and reach the parser in order once it is built. The task is
//! submitted only after the session reports [`Readiness::Ready`].

use std::collections::HashMap;
use std::ffi::OsString;
use std::io::{BufRead, BufReader};
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{self, Receiver};
use std::thread;
use std::time::{Duration, Instant};

use claudine::stream::logs::EarlyTermination;
use color_eyre::eyre::Result;
use tracing::Span;

use super::super::control::{ControlChannels, Readiness, StdioControl};
use super::super::kill_process_group;
use super::super::termination::CompletionTermination;
use super::setup;

/// Lines from one of the child's output pipes.
pub(super) type LineSource = Box<dyn Iterator<Item = std::io::Result<String>> + Send>;

/// How often the pre-flight re-checks readiness and the child.
const READINESS_POLL: Duration = Duration::from_millis(10);
/// How long a failed child's remaining stderr is collected for the report.
const STDERR_DRAIN: Duration = Duration::from_millis(500);

/// A spawned child that proved ready and has been given its task.
pub(super) struct RetainedChild {
    pub(super) child: Child,
    pub(super) stdout: LineSource,
    pub(super) stderr: LineSource,
    pub(super) early: Receiver<EarlyTermination>,
    pub(super) completion: Receiver<CompletionTermination>,
    /// Tools the child starts outside its process group, reaped at teardown.
    #[cfg(unix)]
    pub(super) descendants: super::descendants::DescendantWatch,
}

/// The outcome of the pre-flight.
pub(super) enum Preflight {
    Ready(Box<RetainedChild>),
    /// The child was terminated before any task was submitted.
    NotReady { reason: String, stderr: Vec<String> },
}

/// Spawns the child, opens `control` over its stdin, waits for readiness,
/// and submits `task`.
#[allow(clippy::too_many_arguments)]
pub(super) fn spawn_retained(
    binary: &Path,
    args: &[String],
    env: &HashMap<OsString, OsString>,
    cwd: &Path,
    control: &Arc<dyn StdioControl>,
    task: &str,
    child_spawned: &mut bool,
    run_scope: super::super::super::run_scope::RunScope,
) -> Result<Preflight> {
    let launch_args = control.launch_args();
    let args = launch_args.as_deref().unwrap_or(args);
    let mut command = setup::base_command(binary, args, env, cwd);
    command.stdin(Stdio::piped()).stdout(Stdio::piped()).stderr(Stdio::piped());
    setup::isolate_into_process_group(&mut command);

    let mut child = command.spawn()?;
    let pid = child.id();
    *child_spawned = true;
    crate::budget::record_child(pid);
    Span::current().record("child_pid", tracing::field::display(pid));

    let stdin = child.stdin.take().expect("child stdin is piped");
    let stdout = child.stdout.take().expect("child stdout is piped");
    let stderr = child.stderr.take().expect("child stderr is piped");

    let (stdout_tx, stdout_rx) = mpsc::channel();
    let observer = Arc::clone(control);
    thread::spawn(move || {
        for line in BufReader::new(super::super::super::run_scope::retention::RetainingReader::capture_only(stdout, run_scope)).lines() {
            let failed = line.is_err();
            if let Ok(line) = &line {
                observer.observe(line);
            }
            // The semantic reader may already be gone; the session still
            // needs every line until the pipe closes.
            let _ = stdout_tx.send(line);
            if failed {
                break;
            }
        }
    });
    let (stderr_tx, stderr_rx) = mpsc::channel();
    thread::spawn(move || {
        for line in BufReader::new(stderr).lines() {
            let failed = line.is_err();
            if stderr_tx.send(line).is_err() || failed {
                break;
            }
        }
    });

    let (early_tx, early) = mpsc::channel();
    let (completion_tx, completion) = mpsc::channel();
    let opened = control.open(stdin, pid, ControlChannels { early: early_tx, completion: completion_tx });
    let readiness = match opened {
        Ok(()) => await_readiness(&mut child, control.as_ref()),
        Err(error) if control.abandon_readiness("the provider's input closed before it was ready") => {
            Readiness::Failed(crate::steering::render_chain("the provider's input could not be written", &error))
        }
        Err(_) => control.readiness(),
    };
    if let Readiness::Failed(reason) = readiness {
        // Reap the leader before signalling its group: Linux counts an
        // unreaped zombie as a group member, so the group signal would wait
        // out its whole grace period for a child that is already gone.
        let _ = child.kill();
        let _ = child.wait();
        kill_process_group(&mut child);
        control.finish();
        return Ok(Preflight::NotReady { reason, stderr: drain(stderr_rx) });
    }

    // From here the task may have reached the provider, so a failure is the
    // run's to report; it is never retried or replaced by a fallback.
    if let Err(error) = control.submit(task) {
        tracing::warn!(target: "claudine::wrap", error = %error, "the task could not be fully written to the provider");
    }
    Ok(Preflight::Ready(Box::new(RetainedChild {
        #[cfg(unix)]
        descendants: super::descendants::DescendantWatch::start(pid),
        child,
        stdout: Box::new(stdout_rx.into_iter()),
        stderr: Box::new(stderr_rx.into_iter()),
        early,
        completion,
    })))
}

/// Waits until the session leaves [`Readiness::Pending`], the child exits,
/// the user interrupts, or the session's deadline passes.
fn await_readiness(child: &mut Child, control: &dyn StdioControl) -> Readiness {
    let deadline = Instant::now() + control.readiness_deadline();
    loop {
        let readiness = control.readiness();
        if readiness != Readiness::Pending {
            return readiness;
        }
        let reason = if matches!(child.try_wait(), Ok(Some(_))) {
            Some("the provider exited before it was ready".to_string())
        } else if crate::output::user_interrupt_observed() {
            Some("interrupted before the provider was ready".to_string())
        } else if Instant::now() >= deadline {
            Some(format!(
                "the provider did not answer its readiness check within {}s",
                control.readiness_deadline().as_secs()
            ))
        } else {
            None
        };
        if let Some(reason) = reason {
            if control.abandon_readiness(&reason) {
                return Readiness::Failed(reason);
            }
            return control.readiness();
        }
        thread::sleep(READINESS_POLL);
    }
}

fn drain(stderr: Receiver<std::io::Result<String>>) -> Vec<String> {
    let deadline = Instant::now() + STDERR_DRAIN;
    let mut lines = Vec::new();
    while let Ok(Ok(line)) = stderr.recv_timeout(deadline.saturating_duration_since(Instant::now())) {
        lines.push(line);
    }
    lines
}

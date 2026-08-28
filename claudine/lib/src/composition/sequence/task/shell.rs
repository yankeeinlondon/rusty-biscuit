//! Running a task's approved shell commands and capturing their stdout.
//!
//! A task's commands arrive already resolved by preflight — approved bytes are
//! executed bytes — so nothing here interpolates. What it adds over the
//! lifecycle [`ShellRunner`](super::super::super::lifecycle::executor::ShellRunner)
//! is the two things a task needs and a lifecycle action does not: the command's
//! stdout (which becomes the task's `outputs` entry) and a per-command deadline.
//!
//! A deadline is only worth as much as the termination behind it. `sh -c 'x &'`
//! leaves a descendant that inherits stdout and outlives the shell, so killing
//! the shell alone reports a kill while the pipe stays open — the caller then
//! blocks forever on a command it believes it already terminated. Every command
//! therefore owns a *tree*: a process group on Unix, a kill-on-close Job Object
//! on Windows. Termination targets the tree, capture is bounded by the runaway
//! volume cap, and the wait for the capture thread is bounded too, so no
//! descendant can defeat the deadline by retaining a pipe handle.
//!
//! ## Ownership contract
//!
//! Tree ownership is uniform across macOS, Linux, and Windows, and fail-closed:
//!
//! - **Establishment is fallible.** A command whose tree cannot be owned — the
//!   Unix process group is missing, or the Windows Job Object cannot be
//!   created, configured, assigned, or resumed — is killed and reported as
//!   [`TaskShellError::Isolation`]. There is no degraded direct-child-only
//!   mode: the deadline and the runaway guards are only enforceable against a
//!   tree that is actually owned.
//! - **Descendants are reaped at completion, on every exit path.** When the
//!   command finishes — success, kill, or a failed wait — the remaining
//!   members of its tree are killed before the capture threads are waited on,
//!   so `x &` never outlives its task on any platform and no descendant can
//!   hold the output pipe open to emit frames after the task's footer. Every
//!   post-spawn exit shares the one epilogue in `run` (tree teardown, bounded
//!   reader settlement, live-stream flush); a panic unwind still reaches the
//!   reap through [`ProcessTree`]'s `Drop`.
//! - **Windows assignment is race-free.** The child is spawned suspended,
//!   assigned to the kill-on-close Job, and only then resumed, so it cannot
//!   create a descendant outside the Job.
//!
//! ## Both pipes, live, on their own channels
//!
//! Each pipe is drained by its own chunked reader, and every chunk is framed
//! through the task's [`TaskLiveOutput`] as it is read rather than after the
//! command returns. That is what makes a long-running command's output arrive
//! while it runs, and what makes two concurrent tasks interleave by line arrival
//! instead of by completion (spec → *Reporting Concurrency*).
//!
//! The channel split is the command contract, not a formatting choice: **stdout
//! is task data** and also the bytes that become the `outputs` entry, while
//! **stderr is status only** — attributed and shown, never captured. Streaming
//! reads from the same buffer that feeds capture, so a line cannot reach
//! `outputs` without having been displayed, nor be displayed twice.

use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::SyncSender;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::render::TaskLiveOutput;
use crate::runaway::{CaptureVolumeCap, Trip};

/// The default per-command budget when a task authors no `timeout:`.
pub const DEFAULT_COMMAND_TIMEOUT: Duration = Duration::from_secs(30);

/// How often the wait loop re-checks a running child.
const POLL_INTERVAL: Duration = Duration::from_millis(20);

/// How long the tree gets between `SIGTERM` and `SIGKILL`.
///
/// Deliberately far shorter than the wrapper's 10 s `kill_grace`: that budget
/// covers a provider session flushing a transcript, whereas this is one command
/// inside one task of a sequence, and the task's own deadline has *already*
/// expired by the time we get here. Spending seconds more on politeness would
/// make the per-command timeout a lie.
///
/// Unix-only: the Windows path terminates the Job Object outright, which offers
/// no graceful rung to schedule.
#[cfg(unix)]
const TREE_KILL_GRACE: Duration = Duration::from_millis(250);

/// How long the capture thread gets to notice its pipe closed.
///
/// Bounded rather than joined: the whole defect this guards is a descendant
/// holding the read end open, and a blocked reader must cost the deadline
/// nothing. Whatever bytes it appended before the wait expired are still
/// reported.
const READER_SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

/// Capture read granularity.
const CAPTURE_CHUNK: usize = 8 * 1024;

/// What one command produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShellCommandOutput {
    /// Everything the command wrote to stdout, undecorated.
    pub stdout: String,
    /// The process exit code. `-1` when the platform reported none.
    pub exit_code: i32,
    /// `true` when the command was killed for exceeding its budget. `exit_code`
    /// is then the kill's code and carries no information about the work.
    pub timed_out: bool,
    /// `true` when the command was killed because the user interrupted the run.
    ///
    /// This is how Ctrl+C reaches a *running* child rather than only the gap
    /// between commands: every sibling in a parallel group watches the same flag,
    /// so one press fans out to all of them.
    pub interrupted: bool,
    /// Present when the command was killed because the per-task runaway volume
    /// cap tripped; carries which limit tripped, the observed count, and the
    /// configured limit, so a slightly oversized command reads differently
    /// from an uncontrolled flood.
    ///
    /// Distinct from [`timed_out`](Self::timed_out): this is the
    /// [`ProcessTermination::Aborted`] sense — the command was doing something
    /// wrong rather than merely taking too long — so it fails fast and is never
    /// retried.
    ///
    /// [`ProcessTermination::Aborted`]: crate::harness::ProcessTermination
    pub runaway: Option<RunawayTrip>,
}

/// Which capture-volume limit a runaway command tripped, and by how much.
///
/// `observed` is the counter's value when the trip was detected, which is a
/// chunk boundary rather than the offending byte: the guard checks after each
/// read, so the observed count can exceed the limit by up to one chunk from
/// each pipe. When two pipes trip, the first observed trip wins; when a single
/// chunk pushes *both* counters past their limits there is no observable
/// "first", so the line limit is reported deterministically.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunawayTrip {
    /// Which of the two limits tripped.
    pub kind: RunawayTripKind,
    /// The counter's value when the trip was detected.
    pub observed: u64,
    /// The configured limit that value crossed.
    pub limit: u64,
}

/// The unit of the limit a [`RunawayTrip`] reports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunawayTripKind {
    /// The line-count limit tripped.
    Lines,
    /// The byte-count limit tripped.
    Bytes,
}

impl RunawayTrip {
    /// Classify which limit to report for counters that tripped `cap`.
    fn from_counters(cap: &CaptureVolumeCap, lines: u64, bytes: u64) -> Self {
        if lines > cap.max_lines {
            Self {
                kind: RunawayTripKind::Lines,
                observed: lines,
                limit: cap.max_lines,
            }
        } else {
            Self {
                kind: RunawayTripKind::Bytes,
                observed: bytes,
                limit: cap.max_bytes,
            }
        }
    }
}

impl std::fmt::Display for RunawayTrip {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let unit = match self.kind {
            RunawayTripKind::Lines => "lines",
            RunawayTripKind::Bytes => "bytes",
        };
        write!(f, "{} {unit}, limit {}", self.observed, self.limit)
    }
}

/// Why a task shell command failed outside its own exit status.
///
/// [`Io`](Self::Io) and [`Isolation`](Self::Isolation) precede any result —
/// the command never ran. [`Wait`](Self::Wait) is the post-spawn failure: the
/// command ran, and may have streamed output, but its exit could not be
/// observed. A command that ran to an observable end reports through
/// [`ShellCommandOutput`], including the three kill triggers.
#[derive(Debug, thiserror::Error)]
pub enum TaskShellError {
    /// The process could not be spawned.
    #[error(transparent)]
    Io(#[from] std::io::Error),
    /// The process spawned and ran, but waiting on it failed, so its exit
    /// status is unknown.
    ///
    /// Split from [`Io`](Self::Io) because the operation stage changes what
    /// the user is told: a wait failure means the command *did* run — its
    /// output may already be on screen — where a spawn failure means it never
    /// started.
    #[error("waiting on the spawned process failed: {0}")]
    Wait(#[source] std::io::Error),
    /// The process spawned, but exclusive ownership of its process tree could
    /// not be established, so it was killed without running.
    ///
    /// Fail-closed by design: without an owned tree the deadline, interrupt,
    /// and runaway guards can each be defeated by a descendant, so the command
    /// is refused rather than run with direct-child-only cleanup.
    #[error("process-tree ownership could not be established: {0}")]
    Isolation(#[source] std::io::Error),
}

/// Runs one approved command under a deadline, capturing stdout.
///
/// Injectable so task tests assert dispatch, byte parity, and ordering without
/// spawning processes.
pub trait TaskShellRunner: Sync {
    /// Run `command`, killing it after `timeout` or as soon as `interrupt` is
    /// set, streaming both pipes through `live` as they are read.
    ///
    /// An implementation that writes to `live` owes the caller the two channel
    /// rules: stdout onto [`TaskLiveOutput::append`], stderr onto
    /// [`TaskLiveOutput::append_status`]. Whatever it streams must be the same
    /// undecorated bytes it reports through [`ShellCommandOutput::stdout`], so
    /// the caller never has to emit them a second time.
    ///
    /// ## Errors
    ///
    /// Returns [`TaskShellError::Io`] when the process could not be spawned,
    /// [`TaskShellError::Wait`] when it ran but waiting on it failed, and
    /// [`TaskShellError::Isolation`] when it spawned but whole-tree ownership
    /// could not be established. A command that ran and failed reports its
    /// code through [`ShellCommandOutput::exit_code`], and one that was killed
    /// reports which of the three triggers fired.
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, TaskShellError>;
}

/// Production [`TaskShellRunner`]: the platform's system shell, both pipes
/// streamed live through the task's attributed stream.
///
/// Each command runs as its own process tree — see the module docs for why —
/// and its stdout is capped so a flooding command bounds this process's memory
/// instead of growing an unbounded `Vec<u8>`.
///
/// ## Examples
///
/// ```no_run
/// use std::time::Duration;
/// use claudine::composition::{SystemTaskShell, TaskShellRunner};
///
/// let output = SystemTaskShell::default().run("echo hi", Duration::from_secs(5), None, None)?;
/// assert_eq!(output.stdout.trim_end(), "hi");
/// # Ok::<(), claudine::composition::TaskShellError>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct SystemTaskShell {
    /// The cap applied to one command's captured stdout. Tests inject a tiny
    /// one so the overflow path is reachable in milliseconds; the derived
    /// default is the production 50k-line / 32 MiB guard.
    volume_cap: CaptureVolumeCap,
    /// Fault injection: report ownership establishment as failed right after a
    /// real spawn. The real tree is reaped first, so the injected failure
    /// exercises the fail-closed path without leaking the fixture's processes.
    #[cfg(test)]
    fail_isolation: bool,
    /// Fault injection: fail the first wait attempt after the command's first
    /// captured stdout byte, modeling `try_wait` erroring while the command
    /// and its descendants are still alive with output already in flight.
    /// Gating on the byte is what makes "bytes buffered at the wait failure"
    /// fixtures deterministic — so their commands must write to stdout.
    #[cfg(test)]
    fail_wait: bool,
}

impl SystemTaskShell {
    /// Build a runner with a non-default capture volume cap.
    pub fn with_volume_cap(volume_cap: CaptureVolumeCap) -> Self {
        Self {
            volume_cap,
            #[cfg(test)]
            fail_isolation: false,
            #[cfg(test)]
            fail_wait: false,
        }
    }
}

#[cfg(test)]
impl SystemTaskShell {
    /// A runner whose ownership establishment always fails.
    pub(crate) fn failing_isolation() -> Self {
        Self {
            fail_isolation: true,
            ..Self::default()
        }
    }

    /// A runner whose first wait attempt after captured output always fails.
    #[cfg(unix)]
    pub(crate) fn failing_wait() -> Self {
        Self {
            fail_wait: true,
            ..Self::default()
        }
    }
}

impl TaskShellRunner for SystemTaskShell {
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, TaskShellError> {
        let mut builder = system_shell_command(command);
        builder
            .stdout(Stdio::piped())
            // Piped only when there is somewhere attributed to put it. With no
            // live stream — a `--silent` run, or a caller executing tasks
            // outside a terminal — inheriting keeps a command's diagnostics
            // visible instead of swallowing them into a buffer nobody reads.
            .stderr(if live.is_some() {
                Stdio::piped()
            } else {
                Stdio::inherit()
            })
            .stdin(Stdio::null());
        isolate_process_tree(&mut builder);
        let mut child = builder.spawn()?;
        let ownership = ProcessTree::own(&child);
        #[cfg(test)]
        let ownership = ownership.and_then(|tree| {
            if self.fail_isolation {
                // The injected failure stands in for a real one, so the real
                // tree must not leak: dropping it reaps the group/Job first.
                drop(tree);
                Err(std::io::Error::other("injected isolation failure"))
            } else {
                Ok(tree)
            }
        });
        let tree = match ownership {
            Ok(tree) => tree,
            Err(error) => {
                // Fail closed: a command whose descendants cannot be reaped
                // must not run. The direct child is all that verifiably exists
                // — ownership of anything beyond it is exactly what could not
                // be established — and on Windows it is still suspended, so
                // killing it here is also what prevents it from ever forking.
                let _ = child.kill();
                let _ = child.wait();
                return Err(TaskShellError::Isolation(error));
            }
        };

        // Each pipe is drained on its own thread: a command that fills a pipe
        // buffer would otherwise block forever and the deadline below would
        // report a timeout for what is really a reader that never ran.
        //
        // The stdout buffer is shared rather than returned, because the main
        // thread must be able to take what accumulated even when the reader
        // never finishes.
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        // `None` until a reader trips the volume cap; first trip wins, so a
        // stdout flood and a stderr flood cannot overwrite each other's report.
        let runaway = Arc::new(Mutex::new(None::<RunawayTrip>));
        let (finished_tx, finished_rx) = std::sync::mpsc::sync_channel::<()>(2);

        let mut readers = 1usize;
        let stdout_reader = spawn_capture(
            child.stdout.take().expect("stdout was piped"),
            self.volume_cap.clone(),
            Arc::clone(&runaway),
            finished_tx.clone(),
            Some(Arc::clone(&captured)),
            live.map(Arc::clone).map(|live| (live, Channel::Data)),
        );
        // stderr gets its own budget against the same limits rather than a
        // share of stdout's. The guard exists to stop a command flooding this
        // process, and a command can flood either pipe; splitting the counters
        // keeps one noisy channel from making the other's cap arrive early.
        let stderr_reader = match child.stderr.take() {
            Some(pipe) => {
                readers += 1;
                Some(spawn_capture(
                    pipe,
                    self.volume_cap.clone(),
                    Arc::clone(&runaway),
                    finished_tx.clone(),
                    None,
                    live.map(Arc::clone).map(|live| (live, Channel::Status)),
                ))
            }
            None => None,
        };

        let start = Instant::now();
        let mut timed_out = false;
        let mut interrupted = false;
        // The loop breaks with a `Result` rather than `?`-returning on a
        // failed wait: every post-spawn exit — clean, killed, or a wait error
        // — must reach the one epilogue below (tree teardown, bounded reader
        // settlement, live-stream flush). An early return would leave detached
        // readers free to drain buffered bytes *after* the caller has closed
        // the task stream, appending body frames behind the failure footer.
        let wait_outcome = loop {
            #[cfg(test)]
            if self.fail_wait
                && captured.lock().map(|buffer| !buffer.is_empty()).unwrap_or(true)
            {
                // Modeling `try_wait` failing while everything still runs and
                // output is already in the pipes — the shape the epilogue's
                // settle-then-flush exists for.
                break Err(std::io::Error::other("injected wait failure"));
            }
            match child.try_wait() {
                Ok(Some(status)) => break Ok(status),
                Ok(None) => {}
                Err(error) => break Err(error),
            }
            if interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                interrupted = true;
            } else if observed_trip(&runaway).is_some() {
                // Terminate rather than merely stopping the capture: a command
                // that ignores SIGPIPE would otherwise keep flooding a pipe
                // nobody reads until its deadline.
            } else if start.elapsed() >= timeout {
                timed_out = true;
            } else {
                std::thread::sleep(POLL_INTERVAL);
                continue;
            }
            tree.terminate(&mut child);
            break child.wait();
        };
        // The ratified policy for every post-spawn exit: an owned tree does
        // not outlive its command, on any platform. Dropping here — before the
        // reader settle — reaps whatever the command backgrounded (and, on a
        // wait error, the command itself), which is also what closes the pipes
        // those readers are parked on: a dead descendant cannot emit a frame
        // after the task's footer.
        drop(tree);

        // Never `join()`: see `READER_SHUTDOWN_GRACE`. Dropping the handles
        // detaches a reader that is still parked on a descendant's pipe; the
        // cap bounds what it can still append behind us. The grace is one
        // budget across both readers, not one each, so adding stderr did not
        // double what a wedged descendant can cost the deadline.
        let settle_by = Instant::now() + READER_SHUTDOWN_GRACE;
        for _ in 0..readers {
            let remaining = settle_by.saturating_duration_since(Instant::now());
            if finished_rx.recv_timeout(remaining).is_err() {
                break;
            }
        }
        drop(stdout_reader);
        drop(stderr_reader);
        // A command whose last line carried no newline leaves a fragment held
        // in the stream. Flushing per command rather than per task is what
        // keeps a two-command task from showing the first command's tail only
        // after the second has finished.
        if let Some(live) = live {
            live.flush();
        }
        // Only now — readers settled, stream flushed — may the preserved wait
        // error surface. The tree drop above already killed the still-running
        // command; waiting reaps the direct child so a failed `try_wait` does
        // not also leak a zombie.
        let status = match wait_outcome {
            Ok(status) => status,
            Err(error) => {
                let _ = child.wait();
                return Err(TaskShellError::Wait(error));
            }
        };
        // Read once the reader has settled rather than inside the wait loop:
        // closing the pipe on a trip usually kills the flooder by SIGPIPE
        // first, so the child can reap before the loop ever sees the trip.
        let runaway = observed_trip(&runaway);
        let bytes = match captured.lock() {
            Ok(mut buffer) => std::mem::take(&mut *buffer),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };

        Ok(ShellCommandOutput {
            stdout: String::from_utf8_lossy(&bytes).into_owned(),
            exit_code: status.code().unwrap_or(-1),
            timed_out,
            interrupted,
            runaway,
        })
    }
}

/// Read the shared trip slot, surviving a poisoned lock: a reader that
/// panicked after recording its trip still reported a real trip.
fn observed_trip(slot: &Mutex<Option<RunawayTrip>>) -> Option<RunawayTrip> {
    match slot.lock() {
        Ok(guard) => *guard,
        Err(poisoned) => *poisoned.into_inner(),
    }
}

/// Which of a task stream's two channels a pipe feeds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Channel {
    /// stdout — task data, and the payload `outputs` captures.
    Data,
    /// stderr — status only, attributed and shown but never captured.
    Status,
}

impl Channel {
    /// Frame `text` onto this channel of `live`.
    fn append(self, live: &TaskLiveOutput, text: &str) {
        match self {
            Self::Data => live.append(text),
            Self::Status => live.append_status(text),
        }
    }
}

/// Drain one pipe on its own thread, streaming and optionally capturing it.
///
/// The one read serves both duties: bytes go into `captured` (when this pipe is
/// the one that becomes the `outputs` payload) and, decoded, onto `live`'s
/// channel. A second pass would let the two disagree about what the command
/// produced.
///
/// The returned handle is meant to be *dropped*, not joined — see
/// [`READER_SHUTDOWN_GRACE`].
fn spawn_capture(
    mut pipe: impl Read + Send + 'static,
    cap: CaptureVolumeCap,
    runaway: Arc<Mutex<Option<RunawayTrip>>>,
    finished: SyncSender<()>,
    captured: Option<Arc<Mutex<Vec<u8>>>>,
    live: Option<(Arc<TaskLiveOutput>, Channel)>,
) -> std::thread::JoinHandle<()> {
    std::thread::spawn(move || {
        let mut chunk = [0u8; CAPTURE_CHUNK];
        // Chunk boundaries fall wherever the pipe buffer filled, which is not
        // where code points end. Decoding per chunk with `from_utf8_lossy`
        // would put a replacement character in the middle of any multi-byte
        // character unlucky enough to straddle one.
        let mut decoder = Utf8Stream::default();
        let mut lines: u64 = 0;
        let mut bytes: u64 = 0;
        loop {
            let read = match pipe.read(&mut chunk) {
                Ok(0) | Err(_) => break,
                Ok(read) => read,
            };
            let slice = &chunk[..read];
            bytes += read as u64;
            lines += slice.iter().filter(|byte| **byte == b'\n').count() as u64;
            if let Some(captured) = &captured
                && let Ok(mut buffer) = captured.lock()
            {
                buffer.extend_from_slice(slice);
            }
            if let Some((live, channel)) = &live {
                let text = decoder.push(slice);
                if !text.is_empty() {
                    channel.append(live, &text);
                }
            }
            if let Some(Trip::RunawayVolume { .. }) = cap.check(lines, bytes) {
                let trip = RunawayTrip::from_counters(&cap, lines, bytes);
                let mut slot = match runaway.lock() {
                    Ok(guard) => guard,
                    Err(poisoned) => poisoned.into_inner(),
                };
                slot.get_or_insert(trip);
                break;
            }
        }
        if let Some((live, channel)) = &live {
            let tail = decoder.finish();
            if !tail.is_empty() {
                channel.append(live, &tail);
            }
        }
        let _ = finished.send(());
        // `pipe` drops as this closure returns, closing the read end, so a
        // descendant still writing into it gets EPIPE rather than an audience.
    })
}

/// Decodes a byte stream into `str` chunks without splitting a code point.
///
/// Holds at most three bytes: the longest incomplete prefix of a UTF-8
/// sequence. Genuinely invalid bytes are replaced immediately rather than held,
/// because they will never be completed by a later chunk.
#[derive(Debug, Default)]
struct Utf8Stream {
    carry: Vec<u8>,
}

impl Utf8Stream {
    /// Decode everything `bytes` completes, holding any incomplete tail.
    fn push(&mut self, bytes: &[u8]) -> String {
        self.carry.extend_from_slice(bytes);
        let mut decoded = String::new();
        loop {
            match std::str::from_utf8(&self.carry) {
                Ok(text) => {
                    decoded.push_str(text);
                    self.carry.clear();
                    return decoded;
                }
                Err(error) => {
                    let valid = error.valid_up_to();
                    decoded.push_str(
                        std::str::from_utf8(&self.carry[..valid])
                            .expect("`valid_up_to` bounds a valid prefix"),
                    );
                    let Some(invalid) = error.error_len() else {
                        // An incomplete tail: a later chunk may complete it.
                        self.carry.drain(..valid);
                        return decoded;
                    };
                    decoded.push(char::REPLACEMENT_CHARACTER);
                    self.carry.drain(..valid + invalid);
                }
            }
        }
    }

    /// Release whatever incomplete tail is still held, at end of stream.
    fn finish(&mut self) -> String {
        if self.carry.is_empty() {
            return String::new();
        }
        let held = std::mem::take(&mut self.carry);
        String::from_utf8_lossy(&held).into_owned()
    }
}

/// Ownership of one command's whole process tree.
///
/// Constructing one is fallible — see the module's *Ownership contract* — and
/// dropping one reaps every member still alive, which is what makes every exit
/// path (success, early wait error, panic unwind) end the tree.
#[cfg(unix)]
struct ProcessTree {
    /// The process-group id, equal to the direct child's pid per
    /// `process_group(0)`.
    pgid: i32,
}

/// Ownership of one command's whole process tree. See the Unix twin.
#[cfg(windows)]
struct ProcessTree {
    /// The kill-on-close Job Object owning the tree. Always valid: `own`
    /// refuses to return a tree it does not fully own.
    job: isize,
}

#[cfg(unix)]
impl ProcessTree {
    /// Take ownership of `child`'s tree.
    ///
    /// `process_group(0)` already ran between fork and exec — a failure there
    /// fails the spawn itself — so this *verifies* the group rather than
    /// establishing it: a child observed outside its own group means the
    /// isolation the deadline depends on does not exist, and the command must
    /// not run.
    ///
    /// A child that already exited is still owned: its group id was its pid by
    /// construction, and any descendant that outlives it stays in that group.
    /// macOS answers `getpgid` with `ESRCH` for an exited-but-unreaped process
    /// (Linux still reports the zombie's group), so a command as fast as
    /// `printf` would otherwise fail ownership on macOS purely by timing.
    fn own(child: &Child) -> Result<Self, std::io::Error> {
        let pid = child.id() as i32;
        let pgid = unsafe { libc::getpgid(pid) };
        if pgid < 0 {
            let error = std::io::Error::last_os_error();
            if error.raw_os_error() == Some(libc::ESRCH) {
                return Ok(Self { pgid: pid });
            }
            return Err(error);
        }
        if pgid != pid {
            return Err(std::io::Error::other(format!(
                "child {pid} is in foreign process group {pgid}"
            )));
        }
        Ok(Self { pgid: pid })
    }

    /// Terminate every process in the tree, not merely the direct child.
    fn terminate(&self, child: &mut Child) {
        // SAFETY: a negative pid addresses a process group. `own` verified the
        // group id equals this child's pid, so the group can contain nothing
        // but this command's own descendants.
        let signalled = unsafe { libc::kill(-self.pgid, libc::SIGTERM) } == 0;
        if !signalled {
            let _ = child.kill();
            return;
        }
        let deadline = Instant::now() + TREE_KILL_GRACE;
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            std::thread::sleep(POLL_INTERVAL);
        }
        // Unconditional, even when the direct child already reaped: the
        // descendants this exists for are precisely the ones that outlive it,
        // and they are the ones still holding stdout open.
        //
        // SAFETY: as above.
        unsafe {
            libc::kill(-self.pgid, libc::SIGKILL);
        }
    }
}

#[cfg(unix)]
impl Drop for ProcessTree {
    /// Reap every remaining member of the group.
    ///
    /// `SIGKILL` with no graceful rung: on every path that reaches here the
    /// command itself has already finished or been terminated, so whatever
    /// remains is a backgrounded straggler the ownership contract says dies
    /// with the task. Killing an already-empty group is an `ESRCH` no-op.
    fn drop(&mut self) {
        // SAFETY: as in `terminate`.
        unsafe {
            libc::kill(-self.pgid, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
impl ProcessTree {
    /// Take ownership of `child`'s tree by assigning it to a fresh
    /// kill-on-close Job Object and only then letting it run.
    ///
    /// The child was spawned `CREATE_SUSPENDED` (see [`isolate_process_tree`]),
    /// so the create → configure → assign → resume sequence is race-free: a
    /// process that has never been scheduled cannot have created a descendant
    /// outside the Job. Any failure leaves the child suspended for the caller
    /// to kill.
    fn own(child: &Child) -> Result<Self, std::io::Error> {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let job = unsafe { CreateJobObjectW(None, None) }.map_err(std::io::Error::other)?;
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let established = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .and_then(|()| AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())))
        }
        .and_then(|()| resume_main_thread(child.id()));
        if let Err(error) = established {
            // Kill-on-close reaps the child if assignment got that far; the
            // caller kills the still-suspended direct child either way.
            unsafe {
                let _ = CloseHandle(job);
            }
            return Err(std::io::Error::other(error));
        }
        Ok(Self { job: job.0 as isize })
    }

    /// Terminate every process in the tree, not merely the direct child.
    fn terminate(&self, _child: &mut Child) {
        unsafe {
            let _ =
                windows::Win32::System::JobObjects::TerminateJobObject(as_handle(self.job), 1);
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    /// Reap every remaining member of the Job. See the Unix twin.
    fn drop(&mut self) {
        unsafe {
            let _ =
                windows::Win32::System::JobObjects::TerminateJobObject(as_handle(self.job), 1);
            // Kill-on-close makes the close a backstop for the terminate.
            let _ = windows::Win32::Foundation::CloseHandle(as_handle(self.job));
        }
    }
}

/// Resume the `CREATE_SUSPENDED` child's initial thread.
///
/// A suspended, freshly created process has exactly one thread, so resuming
/// the first thread the snapshot attributes to `pid` completes the spawn. Not
/// finding one — or the resume failing — leaves the child suspended and is
/// reported as an ownership failure.
#[cfg(windows)]
fn resume_main_thread(pid: u32) -> windows::core::Result<()> {
    use windows::Win32::Foundation::CloseHandle;
    use windows::Win32::System::Diagnostics::ToolHelp::{
        CreateToolhelp32Snapshot, TH32CS_SNAPTHREAD, THREADENTRY32, Thread32First, Thread32Next,
    };
    use windows::Win32::System::Threading::{OpenThread, ResumeThread, THREAD_SUSPEND_RESUME};

    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPTHREAD, 0) }?;
    let outcome = (|| {
        let mut entry = THREADENTRY32 {
            dwSize: std::mem::size_of::<THREADENTRY32>() as u32,
            ..Default::default()
        };
        unsafe { Thread32First(snapshot, &mut entry) }?;
        loop {
            if entry.th32OwnerProcessID == pid {
                let thread =
                    unsafe { OpenThread(THREAD_SUSPEND_RESUME, false, entry.th32ThreadID) }?;
                let resumed = unsafe { ResumeThread(thread) };
                unsafe {
                    let _ = CloseHandle(thread);
                }
                // `u32::MAX` is the documented `ResumeThread` failure sentinel.
                if resumed == u32::MAX {
                    return Err(windows::core::Error::from_thread());
                }
                return Ok(());
            }
            // Exhausting the snapshot without a match reports the iterator's
            // own `ERROR_NO_MORE_FILES`, which is exactly what happened.
            unsafe { Thread32Next(snapshot, &mut entry) }?;
        }
    })();
    unsafe {
        let _ = CloseHandle(snapshot);
    }
    outcome
}

#[cfg(windows)]
fn as_handle(raw: isize) -> windows::Win32::Foundation::HANDLE {
    windows::Win32::Foundation::HANDLE(raw as *mut core::ffi::c_void)
}

/// Put the command in its own process group so termination can address the
/// whole tree.
#[cfg(unix)]
fn isolate_process_tree(command: &mut Command) {
    use std::os::unix::process::CommandExt;

    // `0` means "new group whose id is the child's pid", which is what lets
    // `ProcessTree::terminate` name the group without a second lookup.
    command.process_group(0);
}

/// Put the command in its own process group so termination can address the
/// whole tree.
#[cfg(windows)]
fn isolate_process_tree(command: &mut Command) {
    use std::os::windows::process::CommandExt;

    // Defined locally rather than imported so the spawn path does not pull the
    // `windows` crate in for two constants.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    // Suspended so `ProcessTree::own` can assign the child to its Job before
    // it executes a single instruction — the race-free half of the ownership
    // contract. `own` resumes it; an ownership failure kills it suspended.
    const CREATE_SUSPENDED: u32 = 0x0000_0004;

    command.creation_flags(CREATE_NEW_PROCESS_GROUP | CREATE_SUSPENDED);
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(command: &str) -> Command {
    use std::os::windows::process::CommandExt;

    let mut cmd = Command::new("cmd");
    // `cmd.exe` parses the command tail itself rather than with the Windows
    // argv rules `Command::arg` targets. Passing the tail as a normal argument
    // escapes its nested quotes, changing the command before the shell sees it.
    cmd.arg("/D").arg("/C").raw_arg(command);
    cmd
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(not(windows))]
fn system_shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

#[cfg(test)]
mod tests;

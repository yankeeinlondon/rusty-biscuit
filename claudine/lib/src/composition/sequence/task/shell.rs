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
    /// `true` when the command was killed because the per-task runaway volume
    /// cap tripped.
    ///
    /// Distinct from [`timed_out`](Self::timed_out): this is the
    /// [`ProcessTermination::Aborted`] sense — the command was doing something
    /// wrong rather than merely taking too long — so it fails fast and is never
    /// retried.
    ///
    /// [`ProcessTermination::Aborted`]: crate::harness::ProcessTermination
    pub aborted: bool,
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
    /// Returns the underlying [`std::io::Error`] only when the process could
    /// not be spawned or waited on. A command that ran and failed reports its
    /// code through [`ShellCommandOutput::exit_code`], and one that was killed
    /// reports which of the three triggers fired.
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, std::io::Error>;
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
/// # Ok::<(), std::io::Error>(())
/// ```
#[derive(Debug, Clone, Default)]
pub struct SystemTaskShell {
    /// The cap applied to one command's captured stdout. Tests inject a tiny
    /// one so the overflow path is reachable in milliseconds; the derived
    /// default is the production 50k-line / 32 MiB guard.
    volume_cap: CaptureVolumeCap,
}

impl SystemTaskShell {
    /// Build a runner with a non-default capture volume cap.
    pub fn with_volume_cap(volume_cap: CaptureVolumeCap) -> Self {
        Self { volume_cap }
    }
}

impl TaskShellRunner for SystemTaskShell {
    fn run(
        &self,
        command: &str,
        timeout: Duration,
        interrupt: Option<&AtomicBool>,
        live: Option<&Arc<TaskLiveOutput>>,
    ) -> Result<ShellCommandOutput, std::io::Error> {
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
        let tree = ProcessTree::own(&child);

        // Each pipe is drained on its own thread: a command that fills a pipe
        // buffer would otherwise block forever and the deadline below would
        // report a timeout for what is really a reader that never ran.
        //
        // The stdout buffer is shared rather than returned, because the main
        // thread must be able to take what accumulated even when the reader
        // never finishes.
        let captured = Arc::new(Mutex::new(Vec::<u8>::new()));
        let overflow = Arc::new(AtomicBool::new(false));
        let (finished_tx, finished_rx) = std::sync::mpsc::sync_channel::<()>(2);

        let mut readers = 1usize;
        let stdout_reader = spawn_capture(
            child.stdout.take().expect("stdout was piped"),
            self.volume_cap.clone(),
            Arc::clone(&overflow),
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
                    Arc::clone(&overflow),
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
        let status = loop {
            if let Some(status) = child.try_wait()? {
                break status;
            }
            if interrupt.is_some_and(|flag| flag.load(Ordering::SeqCst)) {
                interrupted = true;
            } else if overflow.load(Ordering::SeqCst) {
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
            break child.wait()?;
        };

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
        // Read once the reader has settled rather than inside the wait loop:
        // closing the pipe on a trip usually kills the flooder by SIGPIPE
        // first, so the child can reap before the loop ever sees the flag.
        let aborted = overflow.load(Ordering::SeqCst);
        let bytes = match captured.lock() {
            Ok(mut buffer) => std::mem::take(&mut *buffer),
            Err(poisoned) => std::mem::take(&mut *poisoned.into_inner()),
        };

        Ok(ShellCommandOutput {
            stdout: String::from_utf8_lossy(&bytes).into_owned(),
            exit_code: status.code().unwrap_or(-1),
            timed_out,
            interrupted,
            aborted,
        })
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
    overflow: Arc<AtomicBool>,
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
                overflow.store(true, Ordering::SeqCst);
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
/// Unix needs no state — the process group established at spawn is addressed by
/// the child's own pid — while Windows carries the Job Object handle whose
/// `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` limit is what reaps descendants when
/// this value drops at the end of the command.
#[cfg(unix)]
struct ProcessTree;

/// Ownership of one command's whole process tree. See the Unix twin.
#[cfg(windows)]
struct ProcessTree {
    /// `None` when the Job could not be created or assigned; termination then
    /// degrades to killing the direct child. A command that runs is worth more
    /// than one that fails because the OS refused a Job Object.
    job: Option<isize>,
}

#[cfg(unix)]
impl ProcessTree {
    /// Take ownership of `child`'s tree.
    fn own(_child: &Child) -> Self {
        Self
    }

    /// Terminate every process in the tree, not merely the direct child.
    fn terminate(&self, child: &mut Child) {
        let pid = child.id() as i32;
        // SAFETY: a negative pid addresses a process group. `spawn` used
        // `process_group(0)`, so the group id equals this child's pid and the
        // group can contain nothing but this command's own descendants.
        let signalled = unsafe { libc::kill(-pid, libc::SIGTERM) } == 0;
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
            libc::kill(-pid, libc::SIGKILL);
        }
    }
}

#[cfg(windows)]
impl ProcessTree {
    /// Take ownership of `child`'s tree by assigning it to a fresh
    /// kill-on-close Job Object.
    ///
    /// Assignment is valid here because it happens immediately after `spawn`,
    /// before the child has forked anything of its own.
    fn own(child: &Child) -> Self {
        use std::os::windows::io::AsRawHandle;
        use windows::Win32::Foundation::{CloseHandle, HANDLE};
        use windows::Win32::System::JobObjects::{
            AssignProcessToJobObject, CreateJobObjectW, JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
            JOBOBJECT_EXTENDED_LIMIT_INFORMATION, JobObjectExtendedLimitInformation,
            SetInformationJobObject,
        };

        let Ok(job) = (unsafe { CreateJobObjectW(None, None) }) else {
            return Self { job: None };
        };
        let mut info = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
        info.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
        let assigned = unsafe {
            SetInformationJobObject(
                job,
                JobObjectExtendedLimitInformation,
                std::ptr::from_ref(&info).cast(),
                std::mem::size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
            )
            .and_then(|()| AssignProcessToJobObject(job, HANDLE(child.as_raw_handle())))
        };
        if assigned.is_err() {
            unsafe {
                let _ = CloseHandle(job);
            }
            return Self { job: None };
        }
        Self {
            job: Some(job.0 as isize),
        }
    }

    /// Terminate every process in the tree, not merely the direct child.
    fn terminate(&self, child: &mut Child) {
        match self.job {
            Some(raw) => unsafe {
                let _ = windows::Win32::System::JobObjects::TerminateJobObject(as_handle(raw), 1);
            },
            None => {
                let _ = child.kill();
            }
        }
    }
}

#[cfg(windows)]
impl Drop for ProcessTree {
    fn drop(&mut self) {
        if let Some(raw) = self.job {
            // Closing the last handle is what makes kill-on-close destroy
            // anything still assigned, so a command that returned normally but
            // left a descendant behind still gets cleaned up here.
            unsafe {
                let _ = windows::Win32::Foundation::CloseHandle(as_handle(raw));
            }
        }
    }
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
    // `windows` crate in for a single constant.
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

    command.creation_flags(CREATE_NEW_PROCESS_GROUP);
}

/// Build the platform `Command` that runs `command` through the system shell.
#[cfg(windows)]
fn system_shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
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
mod tests {
    use super::Utf8Stream;

    /// The chunk boundary a fixed-size reader actually produces: mid-character.
    #[test]
    fn a_code_point_split_across_chunks_is_decoded_whole() {
        let bytes = "é".as_bytes();
        let mut stream = Utf8Stream::default();

        assert_eq!(stream.push(&bytes[..1]), "", "a partial code point is held");
        assert_eq!(stream.push(&bytes[1..]), "é");
        assert_eq!(stream.finish(), "");
    }

    /// A three-byte character split at both possible boundaries.
    #[test]
    fn a_three_byte_code_point_survives_either_split() {
        for split in 1..3 {
            let bytes = "日".as_bytes();
            let mut stream = Utf8Stream::default();
            let decoded = format!("{}{}", stream.push(&bytes[..split]), stream.push(&bytes[split..]));
            assert_eq!(decoded, "日", "split after {split} byte(s)");
        }
    }

    /// Text either side of a held fragment still comes through in one piece.
    #[test]
    fn complete_text_around_a_held_fragment_is_emitted_immediately() {
        let mut stream = Utf8Stream::default();
        let mut bytes = b"ready ".to_vec();
        bytes.extend_from_slice(&"✅".as_bytes()[..2]);

        assert_eq!(stream.push(&bytes), "ready ");
        assert_eq!(stream.push(&"✅".as_bytes()[2..]), "✅");
    }

    /// A byte that no continuation can rescue is replaced at once rather than
    /// held — otherwise a binary-emitting command would stall the stream.
    #[test]
    fn a_genuinely_invalid_byte_is_replaced_without_being_held() {
        let mut stream = Utf8Stream::default();

        assert_eq!(stream.push(b"a\xffb"), "a\u{FFFD}b");
        assert_eq!(stream.finish(), "", "nothing was left held");
    }

    /// An incomplete tail at end of stream is released rather than dropped.
    #[test]
    fn an_incomplete_tail_is_released_at_end_of_stream() {
        let mut stream = Utf8Stream::default();

        assert_eq!(stream.push(&"€".as_bytes()[..2]), "");
        assert_eq!(stream.finish(), "\u{FFFD}");
    }
}

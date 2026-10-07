//! Inherited-output spawn mode: PID capture and wall-clock timeout.

use super::*;

/// Spawn must populate `ProcessResult.agent_pid` immediately after
/// `command.spawn()?` returns. The captured PID must match a real positive
/// integer. A platform shell exits successfully so the fixture does not
/// assume a Unix executable layout. This test exercises the
/// legacy/interactive spawn path used by direct wrappers and legacy
/// composition runs.
#[test]
fn run_child_captures_agent_pid_after_successful_spawn() {
    let env = minimal_env();
    let cwd = test_cwd();
    let (binary, args) = test_shell_command("exit 0", "exit 0");
    let mut child_spawned = false;

    let result = run_child(
        &binary,
        &args,
        &env,
        &cwd,
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
    )
    .expect("spawning the platform shell fixture must succeed");

    assert!(child_spawned, "child_spawned flag must flip on success");
    let pid = result
        .agent_pid
        .expect("agent_pid must be Some after a successful spawn");
    assert!(pid > 0, "spawned child PID must be a positive integer");
}

/// VC-5.2 / tasks 5.1+5.2: the same wall-clock routing for the direct
/// (`run_child`) path with inherited stdio. Proves both non-streaming
/// spawn paths share the one signal-aware wait loop.
#[cfg(unix)]
#[test]
fn run_child_wall_clock_timeout_reaps_child() {
    use std::time::{Duration, Instant};

    let env = minimal_env();
    let cwd = Path::new("/tmp");
    let mut child_spawned = false;

    let start = Instant::now();
    let result = run_child(
        sleep_binary(),
        &["30".to_string()],
        &env,
        cwd,
        Some(1),
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &[],
            stderr_noise_prefixes: &[],
            stdin_seed: None,
        },
        &mut child_spawned,
    )
    .expect("spawning sleep must succeed on the test host");
    let elapsed = start.elapsed();

    assert!(child_spawned);
    assert_eq!(
        result.termination,
        claudine::harness::ProcessTermination::TimedOut,
        "a breached wall-clock timeout must report TimedOut"
    );
    assert!(
        elapsed < Duration::from_secs(20),
        "the wall-clock timeout must kill the child promptly; took {elapsed:?}"
    );
}

use super::super::super::reader_join::{ReaderBudget, ReaderProgress, ReaderStream};
use super::super::inherited::{ForwardReader, forward_lines, join_forwarders};
use std::io::{BufReader, Cursor, Write};
use std::sync::{Arc, Mutex, mpsc};
use std::time::{Duration, Instant};

/// Short bounds so no test waits on the production 5 s / 120 s values.
const BUDGET: ReaderBudget = ReaderBudget {
    pipe_cap: Duration::from_millis(100),
    pipe_settle: Duration::from_millis(30),
    drain_limit: Duration::from_millis(300),
};

/// A terminal that refuses every write.
struct BrokenSink;

impl Write for BrokenSink {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("terminal went away"))
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// A terminal that stops accepting output on its first write until released.
struct StalledSink {
    entered: mpsc::Sender<()>,
    release: mpsc::Receiver<()>,
}

impl Write for StalledSink {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        let _ = self.entered.send(());
        let _ = self.release.recv();
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

fn forwarder<W: Write + Send + 'static>(
    stream: ReaderStream,
    input: &'static str,
    mut sink: W,
) -> ForwardReader {
    let progress = ReaderProgress::default();
    let reader_progress = progress.clone();
    let handle = std::thread::spawn(move || {
        forward_lines(
            BufReader::new(Cursor::new(input)),
            &mut sink,
            &["noise:".to_string()],
            &Arc::new(Mutex::new(None)),
            false,
            &reader_progress,
        )
    });
    ForwardReader {
        stream,
        progress,
        handle,
    }
}

/// A forward that fails is a warning naming the stream and the cause, and the
/// reader still drained the whole pipe so the child is never wedged.
#[test]
fn a_failed_forward_is_a_warning_and_the_pipe_is_still_drained() {
    let mut sink = Vec::new();
    let first_error = forward_lines(
        BufReader::new(Cursor::new("noise: skipped\none\ntwo\n")),
        &mut sink,
        &["noise:".to_string()],
        &Arc::new(Mutex::new(None)),
        false,
        &ReaderProgress::default(),
    );
    assert!(first_error.is_none());
    assert_eq!(String::from_utf8(sink).unwrap(), "one\ntwo\n");

    let warnings = join_forwarders(
        [
            Some(forwarder(ReaderStream::Stdout, "one\ntwo\nthree\n", BrokenSink)),
            None,
        ],
        BUDGET,
        Instant::now(),
    );

    assert_eq!(
        warnings,
        ["Claudine could not forward the agent's stdout to the terminal: terminal went away"]
    );
}

/// A terminal that stops accepting output cannot hold the join past the drain
/// limit; the stall is reported as processing, never as a panic or provider
/// failure, and an unaffected stream reports nothing.
#[test]
fn a_stalled_terminal_times_out_the_forwarder_as_a_warning() {
    let (entered, entered_rx) = mpsc::channel();
    let (release, release_rx) = mpsc::channel::<()>();
    let stalled = forwarder(
        ReaderStream::Stderr,
        "one\n",
        StalledSink {
            entered,
            release: release_rx,
        },
    );
    entered_rx.recv_timeout(Duration::from_secs(10)).unwrap();
    let healthy = forwarder(ReaderStream::Stdout, "fine\n", Vec::new());

    let since = Instant::now();
    let warnings = join_forwarders([Some(healthy), Some(stalled)], BUDGET, since);

    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("agent's stderr"), "{}", warnings[0]);
    assert!(
        warnings[0].contains("cleanup operation remains unfinished"),
        "{}",
        warnings[0]
    );
    assert!(!warnings[0].contains("panicked"), "{}", warnings[0]);
    assert!(since.elapsed() < Duration::from_secs(5));
    drop(release);
}

use super::super::super::super::output_worker::tests::{Gate, wedge_worker};
use super::super::super::super::output_worker::{Drained, Stream};
use super::super::super::super::run_scope::RunScope;
use super::super::super::super::stream_io::StreamOutput;
use super::super::inherited::{run_child_on, spawn_forwarder};
use std::io::Read;

/// Opens the gate when the test ends, so an abandoned worker is released even
/// when an assertion fails.
struct ReleaseOnDrop(Gate);

impl Drop for ReleaseOnDrop {
    fn drop(&mut self) {
        self.0.release();
    }
}

fn written(gate: &Gate, stream: Stream) -> String {
    gate.written()
        .into_iter()
        .filter(|(written_to, _)| *written_to == stream)
        .map(|(_, bytes)| String::from_utf8(bytes).unwrap())
        .collect()
}

/// `run_child_on` with both streams filtered, so both forwarders run.
fn run_filtered(script: (PathBuf, Vec<String>), output: &Arc<StreamOutput>) -> super::super::super::ProcessResult<i32> {
    let (binary, args) = script;
    let mut child_spawned = false;
    run_child_on(
        &binary,
        &args,
        &minimal_env(),
        &test_cwd(),
        None,
        false,
        ChildIoOptions {
            stdout_noise_prefixes: &["noise:"],
            stderr_noise_prefixes: &["noise:"],
            stdin_seed: None,
        },
        &mut child_spawned,
        output,
        BUDGET,
    )
    .expect("spawning the platform shell fixture must succeed")
}

/// Filtered lines reach their own streams through the output worker, noise
/// stays out, and a healthy run reports nothing.
#[test]
fn run_child_forwards_filtered_lines_through_the_output_worker() {
    let (gate, _entered) = Gate::new(true);
    let output = StreamOutput::with_sink(Box::new(gate.clone()));

    let result = run_filtered(
        test_shell_command(
            "echo out; echo 'noise: hidden'; echo err >&2",
            "echo out& echo noise: hidden& 1>&2 echo err",
        ),
        &output,
    );

    assert_eq!(result.data, 0);
    assert!(result.reader_warnings.is_empty(), "{:?}", result.reader_warnings);
    assert_eq!(written(&gate, Stream::Stdout).trim_end(), "out");
    assert_eq!(written(&gate, Stream::Stderr).trim_end(), "err");
}

/// With the terminal refusing forwarded stderr, `run_child` still returns
/// within the drain limit, keeps the agent's exit code, and says forwarding
/// failed. The next iteration on the same terminal returns at once, starts no
/// second writer, and presents nothing.
#[test]
fn run_child_returns_within_bound_when_the_terminal_never_accepts_stderr() {
    let (gate, entered) = Gate::new(false);
    let _release = ReleaseOnDrop(gate.clone());
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    let script = || test_shell_command("echo forwarded >&2", "1>&2 echo forwarded");
    wedge_worker(&output, &entered);

    let started = Instant::now();
    let first = run_filtered(script(), &output);

    assert!(started.elapsed() < Duration::from_secs(10), "{:?}", started.elapsed());
    assert_eq!(first.data, 0, "the agent's exit code is kept");
    assert_eq!(first.termination, claudine::harness::ProcessTermination::Completed);
    assert!(
        first
            .reader_warnings
            .iter()
            .any(|warning| warning.contains("could not forward all of the agent's output")),
        "{:?}",
        first.reader_warnings
    );
    assert!(output.loss().stalled);

    let next = Instant::now();
    let second = run_filtered(script(), &output);
    assert!(next.elapsed() < Duration::from_secs(10), "{:?}", next.elapsed());
    assert_eq!(second.data, 0);
    assert!(
        second
            .reader_warnings
            .iter()
            .any(|warning| warning.contains("could not forward all of the agent's output")),
        "{:?}",
        second.reader_warnings
    );
    assert_eq!(output.drain(Instant::now()), Drained::Disabled);
    assert!(gate.written().is_empty(), "nothing reached the stalled terminal");
}

/// A pipe whose data arrives only once the test releases it, and which says
/// when the reader came back for more.
struct GatedPipe {
    data: Option<&'static [u8]>,
    release: mpsc::Receiver<()>,
    reached_eof: mpsc::Sender<()>,
}

impl Read for GatedPipe {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self.data.take() {
            Some(data) => {
                let _ = self.release.recv();
                buf[..data.len()].copy_from_slice(data);
                Ok(data.len())
            }
            None => {
                let _ = self.reached_eof.send(());
                Ok(0)
            }
        }
    }
}

/// A forwarder cut off while its pipe is held open is detached, and lines that
/// arrive afterwards are refused rather than presented into a finished run.
#[test]
fn a_forwarder_released_after_cutoff_presents_nothing() {
    let (gate, _entered) = Gate::new(true);
    let output = StreamOutput::with_sink(Box::new(gate.clone()));
    let scope = RunScope::default();
    let (release, release_rx) = mpsc::channel();
    let (eof_tx, eof_rx) = mpsc::channel();
    let pipe = GatedPipe {
        data: Some(b"one\ntwo\n"),
        release: release_rx,
        reached_eof: eof_tx,
    };
    let reader = spawn_forwarder(
        ReaderStream::Stderr,
        pipe,
        &output,
        &scope,
        &[],
        Arc::new(Mutex::new(None)),
    );

    let warnings = join_forwarders([None, Some(reader)], BUDGET, Instant::now());
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(warnings[0].contains("waiting for pipe data"), "{}", warnings[0]);
    scope.close();
    release.send(()).unwrap();
    eof_rx.recv_timeout(Duration::from_secs(10)).unwrap();

    assert_eq!(output.drain(Instant::now() + Duration::from_secs(10)), Drained::Complete);
    assert!(gate.written().is_empty(), "{:?}", gate.written());
    assert_eq!(output.loss().rejected_frames, 2);
}

/// A terminal that refuses forwarded output with a write error, on stdout and
/// separately on stderr, keeps the agent's exit code and is reported as a
/// forwarding loss; the healthy stream's line is still delivered.
#[test]
fn run_child_reports_a_write_error_on_either_stream_as_a_forwarding_loss() {
    for (failing, healthy, healthy_line) in [(Stream::Stdout, Stream::Stderr, "err"), (Stream::Stderr, Stream::Stdout, "out")] {
        let gate = Gate::failing(failing);
        let output = StreamOutput::with_sink(Box::new(gate.clone()));

        let result = run_filtered(test_shell_command("echo out; echo err >&2", "echo out& 1>&2 echo err"), &output);

        assert_eq!(result.data, 0, "{failing:?}: the agent's exit code is kept");
        assert_eq!(result.termination, claudine::harness::ProcessTermination::Completed);
        assert_eq!(
            result.reader_warnings,
            ["Claudine could not forward all of the agent's output to the terminal: it was not accepting output"],
            "{failing:?}"
        );
        assert_eq!(output.loss().failed_writes, 1, "{failing:?}: {:?}", output.loss());
        assert_eq!(written(&gate, healthy).trim_end(), healthy_line, "{failing:?}");
    }
}

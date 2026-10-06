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
        warnings[0].contains("terminal was not accepting output"),
        "{}",
        warnings[0]
    );
    assert!(!warnings[0].contains("panicked"), "{}", warnings[0]);
    assert!(since.elapsed() < Duration::from_secs(5));
    drop(release);
}

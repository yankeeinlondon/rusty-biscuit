use super::*;
use std::sync::mpsc;

use crate::commands::wrap::output_worker::tests::{Gate, wedge_worker};

/// Run `write` on its own thread while another thread holds std's stdout and
/// stderr locks, as an abandoned output-worker write does, and report whether
/// it returned. The locks are released before returning either way, so a
/// writer that did block finishes instead of outliving the test.
pub(crate) fn returns_while_std_streams_are_locked(write: impl FnOnce() + Send + 'static) -> bool {
    let (locked_tx, locked_rx) = mpsc::channel();
    let (release_tx, release_rx) = mpsc::channel::<()>();
    let holder = std::thread::spawn(move || {
        let _stdout = std::io::stdout().lock();
        let _stderr = std::io::stderr().lock();
        locked_tx.send(()).unwrap();
        let _ = release_rx.recv();
    });
    locked_rx.recv().unwrap();

    let (done_tx, done_rx) = mpsc::channel();
    std::thread::spawn(move || {
        write();
        let _ = done_tx.send(());
    });
    let returned = done_rx.recv_timeout(Duration::from_secs(10)).is_ok();
    release_tx.send(()).unwrap();
    holder.join().unwrap();
    returned
}

/// The diagnostic route pointed at a worker stuck inside a terminal write
/// that never returns, as when a wrapped run's terminal stops reading.
///
/// [`release`](Self::release) (or dropping it, when a test fails) opens the
/// sink, drains, and clears the route, so no blocked thread outlives the test.
pub(crate) struct RoutedToBlockedWorker {
    output: Arc<StreamOutput>,
    gate: Gate,
    released: bool,
}

impl RoutedToBlockedWorker {
    pub(crate) fn new() -> Self {
        let (gate, entered) = Gate::new(false);
        let output = StreamOutput::with_sink(Box::new(gate.clone()));
        wedge_worker(&output, &entered);
        route_to(Arc::clone(&output));
        Self {
            output,
            gate,
            released: false,
        }
    }

    pub(crate) fn output(&self) -> &Arc<StreamOutput> {
        &self.output
    }

    /// Let the worker finish, and return every frame it wrote, the wedging
    /// frame first.
    pub(crate) fn release(mut self) -> Vec<(Stream, Vec<u8>)> {
        self.teardown();
        self.gate.written()
    }

    fn teardown(&mut self) {
        if !self.released {
            self.released = true;
            clear_route();
            self.gate.release();
            let _ = self.output.drain(Instant::now() + Duration::from_secs(10));
        }
    }
}

impl Drop for RoutedToBlockedWorker {
    fn drop(&mut self) {
        self.teardown();
    }
}

#[test]
fn the_gate_admits_writes_until_a_stall_and_counts_only_refusals() {
    assert!(!is_stalled());
    assert!(admit());
    assert_eq!(skipped_writes(), 0);

    mark_stalled();

    assert!(is_stalled());
    assert!(!admit());
    assert!(!admit());
    assert_eq!(skipped_writes(), 2);
}

#[test]
fn an_unrouted_diagnostic_is_written_directly_and_a_routed_one_is_queued() {
    // Unrouted: the direct path, as for every command that is not a wrapped
    // run. Nothing is queued anywhere.
    assert!(write(Stream::Stderr, b""));

    let routed = RoutedToBlockedWorker::new();
    assert!(write(Stream::Stderr, b"queued\n"));
    let written = routed.release();

    assert_eq!(written.last(), Some(&(Stream::Stderr, b"queued\n".to_vec())));
    assert!(write(Stream::Stderr, b""), "the route ends with the run's teardown");
}

/// The tracing writer's frames reach the worker whole: one formatted event
/// per frame.
#[test]
fn the_diagnostic_writer_queues_each_write_as_one_frame() {
    let routed = RoutedToBlockedWorker::new();
    let mut writer = DiagnosticWriter(Stream::Stderr);

    let returned = returns_while_std_streams_are_locked(move || {
        writer.write_all(b"one event\n").unwrap();
        writer.write_all(b"").unwrap();
    });

    assert!(returned, "the tracing writer waited on the terminal");
    let written = routed.release();
    assert_eq!(&written[1..], [(Stream::Stderr, b"one event\n".to_vec())]);
}

/// A forced exit gives queued diagnostics a bounded chance to reach the
/// terminal and returns when the worker never finishes.
#[test]
fn settle_returns_within_its_budget_when_the_worker_is_stuck() {
    let routed = RoutedToBlockedWorker::new();
    write(Stream::Stderr, b"force-exit notice\n");

    let started = Instant::now();
    settle(Duration::from_millis(50));

    assert!(started.elapsed() < Duration::from_secs(5));
    assert!(routed.output().loss().stalled);
}

/// The library's console lines (lifecycle actions, harness reports, messaging
/// warnings) take the same route as the CLI's own diagnostics.
#[test]
fn a_library_console_line_is_queued_while_the_worker_is_stuck() {
    use claudine::render::console::ConsoleStream;
    let routed = RoutedToBlockedWorker::new();

    let returned = returns_while_std_streams_are_locked(|| {
        write_library_line(ConsoleStream::Stderr, "lifecycle info\n");
        write_library_line(ConsoleStream::Stdout, "lifecycle stdout\n");
    });

    assert!(returned, "a library console line waited on the terminal");
    let written = routed.release();
    assert_eq!(
        &written[1..],
        [
            (Stream::Stderr, b"lifecycle info\n".to_vec()),
            (Stream::Stdout, b"lifecycle stdout\n".to_vec()),
        ]
    );
}

/// Clears the diagnostic route when the test ends, even when it fails.
struct ClearRouteOnDrop;

impl Drop for ClearRouteOnDrop {
    fn drop(&mut self) {
        clear_route();
    }
}

/// A routed diagnostic the terminal refuses with a write error, on stdout and
/// separately on stderr, is submitted without blocking and counted: the run's
/// delivery loss and its one session-end record carry it. The healthy control
/// records nothing.
#[test]
fn a_routed_diagnostic_refused_with_a_write_error_is_on_the_session_end_record() {
    use crate::commands::wrap::policy::session_end_tests::{
        RecordHome, publish, section_stream_over, successful_summary,
    };
    for failing in [Some(Stream::Stdout), Some(Stream::Stderr), None] {
        let home = RecordHome::new();
        let gate = match failing {
            Some(stream) => Gate::failing(stream),
            None => Gate::new(true).0,
        };
        let (output, section_stream) = section_stream_over(&gate);
        let _clear = ClearRouteOnDrop;
        route_to(Arc::clone(&output));
        let diagnostic_stream = failing.unwrap_or(Stream::Stderr);

        assert!(write(diagnostic_stream, b"a diagnostic\n"), "{failing:?}");
        // Delivered before the trailer is queued, so only the diagnostic can
        // have failed on stdout.
        output.drain(Instant::now() + Duration::from_secs(10));
        let loss = section_stream.output_loss();
        clear_route();
        publish(&successful_summary(), &section_stream, &[]);

        let record = home.only_session_end();
        match failing {
            Some(stream) => {
                assert_eq!(loss.map(|loss| loss.failed_writes), Some(1), "{stream:?}");
                let incomplete = &record.extra["output_incomplete"];
                assert!(incomplete["failed_writes"].as_u64() >= Some(1), "{stream:?}: {incomplete}");
            }
            None => {
                assert_eq!(loss, None);
                assert!(!record.extra.contains_key("output_incomplete"), "{:?}", record.extra);
                let first = gate.written().into_iter().next();
                assert_eq!(first, Some((Stream::Stderr, b"a diagnostic\n".to_vec())));
            }
        }
    }
}

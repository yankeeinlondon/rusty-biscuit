use super::*;
use std::sync::mpsc;
use std::time::Duration;

pub(crate) type Written = Arc<Mutex<Vec<(Stream, Vec<u8>)>>>;

/// A sink the test can hold shut: `write` blocks until the gate opens, like a
/// terminal that stopped accepting output.
#[derive(Clone)]
pub(crate) struct Gate {
    open: Arc<(Mutex<bool>, Condvar)>,
    written: Written,
    /// Signalled once the worker is inside its first write.
    entered: Arc<Mutex<Option<mpsc::Sender<()>>>>,
    /// Writes to this stream fail with `BrokenPipe`, as when the reader of
    /// the process's pipe has gone away.
    fails: Option<Stream>,
}

impl Gate {
    pub(crate) fn new(open: bool) -> (Self, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let gate = Self {
            open: Arc::new((Mutex::new(open), Condvar::new())),
            written: Arc::new(Mutex::new(Vec::new())),
            entered: Arc::new(Mutex::new(Some(tx))),
            fails: None,
        };
        (gate, rx)
    }

    /// An open gate whose writes to `stream` fail with `BrokenPipe`; the other
    /// stream is healthy.
    pub(crate) fn failing(stream: Stream) -> Self {
        Self {
            fails: Some(stream),
            ..Self::new(true).0
        }
    }

    pub(crate) fn release(&self) {
        *self.open.0.lock().unwrap() = true;
        self.open.1.notify_all();
    }

    pub(crate) fn written(&self) -> Vec<(Stream, Vec<u8>)> {
        self.written.lock().unwrap().clone()
    }
}

impl FrameSink for Gate {
    fn write(&mut self, stream: Stream, bytes: &[u8]) -> io::Result<()> {
        if let Some(tx) = self.entered.lock().unwrap().take() {
            let _ = tx.send(());
        }
        let mut open = self.open.0.lock().unwrap();
        while !*open {
            open = self.open.1.wait(open).unwrap();
        }
        drop(open);
        if self.fails == Some(stream) {
            return Err(io::ErrorKind::BrokenPipe.into());
        }
        self.written.lock().unwrap().push((stream, bytes.to_vec()));
        Ok(())
    }
}

/// Block `output`'s worker inside a write to its closed gate, and return only
/// once it is there.
///
/// Call this before the clock of a short drain deadline starts. The deadline
/// then finds a stalled terminal however late the worker is scheduled, where a
/// frame queued just before the drain could still be waiting for its first
/// write when the deadline passes, so the worker would never enter the gate.
pub(crate) fn wedge_worker(
    output: &crate::commands::wrap::stream_io::StreamOutput,
    entered: &mpsc::Receiver<()>,
) {
    output.emit_stderr_line("earlier output");
    entered
        .recv_timeout(Duration::from_secs(10))
        .expect("the worker entered its write");
}

fn soon() -> Instant {
    Instant::now() + Duration::from_secs(10)
}

fn short() -> Instant {
    Instant::now() + Duration::from_millis(50)
}

#[test]
fn an_unblocked_sink_receives_every_frame_in_order() {
    let (gate, _entered) = Gate::new(true);
    let worker = OutputWorker::new(Box::new(gate.clone()));

    worker.submit(Stream::Stdout, b"one".to_vec());
    worker.submit(Stream::Stderr, b"two\n".to_vec());
    worker.submit(Stream::Stdout, b"three\n".to_vec());

    assert_eq!(worker.drain(soon()), Drained::Complete);
    assert_eq!(
        gate.written(),
        vec![
            (Stream::Stdout, b"one".to_vec()),
            (Stream::Stderr, b"two\n".to_vec()),
            (Stream::Stdout, b"three\n".to_vec()),
        ]
    );
    assert!(worker.loss().is_empty());
}

#[test]
fn a_healthy_worker_can_be_drained_repeatedly_without_being_disabled() {
    let (gate, _entered) = Gate::new(true);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    for round in 0..3u8 {
        worker.submit(Stream::Stdout, vec![round]);
        assert_eq!(worker.drain(soon()), Drained::Complete);
    }
    assert_eq!(gate.written().len(), 3);
    assert!(!worker.loss().stalled);
}

#[test]
fn a_permanently_blocked_sink_never_blocks_submit_or_drain_past_its_deadline() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"stuck".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).expect("the worker entered its write");

    let started = Instant::now();
    for _ in 0..100 {
        worker.submit(Stream::Stderr, b"queued\n".to_vec());
    }
    assert!(started.elapsed() < Duration::from_secs(5), "submit blocked");

    assert_eq!(worker.drain(short()), Drained::Disabled);
    assert!(worker.loss().stalled);

    // Once disabled, frames are refused instantly and counted.
    assert_eq!(worker.submit(Stream::Stdout, b"late".to_vec()), Submitted::Rejected);
    assert_eq!(worker.loss().rejected_frames, 1);
    // A second drain returns at once rather than waiting out a new deadline.
    let second = Instant::now();
    assert_eq!(worker.drain(soon()), Drained::Disabled);
    assert!(second.elapsed() < Duration::from_secs(5));

    gate.release();
}

#[test]
fn the_write_in_progress_may_finish_but_nothing_follows_it_after_disabling() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"in flight".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    worker.submit(Stream::Stdout, b"queued behind it".to_vec());

    assert_eq!(worker.drain(short()), Drained::Disabled);
    gate.release();
    // Let the abandoned writer run to its natural end.
    thread::sleep(Duration::from_millis(100));

    assert_eq!(gate.written(), vec![(Stream::Stdout, b"in flight".to_vec())]);
}

#[test]
fn a_full_queue_drops_whole_frames_and_counts_them() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"stuck".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).unwrap();

    let mut queued = 0;
    let mut dropped = 0;
    for _ in 0..(MAX_QUEUED_FRAMES + 25) {
        match worker.submit(Stream::Stdout, b"{\"k\":1}\n".to_vec()) {
            Submitted::Queued => queued += 1,
            Submitted::Overflowed => dropped += 1,
            Submitted::Rejected => panic!("delivery was never disabled"),
        }
    }

    assert_eq!(queued, MAX_QUEUED_FRAMES);
    assert_eq!(dropped, 25);
    assert_eq!(worker.loss().dropped_frames, 25);

    // Every frame that is delivered is one that was submitted whole, so a JSON
    // stdout stream stays valid.
    gate.release();
    assert_eq!(worker.drain(soon()), Drained::Complete);
    let written = gate.written();
    assert_eq!(written.len(), 1 + MAX_QUEUED_FRAMES);
    assert!(written[1..].iter().all(|(_, bytes)| bytes == b"{\"k\":1}\n"));
}

#[test]
fn the_byte_cap_bounds_queued_memory() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"stuck".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).unwrap();

    let chunk = vec![b'x'; MAX_QUEUED_BYTES / 4 + 1];
    let outcomes: Vec<Submitted> = (0..6)
        .map(|_| worker.submit(Stream::Stdout, chunk.clone()))
        .collect();

    assert_eq!(
        outcomes.iter().filter(|o| **o == Submitted::Queued).count(),
        3,
        "{outcomes:?}"
    );
    assert_eq!(worker.loss().dropped_frames, 3);
    gate.release();
}

#[test]
fn a_stalled_sink_never_gains_a_second_writer_thread() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"stuck".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).unwrap();

    // Several simulated iterations, each submitting and draining.
    for _ in 0..5 {
        worker.submit(Stream::Stderr, b"iteration output\n".to_vec());
        worker.drain(short());
    }

    assert_eq!(worker.threads_started(), 1);
    gate.release();
}

/// A [`Gate`] that claims to be the process terminal, so its stall closes the
/// process-wide terminal gate.
struct ProcessTerminalGate(Gate);

impl FrameSink for ProcessTerminalGate {
    fn write(&mut self, stream: Stream, bytes: &[u8]) -> io::Result<()> {
        self.0.write(stream, bytes)
    }

    fn is_process_terminal(&self) -> bool {
        true
    }
}

#[test]
fn a_stalled_process_terminal_closes_the_terminal_gate_and_counts_its_refusals() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(ProcessTerminalGate(gate.clone())));
    worker.submit(Stream::Stderr, b"stuck\n".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).expect("the worker entered its write");
    assert!(!crate::terminal_gate::is_stalled());

    assert_eq!(worker.drain(short()), Drained::Disabled);

    assert!(crate::terminal_gate::is_stalled());
    let before = worker.loss().rejected_frames;
    assert!(!crate::terminal_gate::admit());
    assert_eq!(worker.loss().rejected_frames, before + 1);
    gate.release();
}

#[test]
fn a_stalled_private_sink_leaves_the_terminal_gate_open() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stderr, b"stuck\n".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).expect("the worker entered its write");

    assert_eq!(worker.drain(short()), Drained::Disabled);

    assert!(!crate::terminal_gate::is_stalled());
    gate.release();
}

/// A write the sink refuses with an error is counted as loss, on either
/// stream, and the drain still completes because nothing is left to write.
/// The other stream keeps receiving its frames.
#[test]
fn a_write_error_on_either_stream_is_counted_as_loss() {
    for (failing, healthy) in [(Stream::Stdout, Stream::Stderr), (Stream::Stderr, Stream::Stdout)] {
        let gate = Gate::failing(failing);
        let worker = OutputWorker::new(Box::new(gate.clone()));

        worker.submit(failing, b"lost\n".to_vec());
        worker.submit(healthy, b"kept\n".to_vec());
        worker.submit(failing, b"lost too\n".to_vec());

        assert_eq!(worker.drain(soon()), Drained::Complete, "{failing:?}");
        let loss = worker.loss();
        assert_eq!(loss.failed_writes, 2, "{failing:?}: {loss:?}");
        assert!(!loss.is_empty() && !loss.stalled, "{failing:?}: {loss:?}");
        assert_eq!(gate.written(), vec![(healthy, b"kept\n".to_vec())], "{failing:?}");
    }
}

/// Control: the same frames to a healthy sink lose nothing.
#[test]
fn a_healthy_sink_reports_no_failed_writes() {
    let (gate, _entered) = Gate::new(true);
    let worker = OutputWorker::new(Box::new(gate.clone()));

    worker.submit(Stream::Stdout, b"kept\n".to_vec());
    worker.submit(Stream::Stderr, b"kept\n".to_vec());

    assert_eq!(worker.drain(soon()), Drained::Complete);
    assert_eq!(worker.loss().failed_writes, 0);
    assert!(worker.loss().is_empty());
    assert_eq!(gate.written().len(), 2);
}

/// One run's failed writes are its own: the difference from an earlier
/// reading excludes failures that came before it.
#[test]
fn failed_writes_are_measured_since_a_reading() {
    let gate = Gate::failing(Stream::Stdout);
    let worker = OutputWorker::new(Box::new(gate));
    worker.submit(Stream::Stdout, b"earlier\n".to_vec());
    worker.drain(soon());
    let mark = worker.loss();

    assert!(worker.loss().since(&mark).is_empty());
    worker.submit(Stream::Stdout, b"this run\n".to_vec());
    worker.drain(soon());
    assert_eq!(worker.loss().since(&mark).failed_writes, 1);
}

#[test]
fn queued_frames_do_not_replace_active_delivery() {
    let (gate, entered) = Gate::new(false);
    let worker = OutputWorker::new(Box::new(gate.clone()));
    worker.submit(Stream::Stdout, b"active".to_vec());
    entered.recv_timeout(Duration::from_secs(10)).unwrap();
    let active = worker.observation().active;
    worker.submit(Stream::Stderr, b"queued".to_vec());
    let observation = worker.observation();
    assert_eq!(observation.active, active);
    assert_eq!(observation.active.unwrap().operation, Operation::TerminalDelivery);
    assert_eq!(observation.queued_frames, 1);
    assert_eq!(observation.queued_bytes, 6);
    assert_eq!(observation.submitted, 2);
    assert_eq!(observation.delivered, 0);
    assert_eq!(worker.drain(short()), Drained::Disabled);
    assert_eq!(worker.observation().active, active);
    gate.release();
}

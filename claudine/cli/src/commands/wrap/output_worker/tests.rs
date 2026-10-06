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
}

impl Gate {
    pub(crate) fn new(open: bool) -> (Self, mpsc::Receiver<()>) {
        let (tx, rx) = mpsc::channel();
        let gate = Self {
            open: Arc::new((Mutex::new(open), Condvar::new())),
            written: Arc::new(Mutex::new(Vec::new())),
            entered: Arc::new(Mutex::new(Some(tx))),
        };
        (gate, rx)
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
        self.written.lock().unwrap().push((stream, bytes.to_vec()));
        Ok(())
    }
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

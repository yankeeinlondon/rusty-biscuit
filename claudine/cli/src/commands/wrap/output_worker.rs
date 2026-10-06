//! The one thread that touches the terminal on behalf of a wrapped run.
//!
//! A terminal that stops accepting output blocks a `write(2)` in the kernel
//! and nothing from the outside can interrupt it. If the thread parsing the
//! agent's stream does that write, the parser stalls with it, and every later
//! write from the wrapper queues on the same lock. [`OutputWorker`] takes the
//! write off those threads: they hand it whole frames through [`submit`], which
//! never blocks, and only the worker can be stuck on the terminal.
//!
//! [`drain`] bounds how long the wrapper waits for the worker. When the
//! deadline passes the worker is *abandoned* and the sink is *disabled*: later
//! frames are rejected instantly, no second writer is created, and the loss is
//! recorded in an [`OutputLoss`] instead of being written to the very terminal
//! that stopped listening.
//!
//! [`submit`]: OutputWorker::submit
//! [`drain`]: OutputWorker::drain

use std::collections::VecDeque;
use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread;
use std::time::Instant;

/// Most frames the queue holds before presentation frames are dropped.
//
// WHY so large: the bound exists to cap memory against a sink that never
// drains, not to pace a healthy one. A provider can emit a long final message
// far faster than a terminal paints it, and dropping frames for a merely slow
// terminal would lose output that was going to be delivered.
pub(crate) const MAX_QUEUED_FRAMES: usize = 4096;

/// Most bytes the queue holds before presentation frames are dropped.
pub(crate) const MAX_QUEUED_BYTES: usize = 8 * 1024 * 1024;

/// Which of the process's two output streams a frame is for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Stream {
    Stdout,
    Stderr,
}

/// Where the worker writes. The real implementation is [`TerminalSink`];
/// tests substitute one they can block.
pub(crate) trait FrameSink: Send {
    /// Write `bytes` to `stream` and flush, so a frame is on the terminal when
    /// this returns.
    fn write(&mut self, stream: Stream, bytes: &[u8]) -> io::Result<()>;
}

/// The process's standard output and standard error.
pub(crate) struct TerminalSink;

impl FrameSink for TerminalSink {
    fn write(&mut self, stream: Stream, bytes: &[u8]) -> io::Result<()> {
        match stream {
            Stream::Stdout => {
                let mut out = io::stdout().lock();
                out.write_all(bytes)?;
                out.flush()
            }
            Stream::Stderr => {
                let mut err = io::stderr().lock();
                err.write_all(bytes)?;
                err.flush()
            }
        }
    }
}

/// What happened to a frame handed to [`OutputWorker::submit`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Submitted {
    Queued,
    /// The queue was full, so the frame was dropped and counted.
    Overflowed,
    /// Delivery is disabled, so the frame was not queued.
    Rejected,
}

/// Terminal output that never reached the terminal.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct OutputLoss {
    /// Frames dropped because the queue was full.
    pub(crate) dropped_frames: u64,
    /// Frames refused because delivery was disabled, or because the run that
    /// produced them had already ended.
    pub(crate) rejected_frames: u64,
    /// The worker did not finish within its deadline and was abandoned.
    pub(crate) stalled: bool,
}

impl OutputLoss {
    pub(crate) fn is_empty(&self) -> bool {
        *self == Self::default()
    }

    /// The loss accumulated after `earlier` was taken.
    ///
    /// The counters are process-wide, so one run's loss is the difference
    /// between two readings. `stalled` is sticky: a sink found stalled stays
    /// silent, so every later run reports it too.
    pub(crate) fn since(&self, earlier: &Self) -> Self {
        Self {
            dropped_frames: self.dropped_frames.saturating_sub(earlier.dropped_frames),
            rejected_frames: self.rejected_frames.saturating_sub(earlier.rejected_frames),
            stalled: self.stalled,
        }
    }
}

/// How [`OutputWorker::drain`] ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Drained {
    /// Every queued frame was written.
    Complete,
    /// Delivery is disabled, either already or because this wait ran out.
    Disabled,
}

struct Frame {
    stream: Stream,
    bytes: Vec<u8>,
}

#[derive(Default)]
struct Queue {
    frames: VecDeque<Frame>,
    bytes: usize,
    /// The worker is inside a write.
    writing: bool,
}

struct Shared {
    queue: Mutex<Queue>,
    /// Signals both "a frame arrived" (to the worker) and "the queue is idle"
    /// (to a drain).
    changed: Condvar,
    disabled: AtomicBool,
    dropped_frames: AtomicU64,
    rejected_frames: AtomicU64,
    threads_started: AtomicUsize,
}

/// A bounded queue of whole frames written to the terminal by one thread.
pub(crate) struct OutputWorker {
    shared: Arc<Shared>,
    /// Taken by the first [`submit`](Self::submit), which starts the thread.
    sink: Mutex<Option<Box<dyn FrameSink>>>,
}

impl OutputWorker {
    pub(crate) fn new(sink: Box<dyn FrameSink>) -> Self {
        Self {
            shared: Arc::new(Shared {
                queue: Mutex::new(Queue::default()),
                changed: Condvar::new(),
                disabled: AtomicBool::new(false),
                dropped_frames: AtomicU64::new(0),
                rejected_frames: AtomicU64::new(0),
                threads_started: AtomicUsize::new(0),
            }),
            sink: Mutex::new(Some(sink)),
        }
    }

    /// Queue `bytes` for `stream` as one indivisible frame.
    ///
    /// Never blocks on the terminal and holds no lock while the worker writes.
    pub(crate) fn submit(&self, stream: Stream, bytes: Vec<u8>) -> Submitted {
        if self.shared.disabled.load(Ordering::Acquire) {
            self.shared.rejected_frames.fetch_add(1, Ordering::Relaxed);
            return Submitted::Rejected;
        }
        self.start_thread();
        let mut queue = lock(&self.shared.queue);
        if self.shared.disabled.load(Ordering::Acquire) {
            self.shared.rejected_frames.fetch_add(1, Ordering::Relaxed);
            return Submitted::Rejected;
        }
        if queue.frames.len() >= MAX_QUEUED_FRAMES || queue.bytes + bytes.len() > MAX_QUEUED_BYTES {
            self.shared.dropped_frames.fetch_add(1, Ordering::Relaxed);
            return Submitted::Overflowed;
        }
        queue.bytes += bytes.len();
        queue.frames.push_back(Frame { stream, bytes });
        self.shared.changed.notify_all();
        Submitted::Queued
    }

    /// Record a frame refused before it reached the queue.
    pub(crate) fn reject(&self) {
        self.shared.rejected_frames.fetch_add(1, Ordering::Relaxed);
    }

    /// Wait until every queued frame is written, or `deadline` passes.
    ///
    /// A deadline that passes with the queue not empty disables delivery for
    /// good: the thread is abandoned, queued frames are discarded, and no
    /// writer is created in its place.
    pub(crate) fn drain(&self, deadline: Instant) -> Drained {
        if self.shared.disabled.load(Ordering::Acquire) {
            return Drained::Disabled;
        }
        let mut queue = lock(&self.shared.queue);
        loop {
            if queue.frames.is_empty() && !queue.writing {
                return Drained::Complete;
            }
            let Some(left) = deadline.checked_duration_since(Instant::now()) else {
                break;
            };
            queue = self
                .shared
                .changed
                .wait_timeout(queue, left)
                .unwrap_or_else(|poisoned| poisoned.into_inner())
                .0;
        }
        self.shared.disabled.store(true, Ordering::Release);
        queue.frames.clear();
        queue.bytes = 0;
        self.shared.changed.notify_all();
        Drained::Disabled
    }

    pub(crate) fn loss(&self) -> OutputLoss {
        OutputLoss {
            dropped_frames: self.shared.dropped_frames.load(Ordering::Relaxed),
            rejected_frames: self.shared.rejected_frames.load(Ordering::Relaxed),
            stalled: self.shared.disabled.load(Ordering::Acquire),
        }
    }

    /// How many writer threads this worker has ever started: at most one.
    #[cfg(test)]
    pub(crate) fn threads_started(&self) -> usize {
        self.shared.threads_started.load(Ordering::Relaxed)
    }

    fn start_thread(&self) {
        let Some(sink) = lock(&self.sink).take() else {
            return;
        };
        self.shared.threads_started.fetch_add(1, Ordering::Relaxed);
        let shared = Arc::clone(&self.shared);
        let spawned = thread::Builder::new()
            .name("claudine-output".into())
            .spawn(move || write_frames(shared, sink));
        if spawned.is_err() {
            // No thread, no delivery: refuse every frame rather than queue
            // frames nothing will ever write.
            self.shared.disabled.store(true, Ordering::Release);
        }
    }
}

fn write_frames(shared: Arc<Shared>, mut sink: Box<dyn FrameSink>) {
    loop {
        let frame = {
            let mut queue = lock(&shared.queue);
            loop {
                if shared.disabled.load(Ordering::Acquire) {
                    return;
                }
                if let Some(frame) = queue.frames.pop_front() {
                    queue.bytes -= frame.bytes.len();
                    queue.writing = true;
                    break frame;
                }
                queue = shared
                    .changed
                    .wait(queue)
                    .unwrap_or_else(|poisoned| poisoned.into_inner());
            }
        };
        // A failed write is dropped like every terminal write before it: the
        // frame was presentation, and the run does not depend on it.
        let _ = sink.write(frame.stream, &frame.bytes);
        lock(&shared.queue).writing = false;
        shared.changed.notify_all();
    }
}

fn lock<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

#[cfg(test)]
pub(crate) mod tests;

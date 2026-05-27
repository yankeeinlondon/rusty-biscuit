//! Asynchronous micro-batching pipeline that drains session-log entries
//! from a flume channel into the DuckDB projection on a dedicated
//! blocking thread.
//!
//! The async daemon enqueues rows via [`BatcherHandle::submit`]; the
//! background thread flushes either every `flush_interval` or after
//! `flush_size` rows have accumulated, whichever comes first. Shutdown
//! is signalled with a dedicated poison-pill message so that
//! [`BatcherWorker::shutdown`] terminates even if other
//! [`BatcherHandle`] clones are still alive.

use std::thread::{self, JoinHandle};
use std::time::Duration;

use flume::{Receiver, RecvTimeoutError, Sender};

use crate::projection::{Projection, ProjectionRow};

/// Default time-based flush interval. Chosen short enough that tests do
/// not need to wait long, but long enough to coalesce realistic bursts.
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_millis(200);

/// Default size-based flush threshold.
pub const DEFAULT_FLUSH_SIZE: usize = 500;

/// Internal message carried over the batcher's flume channel. Rows are
/// the only payload producers send; `Shutdown` is the poison-pill the
/// [`BatcherWorker`] uses to terminate the worker thread.
#[derive(Debug)]
enum Message {
    Row(Box<ProjectionRow>),
    Shutdown,
}

/// Configuration for the batcher thread.
#[derive(Clone, Debug)]
pub struct BatcherConfig {
    pub flush_interval: Duration,
    pub flush_size: usize,
}

impl Default for BatcherConfig {
    fn default() -> Self {
        Self {
            flush_interval: DEFAULT_FLUSH_INTERVAL,
            flush_size: DEFAULT_FLUSH_SIZE,
        }
    }
}

/// Handle to a running batcher. Cloning is cheap (it clones a flume
/// sender). Sending after the worker has been shut down returns
/// [`BatcherError::Closed`].
#[derive(Clone)]
pub struct BatcherHandle {
    sender: Sender<Message>,
}

impl BatcherHandle {
    /// Submit a single row for asynchronous projection.
    ///
    /// Returns an error only when the channel has been closed because
    /// the batcher thread shut down.
    pub fn submit(&self, row: ProjectionRow) -> Result<(), BatcherError> {
        self.sender
            .send(Message::Row(Box::new(row)))
            .map_err(|_| BatcherError::Closed)
    }
}

impl std::fmt::Debug for BatcherHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BatcherHandle")
            .field("queue_len", &self.sender.len())
            .finish_non_exhaustive()
    }
}

/// Errors visible to callers of the batcher.
#[derive(Debug, thiserror::Error)]
pub enum BatcherError {
    /// The batcher thread has exited and the channel is closed.
    #[error("batcher channel is closed")]
    Closed,
}

/// Background worker that owns the batcher thread's [`JoinHandle`] so
/// callers can shut it down deterministically.
pub struct BatcherWorker {
    sender: Sender<Message>,
    join: Option<JoinHandle<()>>,
}

impl BatcherWorker {
    /// Cheap handle that producers can clone freely.
    #[must_use]
    pub fn handle(&self) -> BatcherHandle {
        BatcherHandle {
            sender: self.sender.clone(),
        }
    }

    /// Send the poison-pill and join the worker thread. The thread
    /// flushes any pending rows before returning. Outstanding
    /// [`BatcherHandle`] clones can still call [`BatcherHandle::submit`]
    /// after this returns; those calls will succeed (the channel still
    /// has senders) but the rows will accumulate in the queue without
    /// ever being drained.
    pub fn shutdown(mut self) {
        let _ = self.sender.send(Message::Shutdown);
        if let Some(join) = self.join.take()
            && let Err(panic) = join.join()
        {
            tracing::warn!(
                target: "remote_signal_daemon::batcher",
                ?panic,
                "batcher worker panicked"
            );
        }
    }
}

impl Drop for BatcherWorker {
    fn drop(&mut self) {
        if let Some(join) = self.join.take() {
            let _ = self.sender.send(Message::Shutdown);
            let _ = join.join();
        }
    }
}

/// Start the batcher thread, returning a worker that owns the join
/// handle plus a cheap [`BatcherHandle`] for producers.
pub fn spawn(projection: Projection, config: BatcherConfig) -> BatcherWorker {
    let (sender, receiver) = flume::unbounded::<Message>();
    let join = thread::Builder::new()
        .name("remote-signal-batcher".into())
        .spawn(move || drain_loop(receiver, projection, config))
        .expect("spawn batcher thread");
    BatcherWorker {
        sender,
        join: Some(join),
    }
}

fn drain_loop(receiver: Receiver<Message>, projection: Projection, config: BatcherConfig) {
    let mut buffer: Vec<ProjectionRow> = Vec::with_capacity(config.flush_size.max(1));
    loop {
        match receiver.recv_timeout(config.flush_interval) {
            Ok(Message::Row(row)) => {
                buffer.push(*row);
                if buffer.len() >= config.flush_size {
                    flush(&projection, &mut buffer);
                }
            }
            Ok(Message::Shutdown) => {
                if !buffer.is_empty() {
                    flush(&projection, &mut buffer);
                }
                break;
            }
            Err(RecvTimeoutError::Timeout) => {
                if !buffer.is_empty() {
                    flush(&projection, &mut buffer);
                }
            }
            Err(RecvTimeoutError::Disconnected) => {
                if !buffer.is_empty() {
                    flush(&projection, &mut buffer);
                }
                break;
            }
        }
    }
}

fn flush(projection: &Projection, buffer: &mut Vec<ProjectionRow>) {
    if let Err(error) = projection.append_rows(buffer) {
        tracing::error!(
            target: "remote_signal_daemon::batcher",
            %error,
            "failed to flush projection batch",
        );
    }
    buffer.clear();
}

#[cfg(test)]
mod tests {
    use super::*;
    use remote_signal_core::ChunkId;
    use std::time::Instant;

    fn row(seq: u64) -> ProjectionRow {
        ProjectionRow {
            chunk: ChunkId::new("node-a", "session-1", 0),
            sequence: seq,
            created_at_unix_ms: 1_000 + seq as i64,
            source: "test".into(),
            level: "info".into(),
            message: format!("msg-{seq}"),
            metadata_json: String::new(),
        }
    }

    #[test]
    fn size_based_flush_propagates_rows() {
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(
            projection.clone(),
            BatcherConfig {
                flush_interval: Duration::from_secs(60),
                flush_size: 3,
            },
        );
        let handle = worker.handle();
        for seq in 0..3 {
            handle.submit(row(seq)).expect("submit");
        }
        wait_until(Duration::from_secs(2), || {
            projection.row_count().unwrap_or(0) == 3
        });
        assert_eq!(projection.row_count().unwrap(), 3);
        worker.shutdown();
    }

    #[test]
    fn time_based_flush_propagates_partial_batches() {
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(
            projection.clone(),
            BatcherConfig {
                flush_interval: Duration::from_millis(50),
                flush_size: 1_000,
            },
        );
        let handle = worker.handle();
        handle.submit(row(0)).expect("submit");
        handle.submit(row(1)).expect("submit");
        wait_until(Duration::from_secs(2), || {
            projection.row_count().unwrap_or(0) == 2
        });
        assert_eq!(projection.row_count().unwrap(), 2);
        worker.shutdown();
    }

    #[test]
    fn shutdown_flushes_pending_rows() {
        let projection = Projection::in_memory().expect("projection");
        let worker = spawn(
            projection.clone(),
            BatcherConfig {
                flush_interval: Duration::from_secs(60),
                flush_size: 1_000,
            },
        );
        let handle = worker.handle();
        handle.submit(row(0)).expect("submit");
        handle.submit(row(1)).expect("submit");
        worker.shutdown();
        assert_eq!(projection.row_count().unwrap(), 2);
    }

    fn wait_until(timeout: Duration, mut pred: impl FnMut() -> bool) {
        let deadline = Instant::now() + timeout;
        while !pred() {
            if Instant::now() >= deadline {
                return;
            }
            thread::sleep(Duration::from_millis(10));
        }
    }
}

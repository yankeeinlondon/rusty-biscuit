use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc;

use crate::backend::ClipboardBackend;
use crate::content::ClipboardFormat;
use crate::error::ClipboardError;

pub struct WatcherEvent {
    pub formats: Vec<ClipboardFormat>,
}

pub struct WatcherHandle {
    tx: mpsc::UnboundedSender<WatcherEvent>,
    stop: Arc<AtomicBool>,
    watcher_thread: Option<thread::JoinHandle<()>>,
}

impl WatcherHandle {
    pub fn take_thread(&mut self) -> Option<thread::JoinHandle<()>> {
        self.watcher_thread.take()
    }

    pub fn sender(&self) -> &mpsc::UnboundedSender<WatcherEvent> {
        &self.tx
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }
    }
}

pub fn spawn_watcher<B: ClipboardBackend + Send + 'static>(
    backend: B,
) -> Result<(WatcherHandle, mpsc::UnboundedReceiver<WatcherEvent>), ClipboardError> {
    let (tx, rx) = mpsc::unbounded_channel();
    let stop = Arc::new(AtomicBool::new(false));

    let stop_clone = stop.clone();
    let tx_clone = tx.clone();

    let handle = thread::Builder::new()
        .name("clipboard-watcher".to_string())
        .spawn(move || {
            let mut last_text: Option<String> = None;
            let poll_interval = Duration::from_millis(250);

            while !stop_clone.load(Ordering::Relaxed) {
                if let Ok(text) = backend.get_text() {
                    let changed = match (&last_text, &text) {
                        (Some(prev), Some(cur)) => prev != cur,
                        (None, Some(_)) => true,
                        (Some(_), None) => true,
                        (None, None) => false,
                    };

                    if changed {
                        last_text = text.clone();
                        if let Some(t) = text {
                            let event = WatcherEvent {
                                formats: vec![ClipboardFormat::Text(t)],
                            };
                            let _ = tx_clone.send(event);
                        }
                    }
                }

                thread::sleep(poll_interval);
            }
        })
        .map_err(|e| ClipboardError::Backend(e.to_string()))?;

    let watcher_handle = WatcherHandle {
        tx,
        stop,
        watcher_thread: Some(handle),
    };

    Ok((watcher_handle, rx))
}

pub struct Supervisor {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    status: SupervisorStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupervisorStatus {
    Running,
    Degraded { consecutive_failures: u32 },
    Stopped,
}

impl Default for Supervisor {
    fn default() -> Self {
        Self::new()
    }
}

impl Supervisor {
    pub fn new() -> Self {
        Self {
            max_retries: 5,
            base_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            status: SupervisorStatus::Running,
        }
    }

    pub fn with_max_retries(mut self, retries: u32) -> Self {
        self.max_retries = retries;
        self
    }

    pub fn with_base_delay(mut self, delay: Duration) -> Self {
        self.base_delay = delay;
        self
    }

    pub fn with_max_delay(mut self, delay: Duration) -> Self {
        self.max_delay = delay;
        self
    }

    pub fn status(&self) -> &SupervisorStatus {
        &self.status
    }

    pub fn is_healthy(&self) -> bool {
        matches!(self.status, SupervisorStatus::Running)
    }

    pub fn backoff_delay(&self, attempt: u32) -> Duration {
        let exp_ms = self.base_delay.as_millis() as u64 * 2u64.saturating_pow(attempt);
        let capped = exp_ms.min(self.max_delay.as_millis() as u64);
        Duration::from_millis(capped)
    }

    pub fn record_failure(&mut self) -> SupervisorAction {
        let failures = match &self.status {
            SupervisorStatus::Running => 1,
            SupervisorStatus::Degraded { consecutive_failures } => consecutive_failures + 1,
            SupervisorStatus::Stopped => 0,
        };

        if failures >= self.max_retries {
            self.status = SupervisorStatus::Stopped;
            SupervisorAction::GiveUp
        } else {
            self.status = SupervisorStatus::Degraded {
                consecutive_failures: failures,
            };
            SupervisorAction::Retry {
                delay: self.backoff_delay(failures - 1),
            }
        }
    }

    pub fn record_success(&mut self) {
        self.status = SupervisorStatus::Running;
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum SupervisorAction {
    Retry { delay: Duration },
    GiveUp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockClipboard;
    use std::sync::atomic::AtomicUsize;

    #[test]
    fn test_supervisor_new() {
        let sup = Supervisor::new();
        assert!(sup.is_healthy());
        assert_eq!(*sup.status(), SupervisorStatus::Running);
    }

    #[test]
    fn test_supervisor_first_failure() {
        let mut sup = Supervisor::new();
        let action = sup.record_failure();
        assert!(matches!(action, SupervisorAction::Retry { .. }));
        assert!(matches!(sup.status(), SupervisorStatus::Degraded { .. }));
    }

    #[test]
    fn test_supervisor_max_retries() {
        let mut sup = Supervisor::new().with_max_retries(3);
        sup.record_failure();
        sup.record_failure();
        let action = sup.record_failure();
        assert_eq!(action, SupervisorAction::GiveUp);
        assert_eq!(*sup.status(), SupervisorStatus::Stopped);
    }

    #[test]
    fn test_supervisor_recovery() {
        let mut sup = Supervisor::new();
        sup.record_failure();
        assert!(!sup.is_healthy());
        sup.record_success();
        assert!(sup.is_healthy());
    }

    #[test]
    fn test_backoff_exponential() {
        let sup = Supervisor::new()
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(30));

        assert_eq!(sup.backoff_delay(0), Duration::from_secs(1));
        assert_eq!(sup.backoff_delay(1), Duration::from_secs(2));
        assert_eq!(sup.backoff_delay(2), Duration::from_secs(4));
        assert_eq!(sup.backoff_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn test_backoff_capped() {
        let sup = Supervisor::new()
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(4));

        assert_eq!(sup.backoff_delay(10), Duration::from_secs(4));
    }

    #[test]
    fn test_spawn_watcher_sends_events() {
        let mock = MockClipboard {
            text: Some("hello".to_string()),
            ..Default::default()
        };

        let (_handle, mut rx) = spawn_watcher(mock).unwrap();

        let received = Arc::new(AtomicUsize::new(0));
        let received_clone = received.clone();
        let timeout = Duration::from_secs(2);
        let start = std::time::Instant::now();

        while start.elapsed() < timeout {
            if let Ok(_event) = rx.try_recv() {
                let count = received_clone.fetch_add(1, Ordering::Relaxed);
                if count >= 1 {
                    break;
                }
            }
            thread::sleep(Duration::from_millis(50));
        }

        assert!(received.load(Ordering::Relaxed) >= 1);
    }

    #[test]
    fn test_watcher_stop_on_drop() {
        let mock = MockClipboard {
            text: Some("test".to_string()),
            ..Default::default()
        };

        {
            let (_handle, _rx) = spawn_watcher(mock).unwrap();
        }

        // handle dropped, thread should be joined
    }
}

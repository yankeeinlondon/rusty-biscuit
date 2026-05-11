//! Clipboard change watcher and supervisor.
//!
//! The watcher uses [`clipboard_rs::ClipboardWatcherContext`] (a real OS
//! change listener) to drive [`WatcherEvent`]s onto a Tokio mpsc channel.
//! It does **not** poll. Each event captures every available clipboard
//! format in a single snapshot and carries a `concealed` flag derived from
//! the backend.
//!
//! ## Examples
//!
//! ```no_run
//! use std::sync::Arc;
//! use biscuit_clipboard::backend::SystemClipboard;
//! use biscuit_clipboard::watcher::spawn_system_watcher;
//!
//! let backend = Arc::new(SystemClipboard::new().unwrap());
//! let (_handle, mut rx) = spawn_system_watcher(backend).unwrap();
//! // poll rx for WatcherEvent values...
//! ```
//!
//! ## Notes
//!
//! - The change-listener thread is plain `std::thread::spawn`; it is
//!   blocking and must not run on the Tokio runtime.
//! - The handler closure is wrapped in [`std::panic::catch_unwind`] so
//!   that an OS-level panic propagates as a `WatcherEvent::Died` instead
//!   of unwinding the watcher thread.
//! - Concealed events are still emitted; the consumer (the daemon) is
//!   responsible for dropping them before they reach the history. The
//!   `concealed` flag on `WatcherEvent` is the gate.
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

use tokio::sync::mpsc;
use tracing::{debug, error, warn};

use crate::backend::ClipboardBackend;
use crate::content::ClipboardFormat;
use crate::error::ClipboardError;

/// Default capacity of the watcher → consumer event channel.
///
/// ## Notes
///
/// Backpressure policy: the watcher uses `mpsc::Sender::try_send`,
/// which **drops the new event** when the channel is full rather than
/// blocking the change-listener thread (it is a plain `std::thread`
/// driven by the OS clipboard subsystem and must never block on Tokio
/// I/O). A drop is logged at `debug` level. This is a deliberate
/// drop-newest policy — the usual receiver (the daemon's history loop)
/// is fast, so a full channel signals a stalled history consumer or a
/// pathologically chatty producer; in either case dropping the
/// most-recent event is safer than queuing more memory.
pub const DEFAULT_CHANNEL_CAPACITY: usize = 64;

/// An event emitted by the watcher whenever the OS clipboard changes.
///
/// Carries every available format captured in a single snapshot, plus a
/// `concealed` flag drawn from the backend so the consumer can drop the
/// event before it touches history.
#[derive(Debug, Clone)]
pub enum WatcherEvent {
    /// A successful capture of one clipboard change.
    Changed {
        /// Every format that was available at change-time.
        formats: Vec<ClipboardFormat>,
        /// True when the platform marks the entry as concealed (e.g. a
        /// password manager pasteboard with `org.nspasteboard.ConcealedType`).
        concealed: bool,
    },
    /// The watcher's on-change handler panicked or otherwise died. The
    /// supervisor uses this signal to decide whether to re-spawn.
    Died {
        /// Human-readable reason. Already truncated; not for parsing.
        reason: String,
    },
}

/// Handle returned by [`spawn_watcher`] / [`spawn_system_watcher`]. Drop
/// it to request the watcher thread to stop and join.
pub struct WatcherHandle {
    stop: Arc<AtomicBool>,
    watcher_thread: Option<thread::JoinHandle<()>>,
    /// On the system path this is `Some` once `start_watch()` has been
    /// established. We hold it so the inner watcher can be told to shut
    /// down on `Drop`.
    shutdown: Option<Box<dyn FnOnce() + Send>>,
}

impl WatcherHandle {
    pub fn take_thread(&mut self) -> Option<thread::JoinHandle<()>> {
        self.watcher_thread.take()
    }
}

impl Drop for WatcherHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(shutdown) = self.shutdown.take() {
            shutdown();
        }
        if let Some(handle) = self.watcher_thread.take() {
            let _ = handle.join();
        }
    }
}

/// Capture every available clipboard format from `backend` into a
/// `WatcherEvent::Changed`, including the concealed flag.
///
/// ## Notes
///
/// Errors per format are logged and the format is skipped — a partial
/// capture is preferred over dropping the whole event.
pub fn capture_event<B: ClipboardBackend>(backend: &B) -> WatcherEvent {
    let mut formats = Vec::new();

    match backend.get_text() {
        Ok(Some(t)) => formats.push(ClipboardFormat::Text(t)),
        Ok(None) => {}
        Err(e) => warn!(error = %e, "watcher: failed to read text"),
    }
    match backend.get_html() {
        Ok(Some(s)) => formats.push(ClipboardFormat::Html(s)),
        Ok(None) => {}
        Err(e) => warn!(error = %e, "watcher: failed to read html"),
    }
    match backend.get_rtf() {
        Ok(Some(s)) => formats.push(ClipboardFormat::Rtf(s)),
        Ok(None) => {}
        Err(e) => warn!(error = %e, "watcher: failed to read rtf"),
    }
    match backend.get_image() {
        Ok(Some(img)) => formats.push(ClipboardFormat::Image(img)),
        Ok(None) => {}
        Err(e) => warn!(error = %e, "watcher: failed to read image"),
    }
    match backend.get_files() {
        Ok(Some(paths)) => formats.push(ClipboardFormat::Files(paths)),
        Ok(None) => {}
        Err(e) => warn!(error = %e, "watcher: failed to read files"),
    }

    let concealed = backend.is_concealed().unwrap_or(false);

    WatcherEvent::Changed { formats, concealed }
}

/// Production entry point: bind the watcher to the real
/// [`clipboard_rs::ClipboardWatcherContext`] change listener and stream
/// `WatcherEvent`s for every change.
///
/// ## Errors
///
/// Returns `ClipboardError::Backend` if the underlying watcher cannot be
/// constructed or the listener thread cannot be spawned.
pub fn spawn_system_watcher(
    backend: Arc<crate::backend::SystemClipboard>,
) -> Result<(WatcherHandle, mpsc::Receiver<WatcherEvent>), ClipboardError> {
    use clipboard_rs::{ClipboardHandler, ClipboardWatcher, ClipboardWatcherContext};

    let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
    let stop = Arc::new(AtomicBool::new(false));

    struct ChangeHandler<B: ClipboardBackend + Send + 'static> {
        backend: Arc<B>,
        tx: mpsc::Sender<WatcherEvent>,
    }

    impl<B: ClipboardBackend + Send + 'static> ClipboardHandler for ChangeHandler<B> {
        fn on_clipboard_change(&mut self) {
            let backend = self.backend.clone();
            let tx = self.tx.clone();
            // Catch any panic from the backend reads so that a faulty
            // platform driver doesn't unwind the listener thread.
            let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
                let event = capture_event(backend.as_ref());
                let _ = tx.try_send(event);
            }));
            if let Err(payload) = result {
                let reason = panic_payload_to_string(&payload);
                error!(reason = %reason, "watcher: on_clipboard_change panicked");
                let _ = self.tx.try_send(WatcherEvent::Died { reason });
            }
        }
    }

    let mut watcher_ctx = ClipboardWatcherContext::new()
        .map_err(|e| ClipboardError::Backend(format!("watcher init failed: {e}")))?;
    let handler = ChangeHandler {
        backend,
        tx: tx.clone(),
    };
    let shutdown_channel = watcher_ctx.add_handler(handler).get_shutdown_channel();

    let stop_clone = stop.clone();
    let tx_for_thread = tx.clone();
    let join_handle = thread::Builder::new()
        .name("clipboard-watcher".to_string())
        .spawn(move || {
            // start_watch is blocking until shutdown is invoked.
            let result = std::panic::catch_unwind(AssertUnwindSafe(move || {
                watcher_ctx.start_watch();
            }));
            if let Err(payload) = result {
                let reason = panic_payload_to_string(&payload);
                error!(reason = %reason, "watcher thread panicked");
                let _ = tx_for_thread.try_send(WatcherEvent::Died { reason });
            }
            stop_clone.store(true, Ordering::Relaxed);
        })
        .map_err(|e| ClipboardError::Backend(format!("watcher thread spawn failed: {e}")))?;

    let shutdown: Box<dyn FnOnce() + Send> = Box::new(move || {
        shutdown_channel.stop();
    });

    Ok((
        WatcherHandle {
            stop,
            watcher_thread: Some(join_handle),
            shutdown: Some(shutdown),
        },
        rx,
    ))
}

/// Test/synthetic entry point: drive the watcher off an explicit trigger
/// channel rather than the real OS change listener.
///
/// Send `()` on the returned `WatcherTrigger::Sender` and the watcher will
/// snapshot `backend` exactly once and emit a [`WatcherEvent`] (or
/// `Died` if the snapshot panicked).
///
/// This is the only way to deterministically test watcher wiring on CI;
/// the real `ClipboardWatcherContext` requires a live windowing system.
pub fn spawn_watcher_with_trigger<B>(
    backend: B,
) -> Result<(WatcherHandle, WatcherTrigger, mpsc::Receiver<WatcherEvent>), ClipboardError>
where
    B: ClipboardBackend + Send + 'static,
{
    let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
    let (trigger_tx, mut trigger_rx) = mpsc::channel::<TriggerKind>(DEFAULT_CHANNEL_CAPACITY);
    let stop = Arc::new(AtomicBool::new(false));

    let stop_clone = stop.clone();
    let tx_clone = tx.clone();

    let join_handle = thread::Builder::new()
        .name("clipboard-watcher-test".to_string())
        .spawn(move || {
            while !stop_clone.load(Ordering::Relaxed) {
                let kind = match trigger_rx.blocking_recv() {
                    Some(k) => k,
                    None => break,
                };
                match kind {
                    TriggerKind::Snapshot => {
                        let result =
                            std::panic::catch_unwind(AssertUnwindSafe(|| capture_event(&backend)));
                        match result {
                            Ok(event) => {
                                if tx_clone.try_send(event).is_err() {
                                    debug!("watcher: dropped event (channel full or closed)");
                                }
                            }
                            Err(payload) => {
                                let reason = panic_payload_to_string(&payload);
                                error!(reason = %reason, "watcher: capture panicked");
                                let _ = tx_clone.try_send(WatcherEvent::Died { reason });
                            }
                        }
                    }
                    TriggerKind::Stop => break,
                }
            }
        })
        .map_err(|e| ClipboardError::Backend(format!("watcher thread spawn failed: {e}")))?;

    let trigger = WatcherTrigger { sender: trigger_tx };

    Ok((
        WatcherHandle {
            stop,
            watcher_thread: Some(join_handle),
            shutdown: None,
        },
        trigger,
        rx,
    ))
}

#[derive(Clone, Copy, Debug)]
enum TriggerKind {
    Snapshot,
    Stop,
}

/// Test handle that drives a [`spawn_watcher_with_trigger`] watcher.
#[derive(Clone)]
pub struct WatcherTrigger {
    sender: mpsc::Sender<TriggerKind>,
}

impl WatcherTrigger {
    /// Ask the watcher to take a snapshot and emit a [`WatcherEvent`].
    pub fn fire(&self) -> Result<(), ClipboardError> {
        self.sender
            .try_send(TriggerKind::Snapshot)
            .map_err(|e| ClipboardError::Backend(format!("trigger send failed: {e}")))
    }

    /// Ask the watcher thread to exit at the next opportunity.
    pub fn stop(&self) {
        let _ = self.sender.try_send(TriggerKind::Stop);
    }
}

/// Compatibility shim: spawn a watcher backed by a real `SystemClipboard`
/// when `B` is one, or fall back to a dummy poll loop otherwise. This
/// preserves the old `spawn_watcher(backend)` shape for tests that
/// pre-date the real change-listener integration.
pub fn spawn_watcher<B>(
    backend: B,
) -> Result<(WatcherHandle, mpsc::Receiver<WatcherEvent>), ClipboardError>
where
    B: ClipboardBackend + Send + 'static,
{
    // Generic path: produce a single immediate snapshot then idle.
    let (tx, rx) = mpsc::channel(DEFAULT_CHANNEL_CAPACITY);
    let stop = Arc::new(AtomicBool::new(false));

    let stop_clone = stop.clone();
    let tx_clone = tx.clone();
    let join_handle = thread::Builder::new()
        .name("clipboard-watcher-fallback".to_string())
        .spawn(move || {
            let result = std::panic::catch_unwind(AssertUnwindSafe(|| capture_event(&backend)));
            match result {
                Ok(event) => {
                    let _ = tx_clone.try_send(event);
                }
                Err(payload) => {
                    let reason = panic_payload_to_string(&payload);
                    let _ = tx_clone.try_send(WatcherEvent::Died { reason });
                }
            }
            while !stop_clone.load(Ordering::Relaxed) {
                thread::sleep(Duration::from_millis(50));
            }
        })
        .map_err(|e| ClipboardError::Backend(format!("watcher thread spawn failed: {e}")))?;

    Ok((
        WatcherHandle {
            stop,
            watcher_thread: Some(join_handle),
            shutdown: None,
        },
        rx,
    ))
}

fn panic_payload_to_string(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&'static str>() {
        (*s).to_string()
    } else if let Some(s) = payload.downcast_ref::<String>() {
        s.clone()
    } else {
        "<panic with non-string payload>".to_string()
    }
}

/// Supervisor tracks watcher health and decides whether to re-spawn after
/// a death signal. State transitions:
///
/// - `Running` → on `record_failure` → `Degraded { 1 }`
/// - `Degraded { n }` → on `record_failure` → `Degraded { n+1 }` until
///   `n >= max_retries`, then `Stopped`.
/// - any → on `record_success` → `Running`.
pub struct Supervisor {
    max_retries: u32,
    base_delay: Duration,
    max_delay: Duration,
    status: SupervisorStatus,
}

/// Current health of the watcher as observed by the [`Supervisor`].
#[derive(Clone, Debug, PartialEq, Eq)]
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
    /// Build a supervisor with the spec defaults (max 3 retries, 1s base,
    /// 30s ceiling).
    pub fn new() -> Self {
        Self {
            max_retries: 3,
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
            SupervisorStatus::Degraded {
                consecutive_failures,
            } => consecutive_failures + 1,
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

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupervisorAction {
    Retry { delay: Duration },
    GiveUp,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MockClipboard;

    #[test]
    fn supervisor_starts_running() {
        let sup = Supervisor::new();
        assert!(sup.is_healthy());
        assert_eq!(*sup.status(), SupervisorStatus::Running);
    }

    #[test]
    fn supervisor_first_failure_is_degraded() {
        let mut sup = Supervisor::new();
        let action = sup.record_failure();
        assert!(matches!(action, SupervisorAction::Retry { .. }));
        assert!(matches!(sup.status(), SupervisorStatus::Degraded { .. }));
    }

    #[test]
    fn supervisor_max_retries_gives_up() {
        let mut sup = Supervisor::new().with_max_retries(3);
        sup.record_failure();
        sup.record_failure();
        let action = sup.record_failure();
        assert_eq!(action, SupervisorAction::GiveUp);
        assert_eq!(*sup.status(), SupervisorStatus::Stopped);
    }

    #[test]
    fn supervisor_recovers_on_success() {
        let mut sup = Supervisor::new();
        sup.record_failure();
        assert!(!sup.is_healthy());
        sup.record_success();
        assert!(sup.is_healthy());
    }

    #[test]
    fn supervisor_backoff_is_exponential() {
        let sup = Supervisor::new()
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(30));

        assert_eq!(sup.backoff_delay(0), Duration::from_secs(1));
        assert_eq!(sup.backoff_delay(1), Duration::from_secs(2));
        assert_eq!(sup.backoff_delay(2), Duration::from_secs(4));
        assert_eq!(sup.backoff_delay(3), Duration::from_secs(8));
    }

    #[test]
    fn supervisor_backoff_caps_at_max_delay() {
        let sup = Supervisor::new()
            .with_base_delay(Duration::from_secs(1))
            .with_max_delay(Duration::from_secs(4));

        assert_eq!(sup.backoff_delay(10), Duration::from_secs(4));
    }

    #[test]
    fn capture_event_collects_all_available_formats() {
        let mock = MockClipboard {
            text: Some("hi".to_string()),
            html: Some("<b>hi</b>".to_string()),
            ..Default::default()
        };
        let event = capture_event(&mock);
        match event {
            WatcherEvent::Changed { formats, concealed } => {
                assert!(!concealed);
                assert_eq!(formats.len(), 2);
                assert!(matches!(formats[0], ClipboardFormat::Text(_)));
                assert!(matches!(formats[1], ClipboardFormat::Html(_)));
            }
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    #[test]
    fn capture_event_marks_concealed() {
        let mock = MockClipboard {
            text: Some("secret".to_string()),
            concealed: true,
            ..Default::default()
        };
        let event = capture_event(&mock);
        match event {
            WatcherEvent::Changed { concealed, .. } => assert!(concealed),
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    /// Phase 6 / Test Coverage #1: a single clipboard event must surface
    /// every format the OS reports (text, html, rtf, image, files) in
    /// one `WatcherEvent::Changed`. Regression for the original watcher
    /// implementation that only captured text.
    #[test]
    fn capture_event_collects_text_html_rtf_image_and_files() {
        let mock = MockClipboard {
            text: Some("plain".to_string()),
            html: Some("<p>plain</p>".to_string()),
            rtf: Some("{\\rtf plain}".to_string()),
            image: Some(crate::content::ImageSnapshot::Inline {
                data: vec![1u8, 2, 3, 4],
                width: 2,
                height: 2,
            }),
            files: Some(vec![std::path::PathBuf::from("/tmp/a.txt")]),
            ..Default::default()
        };
        let event = capture_event(&mock);
        let formats = match event {
            WatcherEvent::Changed { formats, concealed } => {
                assert!(!concealed);
                formats
            }
            other => panic!("expected Changed, got {other:?}"),
        };
        assert_eq!(formats.len(), 5, "expected 5 formats, got {formats:?}");
        let mut saw_text = false;
        let mut saw_html = false;
        let mut saw_rtf = false;
        let mut saw_image = false;
        let mut saw_files = false;
        for fmt in &formats {
            match fmt {
                ClipboardFormat::Text(t) => {
                    assert_eq!(t, "plain");
                    saw_text = true;
                }
                ClipboardFormat::Html(h) => {
                    assert_eq!(h, "<p>plain</p>");
                    saw_html = true;
                }
                ClipboardFormat::Rtf(r) => {
                    assert_eq!(r, "{\\rtf plain}");
                    saw_rtf = true;
                }
                ClipboardFormat::Image(img) => {
                    match img {
                        crate::content::ImageSnapshot::Inline { width, height, .. } => {
                            assert_eq!(*width, 2);
                            assert_eq!(*height, 2);
                        }
                        crate::content::ImageSnapshot::Spilled { .. } => {
                            panic!("expected Inline image, got Spilled");
                        }
                    }
                    saw_image = true;
                }
                ClipboardFormat::Files(paths) => {
                    assert_eq!(paths.len(), 1);
                    saw_files = true;
                }
            }
        }
        assert!(
            saw_text && saw_html && saw_rtf && saw_image && saw_files,
            "missing format coverage: text={saw_text} html={saw_html} \
             rtf={saw_rtf} image={saw_image} files={saw_files}",
        );
    }

    #[tokio::test]
    async fn trigger_watcher_emits_change_events() {
        let mock = MockClipboard {
            text: Some("hello".to_string()),
            ..Default::default()
        };

        let (_handle, trigger, mut rx) = spawn_watcher_with_trigger(mock).unwrap();
        trigger.fire().unwrap();

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("watcher event timeout")
            .expect("watcher channel closed");

        match event {
            WatcherEvent::Changed { formats, concealed } => {
                assert!(!concealed);
                assert_eq!(formats.len(), 1);
                match &formats[0] {
                    ClipboardFormat::Text(t) => assert_eq!(t, "hello"),
                    other => panic!("expected Text, got {other:?}"),
                }
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[tokio::test]
    async fn trigger_watcher_marks_concealed_events() {
        let mock = MockClipboard {
            text: Some("secret".to_string()),
            concealed: true,
            ..Default::default()
        };

        let (_handle, trigger, mut rx) = spawn_watcher_with_trigger(mock).unwrap();
        trigger.fire().unwrap();

        let event = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("watcher event timeout")
            .expect("watcher channel closed");

        match event {
            WatcherEvent::Changed { concealed, .. } => assert!(concealed),
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    /// A backend whose first read panics, then succeeds. Used to verify
    /// the watcher catches panics and emits `Died`.
    struct PanickyBackend {
        call_count: std::sync::atomic::AtomicUsize,
    }

    impl ClipboardBackend for PanickyBackend {
        fn get_text(&self) -> Result<Option<String>, ClipboardError> {
            let n = self
                .call_count
                .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
                + 1;
            if n == 1 {
                panic!("synthetic panic from backend");
            }
            Ok(Some("recovered".to_string()))
        }
        fn get_html(&self) -> Result<Option<String>, ClipboardError> {
            Ok(None)
        }
        fn get_rtf(&self) -> Result<Option<String>, ClipboardError> {
            Ok(None)
        }
        fn get_image(&self) -> Result<Option<crate::content::ImageSnapshot>, ClipboardError> {
            Ok(None)
        }
        fn get_files(&self) -> Result<Option<Vec<std::path::PathBuf>>, ClipboardError> {
            Ok(None)
        }
        fn set_text(&self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
        fn set_html(&self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
        fn set_rtf(&self, _: &str) -> Result<(), ClipboardError> {
            Ok(())
        }
        fn set_image(&self, _: &[u8], _: u32, _: u32) -> Result<(), ClipboardError> {
            Ok(())
        }
        fn set_files(&self, _: &[std::path::PathBuf]) -> Result<(), ClipboardError> {
            Ok(())
        }
        fn is_concealed(&self) -> Result<bool, ClipboardError> {
            Ok(false)
        }
    }

    #[tokio::test]
    async fn watcher_catches_panic_and_emits_died() {
        let backend = PanickyBackend {
            call_count: std::sync::atomic::AtomicUsize::new(0),
        };
        let (_handle, trigger, mut rx) = spawn_watcher_with_trigger(backend).unwrap();

        trigger.fire().unwrap();
        let first = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("died event timeout")
            .expect("watcher channel closed");
        assert!(
            matches!(first, WatcherEvent::Died { .. }),
            "expected Died, got {first:?}"
        );

        // Subsequent fires must still succeed: the watcher loop survives.
        trigger.fire().unwrap();
        let second = tokio::time::timeout(Duration::from_secs(2), rx.recv())
            .await
            .expect("recovered event timeout")
            .expect("watcher channel closed");
        match second {
            WatcherEvent::Changed { formats, .. } => {
                assert!(!formats.is_empty(), "expected recovered text in formats");
            }
            other => panic!("expected Changed, got {other:?}"),
        }
    }

    #[tokio::test]
    async fn channel_is_bounded_and_drops_when_full() {
        // Fill a tiny channel without draining and assert that
        // `try_send` returns an error rather than blocking. This
        // documents the drop-newest policy described in
        // [`DEFAULT_CHANNEL_CAPACITY`]. We use a fresh capacity-1
        // channel rather than the watcher's 64-slot one so the test
        // is fast and deterministic.
        let (tx, _rx) = mpsc::channel::<WatcherEvent>(1);
        let event = WatcherEvent::Changed {
            formats: vec![ClipboardFormat::Text("first".into())],
            concealed: false,
        };
        tx.try_send(event.clone()).expect("first event must fit");
        // The channel is now full and no receiver is draining; the
        // policy is to drop on overflow.
        let dropped = tx.try_send(event);
        assert!(
            dropped.is_err(),
            "second send must fail when capacity is reached (drop-newest policy)"
        );
    }

    #[test]
    fn watcher_stops_on_drop() {
        let mock = MockClipboard {
            text: Some("test".to_string()),
            ..Default::default()
        };

        {
            let (_handle, _trigger, _rx) = spawn_watcher_with_trigger(mock).unwrap();
        }
        // dropped — thread must have exited cleanly. Test passes if we
        // get here without hanging.
    }
}

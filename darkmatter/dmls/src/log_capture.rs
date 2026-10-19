//! Captured `tracing` output for tests that assert DMLS's log lines.
//!
//! [`CapturedLogs`] formats every event, at every level, as one plain-text
//! line (`LEVEL target: message field=value …`, no timestamps or color).
//! [`CapturedLogs::scoped`] captures the calling thread only; a test whose
//! events come from DMLS's worker or dispatch threads uses
//! [`CapturedLogs::global`], which installs the process-wide default
//! subscriber and so can be called once per process (nextest runs each test in
//! its own process).

use std::io::Write;
use std::sync::{Arc, Mutex};

use tracing_subscriber::fmt::MakeWriter;

/// A shared buffer of formatted log lines.
#[derive(Clone, Debug, Default)]
pub struct CapturedLogs {
    buffer: Arc<Mutex<Vec<u8>>>,
}

impl CapturedLogs {
    /// Runs `body` with a subscriber capturing the calling thread's events.
    pub fn scoped<R>(body: impl FnOnce() -> R) -> (R, CapturedLogs) {
        let logs = CapturedLogs::default();
        let result = tracing::dispatcher::with_default(&logs.dispatch(), body);
        (result, logs)
    }

    /// Installs a capturing subscriber as the process-wide default.
    ///
    /// ## Panics
    ///
    /// When a global default subscriber is already installed.
    pub fn global() -> CapturedLogs {
        let logs = CapturedLogs::default();
        tracing::dispatcher::set_global_default(logs.dispatch())
            .expect("a global tracing subscriber is already installed");
        logs
    }

    /// A dispatcher writing into this buffer, for a test that runs a thread
    /// under it explicitly.
    pub fn dispatch(&self) -> tracing::Dispatch {
        let subscriber = tracing_subscriber::fmt()
            .with_writer(self.clone())
            .with_max_level(tracing::Level::TRACE)
            .with_ansi(false)
            .without_time()
            .finish();
        tracing::Dispatch::new(subscriber)
    }

    /// Every captured line, in emission order.
    pub fn lines(&self) -> Vec<String> {
        let buffer = self.buffer.lock().expect("log buffer poisoned");
        String::from_utf8_lossy(&buffer).lines().map(str::to_string).collect()
    }

    /// The captured lines containing `needle`.
    pub fn matching(&self, needle: &str) -> Vec<String> {
        self.lines().into_iter().filter(|line| line.contains(needle)).collect()
    }
}

impl Write for CapturedLogs {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.buffer.lock().expect("log buffer poisoned").extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

impl<'a> MakeWriter<'a> for CapturedLogs {
    type Writer = CapturedLogs;

    fn make_writer(&'a self) -> Self::Writer {
        self.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scoped_capture_records_level_target_message_and_fields() {
        let ((), logs) = CapturedLogs::scoped(|| {
            tracing::warn!(scope = "root", "retaining last-good trigger registry");
            tracing::debug!("a debug line");
        });
        let lines = logs.lines();
        assert_eq!(lines.len(), 2, "{lines:?}");
        assert!(lines[0].starts_with(" WARN dmls::log_capture::tests:"), "{lines:?}");
        assert!(lines[0].contains("retaining last-good trigger registry scope=\"root\""), "{lines:?}");
        assert_eq!(logs.matching("debug line").len(), 1);
    }

    #[test]
    fn scoped_capture_ignores_other_threads() {
        let ((), logs) = CapturedLogs::scoped(|| {
            std::thread::spawn(|| tracing::info!("elsewhere")).join().unwrap();
        });
        assert!(logs.matching("elsewhere").is_empty());
    }
}

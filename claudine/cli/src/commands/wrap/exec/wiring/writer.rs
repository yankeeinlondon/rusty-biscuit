//! (split out of `wiring/mod.rs`; see that file for the protocol overview)
#![allow(unused_imports)]
use super::*;

/// Single serialized writer over the child's stdin.
///
/// Wraps `ChildStdin` behind a `Mutex` so multiple sender call sites
/// (main-thread `send_initialize`, reader-thread auto-response handlers,
/// signal-handler cancel path) cannot interleave bytes in the middle of a
/// JSON-RPC line. Every `send_value` call serializes the value, appends
/// `\n`, and flushes immediately.
#[derive(Clone)]
pub(crate) struct WireWriter {
    inner: Arc<Mutex<Box<dyn Write + Send>>>,
}

impl WireWriter {
    /// Wrap the child's stdin pipe.
    pub(crate) fn from_child_stdin(stdin: ChildStdin) -> Self {
        Self::from_writer(Box::new(stdin))
    }

    /// Wrap an arbitrary `Write` impl. Used by tests to capture emitted
    /// JSON-RPC lines without spawning a process.
    pub(crate) fn from_writer(writer: Box<dyn Write + Send>) -> Self {
        Self {
            inner: Arc::new(Mutex::new(writer)),
        }
    }

    /// Serialize `value`, append `\n`, write to the inner writer, and
    /// flush. Returns the serialized bytes (sans newline) on success so
    /// tracing spans can record the body.
    pub(crate) fn send_value(&self, value: &Value) -> std::io::Result<String> {
        let serialized = serde_json::to_string(value)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        guard.write_all(serialized.as_bytes())?;
        guard.write_all(b"\n")?;
        guard.flush()?;
        Ok(serialized)
    }

    /// Close the underlying child stdin pipe by replacing the inner writer
    /// with `io::sink()`. This drops the original `ChildStdin`, signalling
    /// EOF to the Kimi child so it exits its read loop cleanly after the
    /// prompt response has been received. Subsequent `send_value` calls
    /// succeed silently (the bytes go to the sink), so any late cancel
    /// path remains a no-op rather than a panic.
    pub(crate) fn close_stdin(&self) {
        let mut guard = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        *guard = Box::new(io::sink());
    }
}

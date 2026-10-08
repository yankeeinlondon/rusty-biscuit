//! Where Claudine's own diagnostics go while a wrapped run owns the terminal,
//! and whether this process's terminal is known to have stopped accepting
//! output.
//!
//! A write to a terminal that stopped reading blocks in the kernel, and
//! nothing can interrupt it; closing a gate afterward does not release it.
//! So once a wrapped run (`wrap`, `compose`, `inline-compose`, `sequence`)
//! creates the process-wide output worker (`commands/wrap/output_worker.rs`),
//! [`route_to`] makes it the destination of every diagnostic: [`write`] (used
//! by `crate::log`, the tracing writer, the `--perf` report, interrupt
//! feedback, the panic hook, and the library's console lines through
//! [`write_library_line`]) queues a frame and returns at once, so no thread
//! but the worker ever waits on the terminal. Frames are written in the order
//! they were queued, so a healthy terminal shows the same bytes in the same
//! order as direct writes.
//!
//! Until then, and in every other command, [`write`] writes directly, exactly
//! as before.
//!
//! The stall gate is the second half: when the worker's bounded drain runs
//! out it abandons a thread still inside a terminal write, which holds std's
//! stdout or stderr lock for good. [`write`] then refuses a direct write
//! instead of queueing behind that lock. The gate starts open and only ever
//! closes.

use std::io::{self, Write as _};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::time::{Duration, Instant};

use crate::commands::wrap::output_worker::Stream;
use crate::commands::wrap::stream_io::StreamOutput;

static STALLED: AtomicBool = AtomicBool::new(false);
static SKIPPED_WRITES: AtomicU64 = AtomicU64::new(0);
static ROUTE: Mutex<Option<Arc<StreamOutput>>> = Mutex::new(None);

/// Close the gate for the rest of the process.
pub(crate) fn mark_stalled() {
    STALLED.store(true, Ordering::Release);
}

/// Whether the terminal is known stalled.
pub(crate) fn is_stalled() -> bool {
    STALLED.load(Ordering::Acquire)
}

/// Whether a direct terminal write may go ahead.
///
/// A refused write is counted in [`skipped_writes`], which the process-wide
/// output worker folds into its own loss accounting.
pub(crate) fn admit() -> bool {
    if is_stalled() {
        SKIPPED_WRITES.fetch_add(1, Ordering::Relaxed);
        false
    } else {
        true
    }
}

/// Direct terminal writes refused since the gate closed.
pub(crate) fn skipped_writes() -> u64 {
    SKIPPED_WRITES.load(Ordering::Relaxed)
}

/// Send every later diagnostic to `output`'s worker instead of the terminal.
///
/// Called once, by the first `StreamOutput::shared`; the route then holds
/// until the process exits, whose exit path drains the same worker.
pub(crate) fn route_to(output: Arc<StreamOutput>) {
    *lock_route() = Some(output);
}

/// Undo [`route_to`], so a test's blocked sink does not outlive it.
#[cfg(test)]
pub(crate) fn clear_route() {
    *lock_route() = None;
}

fn routed() -> Option<Arc<StreamOutput>> {
    lock_route().clone()
}

// Never held across a call that can panic or block, so a panic hook taking it
// cannot find it held by its own thread.
fn lock_route() -> MutexGuard<'static, Option<Arc<StreamOutput>>> {
    ROUTE.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Write one diagnostic to `stream`; returns whether it was queued or written.
///
/// While a wrapped run owns the terminal this queues `bytes` as one frame and
/// never blocks; otherwise it writes them directly unless the gate is closed.
/// Not for signal handlers: it may allocate and take a lock.
pub(crate) fn write(stream: Stream, bytes: &[u8]) -> bool {
    if let Some(output) = routed() {
        return output.submit_diagnostic(stream, bytes.to_vec());
    }
    if !admit() {
        return false;
    }
    let _ = match stream {
        Stream::Stdout => io::stdout().write_all(bytes),
        Stream::Stderr => io::stderr().write_all(bytes),
    };
    true
}

/// [`write`] as an [`io::Write`], for the tracing subscriber. Each `write`
/// call is one frame; tracing writes each formatted event in one call.
pub(crate) struct DiagnosticWriter(pub(crate) Stream);

impl io::Write for DiagnosticWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if !buf.is_empty() {
            write(self.0, buf);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

/// The library's console lines ([`claudine::render::console`]) as
/// diagnostics; installed once at startup.
pub(crate) fn write_library_line(stream: claudine::render::console::ConsoleStream, text: &str) {
    use claudine::render::console::ConsoleStream;
    let stream = match stream {
        ConsoleStream::Stdout => Stream::Stdout,
        ConsoleStream::Stderr => Stream::Stderr,
    };
    write(stream, text.as_bytes());
}

/// Give diagnostics already queued up to `budget` to reach the terminal,
/// before a forced exit that skips the CLI's ordinary exit drain.
pub(crate) fn settle(budget: Duration) {
    if let Some(output) = routed() {
        let _ = output.drain(Instant::now() + budget);
    }
}

/// Install color-eyre's error and panic hooks, with the panic report sent
/// through [`write`].
///
/// color-eyre's own panic hook writes the report with `eprintln!`. A reader
/// thread that panicked while the worker was stuck on the terminal would wait
/// in that write for good, so its join would see a timeout instead of the
/// panic. Queued instead, the thread unwinds at once and the join reports the
/// panic with its payload.
pub(crate) fn install_report_hooks() -> color_eyre::Result<()> {
    let (panic_hook, eyre_hook) = color_eyre::config::HookBuilder::default().try_into_hooks()?;
    eyre_hook.install()?;
    std::panic::set_hook(queued_panic_hook(panic_hook));
    Ok(())
}

/// color-eyre's panic report, written through [`write`].
pub(crate) fn queued_panic_hook(
    panic_hook: color_eyre::config::PanicHook,
) -> Box<dyn Fn(&std::panic::PanicHookInfo<'_>) + Send + Sync + 'static> {
    Box::new(move |info| {
        let report = format!("{}\n", panic_hook.panic_report(info));
        write(Stream::Stderr, report.as_bytes());
    })
}

#[cfg(test)]
pub(crate) mod tests;

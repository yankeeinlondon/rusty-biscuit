//! A live, single-line activity spinner for long-running foreground work.
//!
//! Unlike the other components, [`Spinner`] is not a [`TerminalRenderable`]
//! tree node: it is time-driven, owns a background thread, and redraws one
//! line in place until it is finished. The frame and clear sequences are
//! exposed as pure values ([`frame`], [`CLEAR_LINE`]) so they can be tested
//! without a terminal.
//!
//! [`TerminalRenderable`]: crate::components::renderable::TerminalRenderable

use std::io::{IsTerminal, Write};
use std::sync::{Arc, Condvar, Mutex, MutexGuard};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::discovery::detection::terminal_width;
use crate::utils::block_constraint::split_at_visible_width;

/// Glyphs cycled by [`frame`].
pub const FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

/// Returns the cursor to column 0 and erases the whole line.
pub const CLEAR_LINE: &str = "\r\x1b[2K";

/// Approximate time between two drawn frames.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(80);

/// Returns frame `index` of a spinner showing `text`, for a line `width`
/// columns wide.
///
/// The frame starts with a carriage return and ends with an erase-to-end-of-line,
/// so it overwrites the previous frame in place even when the text got shorter.
/// Its visible width is at most `width - 1`: filling the last column would let
/// terminals that wrap eagerly (the Windows console) move the cursor to the next
/// line, and the next frame would then no longer overwrite this one.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::components::spinner::frame;
///
/// assert_eq!(frame(0, "updating", 80), "\r⠋ updating\x1b[K");
/// assert_eq!(frame(0, "updating", 6), "\r⠋ upd\x1b[K");
/// ```
pub fn frame(index: usize, text: &str, width: u32) -> String {
    let glyph = FRAMES[index % FRAMES.len()];
    let line = format!("{glyph} {text}");
    let (visible, _) = split_at_visible_width(&line, width.saturating_sub(1));
    format!("\r{visible}\x1b[K")
}

/// Builder for a spinner drawn on one terminal line.
///
/// ## Examples
///
/// ```no_run
/// use std::time::Duration;
/// use biscuit_terminal::prelude::Spinner;
///
/// let spinner = Spinner::new("updating")
///     .with_delay(Duration::from_millis(150))
///     .start_on_stderr();
/// // ... long-running work ...
/// spinner.set_text("rate limited, using fallback method");
/// spinner.finish();
/// ```
#[derive(Debug, Clone)]
pub struct Spinner {
    text: String,
    delay: Duration,
    width: Option<u32>,
}

impl Spinner {
    /// Creates a spinner showing `text`, drawn immediately once started.
    ///
    /// `text` should be a single line; it is truncated to the line width.
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            delay: Duration::ZERO,
            width: None,
        }
    }

    /// Draws nothing until `delay` has elapsed, so work that finishes quickly
    /// never flashes a spinner.
    pub fn with_delay(mut self, delay: Duration) -> Self {
        self.delay = delay;
        self
    }

    /// Fixes the line width instead of querying the terminal before each frame.
    pub fn with_width(mut self, width: u32) -> Self {
        self.width = Some(width);
        self
    }

    /// Starts the spinner on stderr, or returns an inert handle when stderr is
    /// not a terminal.
    pub fn start_on_stderr(self) -> SpinnerHandle {
        let is_terminal = std::io::stderr().is_terminal();
        self.start_on(std::io::stderr(), is_terminal)
    }

    /// Starts the spinner on `writer`.
    ///
    /// When `is_terminal` is `false` no thread is spawned and nothing is ever
    /// written, including by [`SpinnerHandle::finish`] and `Drop`.
    pub fn start_on<W: Write + Send + 'static>(
        self,
        writer: W,
        is_terminal: bool,
    ) -> SpinnerHandle {
        let shared = Arc::new(Shared {
            state: Mutex::new(State {
                text: self.text,
                stopped: false,
            }),
            wake: Condvar::new(),
        });
        let thread = is_terminal.then(|| {
            let shared = Arc::clone(&shared);
            let delay = self.delay;
            let width = self.width;
            std::thread::spawn(move || run(writer, &shared, delay, width))
        });
        SpinnerHandle { shared, thread }
    }
}

/// A running spinner; finishing or dropping it stops the thread and clears
/// its line.
///
/// The line is cleared exactly once, and only if at least one frame was drawn:
/// a spinner stopped before its delay elapsed (or on a non-terminal) writes
/// nothing at all, so no stray escape sequence reaches the output.
#[derive(Debug)]
pub struct SpinnerHandle {
    shared: Arc<Shared>,
    thread: Option<JoinHandle<()>>,
}

impl SpinnerHandle {
    /// Replaces the text; the next frame shows it.
    pub fn set_text(&self, text: impl Into<String>) {
        self.shared.lock().text = text.into();
    }

    /// Stops the spinner and waits until its line has been cleared.
    pub fn finish(mut self) {
        self.stop();
    }

    fn stop(&mut self) {
        let Some(thread) = self.thread.take() else {
            return;
        };
        self.shared.lock().stopped = true;
        self.shared.wake.notify_all();
        // A panic in the drawing thread only loses the spinner line; it must
        // not turn into a panic in the caller (or a double panic in `Drop`).
        let _ = thread.join();
    }
}

impl Drop for SpinnerHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

#[derive(Debug)]
struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        // The state is a plain string and flag, valid even after a panic.
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    /// Waits until `deadline` or until stopped; returns the current text, or
    /// `None` once stopped.
    fn wait_until(&self, deadline: Instant) -> Option<String> {
        let mut state = self.lock();
        loop {
            if state.stopped {
                return None;
            }
            let now = Instant::now();
            if now >= deadline {
                return Some(state.text.clone());
            }
            state = self
                .wake
                .wait_timeout(state, deadline - now)
                .map(|(guard, _)| guard)
                .unwrap_or_else(|poisoned| poisoned.into_inner().0);
        }
    }
}

#[derive(Debug)]
struct State {
    text: String,
    stopped: bool,
}

fn run<W: Write>(mut writer: W, shared: &Shared, delay: Duration, width: Option<u32>) {
    let mut deadline = Instant::now() + delay;
    let mut drawn = false;
    let mut index = 0usize;
    while let Some(text) = shared.wait_until(deadline) {
        let line_width = width.unwrap_or_else(terminal_width);
        // Write errors are ignored: a spinner is decoration and must never
        // fail the work it accompanies.
        let _ = writer
            .write_all(frame(index, &text, line_width).as_bytes())
            .and_then(|()| writer.flush());
        drawn = true;
        index = index.wrapping_add(1);
        deadline = Instant::now() + FRAME_INTERVAL;
    }
    if drawn {
        let _ = writer
            .write_all(CLEAR_LINE.as_bytes())
            .and_then(|()| writer.flush());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils::block_constraint;

    fn visible_width(line: &str) -> u32 {
        block_constraint::visible_width(line.trim_start_matches('\r'))
    }

    const POLL_LIMIT: Duration = Duration::from_secs(10);

    #[derive(Clone, Default)]
    struct SharedBuffer(Arc<Mutex<(Vec<u8>, Option<Instant>)>>);

    impl SharedBuffer {
        fn contents(&self) -> String {
            String::from_utf8(self.0.lock().unwrap().0.clone()).unwrap()
        }

        fn first_write_at(&self) -> Option<Instant> {
            self.0.lock().unwrap().1
        }

        fn wait_for(&self, predicate: impl Fn(&str) -> bool) -> String {
            let started = Instant::now();
            loop {
                let contents = self.contents();
                if predicate(&contents) {
                    return contents;
                }
                assert!(
                    started.elapsed() < POLL_LIMIT,
                    "condition not met; output so far: {contents:?}"
                );
                std::thread::sleep(Duration::from_millis(5));
            }
        }
    }

    impl Write for SharedBuffer {
        fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
            let mut guard = self.0.lock().unwrap();
            guard.1.get_or_insert_with(Instant::now);
            guard.0.extend_from_slice(bytes);
            Ok(bytes.len())
        }

        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    fn started(buffer: &SharedBuffer, delay: Duration) -> SpinnerHandle {
        Spinner::new("alpha")
            .with_delay(delay)
            .with_width(80)
            .start_on(buffer.clone(), true)
    }

    #[test]
    fn first_frame_is_drawn_only_after_the_delay() {
        let buffer = SharedBuffer::default();
        let delay = Duration::from_millis(200);
        let started_at = Instant::now();

        let handle = started(&buffer, delay);
        buffer.wait_for(|out| out.contains("alpha"));

        let first_write = buffer.first_write_at().expect("a frame was written");
        assert!(first_write.duration_since(started_at) >= delay);
        handle.finish();
    }

    #[test]
    fn finish_before_the_delay_writes_nothing() {
        let buffer = SharedBuffer::default();

        started(&buffer, Duration::from_secs(3600)).finish();

        assert_eq!(buffer.contents(), "");
    }

    #[test]
    fn non_terminal_writes_nothing_and_spawns_no_thread() {
        let buffer = SharedBuffer::default();

        let handle = Spinner::new("alpha").start_on(buffer.clone(), false);
        assert!(handle.thread.is_none());
        handle.set_text("beta");
        drop(handle);

        assert_eq!(buffer.contents(), "");
    }

    #[test]
    fn set_text_changes_the_next_frame() {
        let buffer = SharedBuffer::default();
        let handle = started(&buffer, Duration::ZERO);
        buffer.wait_for(|out| out.contains("alpha"));

        handle.set_text("beta");
        let out = buffer.wait_for(|out| out.contains("beta"));
        handle.finish();

        let after_switch = &out[out.find("beta").unwrap()..];
        assert!(!after_switch.contains("alpha"), "{after_switch:?}");
    }

    #[test]
    fn finish_writes_the_clear_sequence_exactly_once() {
        let buffer = SharedBuffer::default();
        let handle = started(&buffer, Duration::ZERO);
        buffer.wait_for(|out| out.contains("alpha"));

        handle.finish();

        let out = buffer.contents();
        assert_eq!(out.matches(CLEAR_LINE).count(), 1, "{out:?}");
        assert!(out.ends_with(CLEAR_LINE));
    }

    #[test]
    fn drop_writes_the_clear_sequence_exactly_once() {
        let buffer = SharedBuffer::default();
        let handle = started(&buffer, Duration::ZERO);
        buffer.wait_for(|out| out.contains("alpha"));

        drop(handle);

        let out = buffer.contents();
        assert_eq!(out.matches(CLEAR_LINE).count(), 1, "{out:?}");
        assert!(out.ends_with(CLEAR_LINE));
    }

    #[test]
    fn frames_are_truncated_below_the_width() {
        let text = "a long status message that cannot fit";

        for width in [0, 1, 2, 5, 12] {
            let line = frame(3, text, width);
            assert!(visible_width(&line) < width.max(1), "{width}: {line:?}");
        }
    }

    #[test]
    fn frames_truncate_wide_characters_by_display_width() {
        let line = frame(0, "日本語のテキスト", 8);

        assert_eq!(line, "\r⠋ 日本\x1b[K");
        assert!(visible_width(&line) <= 7);
    }

    #[test]
    fn frames_cycle_through_the_glyphs() {
        assert_eq!(frame(1, "x", 80), "\r⠙ x\x1b[K");
        assert_eq!(frame(FRAMES.len() + 1, "x", 80), frame(1, "x", 80));
    }
}

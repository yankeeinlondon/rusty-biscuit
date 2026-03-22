use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::Renderable;
use biscuit_terminal::discovery::detection::ColorDepth;
use biscuit_terminal::terminal::Terminal;

static PLAIN: AtomicBool = AtomicBool::new(false);

/// Enable plain mode (strip all ANSI escape codes from output).
pub fn set_plain(plain: bool) {
    PLAIN.store(plain, Ordering::Relaxed);
}

/// Returns true if plain mode is active.
pub fn is_plain() -> bool {
    PLAIN.load(Ordering::Relaxed)
}

/// Returns a [`Terminal`] appropriate for the current mode.
///
/// In plain mode, returns a terminal with `is_tty: false` and
/// `color_depth: None` so components render with correct alignment
/// but no ANSI escape codes. In normal mode, returns a standard
/// detected terminal.
pub fn terminal() -> Terminal {
    if is_plain() {
        Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .build()
    } else {
        Terminal::new()
    }
}

/// Returns an optimistic [`Terminal`] that respects plain mode.
///
/// In plain mode, returns a terminal with `is_tty: false` and
/// `color_depth: None`. Otherwise returns a full-capability optimistic
/// terminal at the given width (or 80 columns if `None`).
pub fn optimistic_terminal(width: Option<u32>) -> Terminal {
    let w = width.unwrap_or(80);
    if is_plain() {
        Terminal::builder()
            .is_tty(false)
            .color_depth(ColorDepth::None)
            .build()
    } else {
        Terminal::new_optimistic(w)
    }
}

/// Strip ANSI escape codes if plain mode is active, otherwise return as-is.
///
/// Use this for output that was already rendered by code you don't control
/// (e.g. child process output or third-party rendering).
pub fn maybe_strip(text: &str) -> String {
    if is_plain() {
        biscuit_terminal::prelude::strip_escape_codes(text)
    } else {
        text.to_string()
    }
}

/// Write a message to stderr (always visible, no verbose flag required).
pub fn message(msg: &str) {
    let _ = writeln!(std::io::stderr(), "{msg}");
}

/// Write an info message to stderr (only when INFO tracing level is enabled).
#[allow(dead_code)]
pub fn info(msg: &str) {
    if tracing::enabled!(tracing::Level::INFO) {
        let _ = writeln!(std::io::stderr(), "{msg}");
    }
}

/// Write data to stdout (for piping/machine consumption).
pub fn data(msg: &str) {
    let _ = writeln!(std::io::stdout(), "{msg}");
}

/// Write output to stdout without a trailing newline.
pub fn output(msg: &str) {
    let _ = write!(std::io::stdout(), "{msg}");
}

/// Write a warning to stderr in yellow, rendered through Prose.
pub fn warn(msg: &str) {
    let term = terminal();
    let rendered =
        Prose::new(format!("<orange><bold>warning:</bold></orange> {msg}")).render(&term);
    let _ = writeln!(std::io::stderr(), "{rendered}");
}

/// Write an error to stderr in red, rendered through Prose.
///
/// Error messages may contain Prose tags (e.g. `<blue>--flag</blue>`) for
/// styled rendering of CLI switches and other highlights.
pub fn error(msg: &str) {
    let term = terminal();
    let rendered = Prose::new(format!("<red><bold>Error:</bold></red> {msg}")).render(&term);
    let _ = writeln!(std::io::stderr(), "\n{rendered}");
}

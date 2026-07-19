use std::io::Write;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::discovery::detection::{ColorDepth, ColorMode};
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

fn colors_disabled() -> bool {
    is_plain() || std::env::var_os("NO_COLOR").is_some()
}

fn force_color_enabled() -> bool {
    !colors_disabled()
        && std::env::var_os("FORCE_COLOR")
            .map(|value| value != "0")
            .unwrap_or(false)
}

fn forced_width(default: u32) -> u32 {
    std::env::var("TERM_WIDTH")
        .ok()
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| {
            std::env::var("COLUMNS")
                .ok()
                .and_then(|value| value.parse::<u32>().ok())
        })
        .unwrap_or(default)
}

fn compute_terminal() -> Terminal {
    if colors_disabled() {
        plain_terminal(forced_width(80))
    } else if force_color_enabled() {
        // FORCE_COLOR forces styling, not geometry. Shells that export it
        // globally still run in real terminals, so default to the detected
        // width (which itself falls back to 80 when stdout is not a tty)
        // rather than pinning wide terminals to 80 columns.
        Terminal::new_optimistic(forced_width(
            biscuit_terminal::discovery::detection::terminal_width(),
        ))
    } else {
        Terminal::new()
    }
}

static TERMINAL: LazyLock<Terminal> = LazyLock::new(compute_terminal);

/// Returns a [`Terminal`] appropriate for the current mode.
///
/// In plain mode, returns a terminal with `is_tty: false` and
/// `color_depth: None` so components render with correct alignment
/// but no ANSI escape codes. In normal mode, returns a standard
/// detected terminal.
///
/// The result is memoised per-process via [`LazyLock`] to avoid
/// repeated capability detection on every call.
pub fn terminal() -> Terminal {
    TERMINAL.clone()
}

/// Returns an optimistic [`Terminal`] that respects plain mode.
///
/// In plain mode, returns a terminal with `is_tty: false` and
/// `color_depth: None`. Otherwise returns a full-capability optimistic
/// terminal at the given width (or 80 columns if `None`).
pub fn optimistic_terminal(width: Option<u32>) -> Terminal {
    let w = width.unwrap_or(80);
    if colors_disabled() {
        plain_terminal(w)
    } else {
        Terminal::new_optimistic(w)
    }
}

fn plain_terminal(width: u32) -> Terminal {
    let mut term = Terminal::new_optimistic(width);
    term.is_tty = false;
    term.color_depth = ColorDepth::None;
    term.color_mode = ColorMode::Dark;
    term
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

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn no_color_disables_terminal_styling() {
        set_plain(false);
        unsafe {
            std::env::set_var("NO_COLOR", "1");
            std::env::remove_var("FORCE_COLOR");
        }

        let term = compute_terminal();
        assert!(!term.is_tty);
        assert_eq!(term.color_depth, ColorDepth::None);
        assert!(matches!(term.color_mode, ColorMode::Dark));

        unsafe {
            std::env::remove_var("NO_COLOR");
        }
    }

    #[test]
    #[serial]
    fn force_color_enables_optimistic_terminal_for_non_tty_runs() {
        set_plain(false);
        unsafe {
            std::env::remove_var("NO_COLOR");
            std::env::set_var("FORCE_COLOR", "1");
            std::env::set_var("TERM_WIDTH", "120");
        }

        let term = compute_terminal();
        assert!(term.is_tty);
        assert_eq!(term.color_depth, ColorDepth::TrueColor);
        assert_eq!(term.width(), 120);

        unsafe {
            std::env::remove_var("FORCE_COLOR");
            std::env::remove_var("TERM_WIDTH");
        }
    }

    #[test]
    #[serial]
    fn plain_mode_overrides_force_color() {
        set_plain(true);
        unsafe {
            std::env::set_var("FORCE_COLOR", "1");
        }

        let term = optimistic_terminal(Some(100));
        assert!(!term.is_tty);
        assert_eq!(term.color_depth, ColorDepth::None);
        assert!(matches!(term.color_mode, ColorMode::Dark));

        set_plain(false);
        unsafe {
            std::env::remove_var("FORCE_COLOR");
        }
    }

    #[test]
    fn terminal_is_memoized() {
        let t1 = terminal();
        let t2 = terminal();
        // Clones from the same LazyLock must have identical properties.
        assert_eq!(t1.is_tty, t2.is_tty);
        assert_eq!(t1.color_depth, t2.color_depth);
        assert_eq!(t1.width(), t2.width());
    }
}

use terminal_size::{Height, Width, terminal_size};

use std::sync::atomic::{AtomicU8, Ordering};

const TTY_OVERRIDE_UNSET: u8 = 0;
const TTY_OVERRIDE_FALSE: u8 = 1;
const TTY_OVERRIDE_TRUE: u8 = 2;

static TTY_OVERRIDE: AtomicU8 = AtomicU8::new(TTY_OVERRIDE_UNSET);

/// Overrides TTY detection for diagnostic probes in the current process.
///
/// Ordinary callers should rely on [`is_tty`]. This hook exists for a PTY
/// host whose child handles carry terminal traffic but are not recognized by
/// [`std::io::IsTerminal`], as happens with Windows ConPTY test processes.
#[doc(hidden)]
pub fn set_tty_override(value: Option<bool>) {
    let value = match value {
        Some(false) => TTY_OVERRIDE_FALSE,
        Some(true) => TTY_OVERRIDE_TRUE,
        None => TTY_OVERRIDE_UNSET,
    };
    TTY_OVERRIDE.store(value, Ordering::Relaxed);
}

/// Check whether this process is attached to a TTY on **either** stdout
/// **or** stderr.
///
/// Most CLI rendering libraries write to one of the two standard output
/// streams: typed/machine-readable output to stdout, human-facing status
/// and prompts to stderr. A terminal-aware library cannot know which
/// stream a downstream caller will ultimately use, so the right question
/// to answer here is "does this process have *a* terminal available?",
/// not "is stdout specifically a terminal?".
///
/// Checking only stdout produced false negatives whenever a CLI piped its
/// stdout (e.g., `claudine ... | jq`, log capture, wrapper subprocesses)
/// while still rendering rich output to stderr — capability detection
/// fell back to "no terminal" and downstream Renderables degraded to
/// ASCII even though the user was looking at a fully capable graphics
/// terminal on their stderr.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::is_tty;
///
/// if is_tty() {
///     eprintln!("\x1b[32mColored output\x1b[0m");
/// } else {
///     eprintln!("Plain output (piped or redirected)");
/// }
/// ```
pub fn is_tty() -> bool {
    use std::io::IsTerminal;
    match TTY_OVERRIDE.load(Ordering::Relaxed) {
        TTY_OVERRIDE_FALSE => false,
        TTY_OVERRIDE_TRUE => true,
        _ => std::io::stdout().is_terminal() || std::io::stderr().is_terminal(),
    }
}

/// Get the terminal width in columns.
///
/// Returns 80 as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::terminal_width;
///
/// let width = terminal_width();
/// println!("Terminal is {} columns wide", width);
/// ```
pub fn terminal_width() -> u32 {
    dimensions().0
}

/// Get the terminal height in rows.
///
/// Returns 24 as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::terminal_height;
///
/// let height = terminal_height();
/// println!("Terminal is {} rows tall", height);
/// ```
pub fn terminal_height() -> u32 {
    dimensions().1
}

/// Get the terminal dimensions as (width, height) in characters.
///
/// Returns (80, 24) as a fallback if detection fails.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::dimensions;
///
/// let (width, height) = dimensions();
/// println!("Terminal size: {}x{}", width, height);
/// ```
pub fn dimensions() -> (u32, u32) {
    terminal_size()
        .map(|(Width(w), Height(h))| (w as u32, h as u32))
        .unwrap_or((80, 24))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_tty_returns_bool() {
        let _ = is_tty();
    }

    #[test]
    fn test_dimensions_returns_tuple() {
        let (w, h) = dimensions();
        assert!(w > 0);
        assert!(h > 0);
    }

    #[test]
    fn test_terminal_width_matches_dimensions() {
        assert_eq!(terminal_width(), dimensions().0);
    }

    #[test]
    fn test_terminal_height_matches_dimensions() {
        assert_eq!(terminal_height(), dimensions().1);
    }
}

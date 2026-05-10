use terminal_size::{Height, Width, terminal_size};

/// Check if stdout is connected to a TTY (terminal).
///
/// Returns `false` when output is piped or redirected to a file.
/// Useful for deciding whether to use colors/formatting.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::is_tty;
///
/// if is_tty() {
///     println!("\x1b[32mColored output\x1b[0m");
/// } else {
///     println!("Plain output (piped or redirected)");
/// }
/// ```
pub fn is_tty() -> bool {
    use std::io::IsTerminal;
    std::io::stdout().is_terminal()
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

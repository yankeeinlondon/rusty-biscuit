//! Mode 2027 (Unicode grapheme cluster width) support detection.
//!
//! Mode 2027 is a terminal feature that ensures proper rendering of complex
//! Unicode characters including:
//!
//! - Emoji with Zero-Width Joiners (ZWJ): Family emoji, skin tone modifiers
//! - Flag emoji: Regional indicator pairs
//! - Combined characters: Accented characters, Indic scripts
//!
//! ## Background
//!
//! Traditional terminals calculate display width using wcwidth(), which can
//! miscalculate the width of complex Unicode sequences. Mode 2027 tells the
//! terminal to use grapheme cluster segmentation instead.
//!
//! ## Terminal Support
//!
//! As of 2025, the following terminals support Mode 2027:
//!
//! - Kitty (native)
//! - WezTerm (native)
//! - Ghostty (native)
//! - Foot (native)
//! - Contour (native)
//!
//! ## Examples
//!
//! ```
//! use biscuit_terminal::discovery::mode_2027::supports_mode_2027;
//!
//! if supports_mode_2027() {
//!     println!("Terminal supports proper Unicode grapheme width");
//! }
//! ```

use crate::discovery::detection::{get_terminal_app, is_tty, TerminalApp};

/// Check if terminal supports Mode 2027 (grapheme cluster width).
///
/// Mode 2027 is a terminal feature that ensures proper rendering of
/// complex Unicode characters (emoji with ZWJ, flags, etc.).
///
/// This is a heuristic check based on terminal application detection,
/// not an actual terminal query (which would require raw mode).
///
/// ## Returns
///
/// - `true` if the terminal is known to support Mode 2027
/// - `false` if not in a TTY or terminal doesn't support Mode 2027
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::mode_2027::supports_mode_2027;
///
/// if supports_mode_2027() {
///     // Safe to use complex emoji sequences
///     println!("Family: \u{1F468}\u{200D}\u{1F469}\u{200D}\u{1F467}");
/// }
/// ```
pub fn supports_mode_2027() -> bool {
    if !is_tty() {
        return false;
    }

    matches!(
        get_terminal_app(),
        TerminalApp::Kitty
            | TerminalApp::Wezterm
            | TerminalApp::Ghostty
            | TerminalApp::Foot
            | TerminalApp::Contour
    )
}

/// Enable Mode 2027 (grapheme cluster width) in the terminal.
///
/// Writes the escape sequence to enable Mode 2027. This only has an effect
/// on terminals that support it.
///
/// Sequence: `\x1b[?2027h`
///
/// ## Returns
///
/// - `Ok(())` if the sequence was written successfully
/// - `Err` if writing to stdout failed
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::mode_2027::enable_mode_2027;
///
/// enable_mode_2027().ok();
/// ```
pub fn enable_mode_2027() -> std::io::Result<()> {
    use std::io::Write;

    if !supports_mode_2027() {
        return Ok(()); // No-op on unsupported terminals
    }

    let mut stdout = std::io::stdout();
    stdout.write_all(b"\x1b[?2027h")?;
    stdout.flush()
}

/// Disable Mode 2027 (grapheme cluster width) in the terminal.
///
/// Writes the escape sequence to disable Mode 2027, reverting to
/// traditional wcwidth-based width calculation.
///
/// Sequence: `\x1b[?2027l`
///
/// ## Returns
///
/// - `Ok(())` if the sequence was written successfully
/// - `Err` if writing to stdout failed
pub fn disable_mode_2027() -> std::io::Result<()> {
    use std::io::Write;

    if !supports_mode_2027() {
        return Ok(()); // No-op on unsupported terminals
    }

    let mut stdout = std::io::stdout();
    stdout.write_all(b"\x1b[?2027l")?;
    stdout.flush()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supports_mode_2027_returns_bool() {
        // Just verify it doesn't panic and returns a bool
        let _ = supports_mode_2027();
    }

    #[test]
    fn test_enable_mode_2027_no_panic() {
        // In test environment (likely not a TTY), this should not panic
        // It may fail or succeed depending on stdout, but shouldn't panic
        let _ = enable_mode_2027();
    }

    #[test]
    fn test_disable_mode_2027_no_panic() {
        // In test environment (likely not a TTY), this should not panic
        let _ = disable_mode_2027();
    }
}

//! Terminal output testing utilities.
//!
//! Provides ANSI escape sequence handling and terminal output verification
//! capabilities for testing terminal-based user interfaces.

use regex::Regex;
use std::sync::{Arc, Mutex};

/// Removes all ANSI escape sequences from a string.
///
/// This function strips ANSI CSI (Control Sequence Introducer) codes used for
/// terminal formatting (colors, bold, underline, etc.) and cursor control.
///
/// ## Examples
///
/// ```rust
/// use darkmatter_lib::testing::strip_ansi_codes;
///
/// let colored = "\x1b[31mRed\x1b[0m \x1b[1mBold\x1b[0m";
/// assert_eq!(strip_ansi_codes(colored), "Red Bold");
/// ```
///
/// ## Implementation Notes
///
/// - Matches standard CSI sequences: `ESC [ ... m` (SGR - Select Graphic Rendition)
/// - Matches CSI sequences with parameters: `ESC [ <params> <cmd>`
/// - Does not strip non-CSI escape sequences (e.g., OSC for title setting)
pub fn strip_ansi_codes(input: &str) -> String {
    lazy_static::lazy_static! {
        static ref ANSI_REGEX: Regex = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    }
    ANSI_REGEX.replace_all(input, "").to_string()
}

/// A test terminal that captures output for verification.
///
/// `TestTerminal` provides a way to capture and verify terminal output in tests,
/// including both plain text content and ANSI escape sequences for color/formatting.
///
/// ## Examples
///
/// ```rust
/// use darkmatter_lib::testing::TestTerminal;
///
/// let mut terminal = TestTerminal::new();
/// terminal.run(|term| {
///     term.push_str("\x1b[32mSuccess!\x1b[0m");
/// });
///
/// // Verify plain text content (ANSI codes stripped)
/// terminal.assert_output("Success!");
///
/// // Verify specific ANSI codes are present
/// terminal.assert_has_color("\x1b[32m"); // Green foreground
/// ```
#[derive(Clone)]
pub struct TestTerminal {
    /// Captured output buffer (shared across threads for test execution)
    output: Arc<Mutex<String>>,
}

impl TestTerminal {
    /// Creates a new test terminal with an empty output buffer.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter_lib::testing::TestTerminal;
    ///
    /// let terminal = TestTerminal::new();
    /// ```
    pub fn new() -> Self {
        Self {
            output: Arc::new(Mutex::new(String::new())),
        }
    }

    /// Runs test code and captures its output.
    ///
    /// The provided closure should print to stdout/stderr. The terminal will
    /// capture all output for later verification.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter_lib::testing::TestTerminal;
    ///
    /// let mut terminal = TestTerminal::new();
    /// terminal.run(|term| {
    ///     term.push_str("Hello, world!\n");
    /// });
    /// terminal.assert_output("Hello, world!\n");
    /// ```
    ///
    /// ## Notes
    ///
    /// This is a simplified implementation for Phase 2. Future enhancements
    /// could include actual stdout/stderr capture using system-level redirection.
    pub fn run<F>(&self, test_code: F)
    where
        F: FnOnce(&mut String),
    {
        let mut buffer = self.output.lock().unwrap();
        buffer.clear();
        test_code(&mut buffer);
    }

    /// Asserts that the captured output matches the expected text (with ANSI codes stripped).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter_lib::testing::TestTerminal;
    ///
    /// let terminal = TestTerminal::new();
    /// terminal.run(|buf| {
    ///     buf.push_str("\x1b[31mError\x1b[0m: File not found");
    /// });
    /// terminal.assert_output("Error: File not found");
    /// ```
    ///
    /// ## Panics
    ///
    /// Panics if the stripped output does not match the expected text.
    pub fn assert_output(&self, expected: &str) {
        let output = self.output.lock().unwrap();
        let stripped = strip_ansi_codes(&output);
        assert_eq!(
            stripped, expected,
            "Terminal output mismatch:\nExpected: {:?}\nActual:   {:?}",
            expected, stripped
        );
    }

    /// Asserts that the captured output contains a specific ANSI escape code.
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter_lib::testing::TestTerminal;
    ///
    /// let terminal = TestTerminal::new();
    /// terminal.run(|buf| {
    ///     buf.push_str("\x1b[1;31mBold Red\x1b[0m");
    /// });
    /// terminal.assert_has_color("\x1b[1;31m"); // Bold + Red
    /// terminal.assert_has_color("\x1b[0m");    // Reset
    /// ```
    ///
    /// ## Panics
    ///
    /// Panics if the ANSI code is not found in the output.
    pub fn assert_has_color(&self, ansi_code: &str) {
        let output = self.output.lock().unwrap();
        assert!(
            output.contains(ansi_code),
            "ANSI code {:?} not found in output:\n{:?}",
            ansi_code,
            *output
        );
    }

    /// Returns the raw captured output (including ANSI codes).
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use darkmatter_lib::testing::TestTerminal;
    ///
    /// let terminal = TestTerminal::new();
    /// terminal.run(|buf| {
    ///     buf.push_str("\x1b[32mOK\x1b[0m");
    /// });
    /// assert!(terminal.get_output().contains("\x1b[32m"));
    /// ```
    pub fn get_output(&self) -> String {
        self.output.lock().unwrap().clone()
    }
}

impl Default for TestTerminal {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_ansi_codes_removes_sgr_sequences() {
        let input = "\x1b[31mRed\x1b[0m \x1b[1mBold\x1b[22m \x1b[4mUnderline\x1b[24m";
        assert_eq!(strip_ansi_codes(input), "Red Bold Underline");
    }

    #[test]
    fn test_strip_ansi_codes_removes_complex_sequences() {
        let input = "\x1b[1;31;4mBold Red Underlined\x1b[0m";
        assert_eq!(strip_ansi_codes(input), "Bold Red Underlined");
    }

    #[test]
    fn test_strip_ansi_codes_preserves_plain_text() {
        let input = "No ANSI codes here!";
        assert_eq!(strip_ansi_codes(input), input);
    }

    #[test]
    fn test_terminal_captures_output() {
        let terminal = TestTerminal::new();
        terminal.run(|buf| {
            buf.push_str("Hello, terminal!");
        });
        terminal.assert_output("Hello, terminal!");
    }

    #[test]
    fn test_terminal_strips_ansi_in_assertion() {
        let terminal = TestTerminal::new();
        terminal.run(|buf| {
            buf.push_str("\x1b[32mSuccess!\x1b[0m");
        });
        terminal.assert_output("Success!");
    }

    #[test]
    fn test_terminal_detects_ansi_codes() {
        let terminal = TestTerminal::new();
        terminal.run(|buf| {
            buf.push_str("\x1b[1;31mBold Red\x1b[0m");
        });
        terminal.assert_has_color("\x1b[1;31m");
        terminal.assert_has_color("\x1b[0m");
    }

    #[test]
    #[should_panic(expected = "ANSI code")]
    fn test_terminal_panics_when_ansi_code_missing() {
        let terminal = TestTerminal::new();
        terminal.run(|buf| {
            buf.push_str("Plain text");
        });
        terminal.assert_has_color("\x1b[31m");
    }

    #[test]
    fn test_terminal_get_output_returns_raw() {
        let terminal = TestTerminal::new();
        terminal.run(|buf| {
            buf.push_str("\x1b[32mGreen\x1b[0m");
        });
        let output = terminal.get_output();
        assert_eq!(output, "\x1b[32mGreen\x1b[0m");
    }
}

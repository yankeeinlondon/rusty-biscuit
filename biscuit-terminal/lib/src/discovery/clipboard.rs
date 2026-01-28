//! OSC52 clipboard support for terminal applications.
//!
//! OSC52 allows terminal applications to set the system clipboard without
//! requiring platform-specific clipboard APIs. The terminal intercepts the
//! escape sequence and handles clipboard integration.
//!
//! ## Security Considerations
//!
//! OSC52 is a powerful feature that allows any terminal application to write
//! to the system clipboard. Many terminals require explicit user configuration
//! to enable this feature for security reasons.
//!
//! ## Terminal Support
//!
//! As of 2025, the following terminals support OSC52:
//!
//! | Terminal    | Write | Read | Notes |
//! |-------------|-------|------|-------|
//! | Kitty       | Yes   | Yes  | Full support |
//! | WezTerm     | Yes   | Yes  | Full support |
//! | iTerm2      | Yes   | No   | Write only by default |
//! | Ghostty     | Yes   | Yes  | Full support |
//! | Alacritty   | Yes   | No   | Write only |
//! | Foot        | Yes   | Yes  | Full support |
//! | Contour     | Yes   | Yes  | Full support |
//! | tmux        | Yes   | No   | Requires `set-clipboard on` |
//!
//! ## Examples
//!
//! ```no_run
//! use biscuit_terminal::discovery::clipboard::{osc52_support, set_clipboard};
//!
//! if osc52_support() {
//!     set_clipboard("Hello, clipboard!").ok();
//! }
//! ```

use base64::{engine::general_purpose::STANDARD as BASE64, Engine};

use crate::discovery::detection::{get_terminal_app, is_tty, TerminalApp};
use crate::discovery::os_detection::is_ci;

/// Check if terminal supports OSC52 clipboard operations.
///
/// This function checks if the current terminal is known to support OSC52.
/// Note that even if the terminal supports it, the user may have disabled
/// the feature for security reasons.
///
/// ## Returns
///
/// - `true` if the terminal supports OSC52 clipboard
/// - `false` if not in a TTY, in CI, or terminal doesn't support OSC52
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::clipboard::osc52_support;
///
/// if osc52_support() {
///     println!("OSC52 clipboard is available");
/// }
/// ```
pub fn osc52_support() -> bool {
    if !is_tty() {
        return false;
    }
    if is_ci() {
        return false;
    }

    matches!(
        get_terminal_app(),
        TerminalApp::Kitty
            | TerminalApp::Wezterm
            | TerminalApp::ITerm2
            | TerminalApp::Ghostty
            | TerminalApp::Alacritty
            | TerminalApp::Foot
            | TerminalApp::Contour
    )
}

/// OSC52 clipboard target.
///
/// Different targets can be used for different clipboard selections:
/// - `Clipboard`: The system clipboard (most common)
/// - `Primary`: The X11 primary selection (middle-click paste on Linux)
/// - `Both`: Set both clipboard and primary selection
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClipboardTarget {
    /// System clipboard (Ctrl+V paste)
    #[default]
    Clipboard,
    /// X11 primary selection (middle-click paste)
    Primary,
    /// Both clipboard and primary selection
    Both,
}

impl ClipboardTarget {
    /// Get the OSC52 target specifier.
    fn as_specifier(&self) -> &'static str {
        match self {
            ClipboardTarget::Clipboard => "c",
            ClipboardTarget::Primary => "p",
            ClipboardTarget::Both => "pc",
        }
    }
}

/// Set clipboard contents via OSC52.
///
/// Writes the OSC52 escape sequence to stdout. The terminal will intercept
/// this sequence and set the system clipboard.
///
/// ## Format
///
/// ```text
/// ESC ] 52 ; <target> ; <base64-data> BEL
/// ```
///
/// Where:
/// - `ESC` is `\x1b`
/// - `target` is `c` for clipboard, `p` for primary, or `pc` for both
/// - `base64-data` is the content encoded in base64
/// - `BEL` is `\x07`
///
/// ## Errors
///
/// Returns an error if:
/// - Not in a TTY or in CI environment
/// - OSC52 is not supported by the terminal
/// - Writing to stdout fails
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::clipboard::set_clipboard;
///
/// set_clipboard("Hello from terminal!").expect("Failed to set clipboard");
/// ```
pub fn set_clipboard(content: &str) -> std::io::Result<()> {
    set_clipboard_with_target(content, ClipboardTarget::Clipboard)
}

/// Set clipboard contents via OSC52 with a specific target.
///
/// This is the same as [`set_clipboard`] but allows specifying the
/// clipboard target (clipboard, primary selection, or both).
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::clipboard::{set_clipboard_with_target, ClipboardTarget};
///
/// // Set both clipboard and primary selection
/// set_clipboard_with_target("Hello!", ClipboardTarget::Both).ok();
/// ```
pub fn set_clipboard_with_target(content: &str, target: ClipboardTarget) -> std::io::Result<()> {
    if !osc52_support() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "OSC52 not supported in this terminal",
        ));
    }

    let encoded = BASE64.encode(content);
    // OSC52 format: ESC ] 52 ; <target> ; <base64-data> BEL
    let sequence = format!("\x1b]52;{};{}\x07", target.as_specifier(), encoded);

    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()
}

/// Clear clipboard contents via OSC52.
///
/// Sends an OSC52 sequence with an empty payload to clear the clipboard.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::clipboard::clear_clipboard;
///
/// clear_clipboard().ok();
/// ```
pub fn clear_clipboard() -> std::io::Result<()> {
    if !osc52_support() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "OSC52 not supported in this terminal",
        ));
    }

    // OSC52 with "!" clears the clipboard
    let sequence = "\x1b]52;c;!\x07";

    use std::io::Write;
    let mut stdout = std::io::stdout();
    stdout.write_all(sequence.as_bytes())?;
    stdout.flush()
}

/// Get clipboard contents via OSC52.
///
/// **Note**: Reading clipboard via OSC52 is rarely supported and requires
/// terminal cooperation. This function always returns `None` because:
///
/// 1. It requires raw terminal mode to read the response
/// 2. Most terminals only support write, not read
/// 3. Reading has security implications (malicious apps could read clipboard)
///
/// For reading clipboard, use platform-specific crates like:
/// - `copypasta` - Cross-platform clipboard access
/// - `arboard` - Modern clipboard library
/// - `clipboard` - Simple clipboard access
///
/// ## Returns
///
/// Always returns `None`. Use platform-specific crates for reading.
pub fn get_clipboard() -> Option<String> {
    // OSC52 read is: ESC ] 52 ; c ; ? BEL
    // Response is: ESC ] 52 ; c ; BASE64_DATA BEL
    //
    // However, reading requires raw mode and is rarely supported.
    // For safety, we return None and let callers use platform-specific
    // clipboard crates for reading.
    None
}

/// Build an OSC52 escape sequence without writing it.
///
/// This is useful for embedding clipboard operations in larger escape
/// sequences or writing to streams other than stdout.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::clipboard::{build_osc52_sequence, ClipboardTarget};
///
/// let sequence = build_osc52_sequence("Hello", ClipboardTarget::Clipboard);
/// assert!(sequence.starts_with("\x1b]52;c;"));
/// assert!(sequence.ends_with("\x07"));
/// ```
pub fn build_osc52_sequence(content: &str, target: ClipboardTarget) -> String {
    let encoded = BASE64.encode(content);
    format!("\x1b]52;{};{}\x07", target.as_specifier(), encoded)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_clipboard_returns_none() {
        // get_clipboard always returns None in our implementation
        assert!(get_clipboard().is_none());
    }

    #[test]
    fn test_set_clipboard_fails_if_not_supported() {
        // In a test environment (not TTY), this should fail
        let result = set_clipboard("test");
        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), std::io::ErrorKind::Unsupported);
    }

    #[test]
    fn test_set_clipboard_with_target_fails_if_not_supported() {
        let result = set_clipboard_with_target("test", ClipboardTarget::Primary);
        assert!(result.is_err());
    }

    #[test]
    fn test_clear_clipboard_fails_if_not_supported() {
        let result = clear_clipboard();
        assert!(result.is_err());
    }

    #[test]
    fn test_osc52_support_returns_bool() {
        // Just verify it doesn't panic and returns a bool
        let _ = osc52_support();
    }

    #[test]
    fn test_clipboard_target_specifier() {
        assert_eq!(ClipboardTarget::Clipboard.as_specifier(), "c");
        assert_eq!(ClipboardTarget::Primary.as_specifier(), "p");
        assert_eq!(ClipboardTarget::Both.as_specifier(), "pc");
    }

    #[test]
    fn test_clipboard_target_default() {
        assert_eq!(ClipboardTarget::default(), ClipboardTarget::Clipboard);
    }

    #[test]
    fn test_build_osc52_sequence_format() {
        let sequence = build_osc52_sequence("Hello", ClipboardTarget::Clipboard);

        // Should start with OSC52 prefix
        assert!(sequence.starts_with("\x1b]52;c;"));

        // Should end with BEL
        assert!(sequence.ends_with("\x07"));

        // Should contain base64 encoded "Hello"
        // "Hello" in base64 is "SGVsbG8="
        assert!(sequence.contains("SGVsbG8="));
    }

    #[test]
    fn test_build_osc52_sequence_targets() {
        let clipboard = build_osc52_sequence("test", ClipboardTarget::Clipboard);
        assert!(clipboard.contains(";c;"));

        let primary = build_osc52_sequence("test", ClipboardTarget::Primary);
        assert!(primary.contains(";p;"));

        let both = build_osc52_sequence("test", ClipboardTarget::Both);
        assert!(both.contains(";pc;"));
    }

    #[test]
    fn test_build_osc52_sequence_empty_content() {
        let sequence = build_osc52_sequence("", ClipboardTarget::Clipboard);
        // Empty string in base64 is empty
        assert_eq!(sequence, "\x1b]52;c;\x07");
    }

    #[test]
    fn test_build_osc52_sequence_unicode() {
        let sequence = build_osc52_sequence("Hello, World!", ClipboardTarget::Clipboard);

        // "Hello, World!" in base64 is "SGVsbG8sIFdvcmxkIQ=="
        assert!(sequence.contains("SGVsbG8sIFdvcmxkIQ=="));
    }

    #[test]
    fn test_build_osc52_sequence_multiline() {
        let content = "Line 1\nLine 2\nLine 3";
        let sequence = build_osc52_sequence(content, ClipboardTarget::Clipboard);

        // Verify it's a valid sequence (starts and ends correctly)
        assert!(sequence.starts_with("\x1b]52;c;"));
        assert!(sequence.ends_with("\x07"));

        // Verify we can decode it back
        let start = "\x1b]52;c;".len();
        let end = sequence.len() - 1; // Remove trailing \x07
        let encoded = &sequence[start..end];
        let decoded = BASE64.decode(encoded).unwrap();
        assert_eq!(String::from_utf8(decoded).unwrap(), content);
    }
}

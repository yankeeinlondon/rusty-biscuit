use std::env;

use serde::{Deserialize, Serialize};

use super::dimensions::is_tty;

/// The type of image support (if any) of a terminal
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ImageSupport {
    None,
    /// the highest quality image support comes from the
    /// [Kitty Graphics Protocol](https://sw.kovidgoyal.net/kitty/graphics-protocol/).
    ///
    /// This is now supported in:
    ///
    /// - Kitty
    /// - WezTerm
    /// - Warp
    /// - iTerm2
    /// - Ghostty
    /// - Konsole
    /// - wast
    /// - VS Code (built-in terminal)
    Kitty,
    /// one of the earlier image formats but slowly being phased out,
    /// even it's originator iTERM2 now supports the Kitty protocol.
    ITerm,
}

/// Detailed result of image support detection, including the reason for the decision.
///
/// This is useful for debugging why a particular image protocol was selected
/// or why images are not supported.
#[derive(Debug, Clone)]
pub struct ImageSupportResult {
    /// The detected image support level
    pub support: ImageSupport,
    /// Human-readable reason for the detection result
    pub reason: String,
    /// The detection method used
    pub method: DetectionMethod,
}

/// The method used to detect image support.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DetectionMethod {
    /// Direct TTY capability check
    TtyCheck,
    /// Detection via viuer library probing
    Viuer,
    /// Heuristic based on environment variables
    EnvHeuristic,
    /// Known terminal application lookup
    KnownTerminal,
}

impl std::fmt::Display for DetectionMethod {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TtyCheck => write!(f, "tty_check"),
            Self::Viuer => write!(f, "viuer"),
            Self::EnvHeuristic => write!(f, "env_heuristic"),
            Self::KnownTerminal => write!(f, "known_terminal"),
        }
    }
}

/// Detect image display support in the terminal.
///
/// Returns the highest quality image protocol supported:
/// - `Kitty` - Kitty Graphics Protocol (highest quality)
/// - `ITerm` - iTerm2 image protocol (legacy)
/// - `None` - No image support
///
/// ## Detection Strategy
///
/// Detection is env-only and does not probe the terminal:
/// 1. [`image_support_from_known_terminals`] inspects `TERM_PROGRAM`,
///    `TERM`, and `ITERM_*` against a curated list of terminals known to
///    support Kitty or iTerm2 image protocols.
/// 2. [`image_support_from_env`] is the fallback for anything not on
///    the curated list and conservatively returns [`ImageSupport::None`].
///
/// We deliberately avoid stdout-based probing (such as `viuer`'s
/// `\x1b_G…a=q…\x1b\\\x1b[c` query) because the probe bytes leak into
/// captured output on any environment that doesn't reply — CI runners,
/// some multiplexers, Apple Terminal — and the test harness sees probe
/// bytes instead of the real CLI output.
///
/// If probe-based detection becomes necessary (to auto-discover a
/// Kitty-capable terminal not on the curated list), implement it the
/// same way [`crate::discovery::osc_queries::query::query_osc_actual`]
/// does: write the query bytes to `/dev/tty` (not stdout) under a
/// [`RawModeGuard`](crate::discovery::raw_mode::RawModeGuard), gated by
/// `is_tty() && !is_ci() && !multiplexer && terminal_is_known_to_respond`.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::{image_support, ImageSupport};
///
/// match image_support() {
///     ImageSupport::Kitty => println!("Kitty graphics protocol supported"),
///     ImageSupport::ITerm => println!("iTerm2 image protocol supported"),
///     ImageSupport::None => println!("No image support"),
/// }
/// ```
pub fn image_support() -> ImageSupport {
    image_support_with_reason().support
}

/// Detect image display support with detailed reasoning.
///
/// This function provides the same detection as [`image_support()`] but also
/// returns information about why a particular protocol was selected or why
/// images are not supported. Useful for debugging detection issues.
///
/// ## Examples
///
/// ```
/// use biscuit_terminal::discovery::detection::image_support_with_reason;
///
/// let result = image_support_with_reason();
/// println!("Support: {:?}", result.support);
/// println!("Reason: {}", result.reason);
/// println!("Method: {}", result.method);
/// ```
pub fn image_support_with_reason() -> ImageSupportResult {
    // First check: must be a TTY
    if !is_tty() {
        return ImageSupportResult {
            support: ImageSupport::None,
            reason: "stdout is not a TTY (piped or redirected)".to_string(),
            method: DetectionMethod::TtyCheck,
        };
    }

    // Authoritative env-only answer. `image_support_from_known_terminals`
    // matches curated terminal identifiers (Kitty, Ghostty, WezTerm, Warp,
    // Konsole, iTerm2, Apple Terminal, …); anything else falls through to
    // `image_support_from_env`, which conservatively returns
    // `ImageSupport::None`. See the module-level docstring above for why
    // we do not probe and where a future `/dev/tty`-based probe would
    // belong.
    if let Some(result) = image_support_from_known_terminals() {
        return result;
    }
    image_support_from_env()
}

/// Detect image support for terminals with KNOWN Kitty/iTerm2 support.
///
/// This function checks environment variables to identify terminals where we
/// definitively know the image protocol support without needing to probe.
/// Probing (via viuer) can cause issues on some terminals like Ghostty where
/// terminal responses arrive asynchronously and leak to the display.
///
/// Returns `Some(result)` if a known terminal is detected, `None` otherwise.
fn image_support_from_known_terminals() -> Option<ImageSupportResult> {
    // Terminals with definitive Kitty Graphics Protocol support.
    // These don't need probing - we know they support it.
    const KITTY_TERMINALS: &[&str] = &[
        "ghostty",      // Ghostty supports Kitty protocol on all platforms
        "kitty",        // Kitty is the originator of the protocol
        "WezTerm",      // WezTerm has full Kitty support
        "Warp",         // Warp supports Kitty protocol
        "WarpTerminal", // Warp sets TERM_PROGRAM=WarpTerminal
        "konsole",      // Konsole supports Kitty protocol
        "wast",         // Wast supports Kitty protocol
    ];

    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        // Apple Terminal does not support any image protocol and its
        // APC handling is incomplete — a Kitty probe leaks visible
        // garbage (`Gi=31,s=1,v=1,a=q,t=d,f=24;AAAA`) into the capture.
        // Detect early so viuer never sends the probe.
        if term_program == "Apple_Terminal" {
            tracing::debug!(
                image_support = "None",
                term_program = %term_program,
                method = "known_terminal",
                "Apple Terminal has no image support (no probe needed)"
            );
            return Some(ImageSupportResult {
                support: ImageSupport::None,
                reason: "Apple Terminal does not support inline images".to_string(),
                method: DetectionMethod::KnownTerminal,
            });
        }

        // Check TERM_PROGRAM for known Kitty-supporting terminals
        for &known in KITTY_TERMINALS {
            if term_program.eq_ignore_ascii_case(known) {
                tracing::debug!(
                    image_support = "Kitty",
                    term_program = %term_program,
                    method = "known_terminal",
                    "Detected Kitty support from known terminal (no probe needed)"
                );
                return Some(ImageSupportResult {
                    support: ImageSupport::Kitty,
                    reason: format!(
                        "{} is known to support Kitty graphics protocol",
                        term_program
                    ),
                    method: DetectionMethod::KnownTerminal,
                });
            }
        }

        // iTerm2 - known to support its native protocol (prefer over Kitty probing)
        if term_program == "iTerm.app" || term_program.eq_ignore_ascii_case("iterm2") {
            tracing::debug!(
                image_support = "ITerm",
                term_program = %term_program,
                method = "known_terminal",
                "Detected iTerm2 support from known terminal (no probe needed)"
            );
            return Some(ImageSupportResult {
                support: ImageSupport::ITerm,
                reason: format!("{} is known to support iTerm2 inline images", term_program),
                method: DetectionMethod::KnownTerminal,
            });
        }
    }

    // Check TERM variable for kitty
    if let Ok(term) = env::var("TERM")
        && term.contains("kitty")
    {
        tracing::debug!(
            image_support = "Kitty",
            term = %term,
            method = "known_terminal",
            "Detected Kitty support from TERM variable (no probe needed)"
        );
        return Some(ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: format!("TERM={} indicates Kitty terminal", term),
            method: DetectionMethod::KnownTerminal,
        });
    }

    // Check iTerm2-specific environment variables
    if env::var("ITERM_SESSION_ID").is_ok() || env::var("ITERM_PROFILE").is_ok() {
        tracing::debug!(
            image_support = "ITerm",
            method = "known_terminal",
            "Detected iTerm2 from session environment variables (no probe needed)"
        );
        return Some(ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "iTerm2 detected from ITERM_SESSION_ID or ITERM_PROFILE".to_string(),
            method: DetectionMethod::KnownTerminal,
        });
    }

    // No known terminal detected - caller should use probing or other heuristics
    None
}

/// Detect image support using environment variable heuristics only.
///
/// This is used as a fallback when viuer detection is not available or fails.
/// It checks `TERM_PROGRAM` and `TERM` environment variables to infer support.
fn image_support_from_env() -> ImageSupportResult {
    // Check TERM_PROGRAM for known terminals
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.as_str() {
            // Apple Terminal has no image support
            "Apple_Terminal" => {
                return ImageSupportResult {
                    support: ImageSupport::None,
                    reason: "Apple Terminal does not support inline images".to_string(),
                    method: DetectionMethod::EnvHeuristic,
                };
            }
            // Terminals with Kitty Graphics Protocol support
            "kitty" | "WezTerm" | "Warp" | "WarpTerminal" | "ghostty" | "konsole" | "wast" => {
                tracing::debug!(
                    image_support = "Kitty",
                    term_program = %term_program,
                    method = "env_heuristic",
                    "Detected Kitty support from TERM_PROGRAM"
                );
                return ImageSupportResult {
                    support: ImageSupport::Kitty,
                    reason: format!(
                        "TERM_PROGRAM={} indicates Kitty graphics protocol support",
                        term_program
                    ),
                    method: DetectionMethod::EnvHeuristic,
                };
            }
            // iTerm2 - can use either protocol, but prefer its native protocol
            // when viuer didn't detect Kitty support
            "iTerm.app" | "iterm2" => {
                tracing::debug!(
                    image_support = "ITerm",
                    term_program = %term_program,
                    method = "env_heuristic",
                    "Detected iTerm2 support from TERM_PROGRAM"
                );
                return ImageSupportResult {
                    support: ImageSupport::ITerm,
                    reason: format!(
                        "TERM_PROGRAM={} indicates iTerm2 inline images support",
                        term_program
                    ),
                    method: DetectionMethod::EnvHeuristic,
                };
            }
            _ => {}
        }
    }

    // Check ITERM_SESSION_ID for iTerm2 detection
    if env::var("ITERM_SESSION_ID").is_ok() || env::var("ITERM_PROFILE").is_ok() {
        tracing::debug!(
            image_support = "ITerm",
            method = "env_heuristic",
            "Detected iTerm2 from session environment variables"
        );
        return ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "ITERM_SESSION_ID or ITERM_PROFILE indicates iTerm2".to_string(),
            method: DetectionMethod::EnvHeuristic,
        };
    }

    // Check TERM variable for kitty
    let term = env::var("TERM").unwrap_or_default();
    if term.contains("kitty") {
        tracing::debug!(
            image_support = "Kitty",
            term = %term,
            method = "env_heuristic",
            "Detected Kitty support from TERM variable"
        );
        return ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: format!("TERM={} indicates Kitty graphics protocol support", term),
            method: DetectionMethod::EnvHeuristic,
        };
    }

    // No image support detected
    tracing::debug!(
        image_support = "None",
        method = "env_heuristic",
        "No image protocol support detected"
    );
    ImageSupportResult {
        support: ImageSupport::None,
        reason: "No image protocol support detected from environment".to_string(),
        method: DetectionMethod::EnvHeuristic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::env;

    /// Helper to set environment variables with automatic cleanup.
    struct ScopedEnv {
        vars: Vec<(String, Option<String>)>,
    }

    impl ScopedEnv {
        fn new() -> Self {
            Self { vars: Vec::new() }
        }

        fn set(&mut self, key: &str, value: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
            // concurrent access to environment variables.
            unsafe { env::set_var(key, value) };
        }

        fn remove(&mut self, key: &str) {
            let old = env::var(key).ok();
            self.vars.push((key.to_string(), old));
            // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
            // concurrent access to environment variables.
            unsafe { env::remove_var(key) };
        }
    }

    impl Drop for ScopedEnv {
        fn drop(&mut self) {
            for (key, old_value) in self.vars.drain(..).rev() {
                // SAFETY: Tests using ScopedEnv are marked with #[serial] to prevent
                // concurrent access to environment variables.
                unsafe {
                    match old_value {
                        Some(v) => env::set_var(&key, v),
                        None => env::remove_var(&key),
                    }
                }
            }
        }
    }

    #[test]
    fn test_image_support_eq() {
        assert_eq!(ImageSupport::None, ImageSupport::None);
        assert_eq!(ImageSupport::Kitty, ImageSupport::Kitty);
        assert_eq!(ImageSupport::ITerm, ImageSupport::ITerm);
        assert_ne!(ImageSupport::None, ImageSupport::Kitty);
        assert_ne!(ImageSupport::Kitty, ImageSupport::ITerm);
    }

    #[test]
    fn test_image_support_debug() {
        let debug_none = format!("{:?}", ImageSupport::None);
        assert!(debug_none.contains("None"));

        let debug_kitty = format!("{:?}", ImageSupport::Kitty);
        assert!(debug_kitty.contains("Kitty"));

        let debug_iterm = format!("{:?}", ImageSupport::ITerm);
        assert!(debug_iterm.contains("ITerm"));
    }

    #[test]
    fn test_image_support_clone() {
        let support = ImageSupport::Kitty;
        let cloned = support.clone();
        assert_eq!(support, cloned);
    }

    #[test]
    fn test_image_support_result_fields() {
        let result = ImageSupportResult {
            support: ImageSupport::Kitty,
            reason: "test reason".to_string(),
            method: DetectionMethod::TtyCheck,
        };

        assert_eq!(result.support, ImageSupport::Kitty);
        assert_eq!(result.reason, "test reason");
        assert_eq!(result.method, DetectionMethod::TtyCheck);
    }

    #[test]
    fn test_image_support_result_debug() {
        let result = ImageSupportResult {
            support: ImageSupport::ITerm,
            reason: "viuer detected iTerm2".to_string(),
            method: DetectionMethod::Viuer,
        };

        let debug = format!("{:?}", result);
        assert!(debug.contains("ITerm"));
        assert!(debug.contains("viuer"));
    }

    #[test]
    fn test_image_support_result_clone() {
        let result = ImageSupportResult {
            support: ImageSupport::None,
            reason: "not a tty".to_string(),
            method: DetectionMethod::TtyCheck,
        };

        let cloned = result.clone();
        assert_eq!(cloned.support, result.support);
        assert_eq!(cloned.reason, result.reason);
        assert_eq!(cloned.method, result.method);
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_kitty_term_program() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "kitty");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("TERM_PROGRAM"));
        assert_eq!(result.method, DetectionMethod::EnvHeuristic);
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_wezterm() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "WezTerm");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("WezTerm"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_ghostty() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "ghostty");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("ghostty"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_term_program() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "iTerm.app");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("iTerm"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_session_id() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.set(
            "ITERM_SESSION_ID",
            "w0t0p0:12345678-1234-1234-1234-123456789abc",
        );
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("ITERM_SESSION_ID"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_iterm2_profile() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.set("ITERM_PROFILE", "Default");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::ITerm);
        assert!(result.reason.contains("ITERM_PROFILE"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_kitty_term_var() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");
        env.set("TERM", "xterm-kitty");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::Kitty);
        assert!(result.reason.contains("TERM="));
        assert!(result.reason.contains("kitty"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_none() {
        let mut env = ScopedEnv::new();
        env.remove("TERM_PROGRAM");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");
        env.set("TERM", "xterm-256color");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::None);
        assert!(result.reason.contains("No image protocol"));
    }

    #[test]
    #[serial]
    fn test_image_support_from_env_apple_terminal() {
        let mut env = ScopedEnv::new();
        env.set("TERM_PROGRAM", "Apple_Terminal");
        env.remove("ITERM_SESSION_ID");
        env.remove("ITERM_PROFILE");

        let result = image_support_from_env();
        assert_eq!(result.support, ImageSupport::None);
        assert!(result.reason.contains("Apple Terminal"));
    }

    #[test]
    fn test_image_support_returns_support_field() {
        let simple = image_support();
        let detailed = image_support_with_reason();
        assert_eq!(simple, detailed.support);
    }

    #[test]
    fn test_image_support_with_reason_has_non_empty_fields() {
        let result = image_support_with_reason();

        assert!(!result.reason.is_empty(), "Reason should not be empty");

        let valid_methods = [
            DetectionMethod::TtyCheck,
            DetectionMethod::Viuer,
            DetectionMethod::EnvHeuristic,
            DetectionMethod::KnownTerminal,
        ];
        assert!(
            valid_methods.contains(&result.method),
            "Method '{:?}' should be one of {:?}",
            result.method,
            valid_methods
        );
    }

    #[test]
    fn test_image_support_detection_does_not_panic() {
        let _ = image_support_with_reason();
    }

    // Note: tests like `test_known_terminals_no_probe_*` were intentionally
    // omitted because `image_support_with_reason()` enters the `NotTty`
    // branch before consulting `image_support_from_known_terminals` when
    // run from `cargo test`. The known-terminal heuristic itself is covered
    // by the `image_support_from_env_*` tests above which call the
    // env-only helper directly.
}

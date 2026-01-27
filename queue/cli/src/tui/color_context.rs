//! Color context management for NO_COLOR support.
//!
//! This module provides `ColorContext`, which detects the `NO_COLOR` environment
//! variable and provides color-aware symbol selection for task status display.
//!
//! ## NO_COLOR Specification
//!
//! Per [no-color.org](https://no-color.org), color is disabled when the `NO_COLOR`
//! environment variable exists and is not empty. This means:
//!
//! - `NO_COLOR` unset: color enabled
//! - `NO_COLOR=""`: color enabled (empty string)
//! - `NO_COLOR="1"`: color disabled
//! - `NO_COLOR="0"`: color disabled (any non-empty value disables)
//! - `NO_COLOR="yes"`: color disabled

use queue_lib::TaskStatus;
use std::env;

/// Context for color-aware rendering decisions.
///
/// Created once at startup and passed through to render functions.
/// Detects the `NO_COLOR` environment variable per the no-color.org spec.
#[derive(Debug, Clone, Copy)]
pub struct ColorContext {
    color_enabled: bool,
}

impl ColorContext {
    /// Creates a new `ColorContext` by detecting the `NO_COLOR` environment variable.
    ///
    /// ## Examples
    ///
    /// ```
    /// use queue_cli::tui::ColorContext;
    ///
    /// let ctx = ColorContext::new();
    /// // Color is enabled unless NO_COLOR is set to a non-empty value
    /// ```
    pub fn new() -> Self {
        Self {
            color_enabled: Self::detect_color_enabled(),
        }
    }

    /// Creates a `ColorContext` with color explicitly enabled.
    ///
    /// Useful for testing or when you want to force color output.
    #[cfg(test)]
    pub fn with_color() -> Self {
        Self {
            color_enabled: true,
        }
    }

    /// Creates a `ColorContext` with color explicitly disabled.
    ///
    /// Useful for testing or when you want to force plain text output.
    #[cfg(test)]
    pub fn without_color() -> Self {
        Self {
            color_enabled: false,
        }
    }

    /// Returns `true` if color output is enabled.
    ///
    /// Color is disabled when the `NO_COLOR` environment variable exists
    /// and contains a non-empty value.
    #[cfg(test)]
    pub fn is_color_enabled(&self) -> bool {
        self.color_enabled
    }

    /// Returns the appropriate status symbol for the given task status.
    ///
    /// When color is enabled, returns Unicode symbols:
    /// - Completed: "✓"
    /// - Cancelled: "×"
    /// - Failed: "✗"
    /// - Running: "▶"
    /// - Pending: "○"
    ///
    /// When NO_COLOR is set, returns ASCII fallbacks:
    /// - Completed: "[OK]"
    /// - Cancelled: "[--]"
    /// - Failed: "[FAIL]"
    /// - Running: "[RUN]"
    /// - Pending: "[..]"
    pub fn status_symbol(&self, status: &TaskStatus) -> &'static str {
        if self.color_enabled {
            match status {
                TaskStatus::Completed => "\u{2713}",   // ✓
                TaskStatus::Cancelled => "\u{00d7}",   // ×
                TaskStatus::Failed { .. } => "\u{2717}", // ✗
                TaskStatus::Running => "\u{25b6}",     // ▶
                TaskStatus::Pending => "\u{25cb}",     // ○
            }
        } else {
            match status {
                TaskStatus::Completed => "[OK]",
                TaskStatus::Cancelled => "[--]",
                TaskStatus::Failed { .. } => "[FAIL]",
                TaskStatus::Running => "[RUN]",
                TaskStatus::Pending => "[..]",
            }
        }
    }

    /// Detects whether color is enabled based on NO_COLOR environment variable.
    ///
    /// Per the no-color.org spec:
    /// - If NO_COLOR is unset, color is enabled
    /// - If NO_COLOR is set but empty, color is enabled
    /// - If NO_COLOR is set to any non-empty value, color is disabled
    fn detect_color_enabled() -> bool {
        match env::var("NO_COLOR") {
            Ok(value) => value.is_empty(),
            Err(env::VarError::NotPresent) => true,
            Err(env::VarError::NotUnicode(_)) => false, // Treat invalid UTF-8 as "set"
        }
    }
}

impl Default for ColorContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Mutex to serialize tests that modify environment variables
    static ENV_LOCK: Mutex<()> = Mutex::new(());

    /// Helper to run a test with a specific NO_COLOR value, then restore.
    ///
    /// ## Safety
    ///
    /// This function modifies environment variables which is unsafe in Rust 2024
    /// edition due to potential data races. The ENV_LOCK mutex serializes all
    /// tests that modify environment variables.
    fn with_no_color<F, R>(value: Option<&str>, f: F) -> R
    where
        F: FnOnce() -> R,
    {
        let _guard = ENV_LOCK.lock().unwrap();
        let original = env::var("NO_COLOR").ok();

        // SAFETY: Tests are serialized by ENV_LOCK mutex, preventing data races.
        // Environment variable modifications are isolated to test scope.
        unsafe {
            match value {
                Some(v) => env::set_var("NO_COLOR", v),
                None => env::remove_var("NO_COLOR"),
            }
        }

        let result = f();

        // Restore original value
        // SAFETY: Same as above - serialized by mutex
        unsafe {
            match original {
                Some(v) => env::set_var("NO_COLOR", v),
                None => env::remove_var("NO_COLOR"),
            }
        }

        result
    }

    #[test]
    fn color_enabled_when_no_color_unset() {
        with_no_color(None, || {
            let ctx = ColorContext::new();
            assert!(
                ctx.is_color_enabled(),
                "Color should be enabled when NO_COLOR is unset"
            );
        });
    }

    #[test]
    fn color_disabled_when_no_color_set_to_1() {
        with_no_color(Some("1"), || {
            let ctx = ColorContext::new();
            assert!(
                !ctx.is_color_enabled(),
                "Color should be disabled when NO_COLOR=1"
            );
        });
    }

    #[test]
    fn color_disabled_when_no_color_set_to_any_value() {
        with_no_color(Some("yes"), || {
            let ctx = ColorContext::new();
            assert!(
                !ctx.is_color_enabled(),
                "Color should be disabled when NO_COLOR=yes"
            );
        });
    }

    #[test]
    fn color_enabled_when_no_color_is_empty() {
        with_no_color(Some(""), || {
            let ctx = ColorContext::new();
            assert!(
                ctx.is_color_enabled(),
                "Color should be enabled when NO_COLOR is empty (per spec)"
            );
        });
    }

    #[test]
    fn color_disabled_when_no_color_set_to_0() {
        // Per spec, NO_COLOR=0 still disables color (any non-empty value)
        with_no_color(Some("0"), || {
            let ctx = ColorContext::new();
            assert!(
                !ctx.is_color_enabled(),
                "Color should be disabled when NO_COLOR=0 (any non-empty value disables)"
            );
        });
    }

    #[test]
    fn status_symbol_returns_unicode_when_color_enabled() {
        let ctx = ColorContext::with_color();

        assert_eq!(ctx.status_symbol(&TaskStatus::Completed), "\u{2713}"); // ✓
        assert_eq!(ctx.status_symbol(&TaskStatus::Cancelled), "\u{00d7}"); // ×
        assert_eq!(
            ctx.status_symbol(&TaskStatus::Failed {
                error: "test".to_string()
            }),
            "\u{2717}"
        ); // ✗
        assert_eq!(ctx.status_symbol(&TaskStatus::Running), "\u{25b6}"); // ▶
        assert_eq!(ctx.status_symbol(&TaskStatus::Pending), "\u{25cb}"); // ○
    }

    #[test]
    fn status_symbol_returns_ascii_when_no_color() {
        let ctx = ColorContext::without_color();

        assert_eq!(ctx.status_symbol(&TaskStatus::Completed), "[OK]");
        assert_eq!(ctx.status_symbol(&TaskStatus::Cancelled), "[--]");
        assert_eq!(
            ctx.status_symbol(&TaskStatus::Failed {
                error: "test".to_string()
            }),
            "[FAIL]"
        );
        assert_eq!(ctx.status_symbol(&TaskStatus::Running), "[RUN]");
        assert_eq!(ctx.status_symbol(&TaskStatus::Pending), "[..]");
    }

    #[test]
    fn with_color_creates_color_enabled_context() {
        let ctx = ColorContext::with_color();
        assert!(ctx.is_color_enabled());
    }

    #[test]
    fn without_color_creates_color_disabled_context() {
        let ctx = ColorContext::without_color();
        assert!(!ctx.is_color_enabled());
    }

    #[test]
    fn default_impl_uses_new() {
        // Just verify default() doesn't panic and behaves like new()
        let ctx = ColorContext::default();
        // We can't assert the exact value since it depends on env,
        // but we can verify it returns a valid ColorContext
        let _ = ctx.is_color_enabled();
    }

    #[test]
    fn color_context_is_copy() {
        let ctx = ColorContext::with_color();
        let ctx2 = ctx; // Copy
        assert!(ctx.is_color_enabled());
        assert!(ctx2.is_color_enabled());
    }
}

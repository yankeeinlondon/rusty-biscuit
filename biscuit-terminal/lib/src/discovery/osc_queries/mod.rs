//! OSC (Operating System Command) queries for terminal colors.
//!
//! This module provides terminal color detection using a **hybrid approach**:
//! actual OSC queries when supported, with heuristic fallbacks for reliability.
//!
//! ## Strategy
//!
//! Color detection uses a cascading fallback chain:
//!
//! 1. **Actual OSC query** (Unix only, supported terminals) - Most accurate,
//!    sends escape sequence and parses response with timeout handling
//! 2. **`COLORFGBG` environment variable** - Set by some terminals with
//!    foreground/background color indices
//! 3. **Terminal application defaults** - Known default colors for detected
//!    terminal emulators (Kitty, iTerm2, etc.)
//!
//! ## Supported Terminals (Actual Queries)
//!
//! The following terminals support actual OSC 10/11/12 queries:
//! - Kitty
//! - WezTerm
//! - iTerm2
//! - Alacritty
//! - Ghostty
//! - Foot
//! - Contour
//!
//! ## Limitations
//!
//! - **Multiplexers**: tmux, Zellij, and GNU Screen may not pass through
//!   OSC queries correctly. When detected, we fall back to heuristics.
//! - **CI environments**: Actual queries are skipped in CI to avoid hangs.
//! - **Non-TTY**: When stdout is not a terminal, returns `None`.
//!
//! ## Submodules
//!
//! - [`types`] — `RgbValue`, `OscQueryError`, `DEFAULT_TIMEOUT`
//! - [`parse`] — `parse_osc_color_response` and helpers
//! - [`query`] — Hybrid query logic and Unix `query_osc_actual`
//! - [`support`] — Cached OSC support heuristics (`osc10_support`, etc.)
//!
//! ## Examples
//!
//! ### Basic Color Detection
//!
//! ```
//! use biscuit_terminal::discovery::osc_queries::{bg_color, text_color, RgbValue};
//!
//! if let Some(bg) = bg_color() {
//!     let luminance = bg.luminance();
//!     if luminance > 0.5 {
//!         println!("Light background detected");
//!     } else {
//!         println!("Dark background detected");
//!     }
//! }
//! ```

use std::sync::OnceLock;
use std::time::Duration;

pub mod parse;
pub mod query;
pub mod support;
pub mod types;

pub use parse::parse_osc_color_response;
pub use query::query_osc_actual;
pub use support::{osc10_support, osc11_support, osc12_support};
pub use types::{DEFAULT_TIMEOUT, OscQueryError, RgbValue};

static BG_COLOR_CACHE: OnceLock<Option<RgbValue>> = OnceLock::new();

/// Query background color via OSC 11 heuristics.
///
/// Returns `None` if:
/// - Not running in a TTY
/// - Running in a CI environment
/// - No color information is available
///
/// The result is cached per-process so repeated calls do not trigger
/// additional terminal queries.
///
/// ## Examples
///
/// ```no_run
/// use biscuit_terminal::discovery::osc_queries::bg_color;
///
/// if let Some(bg) = bg_color() {
///     println!("Background color: {}", bg);
/// }
/// ```
pub fn bg_color() -> Option<RgbValue> {
    *BG_COLOR_CACHE.get_or_init(|| query::query_osc_color(11))
}

/// Query foreground/text color via OSC 10 heuristics.
pub fn text_color() -> Option<RgbValue> {
    query::query_osc_color(10)
}

/// Query cursor color via OSC 12 heuristics.
///
/// Note: Cursor color often defaults to the same as foreground text color.
pub fn cursor_color() -> Option<RgbValue> {
    query::query_osc_color(12)
}

/// Query background color with a custom timeout.
pub fn bg_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query::query_osc_color_with_timeout(11, timeout)
}

/// Query foreground/text color with a custom timeout.
pub fn text_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query::query_osc_color_with_timeout(10, timeout)
}

/// Query cursor color with a custom timeout.
pub fn cursor_color_with_timeout(timeout: Duration) -> Option<RgbValue> {
    query::query_osc_color_with_timeout(12, timeout)
}

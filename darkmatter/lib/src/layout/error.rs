//! Errors produced by the [`DarkmatterPage`](super::DarkmatterPage) rendering
//! pipeline.

use thiserror::Error;

/// Errors that can be produced by [`DarkmatterPage`](super::DarkmatterPage)
/// configuration validation or rendering.
///
/// ## Examples
///
/// ```
/// use darkmatter::layout::PageRenderError;
///
/// let err = PageRenderError::MaxWidthZero;
/// assert_eq!(err.to_string(), "max_width must be greater than zero");
/// ```
#[derive(Debug, Error, PartialEq)]
pub enum PageRenderError {
    /// The combined horizontal margin + padding meets or exceeds the
    /// terminal width, leaving no room for content.
    #[error("margins ({required} cols) meet or exceed terminal width ({terminal_width} cols)")]
    MarginsExceedTerminalWidth {
        /// Width of the underlying terminal in columns.
        terminal_width: u16,
        /// Total required horizontal columns for margin + padding.
        required: u16,
    },

    /// A caller set `max_width = Some(0)` via the library API.
    ///
    /// The CLI rejects `--max-width 0` at parse time and never produces this
    /// variant, but the library API accepts arbitrary `u16` values and
    /// surfaces zero through this error.
    #[error("max_width must be greater than zero")]
    MaxWidthZero,

    /// A [`WidthUnit::Percent`](super::WidthUnit::Percent) value was outside
    /// the valid `0.0..=100.0` range.
    #[error("invalid percent value: {0} (expected 0.0..=100.0)")]
    InvalidPercent(f32),

    /// An underlying markdown render failure surfaced through page rendering.
    #[error("markdown render failed: {0}")]
    Render(String),
}

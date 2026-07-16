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

    /// A percentage value was outside the valid `0.0..=100.0` range.
    #[error("invalid percent value: {0} (expected 0.0..=100.0)")]
    InvalidPercent(f32),

    /// An underlying markdown render failure surfaced through page rendering.
    #[error("markdown render failed: {0}")]
    Render(String),

    /// A requested page feature could not be resolved or placed for the
    /// browser render. Carries the typed
    /// [`FeatureResolveError`](renderable::browser::feature::FeatureResolveError)
    /// so callers can match the underlying variant instead of parsing prose —
    /// e.g. a body-only render whose feature resolves to `<link>` dependencies
    /// fails with
    /// [`HeadRequired`](renderable::browser::feature::FeatureResolveError::HeadRequired).
    #[error(transparent)]
    FeatureResolution(#[from] renderable::browser::feature::FeatureResolveError),

    /// A list left-margin builder was called with a non-`Ul` component.
    ///
    /// Only [`PageComponent::Ul`](super::PageComponent::Ul) supports the
    /// independent left-margin channel in the current sub-spec.
    #[error("list left margin only accepts `PageComponent::Ul`")]
    InvalidListLeftMarginComponent,

    /// A `Length::Css(_)` value was set on `style.hyperlinks.*`,
    /// `style.hyperlinks.local-style.*`, or `style.images.local-style.*` for a
    /// field that affects terminal layout (`width` or `max-width`). HTML can
    /// represent CSS lengths inline, but terminal layout works in cells, so
    /// the CSS length cannot be honored on this target.
    #[error(
        "`style.{bucket}.{field}` uses a CSS length which is not supported for terminal layout"
    )]
    InvalidInlineCssLength {
        /// Source bucket in canonical kebab-case (`hyperlinks`,
        /// `hyperlinks.local-style`, or `images.local-style`).
        bucket: &'static str,
        /// Field name in canonical kebab-case (`width` or `max-width`).
        field: &'static str,
    },
}

impl From<crate::markdown::MarkdownError> for PageRenderError {
    fn from(err: crate::markdown::MarkdownError) -> Self {
        PageRenderError::Render(err.to_string())
    }
}

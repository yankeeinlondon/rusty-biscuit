//! When a Mermaid diagram that cannot be shown as an image may be drawn as
//! text instead of its source, and whether a partial drawing carries a note.
//!
//! Each setting resolves on its own: a value the caller set wins, then
//! [`MERMAID_FALLBACK_ENV`] (text fallback only), then the default. A CLI
//! turns its flags into set values, so a flag beats the variable.

/// The environment variable that turns the text fallback off when the caller
/// left it unset: `code-block`, compared case-insensitively, means
/// [`TextFallback::Never`]. Any other value is ignored.
pub const MERMAID_FALLBACK_ENV: &str = "MERMAID_FALLBACK";

/// Whether a diagram that cannot be an image may be drawn as text.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub enum TextFallback {
    /// Text whenever Merman can draw the diagram and it fits.
    #[default]
    Allowed,
    /// Text only when Merman draws every feature of the family; a partial
    /// drawing shows the source.
    NoPartial,
    /// Always the source.
    Never,
}

impl TextFallback {
    /// Whether a drawing of this coverage may replace the source.
    pub(crate) fn admits(self, coverage: crate::mermaid::TextCoverage) -> bool {
        match self {
            Self::Allowed => true,
            Self::NoPartial => coverage == crate::mermaid::TextCoverage::Full,
            Self::Never => false,
        }
    }
}

/// The fallback policy of a terminal Mermaid component or the Markdown hook.
///
/// A setting left unset resolves at render time; see the
/// [module docs](self).
///
/// ## Examples
///
/// ```
/// use biscuit_visualized::components::{MermaidFallbackPolicy, TextFallback};
///
/// let policy = MermaidFallbackPolicy::default().with_show_partial_note(false);
/// let resolved = policy.resolve_with(|_| Some("Code-Block".to_string()));
/// assert_eq!(resolved.text_fallback, TextFallback::Never);
/// assert!(!resolved.show_partial_note);
///
/// // A value the caller set ignores the environment.
/// let explicit = policy.with_text_fallback(TextFallback::Allowed);
/// let resolved = explicit.resolve_with(|_| Some("code-block".to_string()));
/// assert_eq!(resolved.text_fallback, TextFallback::Allowed);
/// ```
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct MermaidFallbackPolicy {
    text_fallback: Option<TextFallback>,
    show_partial_note: Option<bool>,
}

#[allow(missing_docs)]
impl MermaidFallbackPolicy {
    pub fn with_text_fallback(mut self, text_fallback: TextFallback) -> Self {
        self.text_fallback = Some(text_fallback);
        self
    }

    pub fn with_show_partial_note(mut self, show: bool) -> Self {
        self.show_partial_note = Some(show);
        self
    }

    pub fn text_fallback(&self) -> Option<TextFallback> {
        self.text_fallback
    }

    pub fn show_partial_note(&self) -> Option<bool> {
        self.show_partial_note
    }
}

impl MermaidFallbackPolicy {
    /// The policy for this process: unset settings read
    /// [`MERMAID_FALLBACK_ENV`] or take their default.
    pub fn resolve(&self) -> ResolvedFallbackPolicy {
        self.resolve_with(|name| std::env::var(name).ok())
    }

    /// [`resolve`](Self::resolve) with `env` standing in for the process
    /// environment; it is asked only for [`MERMAID_FALLBACK_ENV`], and only
    /// when the text-fallback setting is unset.
    pub fn resolve_with(&self, env: impl FnOnce(&str) -> Option<String>) -> ResolvedFallbackPolicy {
        let text_fallback = self.text_fallback.unwrap_or_else(|| {
            match env(MERMAID_FALLBACK_ENV) {
                Some(value) if value.eq_ignore_ascii_case("code-block") => TextFallback::Never,
                _ => TextFallback::default(),
            }
        });
        ResolvedFallbackPolicy {
            text_fallback,
            show_partial_note: self.show_partial_note.unwrap_or(true),
        }
    }
}

/// A [`MermaidFallbackPolicy`] with every setting decided.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ResolvedFallbackPolicy {
    /// Whether text may replace the source.
    pub text_fallback: TextFallback,
    /// Whether a partial drawing is followed by the partial note.
    pub show_partial_note: bool,
}

/// The `--feature` names a CLI accepts for the fallback policy.
///
/// [`MermaidFallbackPolicy::from_features`] folds a list of them into a
/// policy; a CLI that receives none leaves the policy unset, so
/// [`MERMAID_FALLBACK_ENV`] applies.
#[cfg(feature = "clap")]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, clap::ValueEnum)]
pub enum MermaidFallbackFeature {
    /// Never draw a diagram as text; show its source.
    NoMermaidFallbacks,
    /// Never draw a partial text diagram; show its source.
    NoMermaidPartialFallbacks,
    /// Hide the note that follows a partial text diagram.
    NoMermaidPartialWarning,
}

#[cfg(feature = "clap")]
impl MermaidFallbackPolicy {
    /// The policy `features` ask for: when two overlap, the more restrictive
    /// wins, and a repeated name changes nothing. Settings no feature names
    /// stay unset.
    ///
    /// ## Examples
    ///
    /// ```
    /// use biscuit_visualized::components::{MermaidFallbackFeature, MermaidFallbackPolicy, TextFallback};
    ///
    /// let policy = MermaidFallbackPolicy::from_features([
    ///     MermaidFallbackFeature::NoMermaidPartialFallbacks,
    ///     MermaidFallbackFeature::NoMermaidFallbacks,
    /// ]);
    /// assert_eq!(policy.text_fallback(), Some(TextFallback::Never));
    /// assert_eq!(policy.show_partial_note(), None);
    /// ```
    pub fn from_features(features: impl IntoIterator<Item = MermaidFallbackFeature>) -> Self {
        features.into_iter().fold(Self::default(), |policy, feature| match feature {
            MermaidFallbackFeature::NoMermaidFallbacks => policy.with_text_fallback(TextFallback::Never),
            MermaidFallbackFeature::NoMermaidPartialFallbacks
                if policy.text_fallback != Some(TextFallback::Never) =>
            {
                policy.with_text_fallback(TextFallback::NoPartial)
            }
            MermaidFallbackFeature::NoMermaidPartialFallbacks => policy,
            MermaidFallbackFeature::NoMermaidPartialWarning => policy.with_show_partial_note(false),
        })
    }
}

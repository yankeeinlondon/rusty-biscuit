use biscuit_visualized::components::MermaidFallbackFeature;
use clap::ValueEnum;

/// A named opt-in accepted by `md --feature` (the repository's `--feature`
/// convention: repeatable, comma-separated, case-insensitive).
///
/// Adding a feature is one variant here plus its lowering in
/// [`apply_features`](crate::render::apply_features).
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, ValueEnum)]
pub enum Feature {
    /// Embed the font static-SVG Mermaid diagrams were measured with in HTML
    /// output. Has no effect on Mermaid rendered any other way.
    EmbedMermaidFont,
    /// With `--mermaid`, never draw a diagram as text; show its source.
    NoMermaidFallbacks,
    /// With `--mermaid`, never draw a partial text diagram; show its source.
    NoMermaidPartialFallbacks,
    /// With `--mermaid`, hide the note that follows a partial text diagram.
    NoMermaidPartialWarning,
    /// Keep every Mermaid fence a code block in every output, overriding
    /// `--mermaid` and `embed-mermaid-font`.
    NoMermaidRendering,
}

impl Feature {
    /// The shared fallback-policy name this feature repeats, if any.
    ///
    /// `md` declares the three names itself, because a clap value enum cannot
    /// include another enum's values; biscuit-visualized owns their meaning
    /// and the fold that combines them.
    pub fn mermaid_fallback(self) -> Option<MermaidFallbackFeature> {
        match self {
            Self::NoMermaidFallbacks => Some(MermaidFallbackFeature::NoMermaidFallbacks),
            Self::NoMermaidPartialFallbacks => Some(MermaidFallbackFeature::NoMermaidPartialFallbacks),
            Self::NoMermaidPartialWarning => Some(MermaidFallbackFeature::NoMermaidPartialWarning),
            Self::EmbedMermaidFont | Self::NoMermaidRendering => None,
        }
    }
}

/// Cache-staleness handling for remote URL artifacts.
///
/// Mirrors `darkmatter::markdown::compose::RemoteFreshnessMode`. A closed value
/// set so an unrecognized `--remote-freshness` argument fails fast with the
/// accepted values rather than silently degrading to a single mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RemoteFreshness {
    /// Serve any cached body without revalidation, even when stale;
    /// `no-cache` responses are always revalidated.
    Optimistic,
    /// Serve within the freshness lifetime; past it, revalidate with a
    /// conditional GET and fail if revalidation fails.
    Strict,
    /// Like `strict`, but serve the stale body when revalidation fails on the
    /// network, except for `no-cache` responses (the default).
    Fallback,
}

/// Graph visualization format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum GraphFormat {
    /// Mermaid flowchart.
    Mermaid,
    /// DOT (Graphviz) graph.
    Dot,
}

impl GraphFormat {
    /// The value's spelling, which is also its code-block language.
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Mermaid => "mermaid",
            Self::Dot => "dot",
        }
    }
}

/// Structural kind for `md hash --kind`.
///
/// CLI-side mirror of [`darkmatter::markdown::hash::MdHashKind`]; the tokens
/// match the library's serde/`FromStr` vocabulary.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum HashKind {
    /// Frontmatter only (keys and values).
    Fm,
    /// Body content only.
    Body,
    /// `{fm}-{body}` — the default.
    Simple,
    /// `{fm}-{fm_keys}-{body}-{body_structure}`.
    Structured,
    /// Nested object with per-section detail.
    Detailed,
}

impl From<HashKind> for darkmatter::markdown::hash::MdHashKind {
    fn from(value: HashKind) -> Self {
        use darkmatter::markdown::hash::MdHashKind;
        match value {
            HashKind::Fm => MdHashKind::Fm,
            HashKind::Body => MdHashKind::Body,
            HashKind::Simple => MdHashKind::Simple,
            HashKind::Structured => MdHashKind::Structured,
            HashKind::Detailed => MdHashKind::Detailed,
        }
    }
}

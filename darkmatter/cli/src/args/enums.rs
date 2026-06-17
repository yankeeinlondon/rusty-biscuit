use clap::ValueEnum;

/// Output format for top-level render mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum OutputFormat {
    /// Auto: terminal rendering on TTY, markdown text on non-TTY.
    Auto,
    /// Markdown text output.
    #[value(alias = "text")]
    Markdown,
    /// Markdown output enhanced with disclosure blocks rendered as HTML
    /// details/summary elements. Only the browser render target produces these
    /// elements; on other targets this falls back to plain Markdown.
    MarkdownPlus,
    /// HTML output.
    #[value(alias = "browser")]
    Html,
    /// AST JSON output.
    #[value(alias = "ast")]
    Json,
}

/// Output format for `md code-block`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum CodeBlockOutput {
    /// Render as ANSI-styled terminal output.
    Terminal,
    /// Render as a `<pre><code class="language-…">…</code></pre>` HTML fragment.
    Html,
    /// Re-emit as a Markdown fenced code block.
    Markdown,
}

/// Cache-staleness handling for remote URL artifacts.
///
/// Mirrors `darkmatter::markdown::compose::RemoteFreshnessMode`. A closed value
/// set so an unrecognized `--remote-freshness` argument fails fast with the
/// accepted values rather than silently degrading to a single mode.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum RemoteFreshness {
    /// Serve any cached artifact without revalidation, even when stale.
    Optimistic,
    /// Always revalidate with a conditional GET.
    Strict,
    /// Serve stale on network failure (the default).
    Fallback,
}

/// Output format for validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ValidateOutputFormat {
    /// Human-readable text.
    Text,
    /// JSON output.
    Json,
}

/// Output format for `md schema validate`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SchemaValidateFormat {
    /// Pretty (terminal) output.
    Pretty,
    /// Newline-delimited JSON, one object per file.
    Json,
}

/// Output format for `md schema detect`.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum SchemaDetectFormat {
    /// SimplifiedSchema YAML.
    Yaml,
    /// JSON Schema (Draft 2020-12).
    Json,
}

/// Graph visualization format.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum GraphFormat {
    /// Mermaid flowchart.
    Mermaid,
    /// DOT (Graphviz) graph.
    Dot,
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

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

/// Output format for `md compose`: the document formats of
/// [`OutputFormat`], plus the composition report.
#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum ComposeOutputFormat {
    /// Auto: Markdown text (compose never renders to the terminal).
    Auto,
    /// Markdown text output.
    #[value(alias = "text")]
    Markdown,
    /// Markdown output with disclosure blocks as HTML details/summary elements.
    MarkdownPlus,
    /// HTML output.
    #[value(alias = "browser")]
    Html,
    /// AST JSON output.
    #[value(alias = "ast")]
    Json,
    /// One JSON object: the outcome, every diagnostic, and the Markdown
    /// document (`null` on failure).
    ReportJson,
}

impl ComposeOutputFormat {
    /// The document format this selects; the report carries Markdown.
    pub fn document_format(self) -> OutputFormat {
        match self {
            Self::Auto => OutputFormat::Auto,
            Self::Markdown | Self::ReportJson => OutputFormat::Markdown,
            Self::MarkdownPlus => OutputFormat::MarkdownPlus,
            Self::Html => OutputFormat::Html,
            Self::Json => OutputFormat::Json,
        }
    }
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

//! Types for prompt reporting configuration.

/// The verbosity level that can be specified via CLI flags, environment
/// variables, or frontmatter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptVerbosity {
    /// Show nothing (suppress all prompt reporting).
    Silent,
    /// Show only the summary.
    Quiet,
    /// Show the summary and the full prompt body.
    Verbose,
}

impl PromptVerbosity {
    /// Parse a verbosity value from a string (case-insensitive).
    ///
    /// Recognizes: "silent", "quiet", "verbose".
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "silent" => Some(Self::Silent),
            "quiet" => Some(Self::Quiet),
            "verbose" => Some(Self::Verbose),
            _ => None,
        }
    }
}

/// The format used when reporting a prompt body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PromptReportFormat {
    /// Only show summary information (no body).
    Summary,
    /// Show a truncated portion of the prompt body.
    PartialPrompt,
    /// Show the complete prompt body.
    FullPrompt,
}

/// How to truncate a partial prompt body.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TruncationMode {
    /// Show the first N lines, then truncate the rest.
    Truncate,
    /// Show the first N lines, an `hr` marker, then the last M lines.
    FrontBack,
}

/// Resolved verbosity for a prompt report.
///
/// Replaces the boolean-bag config structs and the
/// `(PromptVerbosity, PromptReportFormat)` pair. Each variant fully
/// describes what the renderer should produce.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReportMode {
    /// Suppress all output (no header, no summary, no body).
    Silent,
    /// Show the header and summary prose only; no body.
    Summary,
    /// Show the header, summary, and a truncated body using the given
    /// truncation strategy.
    Partial {
        /// Truncation strategy applied to the body.
        truncation: TruncationMode,
    },
    /// Show the header, summary, and the full body.
    Full,
}

impl ReportMode {
    /// Parse a [`ReportMode`] from the same case-insensitive string set as
    /// [`PromptVerbosity::parse`]:
    ///
    /// - `"silent"` → [`ReportMode::Silent`]
    /// - `"quiet"`  → [`ReportMode::Summary`]
    /// - `"verbose"` → [`ReportMode::Full`]
    ///
    /// Returns `None` for any other input. The [`ReportMode::Partial`]
    /// variant is intentionally not reachable from string input; it is
    /// produced by future CLI surface (Stage 2+).
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_lowercase().as_str() {
            "silent" => Some(Self::Silent),
            "quiet" => Some(Self::Summary),
            "verbose" => Some(Self::Full),
            _ => None,
        }
    }
}

impl From<PromptVerbosity> for ReportMode {
    fn from(verbosity: PromptVerbosity) -> Self {
        match verbosity {
            PromptVerbosity::Silent => Self::Silent,
            PromptVerbosity::Quiet => Self::Summary,
            PromptVerbosity::Verbose => Self::Full,
        }
    }
}

/// Resolved reporting configuration for a system prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SystemPromptReportConfig {
    /// Whether to show the header line (e.g. "📔 System Prompt(appended)").
    pub show_header: bool,
    /// Whether the summary prose line should appear in addition to any body.
    ///
    /// In verbose modes the summary is rendered alongside the full prompt
    /// body; in quiet/summary modes the body is omitted but the summary
    /// still appears.
    pub show_summary: bool,
    /// The format to use for the body.
    pub format: PromptReportFormat,
    /// How to truncate when format is `PartialPrompt`.
    pub truncation: TruncationMode,
}

/// Resolved reporting configuration for a user (agent) prompt.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UserPromptReportConfig {
    /// Whether to show the header line (e.g. "🗣️ Agent Prompt").
    pub show_header: bool,
    /// Whether to show the body at all.
    pub show_body: bool,
    /// The format to use for the body (when shown).
    pub format: PromptReportFormat,
    /// How to truncate when format is `PartialPrompt`.
    pub truncation: TruncationMode,
}

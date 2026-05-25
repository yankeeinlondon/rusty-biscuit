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

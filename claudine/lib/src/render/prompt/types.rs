//! Types for prompt reporting configuration.

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
/// Replaces the boolean-bag config structs and the legacy verbosity/format
/// pair. Each variant fully describes what the renderer should produce.
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
    /// Parse a [`ReportMode`] from a case-insensitive string:
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

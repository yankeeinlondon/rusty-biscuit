//! Type definitions for the `::toc-linking` directive.

use crate::markdown::compose::parse_utils::CursorError;
use std::collections::HashSet;
use std::ops::Range;
use thiserror::Error;

/// Errors that can occur during TOC linking.
#[derive(Error, Debug)]
pub enum TocLinkingError {
    /// Failed to parse a `::toc-linking` directive.
    #[error("Failed to parse toc-linking directive at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    /// An unknown cleanup service was specified.
    #[error("Invalid cleanup service '{service}' at line {line}")]
    InvalidCleanupService { service: String, line: usize },

    /// A heading level outside 1-6 was specified.
    #[error("Invalid heading level '{level}' at line {line}")]
    InvalidLevel { level: String, line: usize },

    /// A referenced file was not found and no fallback resolved.
    #[error("File not found '{path}' at line {line}")]
    FileNotFound { path: String, line: usize },

    /// A glob pattern failed to compile.
    #[error("Invalid glob pattern '{pattern}' at line {line}: {message}")]
    InvalidGlob {
        pattern: String,
        line: usize,
        message: String,
    },

    /// I/O error reading a referenced file.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<CursorError> for TocLinkingError {
    fn from(e: CursorError) -> Self {
        TocLinkingError::ParseDirective {
            line: e.line,
            message: e.message,
        }
    }
}

/// A cleanup service that transforms heading text for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupService {
    /// Strip leading emoji (and trailing space).
    EmojiLeader,
    /// Strip trailing emoji (and leading space).
    EmojiTrailing,
    /// Strip all emoji sequences.
    Emoji,
    /// Strip a leading numeric index (e.g., `1.2.3 `).
    Number,
    /// Capitalize the first alphanumeric character.
    Capitalize,
}

impl CleanupService {
    /// Parses a cleanup service name (case-insensitive).
    pub fn parse(s: &str, line: usize) -> Result<Self, TocLinkingError> {
        match s.to_ascii_lowercase().as_str() {
            "emoji_leader" => Ok(Self::EmojiLeader),
            "emoji_trailing" => Ok(Self::EmojiTrailing),
            "emoji" => Ok(Self::Emoji),
            "number" => Ok(Self::Number),
            "capitalize" => Ok(Self::Capitalize),
            _ => Err(TocLinkingError::InvalidCleanupService {
                service: s.to_string(),
                line,
            }),
        }
    }

    /// Returns all available cleanup services.
    pub fn all() -> Vec<Self> {
        vec![
            Self::EmojiLeader,
            Self::EmojiTrailing,
            Self::Emoji,
            Self::Number,
            Self::Capitalize,
        ]
    }
}

/// Heading level filter.
///
/// When `levels` is empty, the default H2-H6 range applies.
#[derive(Debug, Clone, Default)]
pub struct LevelFilter {
    pub levels: HashSet<u8>,
}

impl LevelFilter {
    /// Returns true if the given level passes this filter.
    pub fn includes(&self, level: u8) -> bool {
        if self.levels.is_empty() {
            // Default: H2-H6
            (2..=6).contains(&level)
        } else {
            self.levels.contains(&level)
        }
    }
}

/// A glob pattern for heading text filtering.
#[derive(Debug, Clone)]
pub struct HeadingGlob {
    /// The raw glob pattern.
    pub pattern: String,
    /// If true, matching is case-sensitive (prefixed with `^`).
    pub case_sensitive: bool,
}

/// Options parsed from a `::toc-linking` directive.
#[derive(Debug, Clone, Default)]
pub struct TocLinkingOptions {
    /// Heading level filter.
    pub levels: LevelFilter,
    /// Cleanup services to apply to display text.
    pub cleanup_services: Vec<CleanupService>,
    /// Keep (whitelist) glob patterns.
    pub keep_patterns: Vec<HeadingGlob>,
    /// Filter (blacklist) glob patterns.
    pub filter_patterns: Vec<HeadingGlob>,
    /// Text to emit when no headings remain after filtering.
    pub empty_text: Option<String>,
}

/// A parsed `::toc-linking` directive.
#[derive(Debug, Clone)]
pub struct TocLinkingDirective {
    /// Pipe-separated target file paths (fallback chain).
    pub targets: Vec<String>,
    /// If the chain terminates with `| false`, missing files are suppressed.
    pub suppress_not_found: bool,
    /// Parsed options for this directive.
    pub options: TocLinkingOptions,
    /// Byte range of the directive line in the source document.
    pub span: Range<usize>,
    /// 1-indexed line number.
    pub line: usize,
}

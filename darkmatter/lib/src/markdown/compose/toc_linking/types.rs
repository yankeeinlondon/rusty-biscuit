//! Type definitions for the `::toc-linking` directive.

use crate::markdown::compose::parse_utils::CursorError;
use crate::markdown::normalize::HeadingLevel;
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

impl biscuit_terminal::errors::BlockError for TocLinkingError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            TocLinkingError::ParseDirective { line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TocLinkingError",
                    "directive parse failed",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Syntax: <cyan>::toc-linking ./doc.md levels=2,3 cleanup=number,capitalize</cyan>."),

            TocLinkingError::InvalidCleanupService { service, line } => {
                let valid = CleanupService::all()
                    .iter()
                    .map(|service| {
                        let descriptor = cleanup_service_descriptor(service);
                        format!(
                            "  <cyan>{}</cyan> - {}",
                            descriptor.name, descriptor.description
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("\n");
                StatusBlock::new(StatusState::Error)
                    .error_header(ErrorHeader::new(
                        "TocLinkingError",
                        "invalid cleanup service",
                    ))
                    .body(format!(
                        "<dim>Service:</dim> <cyan>{service}</cyan>\n<dim>Line:</dim> {line}\n<dim>Valid services:</dim>\n{valid}"
                    ))
                    .hint("Pass a comma-separated list of valid service names to <cyan>cleanup=</cyan>.")
            }

            TocLinkingError::InvalidLevel { level, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "invalid heading level"))
                .body(format!(
                    "<dim>Level:</dim> <cyan>{level}</cyan>\n<dim>Line:</dim> {line}"
                ))
                .hint("Heading levels must be integers between <cyan>1</cyan> and <cyan>6</cyan>."),

            TocLinkingError::FileNotFound { path, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "file not found"))
                .body(format!(
                    "<dim>Path:</dim> <cyan>{path}</cyan>\n<dim>Line:</dim> {line}"
                ))
                .hint("Add a <cyan>| fallback.md</cyan> option or end the chain with <cyan>| false</cyan> to allow missing files."),

            TocLinkingError::InvalidGlob { pattern, line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "invalid glob pattern"))
                .body(format!(
                    "<dim>Pattern:</dim> <cyan>{pattern}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("See the globset crate docs for supported pattern syntax."),

            TocLinkingError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "I/O error"))
                .body(format!(
                    "<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                ))
                .hint("Confirm the referenced file exists and is readable."),
        }
    }
}

/// Name/description pair for a cleanup service.
struct CleanupServiceDescriptor {
    name: &'static str,
    description: &'static str,
}

/// Canonical CLI token and user-facing description for a cleanup service.
fn cleanup_service_descriptor(service: &CleanupService) -> CleanupServiceDescriptor {
    match service {
        CleanupService::EmojiLeader => CleanupServiceDescriptor {
            name: "emoji_leader",
            description: "Strip leading emoji and any following space.",
        },
        CleanupService::EmojiTrailing => CleanupServiceDescriptor {
            name: "emoji_trailing",
            description: "Strip trailing emoji and any preceding space.",
        },
        CleanupService::Emoji => CleanupServiceDescriptor {
            name: "emoji",
            description: "Remove emoji sequences anywhere in the heading.",
        },
        CleanupService::Number => CleanupServiceDescriptor {
            name: "number",
            description: "Remove a leading numeric outline like `1.2.3`.",
        },
        CleanupService::Capitalize => CleanupServiceDescriptor {
            name: "capitalize",
            description: "Uppercase the first alphanumeric character.",
        },
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
    pub levels: HashSet<HeadingLevel>,
}

impl LevelFilter {
    /// Returns true if the given level passes this filter.
    pub fn includes(&self, level: HeadingLevel) -> bool {
        if self.levels.is_empty() {
            // Default: H2-H6
            level >= HeadingLevel::H2
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

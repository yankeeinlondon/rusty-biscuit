//! Type definitions for the `::toc-linking` directive.

use crate::markdown::compose::parse_utils::CursorError;
use crate::markdown::normalize::HeadingLevel;
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::ops::Range;
use crate::markdown::compose::transclusion::at_line;
use crate::markdown::types::AuthoredSpan;
use thiserror::Error;

/// Errors that can occur during TOC linking.
///
/// Every variant with a `line` also names its `document`: the file of the
/// document whose body holds the directive, or the context identity of a
/// document with no file ([`Markdown::with_context_identity`](crate::markdown::Markdown::with_context_identity)).
/// For a document with a file, `line` is the directive's line in that file
/// as written and `authored` its opener, or `line` is `0` (unknown, and the
/// message names no line) when the body's map does not prove where the
/// directive was written. For a document with no file, `line` counts its
/// body, so a host that composes text it generated tells its own failures
/// (`document` is its identity) from a transcluded child's, and publishes
/// [`Self::reason`] instead.
#[derive(Error, Debug)]
pub enum TocLinkingError {
    /// Failed to parse a `::toc-linking` directive.
    #[error("Failed to parse toc-linking directive{}: {message}", at_line(*.line))]
    ParseDirective {
        line: usize,
        message: String,
        document: Option<PathBuf>,
        /// See [`TocLinkingError::authored`].
        authored: Option<AuthoredSpan>,
    },

    /// An unknown cleanup service was specified.
    #[error("Invalid cleanup service '{service}'{}", at_line(*.line))]
    InvalidCleanupService {
        service: String,
        line: usize,
        document: Option<PathBuf>,
        /// See [`TocLinkingError::authored`].
        authored: Option<AuthoredSpan>,
    },

    /// A heading level outside 1-6 was specified.
    #[error("Invalid heading level '{level}'{}", at_line(*.line))]
    InvalidLevel {
        level: String,
        line: usize,
        document: Option<PathBuf>,
        /// See [`TocLinkingError::authored`].
        authored: Option<AuthoredSpan>,
    },

    /// No target in the chain resolved to a file and the chain does not end
    /// in `| false`. `failure` is the class of the first (authored) target's
    /// failure; `None` when that failure was not a file-reference failure.
    /// `glob_hint` is that target's literal-glob hint
    /// ([`ResolutionFailure::glob_hint`](biscuit_file::ResolutionFailure::glob_hint)).
    #[error("{}", crate::markdown::errors::with_hint_line(format!("Could not resolve '{}'{}", .path, at_line(*.line)), *.glob_hint))]
    Unresolved {
        path: String,
        line: usize,
        failure: Option<biscuit_file::ResolutionFailure>,
        glob_hint: Option<&'static str>,
        document: Option<PathBuf>,
        /// See [`TocLinkingError::authored`].
        authored: Option<AuthoredSpan>,
    },

    /// A glob pattern failed to compile.
    #[error("Invalid glob pattern '{pattern}'{}: {message}", at_line(*.line))]
    InvalidGlob {
        pattern: String,
        line: usize,
        message: String,
        document: Option<PathBuf>,
        /// See [`TocLinkingError::authored`].
        authored: Option<AuthoredSpan>,
    },

    /// I/O error reading a referenced file.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl TocLinkingError {
    /// The file-reference failure class, when a target failed to resolve.
    pub fn resolution_failure(&self) -> Option<biscuit_file::ResolutionFailure> {
        match self {
            Self::Unresolved { failure, .. } => *failure,
            _ => None,
        }
    }

    /// The document whose body holds the directive (see [`TocLinkingError`]);
    /// `None` for an I/O error, or a document with neither file nor identity.
    pub fn document(&self) -> Option<&Path> {
        match self {
            Self::ParseDirective { document, .. }
            | Self::InvalidCleanupService { document, .. }
            | Self::InvalidLevel { document, .. }
            | Self::Unresolved { document, .. }
            | Self::InvalidGlob { document, .. } => document.as_deref(),
            Self::Io(_) => None,
        }
    }

    /// The directive's opener in its loaded file, when `line` is authored.
    pub fn authored(&self) -> Option<&AuthoredSpan> {
        match self {
            Self::ParseDirective { authored, .. }
            | Self::InvalidCleanupService { authored, .. }
            | Self::InvalidLevel { authored, .. }
            | Self::Unresolved { authored, .. }
            | Self::InvalidGlob { authored, .. } => authored.as_ref(),
            Self::Io(_) => None,
        }
    }

    /// This error placed at `site`, its directive's site in the document
    /// whose current body its `line` counted; an I/O error is unchanged.
    #[must_use]
    pub(crate) fn at_site(mut self, site: &crate::markdown::compose::site_scope::DirectiveSite) -> Self {
        match &mut self {
            Self::ParseDirective { line, authored, .. }
            | Self::InvalidCleanupService { line, authored, .. }
            | Self::InvalidLevel { line, authored, .. }
            | Self::Unresolved { line, authored, .. }
            | Self::InvalidGlob { line, authored, .. } => site.place(line, authored),
            Self::Io(_) => {}
        }
        self
    }

    /// This error, raised for `markdown`'s current body, at its directive's
    /// line as written; `origin` maps that body.
    #[must_use]
    pub(crate) fn at_authored_line(
        self,
        markdown: &crate::markdown::Markdown,
        origin: Option<&crate::markdown::compose::body_origin::BodyOrigin>,
    ) -> Self {
        match self.line() {
            Some(line) => self.at_site(&crate::markdown::compose::site_scope::DirectiveSite::of_line(markdown, origin, line)),
            None => self,
        }
    }

    fn line(&self) -> Option<usize> {
        match self {
            Self::ParseDirective { line, .. }
            | Self::InvalidCleanupService { line, .. }
            | Self::InvalidLevel { line, .. }
            | Self::Unresolved { line, .. }
            | Self::InvalidGlob { line, .. } => Some(*line),
            Self::Io(_) => None,
        }
    }

    /// This error with `document` named; an I/O error is unchanged.
    #[must_use]
    pub fn with_document(mut self, named: Option<PathBuf>) -> Self {
        match &mut self {
            Self::ParseDirective { document, .. }
            | Self::InvalidCleanupService { document, .. }
            | Self::InvalidLevel { document, .. }
            | Self::Unresolved { document, .. }
            | Self::InvalidGlob { document, .. } => *document = named,
            Self::Io(_) => {}
        }
        self
    }

    /// The failure without its line.
    pub fn reason(&self) -> String {
        match self {
            Self::ParseDirective { message, .. } => format!("invalid `::toc-linking` directive: {message}"),
            Self::InvalidCleanupService { service, .. } => format!("invalid `::toc-linking` cleanup service '{service}'"),
            Self::InvalidLevel { level, .. } => format!("invalid `::toc-linking` heading level '{level}'"),
            Self::Unresolved { path, glob_hint, .. } => crate::markdown::errors::with_hint_line(
                format!("`::toc-linking` could not resolve '{path}'"),
                *glob_hint,
            ),
            Self::InvalidGlob { pattern, message, .. } => {
                format!("invalid `::toc-linking` glob pattern '{pattern}': {message}")
            }
            Self::Io(source) => source.to_string(),
        }
    }
}

impl From<CursorError> for TocLinkingError {
    fn from(e: CursorError) -> Self {
        TocLinkingError::ParseDirective {
            line: e.line,
            message: e.message,
            document: None,
            authored: None,
        }
    }
}

impl biscuit_terminal::errors::BlockError for TocLinkingError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::prose::{LineBreaks, Prose};
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};
        use crate::markdown::compose::site_scope::{line_row, status_rows};

        match self {
            TocLinkingError::ParseDirective { line, message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "TocLinkingError",
                    "directive parse failed",
                ))
                .body(Prose::new(status_rows([line_row(*line), Some(format!("<dim>Message:</dim> {message}"))]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("Syntax: <cyan>::toc-linking ./doc.md levels=2,3 cleanup=number,capitalize</cyan>."),

            TocLinkingError::InvalidCleanupService { service, line, .. } => {
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
                    .body(Prose::new(status_rows([
                        Some(format!("<dim>Service:</dim> <cyan>{service}</cyan>")),
                        line_row(*line),
                        Some(format!("<dim>Valid services:</dim>\n{valid}")),
                    ]))
                    .with_line_breaks(LineBreaks::Hard))
                    .hint("Pass a comma-separated list of valid service names to <cyan>cleanup=</cyan>.")
            }

            TocLinkingError::InvalidLevel { level, line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "invalid heading level"))
                .body(Prose::new(status_rows([Some(format!("<dim>Level:</dim> <cyan>{level}</cyan>")), line_row(*line)]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("Heading levels must be integers between <cyan>1</cyan> and <cyan>6</cyan>."),

            TocLinkingError::Unresolved { path, line, failure, glob_hint, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "file reference failure"))
                .body(Prose::new(
                    [
                        Some(format!("<dim>Path:</dim> <cyan>{path}</cyan>")),
                        line_row(*line),
                        failure.map(|failure| {
                            format!(
                                "<dim>failure:</dim> {}",
                                crate::markdown::errors::resolution_failure_name(failure)
                            )
                        }),
                        glob_hint.map(|hint| format!("<dim>hint:</dim> {hint}")),
                    ]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join("\n"),
                ).with_line_breaks(LineBreaks::Hard))
                .hint("Add a <cyan>| fallback.md</cyan> option or end the chain with <cyan>| false</cyan> to allow missing files."),

            TocLinkingError::InvalidGlob { pattern, line, message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "invalid glob pattern"))
                .body(Prose::new(status_rows([
                    Some(format!("<dim>Pattern:</dim> <cyan>{pattern}</cyan>")),
                    line_row(*line),
                    Some(format!("<dim>Message:</dim> {message}")),
                ]))
                .with_line_breaks(LineBreaks::Hard))
                .hint("See the globset crate docs for supported pattern syntax."),

            TocLinkingError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("TocLinkingError", "I/O error"))
                .body(Prose::new(format!(
                    "<dim>Kind:</dim> {:?}\n{source}",
                    source.kind()
                )).with_line_breaks(LineBreaks::Hard))
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
                document: None,
                authored: None,
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
    /// Leading whitespace of the directive line.
    pub indent: String,
    /// Inferred container indentation when directive is at column 1.
    pub inferred_indent: Option<String>,
}

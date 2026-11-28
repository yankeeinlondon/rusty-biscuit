//! Type definitions for the `::file-links` directive.
//!
//! [`FileLinksDirective`] holds the parsed directive shape; [`FileLinksResult`]
//! carries the discovered files and rendering metadata produced by
//! [`discover`](super::discovery::discover) for the
//! [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! component.

use std::ops::Range;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use thiserror::Error;

use crate::markdown::compose::parse_utils::CursorError;
use crate::markdown::compose::transclusion::at_line;
use crate::markdown::types::AuthoredSpan;

/// ` (line N)`, or nothing for the unknown line `0`.
fn line_note(line: usize) -> String {
    if line == 0 { String::new() } else { format!(" (line {line})") }
}

/// Allowed document extensions (without leading dots), compared case-insensitively.
///
/// Mirrors the set baked into
/// [`FileSystem::document_extensions`](biscuit_terminal::components::filesystem::FileSystem::document_extensions):
/// Markdown plus the additional document/text formats the directive lists.
pub(crate) const ALLOWED_EXTENSIONS: &[&str] = &["md", "txt", "doc", "docx", "xls", "xlsx", "pdf"];

/// Default recursion depth for `--dir` mode.
///
/// `0` means the target directory itself is scanned but subdirectories are not
/// descended into.
pub(crate) const DEFAULT_DIR_DEPTH: u32 = 0;

/// The source form a `::file-links` directive takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FileLinksMode {
    /// `::file-links <glob>` — a glob reference (`[!][prefix]glob`) authored in
    /// the document; a bare glob searches the document's folder, then the
    /// repository root.
    Glob(String),
    /// `::file-links --dir <path> [--depth <u32>]` — a directory scan.
    Dir {
        /// Directory path resolved relative to the document.
        path: String,
        /// Requested recursion depth. Defaults to `0` (no recursion).
        depth: u32,
    },
}

/// A parsed `::file-links` directive.
#[derive(Debug, Clone)]
pub struct FileLinksDirective {
    /// Parsed source form.
    pub mode: FileLinksMode,
    /// Byte range of the directive line in the source document.
    pub span: Range<usize>,
    /// 1-indexed line number.
    pub line: usize,
    /// Leading whitespace of the directive line.
    pub indent: String,
    /// Inferred container indentation when the directive is at column 1.
    pub inferred_indent: Option<String>,
}

/// Rendering metadata computed from discovery.
///
/// These fields map directly onto the
/// [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
/// builder API: the component root is the directory the tree is rooted at; the
/// included paths are the exact matched files relative to that root; the prefix
/// and target name drive the dimmed/highlighted root label; and the repository
/// flag selects the repo folder icon.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileLinksRender {
    /// Absolute directory the [`FileSystem`] tree should be rooted at.
    pub component_root: PathBuf,
    /// Matched files relative to `component_root`.
    pub included_paths: Vec<PathBuf>,
    /// Boundary-relative prefix rendered dimmed before the target name.
    ///
    /// Has no leading separator and ends in `/` when non-empty (e.g. `docs/`). Empty when
    /// the component root is the boundary itself.
    pub dimmed_prefix: String,
    /// Highlighted target directory name, ending in `/`, on the unlinked root line.
    pub target_name: String,
    /// Whether the component root is the repository root itself.
    pub uses_repo_icon: bool,
}

/// The outcome of discovering file-link targets for one directive.
#[derive(Debug, Clone)]
pub struct FileLinksResult {
    /// The directive that produced this result.
    pub directive: FileLinksDirective,
    /// Rendering metadata. `None` when no files matched.
    pub render: Option<FileLinksRender>,
    /// File symlinks a glob matched but left out because their target lies
    /// outside the file tree. Always empty for `--dir`.
    pub skipped: Vec<biscuit_file::SkippedEntry>,
}

/// Errors that can occur during `::file-links` parsing or discovery.
///
/// Every variant with a `line` also names its `document`, as
/// [`TocLinkingError`](crate::markdown::compose::TocLinkingError) does: the
/// file of the document whose body holds the directive, or a file-less
/// document's context identity. `line` and `authored` follow
/// [`TocLinkingError`](crate::markdown::compose::TocLinkingError)'s rule: the
/// line as written in a file (`0`, unknown, when unproven), or the body's
/// line in a document with no file.
#[derive(Error, Debug)]
pub enum FileLinksError {
    /// Failed to parse a `::file-links` directive line.
    #[error("Failed to parse file-links directive{}: {message}", at_line(*.line))]
    ParseDirective { line: usize, message: String, document: Option<PathBuf>, authored: Option<AuthoredSpan> },

    /// A `--depth` value is not a non-negative integer.
    #[error("Failed to parse file-links directive{}: '--depth' expects a non-negative integer, got '{value}'", at_line(*.line))]
    InvalidDepth { value: String, line: usize, document: Option<PathBuf>, authored: Option<AuthoredSpan> },

    /// The directive was used without a containing source file.
    #[error("file-links directive{} requires a source file", at_line(*.line))]
    MissingSourceContext { line: usize, document: Option<PathBuf>, authored: Option<AuthoredSpan> },

    /// The referenced target directory does not exist.
    #[error("Target directory '{path}' does not exist{}", line_note(*.line))]
    TargetNotFound { path: String, line: usize, document: Option<PathBuf>, authored: Option<AuthoredSpan> },

    /// The `--dir` target exists but is not a directory.
    #[error("Target '{path}' is not a directory{}", line_note(*.line))]
    TargetNotDirectory { path: String, line: usize, document: Option<PathBuf>, authored: Option<AuthoredSpan> },

    /// The glob reference could not be parsed, rooted, or listed: an invalid
    /// glob or prefix, a relative glob leaving the file tree, `&`/`^` outside
    /// a repository, or a search root that cannot be read.
    #[error("{source}{}", line_note(*.line))]
    GlobReference {
        line: usize,
        // Shared like `ExpressionError::GlobReference`, keeping this enum
        // under clippy's `result_large_err` limit; `PathBuf` is wider on
        // Windows.
        #[source]
        source: Arc<biscuit_file::GlobReferenceError>,
        document: Option<PathBuf>,
        authored: Option<AuthoredSpan>,
    },

    /// A `--dir` scan reached a directory it could not read, or an entry
    /// whose type could not be determined. The scan fails rather than
    /// rendering a tree with that directory's files left out.
    #[error("cannot read '{}'{}: {source}", path.display(), line_note(*.line))]
    Unreadable {
        path: std::path::PathBuf,
        line: usize,
        #[source]
        source: std::io::Error,
        document: Option<PathBuf>,
        authored: Option<AuthoredSpan>,
    },

    /// I/O error during discovery.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl FileLinksError {
    /// The document whose body holds the directive (see [`FileLinksError`]);
    /// `None` for an I/O error, or a document with neither file nor identity.
    pub fn document(&self) -> Option<&Path> {
        match self {
            Self::ParseDirective { document, .. }
            | Self::InvalidDepth { document, .. }
            | Self::MissingSourceContext { document, .. }
            | Self::TargetNotFound { document, .. }
            | Self::TargetNotDirectory { document, .. }
            | Self::GlobReference { document, .. }
            | Self::Unreadable { document, .. } => document.as_deref(),
            Self::Io(_) => None,
        }
    }

    /// The directive's opener in its loaded file, when `line` is authored.
    pub fn authored(&self) -> Option<&AuthoredSpan> {
        match self {
            Self::ParseDirective { authored, .. }
            | Self::InvalidDepth { authored, .. }
            | Self::MissingSourceContext { authored, .. }
            | Self::TargetNotFound { authored, .. }
            | Self::TargetNotDirectory { authored, .. }
            | Self::GlobReference { authored, .. }
            | Self::Unreadable { authored, .. } => authored.as_ref(),
            Self::Io(_) => None,
        }
    }

    /// This error placed at `site`, its directive's site in the document
    /// whose current body its `line` counted; an I/O error is unchanged.
    #[must_use]
    pub(crate) fn at_site(mut self, site: &crate::markdown::compose::site_scope::DirectiveSite) -> Self {
        match &mut self {
            Self::ParseDirective { line, authored, .. }
            | Self::InvalidDepth { line, authored, .. }
            | Self::MissingSourceContext { line, authored, .. }
            | Self::TargetNotFound { line, authored, .. }
            | Self::TargetNotDirectory { line, authored, .. }
            | Self::GlobReference { line, authored, .. }
            | Self::Unreadable { line, authored, .. } => site.place(line, authored),
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
        let line = match &self {
            Self::ParseDirective { line, .. }
            | Self::InvalidDepth { line, .. }
            | Self::MissingSourceContext { line, .. }
            | Self::TargetNotFound { line, .. }
            | Self::TargetNotDirectory { line, .. }
            | Self::GlobReference { line, .. }
            | Self::Unreadable { line, .. } => *line,
            Self::Io(_) => return self,
        };
        self.at_site(&crate::markdown::compose::site_scope::DirectiveSite::of_line(markdown, origin, line))
    }

    /// This error with `document` named; an I/O error is unchanged.
    #[must_use]
    pub fn with_document(mut self, named: Option<PathBuf>) -> Self {
        match &mut self {
            Self::ParseDirective { document, .. }
            | Self::InvalidDepth { document, .. }
            | Self::MissingSourceContext { document, .. }
            | Self::TargetNotFound { document, .. }
            | Self::TargetNotDirectory { document, .. }
            | Self::GlobReference { document, .. }
            | Self::Unreadable { document, .. } => *document = named,
            Self::Io(_) => {}
        }
        self
    }

    /// The failure without its line.
    pub fn reason(&self) -> String {
        match self {
            Self::ParseDirective { message, .. } => format!("invalid `::file-links` directive: {message}"),
            Self::InvalidDepth { value, .. } => {
                format!("invalid `::file-links` directive: '--depth' expects a non-negative integer, got '{value}'")
            }
            Self::MissingSourceContext { .. } => "`::file-links` requires a source file".to_string(),
            Self::TargetNotFound { path, .. } => format!("`::file-links` target directory '{path}' does not exist"),
            Self::TargetNotDirectory { path, .. } => format!("`::file-links` target '{path}' is not a directory"),
            Self::GlobReference { source, .. } => format!("`::file-links`: {source}"),
            Self::Unreadable { path, source, .. } => {
                format!("`::file-links` cannot read '{}': {source}", path.display())
            }
            Self::Io(source) => source.to_string(),
        }
    }
}

impl From<CursorError> for FileLinksError {
    fn from(e: CursorError) -> Self {
        FileLinksError::ParseDirective {
            line: e.line,
            message: e.message,
            document: None,
            authored: None,
        }
    }
}

impl biscuit_terminal::errors::BlockError for FileLinksError {
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
            FileLinksError::ParseDirective { line, message, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "directive parse failed",
                ))
                .body(Prose::new(status_rows([line_row(*line), Some(format!("<dim>Message:</dim> {message}"))]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("Syntax: <cyan>::file-links \"docs/**/*.md\"</cyan> or <cyan>::file-links --dir docs --depth 2</cyan>."),

            FileLinksError::InvalidDepth { value, line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "invalid depth",
                ))
                .body(Prose::new(status_rows([
                    line_row(*line),
                    Some(format!("<dim>Depth:</dim> <cyan>{}</cyan>", Prose::escape_text(value))),
                ]))
                .with_line_breaks(LineBreaks::Hard))
                .hint("<cyan>--depth</cyan> takes a non-negative integer, such as <cyan>::file-links --dir docs --depth 2</cyan>."),

            FileLinksError::MissingSourceContext { line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "no source file",
                ))
                .body(Prose::new(status_rows([
                    line_row(*line),
                    Some("Paths resolve relative to the containing document, but no source file was provided.".to_string()),
                ]))
                .with_line_breaks(LineBreaks::Hard))
                .hint("Compose from a file (e.g. <cyan>md compose doc.md</cyan>) or pass <cyan>ComposeOptions::with_source_file</cyan>."),

            FileLinksError::TargetNotFound { path, line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "target directory not found",
                ))
                .body(Prose::new(status_rows([Some(format!("<dim>Path:</dim> <cyan>{path}</cyan>")), line_row(*line)]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("Confirm the directory exists relative to the document."),

            FileLinksError::TargetNotDirectory { path, line, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "target is not a directory",
                ))
                .body(Prose::new(status_rows([Some(format!("<dim>Path:</dim> <cyan>{path}</cyan>")), line_row(*line)]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("<cyan>--dir</cyan> requires a directory; use a glob (e.g. <cyan>::file-links \"docs/*.md\"</cyan>) to match a file."),

            FileLinksError::GlobReference { line, source, .. } if let biscuit_file::GlobReferenceError::Io { path, .. } = source.as_ref() => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "glob search failed",
                ))
                .body(Prose::new(status_rows([
                    line_row(*line),
                    Some(format!("<dim>Path:</dim> <cyan>{}</cyan>", path.display())),
                    Some(format!("<dim>Message:</dim> {source}")),
                ]))
                .with_line_breaks(LineBreaks::Hard))
                .hint("Make the directory readable, or narrow the glob so its search does not enter it."),

            FileLinksError::GlobReference { line, source, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "invalid glob reference",
                ))
                .body(Prose::new(status_rows([line_row(*line), Some(format!("<dim>Message:</dim> {source}"))]))
                    .with_line_breaks(LineBreaks::Hard))
                .hint("A glob takes a file-reference prefix (<cyan>&</cyan>, <cyan>^</cyan>, <cyan>@</cyan>, <cyan>~/</cyan>, <cyan>./</cyan>) and stays inside the file tree unless it names its root."),

            FileLinksError::Unreadable { path, line, source, .. } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("FileLinksError", "directory unreadable"))
                .body(Prose::new(status_rows([
                    Some(format!("<dim>Path:</dim> <cyan>{}</cyan>", path.display())),
                    line_row(*line),
                    Some(format!("<dim>Message:</dim> {source}")),
                ]))
                .with_line_breaks(LineBreaks::Hard))
                .hint("Make the directory readable, or lower <cyan>--depth</cyan> so the scan does not enter it."),

            FileLinksError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("FileLinksError", "I/O error"))
                .body(Prose::new(format!("<dim>Kind:</dim> {:?}\n{source}", source.kind())).with_line_breaks(LineBreaks::Hard))
                .hint("Confirm the referenced paths are readable."),
        }
    }
}

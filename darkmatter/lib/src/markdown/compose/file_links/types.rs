//! Type definitions for the `::file-links` directive.
//!
//! [`FileLinksDirective`] holds the parsed directive shape; [`FileLinksResult`]
//! carries the discovered files and rendering metadata produced by
//! [`discover`](super::discovery::discover) for the
//! [`FileSystem`](biscuit_terminal::components::filesystem::FileSystem)
//! component.

use std::ops::Range;
use std::path::PathBuf;

use thiserror::Error;

use crate::markdown::compose::parse_utils::CursorError;

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
    /// `::file-links <glob>` — a glob pattern resolved relative to the document.
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
    /// Includes a leading separator when non-empty (e.g. `/docs/`). Empty when
    /// the component root is the boundary itself.
    pub dimmed_prefix: String,
    /// Highlighted target directory name rendered on the root line.
    pub target_name: String,
    /// Whether the repository folder icon should be used on the root line.
    pub uses_repo_icon: bool,
}

/// The outcome of discovering file-link targets for one directive.
#[derive(Debug, Clone)]
pub struct FileLinksResult {
    /// The directive that produced this result.
    pub directive: FileLinksDirective,
    /// Rendering metadata. `None` when no files matched.
    pub render: Option<FileLinksRender>,
}

/// Errors that can occur during `::file-links` parsing or discovery.
#[derive(Error, Debug)]
pub enum FileLinksError {
    /// Failed to parse a `::file-links` directive line.
    #[error("Failed to parse file-links directive at line {line}: {message}")]
    ParseDirective { line: usize, message: String },

    /// The directive was used without a containing source file.
    #[error("file-links directive at line {line} requires a source file")]
    MissingSourceContext { line: usize },

    /// The referenced target directory does not exist.
    #[error("Target directory '{path}' does not exist (line {line})")]
    TargetNotFound { path: String, line: usize },

    /// A glob pattern failed to compile.
    #[error("Invalid glob pattern '{pattern}' at line {line}: {message}")]
    InvalidGlob {
        pattern: String,
        line: usize,
        message: String,
    },

    /// I/O error during discovery.
    #[error(transparent)]
    Io(#[from] std::io::Error),
}

impl From<CursorError> for FileLinksError {
    fn from(e: CursorError) -> Self {
        FileLinksError::ParseDirective {
            line: e.line,
            message: e.message,
        }
    }
}

impl biscuit_terminal::errors::BlockError for FileLinksError {
    fn status_block(
        &self,
        _term: &biscuit_terminal::terminal::Terminal,
    ) -> biscuit_terminal::components::status_block::StatusBlock {
        use biscuit_terminal::components::status::StatusState;
        use biscuit_terminal::components::status_block::StatusBlock;
        use biscuit_terminal::errors::{ErrorHeader, StatusBlockExt};

        match self {
            FileLinksError::ParseDirective { line, message } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "directive parse failed",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("Syntax: <cyan>::file-links \"docs/**/*.md\"</cyan> or <cyan>::file-links --dir docs --depth 2</cyan>."),

            FileLinksError::MissingSourceContext { line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "no source file",
                ))
                .body(format!(
                    "<dim>Line:</dim> {line}\nPaths resolve relative to the containing document, but no source file was provided."
                ))
                .hint("Compose from a file (e.g. <cyan>md compose doc.md</cyan>) or pass <cyan>ComposeOptions::with_source_file</cyan>."),

            FileLinksError::TargetNotFound { path, line } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "target directory not found",
                ))
                .body(format!(
                    "<dim>Path:</dim> <cyan>{path}</cyan>\n<dim>Line:</dim> {line}"
                ))
                .hint("Confirm the directory exists relative to the document."),

            FileLinksError::InvalidGlob {
                pattern,
                line,
                message,
            } => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new(
                    "FileLinksError",
                    "invalid glob pattern",
                ))
                .body(format!(
                    "<dim>Pattern:</dim> <cyan>{pattern}</cyan>\n<dim>Line:</dim> {line}\n<dim>Message:</dim> {message}"
                ))
                .hint("See the globset crate docs for supported pattern syntax."),

            FileLinksError::Io(source) => StatusBlock::new(StatusState::Error)
                .error_header(ErrorHeader::new("FileLinksError", "I/O error"))
                .body(format!("<dim>Kind:</dim> {:?}\n{source}", source.kind()))
                .hint("Confirm the referenced paths are readable."),
        }
    }
}

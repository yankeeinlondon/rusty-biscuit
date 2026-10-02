//! Shared file-input helpers used by every subcommand.

use crate::request::MdRequest;
use biscuit_file::{FileReference, FileReferenceError, FileResolutionContext, ResolutionFailure};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::status::StatusState;
use biscuit_terminal::components::status_block::StatusBlock;
use biscuit_terminal::errors::{BlockError, ErrorHeader, StatusBlockExt};
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::errors::resolution_failure_row;
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};

/// Loads markdown from a file path or stdin.
///
/// A path opens through [`open_argument`], so `@`, `&`, `^`, `~`, and
/// `{{VAR}}` arguments work and malformed reference syntax is refused. `-` (or
/// no path with piped input) reads stdin and builds no context.
pub fn load_markdown(path: Option<&PathBuf>, request: &MdRequest) -> Result<Markdown> {
    Ok(load_document(path, request)?.0)
}

/// [`load_markdown`], also returning the file the argument resolved to
/// (`None` for stdin).
pub fn load_document(
    path: Option<&PathBuf>,
    request: &MdRequest,
) -> Result<(Markdown, Option<PathBuf>)> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            Ok((read_from_stdin()?, None))
        } else {
            let resolved = open_argument(p, request)?.into_path();
            let markdown = Markdown::try_from(resolved.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
            Ok((markdown, Some(resolved)))
        }
    } else {
        // No path provided - check if stdin has data
        if io::stdin().is_terminal() {
            // Interactive terminal - no input available
            Err(eyre!("No input file provided. Use `md --help` for usage."))
        } else {
            // Piped input available
            Ok((read_from_stdin()?, None))
        }
    }
}

/// Reads an already-resolved Markdown file while retaining its exact UTF-8
/// source.
///
/// For commands that must write back without losing authored formatting. The
/// returned [`Markdown`] is parsed from the same text returned to the caller.
pub fn read_markdown_text(resolved: &Path) -> Result<(String, Markdown)> {
    let source = std::fs::read_to_string(resolved)
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    let markdown = Markdown::try_from_content(source.clone())
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    Ok((source, markdown))
}

/// A source-file argument opened through the reference grammar: the
/// reference as the caller spelled it and the file it resolved to.
#[derive(Debug, Clone)]
pub struct OpenedArgument {
    reference: FileReference,
    path: PathBuf,
}

impl OpenedArgument {
    /// The argument as parsed, the opening spelling a document context
    /// derives from (a quoted `~/…` or `{{VAR}}/…` supplies its tree root).
    pub fn reference(&self) -> &FileReference {
        &self.reference
    }

    /// The file the argument resolved to.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn into_path(self) -> PathBuf {
        self.path
    }

    /// The context the opened document resolves its own references against
    /// ([`MdRequest::document_context`] with this opening spelling).
    ///
    /// ## Errors
    ///
    /// Returns the context builder's error.
    pub fn document_context(&self, request: &MdRequest) -> Result<FileResolutionContext> {
        request.document_context(Some(&self.reference), &self.path)
    }
}

/// Opens a source-file argument: every `md` reader of a file the caller names
/// goes through here (or [`resolve_file_path`] for a context of its own).
///
/// The argument is parsed before any context is built or any file is
/// touched, so malformed reserved syntax (a bare `@`, `&`, or `^`, or the
/// removed `!` sigil) is refused as `InvalidReference` even when a file of
/// that literal name exists; spell such a file `./@`.
///
/// ## Errors
///
/// A [`DocumentArgumentError`] carrying the failure class when the argument
/// is not valid reference syntax, fails to resolve, or matches no file; the
/// launch context builder's error when it cannot be built.
pub fn open_argument(raw_path: &Path, request: &MdRequest) -> Result<OpenedArgument> {
    let reference = parse_argument(raw_path)?;
    let path = resolve_reference(raw_path, &reference, request.launch_context()?)?;
    Ok(OpenedArgument { reference, path })
}

/// Parses a file argument with the shared reference grammar.
///
/// ## Errors
///
/// A [`DocumentArgumentError`] of class `InvalidReference` for text that is
/// not a file reference.
pub fn parse_argument(raw_path: &Path) -> Result<FileReference> {
    let raw = raw_path.to_string_lossy();
    FileReference::new(&raw)
        .map_err(|source| DocumentArgumentError::new(raw.into_owned(), source).into())
}

/// Resolves a file argument through biscuit-file's `FileReference` system in
/// `context`.
///
/// `@`, `&`, `^`, `~`, `{{VAR}}`, and relative forms resolve exactly as they
/// would in a document opened at the context's directory, so a relative
/// argument may not climb out of the launch repository (`InvalidReference`);
/// an absolute path may name any file. There is no plain-path fallback: text
/// that is not valid reference syntax is `InvalidReference`.
///
/// ## Errors
///
/// Returns a [`DocumentArgumentError`] carrying the failure class when the
/// argument does not parse, fails to resolve, or matches no file.
pub fn resolve_file_path(raw_path: &Path, context: &FileResolutionContext) -> Result<PathBuf> {
    let reference = parse_argument(raw_path)?;
    resolve_reference(raw_path, &reference, context)
}

fn resolve_reference(
    raw_path: &Path,
    reference: &FileReference,
    context: &FileResolutionContext,
) -> Result<PathBuf> {
    let argument = || raw_path.to_string_lossy().into_owned();
    match reference.resolve_in_context(context) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(DocumentArgumentError::no_match(argument()).into()),
        Err(source) => Err(DocumentArgumentError::new(argument(), source).into()),
    }
}

/// A file argument that did not resolve to a file.
///
/// Rendered as an error block whose `failure` row names the
/// [`ResolutionFailure`] class (see `darkmatter/docs/errors/file-reference-failures.md`).
#[derive(Debug)]
pub struct DocumentArgumentError {
    argument: String,
    failure: ResolutionFailure,
    source: Option<FileReferenceError>,
}

impl std::fmt::Display for DocumentArgumentError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Failed to load file: {:?}", self.argument)
    }
}

impl std::error::Error for DocumentArgumentError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|source| source as &(dyn std::error::Error + 'static))
    }
}

impl DocumentArgumentError {
    /// The error for `argument`, classed by `source`.
    pub fn new(argument: String, source: FileReferenceError) -> Self {
        Self { argument, failure: source.resolution_failure(), source: Some(source) }
    }

    /// A reference that resolved without error but matched no file.
    pub fn no_match(argument: String) -> Self {
        Self { argument, failure: ResolutionFailure::NoMatch, source: None }
    }

    /// The failure class.
    pub fn resolution_failure(&self) -> ResolutionFailure {
        self.failure
    }
}

impl BlockError for DocumentArgumentError {
    fn status_block(&self, _term: &biscuit_terminal::terminal::Terminal) -> StatusBlock {
        let mut body = vec![Prose::new(format!(
            "The argument <cyan>{}</cyan> did not resolve to a file.",
            Prose::escape_text(&self.argument)
        ))];
        if let Some(source) = &self.source {
            body.push(Prose::new(Prose::escape_text(&source.to_string())));
        }
        body.push(resolution_failure_row(self.failure));
        let hint = if self.failure == ResolutionFailure::InvalidReference {
            "A relative path stays inside the repository; a file whose name starts with `@`, `&`, `^`, or `!` is spelled with `./` (`./@`)."
        } else {
            "Check the path, or the sigil: `@` magic, `&` repository root, `^` repository-scoped."
        };
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("FileReferenceError", "file argument not resolved"))
            .body(body)
            .hint(hint)
    }
}

/// An opened document that a route can only inspect inside a repository
/// (`md schema triggers`) lies in none. Its class is `MissingContext`.
#[derive(Debug)]
pub struct DocumentOutsideRepository {
    pub document: PathBuf,
}

impl std::fmt::Display for DocumentOutsideRepository {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "no repository boundary found for `{}`", self.document.display())
    }
}

impl std::error::Error for DocumentOutsideRepository {}

impl BlockError for DocumentOutsideRepository {
    fn status_block(&self, _term: &biscuit_terminal::terminal::Terminal) -> StatusBlock {
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("FileReferenceError", "document outside every repository"))
            .body(vec![
                Prose::new(format!(
                    "No repository contains <cyan>{}</cyan>.",
                    Prose::escape_text(&self.document.display().to_string())
                )),
                resolution_failure_row(ResolutionFailure::MissingContext),
            ])
            .hint("Trigger schemas are discovered within a repository.")
    }
}

/// Views `err` as one of the CLI's own [`BlockError`] types.
pub fn as_block_error<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    err.downcast_ref::<DocumentArgumentError>()
        .map(|error| error as &(dyn BlockError + 'static))
        .or_else(|| {
            err.downcast_ref::<DocumentOutsideRepository>()
                .map(|error| error as &(dyn BlockError + 'static))
        })
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Markdown::try_from_content(buffer).map_err(Into::into)
}

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
/// A path resolves through [`resolve_file_path`] against the request's launch
/// context, so `@`, `&`, `^`, `~`, and `{{VAR}}` arguments work. `-` (or no
/// path with piped input) reads stdin and builds no context.
pub fn load_markdown(path: Option<&PathBuf>, request: &MdRequest) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            let resolved = resolve_file_path(p, request.launch_context()?)?;
            Markdown::try_from(resolved.as_path())
                .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))
        }
    } else {
        // No path provided - check if stdin has data
        if io::stdin().is_terminal() {
            // Interactive terminal - no input available
            Err(eyre!("No input file provided. Use `md --help` for usage."))
        } else {
            // Piped input available
            read_from_stdin()
        }
    }
}

/// Resolves and reads a Markdown file while retaining its exact UTF-8 source.
///
/// This file-only path is for commands that must write back without losing
/// authored formatting. The returned [`Markdown`] is parsed from the same text
/// returned to the caller.
pub fn load_markdown_text(
    path: &Path,
    request: &MdRequest,
) -> Result<(PathBuf, String, Markdown)> {
    if path.to_str() == Some("-") {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }
    let resolved = resolve_file_path(path, request.launch_context()?)?;
    let source = std::fs::read_to_string(&resolved)
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    let markdown = Markdown::try_from_content(source.clone())
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    Ok((resolved, source, markdown))
}

/// Resolves a file argument through biscuit-file's `FileReference` system in
/// `context`.
///
/// `@`, `&`, `^`, `~`, `{{VAR}}`, and relative forms resolve exactly as they
/// would in a document opened at the context's directory, so a relative
/// argument may not climb out of the launch repository (`InvalidReference`);
/// an absolute path may name any file. An argument that is not valid
/// reference syntax is returned unchanged as a plain path.
///
/// ## Errors
///
/// Returns a [`DocumentArgumentError`] carrying the failure class when the
/// reference fails to resolve or matches no file.
pub fn resolve_file_path(raw_path: &Path, context: &FileResolutionContext) -> Result<PathBuf> {
    let raw = raw_path.to_string_lossy();
    let Ok(reference) = FileReference::new(&raw) else {
        // Not a valid file reference syntax — treat as plain path
        return Ok(raw_path.to_path_buf());
    };
    match reference.resolve_in_context(context) {
        Ok(Some(path)) => Ok(path),
        Ok(None) => Err(DocumentArgumentError {
            argument: raw.into_owned(),
            failure: ResolutionFailure::NoMatch,
            source: None,
        }
        .into()),
        Err(source) => Err(DocumentArgumentError {
            argument: raw.into_owned(),
            failure: source.resolution_failure(),
            source: Some(source),
        }
        .into()),
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
        StatusBlock::new(StatusState::Error)
            .error_header(ErrorHeader::new("FileReferenceError", "file argument not resolved"))
            .body(body)
            .hint("Check the path, or the sigil: `@` magic, `&` repository root, `^` repository-scoped.")
    }
}

/// Views `err` as one of the CLI's own [`BlockError`] types.
pub fn as_block_error<'a>(
    err: &'a (dyn std::error::Error + 'static),
) -> Option<&'a (dyn BlockError + 'static)> {
    err.downcast_ref::<DocumentArgumentError>()
        .map(|error| error as &(dyn BlockError + 'static))
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Markdown::try_from_content(buffer).map_err(Into::into)
}

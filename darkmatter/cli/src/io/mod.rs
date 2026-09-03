//! Shared file-input helpers used by every subcommand.

use biscuit_file::FileReference;
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use std::io::{self, IsTerminal, Read};
use std::path::PathBuf;

/// Loads markdown from a file path or stdin.
///
/// Paths are resolved through biscuit-file's `FileReference` system, which
/// supports `@`-prefixed magic paths (e.g. `@prompts/feature.md`), vault
/// references, and other reference syntaxes. Plain paths and `-` (stdin)
/// are handled as before.
pub fn load_markdown(path: Option<&PathBuf>) -> Result<Markdown> {
    if let Some(p) = path {
        if p.to_str() == Some("-") {
            // Explicit stdin marker
            read_from_stdin()
        } else {
            let resolved = resolve_file_path(p)?;
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
pub fn load_markdown_text(path: &PathBuf) -> Result<(PathBuf, String, Markdown)> {
    if path.to_str() == Some("-") {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }
    let resolved = resolve_file_path(path)?;
    let source = std::fs::read_to_string(&resolved)
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    let markdown = Markdown::try_from_content(source.clone())
        .wrap_err_with(|| format!("Failed to read file: {:?}", resolved))?;
    Ok((resolved, source, markdown))
}

/// Resolves a file path through biscuit-file's `FileReference` system.
///
/// If the path contains `@`-prefixed magic references or other FileReference
/// syntax, it will be resolved accordingly. Plain paths are returned as-is
/// (made absolute if relative).
pub fn resolve_file_path(raw_path: &PathBuf) -> Result<PathBuf> {
    let raw = raw_path.to_string_lossy();
    match FileReference::new(&raw) {
        Ok(file_ref) => {
            let resolved = file_ref
                .resolve()
                .wrap_err_with(|| format!("Failed to resolve file reference: {:?}", raw_path))?;
            match resolved {
                Some(p) => Ok(p),
                None => {
                    // FileReference couldn't resolve it — fall back to raw path
                    Err(eyre!("Failed to load file: {:?}", raw_path))
                }
            }
        }
        Err(_) => {
            // Not a valid file reference syntax — treat as plain path
            Ok(raw_path.clone())
        }
    }
}

/// Reads markdown content from stdin.
fn read_from_stdin() -> Result<Markdown> {
    let mut buffer = String::new();
    io::stdin()
        .read_to_string(&mut buffer)
        .wrap_err("Failed to read from stdin")?;
    Markdown::try_from_content(buffer).map_err(Into::into)
}

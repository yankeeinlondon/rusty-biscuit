//! `md clean` subcommand implementation.

use crate::delta::print_delta;
use crate::io::{load_markdown, resolve_file_path};
use color_eyre::eyre::{Context, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::ListSpacingMode;
use std::path::PathBuf;
use tracing::instrument;

/// Clean markdown formatting, optionally saving in place and printing a delta report.
/// Converts CLI flags to a `ListSpacingMode`.
pub(crate) fn resolve_list_spacing(compact: bool, loose: bool) -> ListSpacingMode {
    match (compact, loose) {
        (true, _) => ListSpacingMode::Compact,
        (_, true) => ListSpacingMode::Loose,
        _ => ListSpacingMode::Normal,
    }
}

#[instrument(skip_all)]
pub fn run_clean(
    input: Option<&PathBuf>,
    save: bool,
    indent: Option<usize>,
    list_spacing: ListSpacingMode,
    verbose: bool,
) -> Result<()> {
    if !save {
        let mut md = load_markdown(input)?;
        apply_cleanup(&mut md, indent, list_spacing);
        println!("{}", md.as_string());
        return Ok(());
    }

    let input_path = input
        .ok_or_else(|| eyre!("--save requires an input file path (stdin is not supported)"))?;
    if input_path.to_str() == Some("-") {
        return Err(eyre!(
            "--save requires an input file path (stdin is not supported)"
        ));
    }

    let resolved = resolve_file_path(input_path)?;
    let original = load_markdown(Some(input_path))?;
    let mut cleaned = original.clone();
    apply_cleanup(&mut cleaned, indent, list_spacing);

    let delta = original.delta(&cleaned);
    if !delta.is_unchanged() {
        std::fs::write(&resolved, cleaned.as_string())
            .wrap_err_with(|| format!("Failed to write cleaned markdown to {:?}", resolved))?;
    }

    print_delta(&delta, verbose, &original, &cleaned);
    Ok(())
}

fn apply_cleanup(md: &mut Markdown, indent: Option<usize>, mode: ListSpacingMode) {
    match (indent, mode) {
        (Some(size), ListSpacingMode::Compact) => md.cleanup_with_indent_compact(size),
        (Some(size), ListSpacingMode::Loose) => md.cleanup_with_indent_loose(size),
        (Some(size), ListSpacingMode::Normal) => md.cleanup_with_indent(size),
        (None, ListSpacingMode::Compact) => md.cleanup_compact(),
        (None, ListSpacingMode::Loose) => md.cleanup_loose(),
        (None, ListSpacingMode::Normal) => md.cleanup(),
    };
}

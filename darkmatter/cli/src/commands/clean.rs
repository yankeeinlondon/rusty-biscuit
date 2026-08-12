//! `md clean` subcommand implementation.
//!
//! `clean` runs a raw-source frontmatter pass *before* it builds a
//! [`Markdown`], so documents whose frontmatter does not parse can still be
//! repaired. Output is assembled from the repaired frontmatter block verbatim
//! plus the cleaned body, which keeps untouched frontmatter ranges — comments,
//! key order, quote style — byte-identical rather than reserialized.

mod frontmatter_repair;

use crate::io::resolve_file_path;
use color_eyre::eyre::{Context, Result, eyre};
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::cleanup::{
    DEFAULT_INDENT, ListSpacingMode, cleanup_content_with_indent_compact_preserving_incidental,
    cleanup_content_with_indent_loose_preserving_incidental,
    cleanup_content_with_indent_preserving_incidental, reflow_to_width,
};
use darkmatter::markdown::delta::DeltaReport;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::Markdown;
use std::io::{self, IsTerminal, Read, Write};
use std::path::PathBuf;
use tracing::instrument;

pub use frontmatter_repair::CleanSchemaFlags;
use frontmatter_repair::{repair_frontmatter, report_suggestions};

/// Everything `md clean` needs beyond its input path.
#[derive(Debug, Clone)]
pub struct CleanOptions {
    /// Write the cleaned document back in place and print a delta report.
    pub save: bool,
    pub indent: Option<usize>,
    pub list_spacing: ListSpacingMode,
    pub fixed_width: Option<usize>,
    pub ignore_incidental_newlines: bool,
    pub verbose: bool,
    /// Emit the machine-readable diagnostic envelope on STDOUT instead of the
    /// cleaned document or the delta report.
    pub json: bool,
    pub schema: CleanSchemaFlags,
}

impl Default for CleanOptions {
    fn default() -> Self {
        Self {
            save: false,
            indent: None,
            list_spacing: ListSpacingMode::Normal,
            fixed_width: None,
            ignore_incidental_newlines: false,
            verbose: false,
            json: false,
            schema: CleanSchemaFlags::default(),
        }
    }
}

/// Converts CLI flags to a `ListSpacingMode`.
pub(crate) fn resolve_list_spacing(compact: bool, loose: bool) -> ListSpacingMode {
    match (compact, loose) {
        (true, _) => ListSpacingMode::Compact,
        (_, true) => ListSpacingMode::Loose,
        _ => ListSpacingMode::Normal,
    }
}

#[instrument(skip_all)]
pub fn run_clean(input: Option<&PathBuf>, options: &CleanOptions) -> Result<()> {
    let resolved = resolve_input_path(input)?;
    // Resolving the destination up front means the stdin-under-`--save`
    // rejection happens once, before any read, and the write branch below
    // needs no second guard.
    let save_path = match (options.save, &resolved) {
        (true, None) => {
            return Err(eyre!(
                "--save requires an input file path (stdin is not supported)"
            ));
        }
        (true, Some(path)) => Some(path.clone()),
        (false, _) => None,
    };
    let raw = read_source(resolved.as_ref())?;

    let repair = repair_frontmatter(&raw, resolved.as_deref(), &options.schema)?;

    // Building the document is what enforces the unrepairable-YAML contract.
    // JSON mode reports that domain failure through its machine channel before
    // preserving the existing exit code; human mode keeps the normal renderer.
    let original = match Markdown::try_from_content(repair.source.clone()) {
        Ok(markdown) => markdown,
        Err(_error) if options.json => {
            let report = repair.json_report(resolved, &raw, false);
            println!("{}", serde_json::to_string_pretty(&report)?);
            io::stdout().flush()?;
            std::process::exit(1);
        }
        Err(error) => return Err(error.into()),
    };
    let mut cleaned = original.clone();
    apply_cleanup(
        &mut cleaned,
        options.indent,
        options.list_spacing,
        options.fixed_width,
        options.ignore_incidental_newlines,
    );
    let output = assemble(&repair.source, &cleaned);
    let changed = output != raw;

    let Some(path) = save_path else {
        if options.json {
            let report = repair.json_report(resolved, &raw, changed);
            println!("{}", serde_json::to_string_pretty(&report)?);
        } else {
            report_suggestions(&repair.diagnostics, &Terminal::new());
            print!("{}", output);
        }
        return Ok(());
    };

    if output != raw {
        std::fs::write(&path, &output)
            .wrap_err_with(|| format!("Failed to write cleaned markdown to {:?}", path))?;
    }

    if options.json {
        let report = repair.json_report(resolved, &raw, changed);
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    let terminal = Terminal::new();
    report_suggestions(&repair.diagnostics, &terminal);
    if repair.repaired {
        // `original` is built from the *repaired* source, so the delta below is
        // blind to the frontmatter rewrite and would report "No changes" for a
        // file that was in fact modified. Say so at the text level instead of
        // pretending a baseline `Markdown` existed for unparseable input.
        println!(
            "{}",
            Prose::new("Frontmatter: repairs applied").render(&terminal)
        );
    }
    let delta = original.delta(&cleaned);
    let mut report = DeltaReport::new(delta).with_documents(original, cleaned);
    if options.verbose {
        report = report.verbose();
    }
    print!("{}", report.render(&terminal));
    Ok(())
}

/// Rebuilds the document from its repaired frontmatter block and cleaned body.
///
/// The block is copied verbatim out of `source` rather than reserialized from
/// the parsed frontmatter, which is what makes repairs format-preserving and
/// makes `md clean` a fixed point: re-cleaning this output reproduces it byte
/// for byte. Authored delimiter whitespace and line terminators remain part of
/// that preserved block; only accepted YAML edits and ordinary body cleanup
/// can change the assembled document.
fn assemble(source: &str, cleaned: &Markdown) -> String {
    match extract_frontmatter_block(source) {
        Ok(Some(extraction)) if !extraction.yaml.trim().is_empty() => {
            let mut out = String::with_capacity(source.len());
            out.push_str(&source[extraction.block_span.clone()]);
            out.push_str(cleaned.content());
            out
        }
        _ => cleaned.content().to_string(),
    }
}

/// Resolves the input path through biscuit-file's reference system.
///
/// ## Returns
///
/// `None` when the document comes from stdin — either no path at all, or the
/// explicit `-` marker.
fn resolve_input_path(input: Option<&PathBuf>) -> Result<Option<PathBuf>> {
    match input {
        None => Ok(None),
        Some(path) if path.to_str() == Some("-") => Ok(None),
        Some(path) => resolve_file_path(path).map(Some),
    }
}

/// Reads the document's raw bytes as text, from a path or from stdin.
fn read_source(path: Option<&PathBuf>) -> Result<String> {
    match path {
        Some(path) => std::fs::read_to_string(path)
            .wrap_err_with(|| format!("Failed to read file: {:?}", path)),
        None => {
            if io::stdin().is_terminal() {
                return Err(eyre!("No input file provided. Use `md --help` for usage."));
            }
            let mut buffer = String::new();
            io::stdin()
                .read_to_string(&mut buffer)
                .wrap_err("Failed to read from stdin")?;
            Ok(buffer)
        }
    }
}

fn apply_cleanup(
    md: &mut Markdown,
    indent: Option<usize>,
    mode: ListSpacingMode,
    fixed_width: Option<usize>,
    ignore_incidental_newlines: bool,
) {
    if ignore_incidental_newlines {
        apply_cleanup_no_strip(md, indent, mode);
        return;
    }

    match (indent, mode) {
        (Some(size), ListSpacingMode::Compact) => md.cleanup_with_indent_compact(size),
        (Some(size), ListSpacingMode::Loose) => md.cleanup_with_indent_loose(size),
        (Some(size), ListSpacingMode::Normal) => md.cleanup_with_indent(size),
        (None, ListSpacingMode::Compact) => md.cleanup_compact(),
        (None, ListSpacingMode::Loose) => md.cleanup_loose(),
        (None, ListSpacingMode::Normal) => md.cleanup(),
    };

    if let Some(width) = fixed_width {
        let reflowed = reflow_to_width(md.content(), width);
        *md.content_mut() = reflowed;
    }
}

fn apply_cleanup_no_strip(md: &mut Markdown, indent: Option<usize>, mode: ListSpacingMode) {
    let indent_size = indent.unwrap_or(DEFAULT_INDENT);
    let cleaned = match mode {
        ListSpacingMode::Compact => {
            cleanup_content_with_indent_compact_preserving_incidental(md.content(), indent_size)
        }
        ListSpacingMode::Loose => {
            cleanup_content_with_indent_loose_preserving_incidental(md.content(), indent_size)
        }
        ListSpacingMode::Normal => {
            cleanup_content_with_indent_preserving_incidental(md.content(), indent_size)
        }
    };
    *md.content_mut() = cleaned;
}

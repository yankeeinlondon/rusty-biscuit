//! `md clean` subcommand implementation.
//!
//! `clean` runs the library's raw-source frontmatter repair
//! ([`repair_frontmatter`]) *before* it builds a [`Markdown`], so documents
//! whose frontmatter does not parse can still be repaired. Output is assembled
//! from the repaired frontmatter block verbatim plus the cleaned body, which
//! keeps untouched frontmatter ranges — comments, key order, quote style —
//! byte-identical rather than reserialized.
//!
//! This module owns what is specific to the `md` binary: schema flags and the
//! request contexts they resolve in, terminal suggestions, and (in [`json`])
//! the version-1 JSON envelope.

mod json;

use crate::io::{OpenedArgument, open_argument};
use crate::request::MdRequest;
use color_eyre::eyre::{Context, Report, Result, eyre};
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use darkmatter::markdown::cleanup::diagnostic::CleanDiagnostic;
use darkmatter::markdown::cleanup::frontmatter::{
    FrontmatterRepair, FrontmatterRepairError, repair_frontmatter,
};
use darkmatter::markdown::cleanup::{
    DEFAULT_INDENT, ListSpacingMode, cleanup_content_with_indent_compact_preserving_incidental,
    cleanup_content_with_indent_loose_preserving_incidental,
    cleanup_content_with_indent_preserving_incidental, reflow_to_width,
};
use darkmatter::markdown::delta::DeltaReport;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::schemas::CleanSchemaConfig;
use darkmatter::markdown::Markdown;
use std::io::{self, IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use tracing::instrument;

use json::json_report;

/// Schema controls for `md clean`, mirroring the `md compose` flags of the
/// same name.
#[derive(Debug, Clone, Default)]
pub struct CleanSchemaFlags {
    /// `--schema PATH`: an explicit schema that replaces the document's own
    /// `$schema` layer. Baseline and trigger layers still apply beneath it.
    pub schema: Option<PathBuf>,
    /// `--baseline-schema PATH`
    pub baseline_schema: Option<PathBuf>,
    /// `--no-baseline-schema`
    pub no_baseline_schema: bool,
    /// `--no-trigger-schemas`
    pub no_trigger_schemas: bool,
}

/// The `md` request a `clean` run resolves schema references in, and the
/// reference that opened the document (for a quoted `~/…` argument).
struct DocumentRequest<'a> {
    request: &'a MdRequest,
    opening: Option<&'a biscuit_file::FileReference>,
}

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
pub fn run_clean(input: Option<&PathBuf>, options: &CleanOptions, request: &MdRequest) -> Result<()> {
    let opened = open_input(input, request)?;
    let resolved = opened.as_ref().map(|opened| opened.path().to_path_buf());
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

    let document_request = DocumentRequest { request, opening: opened.as_ref().map(|opened| opened.reference()) };
    let repair = repair_document(&raw, resolved.as_deref(), &options.schema, &document_request)?;

    // Building the document is what enforces the unrepairable-YAML contract.
    // JSON mode reports that domain failure through its machine channel before
    // preserving the existing exit code; human mode keeps the normal renderer.
    let original = match Markdown::try_from_content(repair.source.clone()) {
        Ok(markdown) => markdown,
        Err(_error) if options.json => {
            let report = json_report(&repair, resolved, &raw, false);
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
            let report = json_report(&repair, resolved, &raw, changed);
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
        let report = json_report(&repair, resolved, &raw, changed);
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

/// Runs the library frontmatter repair with this invocation's schema
/// configuration, keeping each failure's original error so messages are
/// unchanged.
fn repair_document(
    raw: &str,
    document_path: Option<&Path>,
    flags: &CleanSchemaFlags,
    request: &DocumentRequest<'_>,
) -> Result<FrontmatterRepair> {
    repair_frontmatter(raw, document_path, || {
        schema_config(flags, document_path, request)
    })
    .map_err(|error| match error {
        FrontmatterRepairError::Extraction(error) => Report::new(error),
        FrontmatterRepairError::Schema(error) => Report::new(error),
        FrontmatterRepairError::Context(error) => error,
    })
}

/// Builds the schema configuration for this `clean` invocation.
///
/// The library calls this at most once and only behind a non-empty
/// frontmatter block, so contexts are built — and repository discovery paid
/// for — only when the schema tier runs.
fn schema_config(
    flags: &CleanSchemaFlags,
    document_path: Option<&Path>,
    request: &DocumentRequest<'_>,
) -> Result<CleanSchemaConfig> {
    // A file resolves through its own context; stdin through the launch one.
    let context = match document_path {
        Some(path) => request.request.document_context(request.opening, path)?,
        None => request.request.launch_context()?.clone(),
    };
    let mut config = CleanSchemaConfig::new(context);

    if flags.no_baseline_schema || crate::commands::compose::env_disables_baseline_schema() {
        config = config.without_baseline_schema();
    } else if let Some(path) = &flags.baseline_schema {
        config = config.with_baseline_schema_file(crate::io::resolve_file_path(
            path,
            request.request.launch_context()?,
        )?);
    }

    if let Some(schema) = &flags.schema {
        // A string `$schema` is biscuit-file's file-reference form, which is
        // exactly what a path-valued `--schema` means.
        config = config.with_schema_override(serde_json::Value::String(
            schema.to_string_lossy().into_owned(),
        ));
    }

    Ok(config.with_trigger_schemas(!flags.no_trigger_schemas))
}

/// Renders report-only findings to STDERR as suggestions.
///
/// Suggestions deliberately do not affect the exit code: `md clean` runs in
/// pre-commit hooks and agent loops, where a new failure mode would be more
/// disruptive than the smells it reports.
fn report_suggestions(diagnostics: &[CleanDiagnostic], terminal: &Terminal) {
    let mut list = UnorderedList::empty();
    let mut reported = 0_usize;

    for entry in diagnostics {
        if entry.classification.is_auto_apply_eligible() {
            continue;
        }
        reported += 1;
        list.add(Prose::new(format!(
            "<b>{}</b>. {}",
            escape_prose(entry.code.as_str()),
            escape_prose(&entry.message)
        )));
    }

    if reported == 0 {
        return;
    }

    // `Prose`/`UnorderedList` render without a trailing newline, so the header
    // and the list need their own line breaks or they arrive glued together.
    eprintln!(
        "{}",
        Prose::new("frontmatter suggestions (not applied)").render(terminal)
    );
    eprintln!("{}", list.render(terminal));
}

/// Escape text so it renders exactly as written inside Prose markup; code
/// spans it marks with backticks stay literal.
fn escape_prose(text: &str) -> String {
    Prose::escape_text_outside_code_spans(text)
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

/// Opens the input argument through the shared reference reader.
///
/// ## Returns
///
/// `None` when the document comes from stdin — either no path at all, or the
/// explicit `-` marker.
fn open_input(input: Option<&PathBuf>, request: &MdRequest) -> Result<Option<OpenedArgument>> {
    match input {
        None => Ok(None),
        Some(path) if path.to_str() == Some("-") => Ok(None),
        Some(path) => open_argument(path, request).map(Some),
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

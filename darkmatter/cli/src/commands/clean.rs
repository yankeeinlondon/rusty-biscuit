//! `md clean` subcommand implementation.
//!
//! `clean` hands the raw document to the library's clean transaction
//! ([`clean_document`]): frontmatter repair, the document's `style.markdown`
//! rules, and guarded body formatting, checked once as a whole. The emitted
//! text is either the original document byte for byte or a fully accepted
//! result; untouched frontmatter (comments, key order, quote style, empty
//! blocks) and line terminators are never reserialized.
//!
//! Per outcome:
//!
//! - `Changed`/`Unchanged`: the text is printed, or under `--save` written
//!   only when it differs, followed by a delta report.
//! - `Declined`: the original is printed and `--save` writes nothing; the
//!   `clean.fidelity-fallback` finding is reported and the exit code is `0`.
//! - `Invalid`: Darkmatter cannot load the frontmatter that would be
//!   emitted; nothing is saved and the run fails (exit `1`).
//!
//! This module owns what is specific to the `md` binary: flags as explicit
//! rule overrides, schema flags and the request contexts they resolve in,
//! terminal findings, and (in [`json`]) the version-1 JSON envelope.

mod json;

use crate::io::{OpenedArgument, open_argument};
use crate::request::MdRequest;
use biscuit_terminal::components::list::UnorderedList;
use biscuit_terminal::components::prose::Prose;
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::{Context, Report, Result, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::cleanup::diagnostic::{CleanDiagnostic, CleanStage};
use darkmatter::markdown::cleanup::frontmatter::FrontmatterRepairError;
use darkmatter::markdown::cleanup::rules::{
    FixedWidth, Indentation, Leaf, ListItemPadding, MarkdownRuleOverrides,
};
use darkmatter::markdown::cleanup::{CleanOutcome, CleanRequest, CleanResult, clean_document};
use darkmatter::markdown::delta::DeltaReport;
use darkmatter::markdown::extract_frontmatter_block;
use darkmatter::markdown::schemas::CleanSchemaConfig;
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
///
/// The formatting fields are explicit overrides: `None`/`false` leaves the
/// rule to the document's `style.markdown` and then the runtime default.
#[derive(Debug, Clone, Default)]
pub struct CleanOptions {
    /// Write the cleaned document back in place and print a delta report.
    pub save: bool,
    /// `--indent N` (2, 4, or 8): the `indentation` rule.
    pub indent: Option<usize>,
    /// `--compact` (`None`) or `--loose` (`One`): the `list-item-padding` rule.
    pub list_item_padding: Option<ListItemPadding>,
    /// `--fixed-width N`: the `fixed-width` rule, which implies `unwrap`.
    pub fixed_width: Option<usize>,
    /// `--ignore-incidental-newlines`: `unwrap: false` and `fixed-width:
    /// false`, overriding a width the document selects.
    pub ignore_incidental_newlines: bool,
    pub verbose: bool,
    /// Emit the machine-readable diagnostic envelope on STDOUT instead of the
    /// cleaned document or the delta report.
    pub json: bool,
    pub schema: CleanSchemaFlags,
}

/// The `list-item-padding` override `--compact`/`--loose` select, if any.
pub(crate) fn resolve_list_spacing(compact: bool, loose: bool) -> Option<ListItemPadding> {
    match (compact, loose) {
        (true, _) => Some(ListItemPadding::None),
        (_, true) => Some(ListItemPadding::One),
        _ => None,
    }
}

/// The rule overrides this invocation's flags state explicitly.
///
/// ## Errors
///
/// An `indent` other than 2, 4, or 8, or a `fixed_width` outside `u16`;
/// the command-line parsers already reject both.
fn rule_overrides(options: &CleanOptions) -> Result<MarkdownRuleOverrides> {
    let mut overrides = MarkdownRuleOverrides::default();
    if let Some(spaces) = options.indent {
        let indentation = Indentation::from_spaces(spaces)
            .ok_or_else(|| eyre!("indent must be one of: 2, 4, 8 (got {spaces})"))?;
        overrides.indentation = Leaf::Value(indentation);
    }
    if let Some(padding) = options.list_item_padding {
        overrides.list_item_padding = Leaf::Value(padding);
    }
    if let Some(width) = options.fixed_width {
        let columns = u16::try_from(width)
            .map_err(|_| eyre!("--fixed-width must be between 1 and 1000 (got {width})"))?;
        overrides.fixed_width = Leaf::Value(FixedWidth::Columns(columns));
    }
    if options.ignore_incidental_newlines {
        overrides.unwrap = Leaf::Value(false);
        overrides.fixed_width = Leaf::Value(FixedWidth::Off);
    }
    Ok(overrides)
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
    let overrides = rule_overrides(options)?;

    let document_request = DocumentRequest { request, opening: opened.as_ref().map(|opened| opened.reference()) };
    let result = clean(&raw, &overrides, resolved.as_deref(), &options.schema, &document_request)?;

    if options.json {
        let report = json_report(&result, resolved, &raw);
        println!("{}", serde_json::to_string_pretty(&report)?);
        if result.outcome == CleanOutcome::Invalid {
            io::stdout().flush()?;
            std::process::exit(1);
        }
        if let Some(path) = save_path {
            write_if_changed(&path, &result)?;
        }
        return Ok(());
    }

    if result.outcome == CleanOutcome::Invalid {
        // The error the document itself reports keeps the message today's
        // users see for unrepairable YAML.
        return Err(match Markdown::try_from_content(raw) {
            Err(error) => error.into(),
            Ok(_) => eyre!("the frontmatter is not valid YAML"),
        });
    }

    let terminal = Terminal::new();
    report_findings(&result.diagnostics, &terminal);

    let Some(path) = save_path else {
        print!("{}", result.text);
        return Ok(());
    };
    if result.outcome == CleanOutcome::Declined {
        return Ok(());
    }

    write_if_changed(&path, &result)?;
    if !result.applied.is_empty() {
        // The delta below compares bodies; frontmatter repairs are reported
        // here instead.
        println!(
            "{}",
            Prose::new("Frontmatter: repairs applied").render(&terminal)
        );
    }
    let cleaned = Markdown::try_from_content(result.text.clone())?;
    let original = baseline(&raw, &result.text)?;
    let delta = original.delta(&cleaned);
    if !result.applied.is_empty() && delta.is_unchanged() {
        // A body-only "No changes" would contradict the repair notice above.
        return Ok(());
    }
    let mut report = DeltaReport::new(delta).with_documents(original, cleaned);
    if options.verbose {
        report = report.verbose();
    }
    print!("{}", report.render(&terminal));
    Ok(())
}

/// Writes a `Changed` result to `path`; any other outcome leaves the file
/// untouched.
fn write_if_changed(path: &Path, result: &CleanResult) -> Result<()> {
    if result.outcome == CleanOutcome::Changed {
        std::fs::write(path, &result.text)
            .wrap_err_with(|| format!("Failed to write cleaned markdown to {:?}", path))?;
    }
    Ok(())
}

/// The delta report's "before" document.
///
/// Raw frontmatter that does not parse was repaired, so the baseline pairs
/// the cleaned frontmatter with the raw body and the delta covers only the
/// body.
fn baseline(raw: &str, cleaned: &str) -> Result<Markdown> {
    if let Ok(markdown) = Markdown::try_from_content(raw.to_string()) {
        return Ok(markdown);
    }
    // Repairs edit only the YAML, so the raw body is a suffix of both texts.
    let raw_body = &raw[body_offset(raw)..];
    let mut spliced = cleaned[..body_offset(cleaned)].to_string();
    spliced.push_str(raw_body);
    Ok(Markdown::try_from_content(spliced)?)
}

fn body_offset(text: &str) -> usize {
    match extract_frontmatter_block(text) {
        Ok(Some(extraction)) => extraction.body_span.start,
        _ => 0,
    }
}

/// Runs the library clean transaction with this invocation's overrides and
/// schema configuration, keeping each failure's original error so messages
/// are unchanged.
fn clean(
    raw: &str,
    overrides: &MarkdownRuleOverrides,
    document_path: Option<&Path>,
    flags: &CleanSchemaFlags,
    request: &DocumentRequest<'_>,
) -> Result<CleanResult> {
    let clean_request = CleanRequest {
        source: raw,
        overrides,
        workspace: None,
        document_path,
    };
    clean_document(&clean_request, || schema_config(flags, document_path, request)).map_err(
        |error| match error {
            FrontmatterRepairError::Extraction(error) => Report::new(error),
            FrontmatterRepairError::Schema(error) => Report::new(error),
            FrontmatterRepairError::Context(error) => error,
        },
    )
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

/// Renders report-only findings to STDERR: frontmatter suggestions, then
/// Markdown cleanup findings (`clean.invalid-rules`,
/// `clean.fidelity-fallback`).
///
/// Findings deliberately do not affect the exit code: `md clean` runs in
/// pre-commit hooks and agent loops, where a new failure mode would be more
/// disruptive than the smells it reports.
fn report_findings(diagnostics: &[CleanDiagnostic], terminal: &Terminal) {
    let frontmatter = diagnostics.iter().filter(|entry| {
        matches!(entry.stage, CleanStage::Syntax | CleanStage::Schema)
            && !entry.classification.is_auto_apply_eligible()
    });
    report_section("frontmatter suggestions (not applied)", frontmatter, terminal);

    let markdown = diagnostics
        .iter()
        .filter(|entry| matches!(entry.stage, CleanStage::Rules | CleanStage::Fidelity));
    report_section("markdown cleanup", markdown, terminal);
}

fn report_section<'a>(
    header: &str,
    entries: impl Iterator<Item = &'a CleanDiagnostic>,
    terminal: &Terminal,
) {
    let mut list = UnorderedList::empty();
    let mut reported = 0_usize;
    for entry in entries {
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
    eprintln!("{}", Prose::new(header).render(terminal));
    eprintln!("{}", list.render(terminal));
}

/// Escape text so it renders exactly as written inside Prose markup; code
/// spans it marks with backticks stay literal.
fn escape_prose(text: &str) -> String {
    Prose::escape_text_outside_code_spans(text)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_flags_override_nothing() {
        let overrides = rule_overrides(&CleanOptions::default()).unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn flags_become_explicit_overrides() {
        let options = CleanOptions {
            indent: Some(2),
            list_item_padding: resolve_list_spacing(true, false),
            fixed_width: Some(80),
            ..CleanOptions::default()
        };
        let overrides = rule_overrides(&options).unwrap();
        assert_eq!(overrides.indentation, Leaf::Value(Indentation::Two));
        assert_eq!(overrides.list_item_padding, Leaf::Value(ListItemPadding::None));
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Columns(80)));
        assert_eq!(overrides.unwrap, Leaf::Absent);
    }

    #[test]
    fn ignore_incidental_newlines_turns_unwrap_and_width_off() {
        let options = CleanOptions {
            ignore_incidental_newlines: true,
            ..CleanOptions::default()
        };
        let overrides = rule_overrides(&options).unwrap();
        assert_eq!(overrides.unwrap, Leaf::Value(false));
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Off));
    }

    #[test]
    fn unsupported_indent_is_an_error() {
        let options = CleanOptions {
            indent: Some(3),
            ..CleanOptions::default()
        };
        assert!(rule_overrides(&options).is_err());
    }

    #[test]
    fn list_spacing_flags_map_to_padding() {
        assert_eq!(resolve_list_spacing(false, false), None);
        assert_eq!(resolve_list_spacing(true, false), Some(ListItemPadding::None));
        assert_eq!(resolve_list_spacing(false, true), Some(ListItemPadding::One));
    }
}

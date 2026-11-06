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
//! A referenced schema that is missing, unreadable, or invalid fails the run
//! (exit `1`) before any outcome; a data output still reports the version-1
//! value, carrying the skipped-trigger `warnings` discovery retained.
//!
//! This module owns what is specific to the `md` binary: flags as explicit
//! rule overrides, schema flags and the request contexts they resolve in,
//! stderr findings, and the formatted outputs.

use crate::args::Cli;
use crate::commands::render::document_value;
use crate::commands::trigger_findings::emit_warnings;
use crate::io::{OpenedArgument, open_argument};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::output::report::failure_diagnostics;
use crate::output::{ResolvedOutput, stderr_terminal};
use crate::request::MdRequest;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use darkmatter::markdown::output::command::{CleanJsonReport, CommandValue, clean_report, clean_schema_failure_report, code_document};
use renderable::Width;
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
use darkmatter::markdown::schemas::{CleanSchemaConfig, InvalidTriggerFinding};
use std::io::{self, IsTerminal, Read};
use std::path::{Path, PathBuf};
use tracing::instrument;

/// Schema controls for `md clean`, mirroring the `md compose` flags of the
/// same name.
#[derive(Debug, Clone, Default)]
pub struct CleanSchemaFlags {
    /// `--schema PATH`: an explicit schema that replaces the document's own
    /// `$schema` layer. Baseline and trigger layers still apply beneath it.
    pub schema: Option<PathBuf>,
    /// `--baseline-schema PATH`
    pub baseline_schema: Option<PathBuf>,
    /// `--disable baseline-schema`
    pub no_baseline_schema: bool,
    /// `--disable schema-triggers`
    pub no_schema_triggers: bool,
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
    pub verbose: bool,
    /// `--output`: the cleaned document, or in a data format its diagnostics.
    pub output: Option<OutputTarget>,
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
/// An explicit `--width` overrides the document's own `fixed-width:` and
/// `unwrap:`; with no `--width` both stay the document's.
///
/// ## Errors
///
/// An `indent` other than 2, 4, or 8; the command-line parser already rejects it.
fn rule_overrides(options: &CleanOptions, width: Option<Width>) -> Result<MarkdownRuleOverrides> {
    let mut overrides = MarkdownRuleOverrides::default();
    if let Some(spaces) = options.indent {
        let indentation = Indentation::from_spaces(spaces)
            .ok_or_else(|| eyre!("indent must be one of: 2, 4, 8 (got {spaces})"))?;
        overrides.indentation = Leaf::Value(indentation);
    }
    if let Some(padding) = options.list_item_padding {
        overrides.list_item_padding = Leaf::Value(padding);
    }
    match width {
        None => {}
        Some(Width::Source) => {
            overrides.unwrap = Leaf::Value(false);
            overrides.fixed_width = Leaf::Value(FixedWidth::Off);
        }
        Some(Width::Unlimited) => {
            overrides.unwrap = Leaf::Value(true);
            overrides.fixed_width = Leaf::Value(FixedWidth::Off);
        }
        // The rule holds a `u16`; a wider value wraps nothing either way.
        Some(Width::Columns(columns)) => {
            overrides.fixed_width = Leaf::Value(FixedWidth::Columns(u16::try_from(columns).unwrap_or(u16::MAX)));
        }
    }
    Ok(overrides)
}

#[instrument(skip_all)]
pub fn run_clean(input: Option<&PathBuf>, options: &CleanOptions, cli: &Cli, request: &MdRequest) -> Result<()> {
    let run = CommandRun::begin(
        options.output.unwrap_or(OutputTarget::Markdown),
        cli.width,
        WidthRequest::Source,
        cli.verbose,
        DataShape::Object,
    )?
    .exact_markdown();
    let result = clean_value(input, options, cli, request, run.output());
    run.finish(result)
}

type CleanValue = CommandValue<Option<CleanJsonReport>>;

fn clean_value(
    input: Option<&PathBuf>,
    options: &CleanOptions,
    cli: &Cli,
    request: &MdRequest,
    output: &ResolvedOutput,
) -> Result<Completed<CleanValue>> {
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
    let overrides = rule_overrides(options, output.explicit_width.then_some(output.options.width))?;
    let is_data = output.target.is_data();

    let document_request = DocumentRequest { request, opening: opened.as_ref().map(|opened| opened.reference()) };
    let result = match clean(&raw, &overrides, resolved.as_deref(), &options.schema, &document_request) {
        Ok(result) => result,
        Err(CleanFailure { error, trigger_findings: None }) => return Err(error),
        Err(CleanFailure { error, trigger_findings: Some(findings) }) => {
            if is_data {
                let report = clean_schema_failure_report(&findings, resolved, &raw);
                return Ok(Completed::failure(data_value(report)).with_diagnostics(failure_diagnostics(&error)));
            }
            emit_warnings(&findings, &stderr_terminal());
            return Err(error);
        }
    };

    if is_data {
        let report = clean_report(&result, resolved, &raw);
        if result.outcome == CleanOutcome::Invalid {
            return Ok(Completed::failure(data_value(report)));
        }
        if let Some(path) = save_path {
            write_if_changed(&path, &result)?;
        }
        return Ok(Completed::success(data_value(report)));
    }

    let terminal = stderr_terminal();
    emit_warnings(&result.trigger_findings, &terminal);

    if result.outcome == CleanOutcome::Invalid {
        // The error the document itself reports keeps the message today's
        // users see for unrepairable YAML.
        return Err(match Markdown::try_from_content(raw) {
            Err(error) => error.into(),
            Ok(_) => eyre!("the frontmatter is not valid YAML"),
        });
    }

    report_findings(&result.diagnostics, &terminal);

    let Some(path) = save_path else {
        // What was printed is what a later `--save` would write, so a document
        // the guards declined prints unchanged.
        let document = Markdown::from(result.text.as_str());
        return document_value(None, &document, result.text.clone(), resolved.as_ref(), output, cli).map(Completed::success);
    };
    if result.outcome == CleanOutcome::Declined {
        return Ok(Completed::success(acknowledgment(String::new())));
    }

    write_if_changed(&path, &result)?;
    Ok(Completed::success(acknowledgment(save_report(&raw, &result, options, output)?)))
}

fn data_value(report: CleanJsonReport) -> CleanValue {
    CommandValue::new(Some(report), darkmatter::markdown::output::command::empty_document())
}

/// A saved document's result: what changed, in every formatted output.
fn acknowledgment(text: String) -> CleanValue {
    let terminal = text.clone();
    CommandValue::new(None, code_document(None, text.trim_end()))
        .with_terminal(move |_| terminal.clone())
        .with_markdown(text)
}

/// What `--save` reports: any frontmatter repairs, then the delta.
fn save_report(raw: &str, result: &CleanResult, options: &CleanOptions, output: &ResolvedOutput) -> Result<String> {
    let terminal = Terminal::from_output_options(&output.options);
    let mut text = String::new();
    if !result.applied.is_empty() {
        // The delta below compares bodies; frontmatter repairs are reported
        // here instead.
        text.push_str(&Prose::new("Frontmatter: repairs applied").render(&terminal));
        text.push('\n');
    }
    let cleaned = Markdown::try_from_content(result.text.clone())?;
    let original = baseline(raw, &result.text)?;
    let delta = original.delta(&cleaned);
    if !result.applied.is_empty() && delta.is_unchanged() {
        // A body-only "No changes" would contradict the repair notice above.
        return Ok(text);
    }
    let mut report = DeltaReport::new(delta).with_documents(original, cleaned);
    if options.verbose {
        report = report.verbose();
    }
    let rendered = report.render(&terminal);
    text.push_str(&if output.options.terminal_profile.degrades_to_plain() { strip_escape_codes(rendered) } else { rendered });
    Ok(text)
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

/// A failed clean transaction: the original error, so messages are
/// unchanged, and for a schema failure the triggers discovery skipped before
/// it (`None` for any other failure, where no envelope is written).
struct CleanFailure {
    error: Report,
    trigger_findings: Option<Vec<InvalidTriggerFinding>>,
}

/// Runs the library clean transaction with this invocation's overrides and
/// schema configuration.
fn clean(
    raw: &str,
    overrides: &MarkdownRuleOverrides,
    document_path: Option<&Path>,
    flags: &CleanSchemaFlags,
    request: &DocumentRequest<'_>,
) -> std::result::Result<CleanResult, CleanFailure> {
    let clean_request = CleanRequest {
        source: raw,
        overrides,
        workspace: None,
        document_path,
    };
    clean_document(&clean_request, || schema_config(flags, document_path, request)).map_err(
        |error| match error {
            FrontmatterRepairError::Extraction(error) => {
                CleanFailure { error: Report::new(error), trigger_findings: None }
            }
            FrontmatterRepairError::Schema { error, trigger_findings } => {
                CleanFailure { error: Report::new(*error), trigger_findings: Some(trigger_findings) }
            }
            FrontmatterRepairError::Context(error) => CleanFailure { error, trigger_findings: None },
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

    Ok(config.with_trigger_schemas(!flags.no_schema_triggers))
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
        let overrides = rule_overrides(&CleanOptions::default(), None).unwrap();
        assert!(overrides.is_empty());
    }

    #[test]
    fn flags_become_explicit_overrides() {
        let options = CleanOptions {
            indent: Some(2),
            list_item_padding: resolve_list_spacing(true, false),
            ..CleanOptions::default()
        };
        let overrides = rule_overrides(&options, Some(Width::Columns(80))).unwrap();
        assert_eq!(overrides.indentation, Leaf::Value(Indentation::Two));
        assert_eq!(overrides.list_item_padding, Leaf::Value(ListItemPadding::None));
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Columns(80)));
        assert_eq!(overrides.unwrap, Leaf::Absent);
    }

    #[test]
    fn an_explicit_source_width_turns_unwrap_and_width_off() {
        let overrides = rule_overrides(&CleanOptions::default(), Some(Width::Source)).unwrap();
        assert_eq!(overrides.unwrap, Leaf::Value(false));
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Off));
    }

    #[test]
    fn an_explicit_unlimited_width_unwraps_and_fills_nothing() {
        let overrides = rule_overrides(&CleanOptions::default(), Some(Width::Unlimited)).unwrap();
        assert_eq!(overrides.unwrap, Leaf::Value(true));
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Off));
    }

    #[test]
    fn a_width_past_the_rule_range_is_still_a_width() {
        let overrides = rule_overrides(&CleanOptions::default(), Some(Width::Columns(100_000))).unwrap();
        assert_eq!(overrides.fixed_width, Leaf::Value(FixedWidth::Columns(u16::MAX)));
    }

    #[test]
    fn unsupported_indent_is_an_error() {
        let options = CleanOptions {
            indent: Some(3),
            ..CleanOptions::default()
        };
        assert!(rule_overrides(&options, None).is_err());
    }

    #[test]
    fn list_spacing_flags_map_to_padding() {
        assert_eq!(resolve_list_spacing(false, false), None);
        assert_eq!(resolve_list_spacing(true, false), Some(ListItemPadding::None));
        assert_eq!(resolve_list_spacing(false, true), Some(ListItemPadding::One));
    }
}

//! `md schema validate` implementation.

use crate::args::Cli;
use crate::commands::compose::{NO_BASELINE_SCHEMA_ENV, disables_baseline_schema};
use crate::commands::trigger_findings::{DeliveredFindings, emit_warnings};
use crate::commands::schema::assignment::{self, Assignment, PositionalKind};
use crate::io::{DocumentArgumentError, OpenedArgument, open_argument, resolve_file_path};
use crate::output::command::{CommandRun, Completed, DataShape};
use crate::output::{ResolvedOutput, output_diagnostic, stderr_terminal};
use crate::request::MdRequest;
use biscuit_terminal::output::{OutputTarget, WidthRequest};
use biscuit_terminal::utils::escape_codes::strip_escape_codes;
use clap::CommandFactory as _;
use darkmatter::markdown::compose::expression::Severity;
use darkmatter::markdown::output::command::{CommandValue, FindingRow, ValidateData, prose_document};
use renderable::{OutputOptions, TerminalProfile, Width};
use darkmatter::markdown::errors::resolution_failure_name;
use biscuit_terminal::components::prose::{LineBreaks, Prose};
use biscuit_terminal::components::renderable::TerminalRenderable;
use biscuit_terminal::components::table::{Table, TableCellContent, TableColumn};
use biscuit_terminal::errors::BlockError;
use biscuit_terminal::terminal::Terminal;
use color_eyre::eyre::Result;
use darkmatter::markdown::Markdown;
use biscuit_file::FileResolutionContext;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, InvalidTriggerFinding, LoadedYamlDocument, SchemaLayers, TriggerSource, YamlOutcome,
};
use simplified_schema::{
    SchemaAdvisory, SchemaError, ValidationProblem, ValidationProblemCode, ValidationProblemKind, ValidationTimings,
};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// Environment variable naming a custom baseline schema, read from the
/// captured request. Used when neither `--schema` nor `--disable baseline-schema`
/// is given and `DARKMATTER_NO_BASELINE_SCHEMA` is not truthy.
pub const BASELINE_SCHEMA_ENV: &str = "BASELINE_SCHEMA";

/// Per-file validation outcome.
enum FileOutcome {
    /// Validation completed (may or may not be valid).
    Validated {
        report_valid: bool,
        problems: Vec<ValidationProblem>,
        advisories: Vec<SchemaAdvisory>,
        schema_label: Option<String>,
        layers: AppliedLayers,
    },
    /// The file argument did not open (it is not valid reference syntax,
    /// failed to resolve, or matched no file); no document was read.
    Unopened(color_eyre::eyre::Report),
    /// Frontmatter could not be parsed.
    ParseError(String),
    /// A YAML document is not one admissible YAML document; the location is the 1-based line and UTF-8 byte column of the failure.
    YamlLoadError { message: String, location: Option<(usize, usize)> },
    /// Schema could not be resolved/built for this file.
    SchemaError(Box<SchemaError>),
}

/// The schema layers that judged a validated document, kept apart so the
/// output can say which of them applied. All empty means none did.
#[derive(Default)]
struct AppliedLayers {
    document: Option<DocumentSchema>,
    /// The matching triggers that layered a payload, in layering order.
    triggers: Vec<TriggerSource>,
    baseline: Option<AppliedBaseline>,
}

/// The form of the document's own schema.
enum DocumentSchema {
    /// An inline `$schema` mapping.
    Inline,
    /// A `$schema` file reference or bare name.
    Reference(String),
    /// A `$schema` root union (a sequence of arms).
    Union,
    /// A YAML document's selected definition: its claimed `kind`'s label or
    /// the `$schema` it named.
    Definition(String),
}

/// The baseline that applied.
enum AppliedBaseline {
    Embedded,
    Custom(PathBuf),
}

impl AppliedLayers {
    /// The layers a Markdown document's preparation reported. `md` is the
    /// validated document, whose `$schema` value tells the document layer's
    /// form; `baseline` is the selection that produced the baseline layer.
    fn for_markdown(layers: SchemaLayers, md: &Markdown, baseline: &Baseline) -> Self {
        let document = layers.document.then(|| match md.frontmatter().as_map().get("$schema") {
            Some(serde_json::Value::String(reference)) => DocumentSchema::Reference(reference.clone()),
            Some(serde_json::Value::Array(_)) => DocumentSchema::Union,
            _ => DocumentSchema::Inline,
        });
        let baseline = match (layers.baseline, baseline) {
            (true, Baseline::Custom(path)) => Some(AppliedBaseline::Custom(path.clone())),
            (true, _) => Some(AppliedBaseline::Embedded),
            (false, _) => None,
        };
        Self { document, triggers: layers.triggers, baseline }
    }

    /// The pretty success line's tail, naming every layer that applied.
    fn success_tail(&self) -> String {
        let mut named = Vec::new();
        named.extend(self.document.as_ref().map(|document| match document {
            DocumentSchema::Inline => "its inline $schema".to_string(),
            DocumentSchema::Reference(reference) => format!("its $schema {reference}"),
            DocumentSchema::Union => "its $schema union".to_string(),
            DocumentSchema::Definition(label) => format!("its definition {label}"),
        }));
        if !self.triggers.is_empty() {
            let names: Vec<String> = self.triggers.iter().map(trigger_display_name).collect();
            let noun = if names.len() == 1 { "trigger schema" } else { "trigger schemas" };
            named.push(format!("the {noun} {}", join_names(&names)));
        }
        named.extend(self.baseline.as_ref().map(|baseline| match baseline {
            AppliedBaseline::Embedded => "the Darkmatter default schema".to_string(),
            AppliedBaseline::Custom(path) => format!("the baseline schema {}", path.display()),
        }));
        if named.is_empty() {
            return "doesn't have a schema definition so is valid by default.".to_string();
        }
        let head = if self.document.is_some() {
            "has a schema definition and is valid based on"
        } else {
            "does not have an explicit schema definition but it is valid based on"
        };
        format!("{head} {}.", escape_prose(&join_names(&named)))
    }

    /// The JSON record's `layers` object.
    fn to_json(&self) -> serde_json::Value {
        let document = self.document.as_ref().map(|document| match document {
            DocumentSchema::Inline => serde_json::json!({ "form": "inline" }),
            DocumentSchema::Reference(reference) => serde_json::json!({ "form": "reference", "reference": reference }),
            DocumentSchema::Union => serde_json::json!({ "form": "union" }),
            DocumentSchema::Definition(label) => serde_json::json!({ "form": "definition", "reference": label }),
        });
        let baseline = self.baseline.as_ref().map(|baseline| match baseline {
            AppliedBaseline::Embedded => serde_json::json!({ "form": "darkmatter" }),
            AppliedBaseline::Custom(path) => serde_json::json!({ "form": "custom", "reference": path.display().to_string() }),
        });
        serde_json::json!({
            "document": document,
            "triggers": self.triggers.iter().map(ToString::to_string).collect::<Vec<_>>(),
            "baseline": baseline,
        })
    }
}

/// A trigger's file name, or its full `embedded:{bundle}/{name}` identity
/// for a bundle document, whose bare name would hide the bundle.
fn trigger_display_name(trigger: &TriggerSource) -> String {
    match (trigger.path(), trigger.file_name()) {
        (Some(_), Some(name)) => name.to_string(),
        _ => trigger.to_string(),
    }
}

/// Joins names as prose: `a`, `a and b`, `a, b, and c`.
fn join_names(names: &[String]) -> String {
    match names {
        [] => String::new(),
        [one] => one.clone(),
        [first, second] => format!("{first} and {second}"),
        [rest @ .., last] => format!("{}, and {last}", rest.join(", ")),
    }
}

/// What `md schema validate` was asked for, beyond its inputs.
#[derive(Debug, Clone, Copy)]
pub struct ValidateOptions<'a> {
    /// `--schema`.
    pub schema: Option<&'a Path>,
    /// `--disable baseline-schema`.
    pub no_baseline_schema: bool,
    /// `--disable schema-triggers`.
    pub no_schema_triggers: bool,
    /// `--quiet`: hide formatted success rows; a data output keeps every file.
    pub quiet: bool,
    /// `--output`.
    pub output: OutputTarget,
}

/// One input's result.
struct FileResult {
    file: PathBuf,
    document: Option<PathBuf>,
    outcome: FileOutcome,
    findings: Vec<InvalidTriggerFinding>,
}

/// Run `md schema validate`.
///
/// The report is what the command was run to produce, so it is the deliverable
/// on stdout, with every file's record collected before anything prints. A
/// failure of any kind (a file that fails validation, cannot be opened or
/// parsed, or is not admissible YAML; a schema or baseline that cannot be
/// loaded) exits `1`. A data output tells them apart by each record's `error`
/// field. Arguments that cannot work together are usage errors (exit `2`).
///
/// An input whose resolved path ends in `.yaml` or `.yml` (any letter case)
/// is a whole YAML document, validated by the schema it selects (its claimed
/// `kind`, else its root `$schema`) with no baseline and no triggers. A
/// preflight opens every argument before anything is validated and refuses
/// assignments or `--schema` alongside any YAML input. The baseline is loaded
/// only when some input is not YAML.
///
/// At `verbose >= 2` a formatted run's stage timings go to stderr, per document;
/// stdout and the exit status are unaffected.
///
/// A schema trigger discovery skipped as invalid is a warning, never a failure:
/// it is reported once per invocation, on the first file that discovers it,
/// whatever that file's outcome (stderr in a formatted output, the record's
/// `warnings` in a data output).
pub fn run_validate(inputs: &[String], options: &ValidateOptions<'_>, cli: &Cli, request: &MdRequest) -> Result<()> {
    let usage = |message: &str| -> color_eyre::Report {
        Cli::command().error(clap::error::ErrorKind::ValueValidation, message).into()
    };
    let (files, assignments) = split_inputs(inputs).map_err(|message| usage(&message))?;
    if files.is_empty() {
        return Err(usage("no Markdown or YAML files were provided"));
    }

    // Preflight: every argument is opened, and the batch judged, before any
    // file is validated or any outcome printed.
    let opened: Vec<(Result<OpenedArgument>, bool)> = files
        .iter()
        .map(|file| {
            let opened = open_argument(file, request);
            // An argument that did not open has no resolved path; its own
            // spelling decides whether it would have been YAML.
            let yaml = is_yaml_path(opened.as_ref().map_or(file.as_path(), OpenedArgument::path));
            (opened, yaml)
        })
        .collect();
    let any_yaml = opened.iter().any(|(_, yaml)| *yaml);
    if any_yaml && let Some(message) = yaml_usage_error(&assignments, options.schema) {
        return Err(usage(message));
    }

    let run = CommandRun::begin(options.output, cli.width, WidthRequest::Auto, cli.verbose, DataShape::Object)?;
    let result = validate_all(&files, opened, &assignments, options, cli.verbose, request, run.output());
    run.finish(result)
}

type ValidateValue = CommandValue<ValidateData>;

fn validate_all(
    files: &[PathBuf],
    opened: Vec<(Result<OpenedArgument>, bool)>,
    assignments: &[Assignment],
    options: &ValidateOptions<'_>,
    verbose: u8,
    request: &MdRequest,
    output: &ResolvedOutput,
) -> Result<Completed<ValidateValue>> {
    let is_data = output.target.is_data();
    // YAML documents never use the baseline, so a YAML-only batch never
    // loads (or fails on) one.
    let loaded = if opened.iter().any(|(_, yaml)| !*yaml) {
        load_api(options.schema, options.no_baseline_schema, request)
    } else {
        request
            .launch_context()
            .map(|context| (DarkmatterSchemas::new(context.clone()), Baseline::None))
            .map_err(|error| SchemaError::Baseline { message: format!("{error:#}"), source: None })
    };
    let (api, baseline) = match loaded {
        Ok(loaded) => loaded,
        Err(err) if is_data => {
            let diagnostic = output_diagnostic("dm.schema.baseline", Severity::Error, &err.to_string());
            let value = validate_value(Vec::new(), options.quiet, verbose);
            return Ok(Completed::failure(value).with_diagnostics(vec![diagnostic]));
        }
        Err(err) => {
            let terminal = stderr_terminal();
            eprintln!("{}", crate::output::stderr_text(err.status_block(&terminal).render(&terminal)));
            return Err(crate::output::report::ReportedExit(1).into());
        }
    };

    let mut delivered = DeliveredFindings::default();
    let mut results = Vec::with_capacity(files.len());
    let stderr = (!is_data).then(stderr_terminal);

    for (file, (opened, yaml)) in files.iter().zip(opened) {
        let (outcome, timings, findings, document) = match opened {
            Ok(opened) if yaml => (validate_yaml(&api, &opened, request), None, Vec::new(), Some(opened.into_path())),
            Ok(opened) => {
                let checked = validate_one(&api, &baseline, &opened, assignments, options.no_schema_triggers, request);
                (checked.outcome, checked.timings, checked.findings, Some(opened.into_path()))
            }
            Err(err) => (FileOutcome::Unopened(err), None, Vec::new(), None),
        };
        let findings = delivered.first_seen(&findings);
        if let Some(terminal) = &stderr {
            // Warnings, so `--quiet` (which hides successes) keeps them.
            emit_warnings(&findings, terminal);
            if verbose > 1
                && let Some(timings) = &timings
            {
                emit_timings(file, timings, terminal);
            }
        }
        results.push(FileResult { file: file.clone(), document, outcome, findings });
    }

    let failed = results.iter().any(|result| !matches!(result.outcome, FileOutcome::Validated { report_valid: true, .. }));
    let value = validate_value(results, options.quiet, verbose);
    Ok(if failed { Completed::failure(value) } else { Completed::success(value) })
}

fn validate_value(results: Vec<FileResult>, quiet: bool, verbose: u8) -> ValidateValue {
    let data = ValidateData { files: results.iter().map(|result| file_record(&result.file, &result.outcome, &result.findings)).collect() };
    // The report's rows are already Markdown list items.
    let plain = pretty_report(&results, quiet, 0, &OutputOptions::new(Width::Unlimited, TerminalProfile::plain()));
    CommandValue::new(data, prose_document(&plain)).with_terminal(move |options| pretty_report(&results, quiet, verbose, options))
}

fn pretty_report(results: &[FileResult], quiet: bool, verbose: u8, options: &OutputOptions) -> String {
    let terminal = Terminal::from_output_options(options);
    let mut report = String::new();
    for result in results {
        render_pretty(&mut report, &result.file, result.document.as_deref(), &result.outcome, quiet, verbose, &terminal);
    }
    // An error block paints regardless of its terminal.
    if options.terminal_profile.degrades_to_plain() { strip_escape_codes(report) } else { report }
}

/// Builds the [`DarkmatterSchemas`] entry point at the launch context with
/// the selected baseline, returned beside it. Each file re-anchors it on its
/// own context.
///
/// The first matching rule selects the baseline:
///
/// 1. `--disable baseline-schema`: none, ignoring `BASELINE_SCHEMA`.
/// 2. `--schema <path>`: that custom baseline, even when
///    `DARKMATTER_NO_BASELINE_SCHEMA` is truthy.
/// 3. A truthy `DARKMATTER_NO_BASELINE_SCHEMA`: none, ignoring
///    `BASELINE_SCHEMA`.
/// 4. `BASELINE_SCHEMA`: that custom baseline.
/// 5. Otherwise the embedded Darkmatter baseline.
///
/// Both variables come from the captured request, never the live process
/// environment. A custom path is a file reference resolved in the launch
/// context, like a document argument, and one that does not resolve or load
/// is an error, never a fallback to the embedded baseline. Removing the
/// baseline leaves document and trigger schemas active.
fn load_api(
    schema: Option<&Path>,
    no_baseline_schema: bool,
    request: &MdRequest,
) -> Result<(DarkmatterSchemas, Baseline), SchemaError> {
    let env = request.snapshot().env();
    let selection = if no_baseline_schema {
        Baseline::None
    } else if let Some(path) = schema {
        Baseline::Custom(path.to_path_buf())
    } else if disables_baseline_schema(env.get(NO_BASELINE_SCHEMA_ENV).map(String::as_str)) {
        Baseline::None
    } else if let Some(path) = env.get(BASELINE_SCHEMA_ENV) {
        Baseline::Custom(PathBuf::from(path))
    } else {
        Baseline::Embedded
    };

    let context = request.launch_context().map_err(|error| SchemaError::Baseline {
        message: format!("{error:#}"),
        source: None,
    })?;
    let api = DarkmatterSchemas::new(context.clone());
    let api = match &selection {
        Baseline::None => api,
        Baseline::Custom(path) => {
            let resolved = resolve_file_path(path, context).map_err(|error| SchemaError::Baseline {
                message: format!("could not load baseline from `{}`: {error:#}", path.display()),
                source: None,
            })?;
            api.with_baseline_from_file(resolved)?
        }
        Baseline::Embedded => api.with_darkmatter_baseline_json_schema()?,
    };
    Ok((api, selection))
}

/// The [`DarkmatterSchemas`] an ordinary run of `md schema validate` would use,
/// at the launch context: the same baseline selection, with no `--schema`.
///
/// ## Errors
///
/// As [`load_api`].
pub(crate) fn request_schemas(request: &MdRequest) -> Result<DarkmatterSchemas, SchemaError> {
    load_api(None, false, request).map(|(api, _)| api)
}

/// [`request_schemas`] for a document in `context`: its own resolution context,
/// with trigger discovery when it is inside a repository. The caller decides how
/// to report what discovery skipped.
///
/// ## Errors
///
/// As [`load_api`], or when trigger discovery fails.
pub(crate) fn document_schemas(request: &MdRequest, context: biscuit_file::FileResolutionContext) -> Result<DarkmatterSchemas, SchemaError> {
    discover_document_triggers(request_schemas(request)?.with_file_resolution_context(context))
}

/// `schemas` with trigger discovery when its context is inside a repository.
///
/// ## Errors
///
/// When trigger discovery fails.
pub(crate) fn discover_document_triggers(schemas: DarkmatterSchemas) -> Result<DarkmatterSchemas, SchemaError> {
    if schemas.file_resolution_context().repository_root().is_some() { schemas.with_trigger_discovery() } else { Ok(schemas) }
}

/// A document's effective schema, selected passively, and the schema triggers
/// its discovery skipped as invalid.
pub(crate) struct SelectedDocumentSchema {
    /// `None` when no schema layer applies to the document.
    pub(crate) schema: Option<EffectiveSchema>,
    pub(crate) trigger_findings: Vec<InvalidTriggerFinding>,
}

impl SelectedDocumentSchema {
    /// No schema, and no trigger found.
    pub(crate) fn none() -> Self {
        Self { schema: None, trigger_findings: Vec::new() }
    }
}

/// The effective schema of `document` from `schemas` (a baseline selection)
/// in the document's `context`, with trigger discovery when
/// `discover_triggers` and the context is inside a repository.
///
/// Passive: the document is not composed, and its `$schema` is read as
/// written, never overlaid by anything a caller supplies.
///
/// ## Errors
///
/// When trigger discovery fails or the document's schema cannot be assembled.
pub(crate) fn effective_document_schema(
    schemas: DarkmatterSchemas,
    document: &Markdown,
    context: FileResolutionContext,
    discover_triggers: bool,
) -> Result<SelectedDocumentSchema, SchemaError> {
    let schemas = schemas.with_file_resolution_context(context);
    let schemas = if discover_triggers { discover_document_triggers(schemas)? } else { schemas };
    Ok(SelectedDocumentSchema { trigger_findings: schemas.discovery_findings().to_vec(), schema: schemas.effective_for(document)? })
}

/// [`effective_document_schema`] with the baseline an ordinary run selects
/// ([`request_schemas`], so `BASELINE_SCHEMA` and
/// `DARKMATTER_NO_BASELINE_SCHEMA` are read from the request's snapshot) and
/// trigger discovery, in the context of the document `opened` names, or the
/// launch context for a document read from stdin.
///
/// ## Errors
///
/// The context builder's error, or a [`SchemaError`] from the baseline,
/// trigger discovery, or the document's schema.
pub(crate) fn select_document_schema(
    document: &Markdown,
    opened: Option<&OpenedArgument>,
    request: &MdRequest,
) -> Result<SelectedDocumentSchema> {
    let context = match opened {
        Some(opened) => opened.document_context(request)?,
        None => request.launch_context()?.clone(),
    };
    Ok(effective_document_schema(request_schemas(request)?, document, context, true)?)
}

/// The baseline [`load_api`] selected.
enum Baseline {
    None,
    Custom(PathBuf),
    Embedded,
}

/// One file's validation: its outcome, the schema timings, and the triggers
/// its discovery skipped as invalid.
///
/// The timings accumulate every schema step of the request: trigger
/// discovery, the pre-assignment resolution, and the final validation. They are `None` when the file failed before
/// discovery. The findings are kept whatever the outcome, so a later schema
/// or validation failure still reports them.
struct CheckedFile {
    outcome: FileOutcome,
    timings: Option<ValidationTimings>,
    findings: Vec<InvalidTriggerFinding>,
}

/// Validates a single file, keeping parse and schema failures apart so its
/// record can tell them apart.
fn validate_one(
    api: &DarkmatterSchemas,
    baseline: &Baseline,
    opened: &OpenedArgument,
    assignments: &[Assignment],
    no_schema_triggers: bool,
    request: &MdRequest,
) -> CheckedFile {
    let early = |outcome, timings| CheckedFile { outcome, timings, findings: Vec::new() };
    // Legacy-spelling canonicalization: a verbatim `\\?\` result would gain a
    // path segment the gix-derived boundary lacks, disabling trigger discovery.
    let discovery_path = biscuit_file::canonicalize_simplified(opened.path())
        .unwrap_or_else(|_| opened.path().to_path_buf());
    let mut md = match Markdown::try_from(discovery_path.as_path()) {
        Ok(md) => md,
        Err(err) => return early(FileOutcome::ParseError(err.to_string()), None),
    };

    // The document's context resolves its schema `file` values and bounds
    // trigger discovery. A context that cannot be built has no better outcome
    // than the file's parse error.
    let document_context = match request.document_context(Some(opened.reference()), &discovery_path) {
        Ok(context) => context,
        Err(err) => return early(FileOutcome::ParseError(format!("{err:#}")), None),
    };
    let in_repository = document_context.repository_root().is_some();
    let api = api.clone().with_file_resolution_context(document_context);
    let (api, mut timings) = if in_repository && !no_schema_triggers {
        match ValidationTimings::record(|| api.with_trigger_discovery()) {
            (Ok(api), discovery) => (api, discovery),
            (Err(err), discovery) => return early(FileOutcome::SchemaError(Box::new(err)), Some(discovery)),
        }
    } else {
        (api, ValidationTimings::default())
    };
    let findings = api.discovery_findings().to_vec();
    let checked = |outcome, timings| CheckedFile { outcome, timings: Some(timings), findings: findings.clone() };

    if !assignments.is_empty() {
        // Build the pre-assignment effective schema solely to coerce assignment
        // RHS values to declared property types. Validation resolves again
        // below because assignments may change trigger activation.
        // Recorded: on a cold request this is where the validator is built,
        // so leaving it out would report the later validation as a cache hit.
        let (effective, preparation) = ValidationTimings::record(|| api.effective_for(&md));
        timings.absorb(preparation);
        let effective = match effective {
            Ok(effective) => effective,
            Err(err) => return checked(FileOutcome::SchemaError(Box::new(err)), timings),
        };
        assignment::apply_all(
            md.frontmatter_mut().as_map_mut(),
            assignments,
            effective.as_ref(),
        );
    }

    let schema_label = schema_label_from(&md);
    // Resolve again after assignments: assignments may change which trigger
    // arms match, so the pre-assignment schema is only valid for coercion.
    // The schema-presence wording comes from this same preparation.
    let (validated, validation) = ValidationTimings::record(|| api.validate_summarized(&md));
    timings.absorb(validation);
    let outcome = match validated {
        Ok((report, summary)) => FileOutcome::Validated {
            report_valid: report.valid,
            problems: report.problems,
            advisories: report.advisories,
            schema_label,
            layers: AppliedLayers::for_markdown(summary.layers, &md, baseline),
        },
        Err(err) => FileOutcome::SchemaError(Box::new(err)),
    };
    checked(outcome, timings)
}

/// Whether `path` names a whole YAML document: its extension is `yaml` or
/// `yml`, ASCII case-insensitively.
fn is_yaml_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("yaml") || extension.eq_ignore_ascii_case("yml"))
}

/// Why a batch holding a YAML document is a usage error, if it is: an
/// assignment edits frontmatter, which a YAML document does not have, and
/// `--schema` replaces a baseline YAML documents never use.
fn yaml_usage_error(assignments: &[Assignment], schema: Option<&Path>) -> Option<&'static str> {
    if !assignments.is_empty() {
        Some("`<prop>=<value>` assignments apply to Markdown frontmatter and cannot be used with a YAML document")
    } else if schema.is_some() {
        Some("`--schema` replaces the Markdown baseline and cannot be used with a YAML document")
    } else {
        None
    }
}

/// Validates one whole YAML document in its own context: its claimed `kind`
/// or root `$schema` selects the schema, and nothing else applies.
fn validate_yaml(api: &DarkmatterSchemas, opened: &OpenedArgument, request: &MdRequest) -> FileOutcome {
    let path = biscuit_file::canonicalize_simplified(opened.path()).unwrap_or_else(|_| opened.path().to_path_buf());
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(err) => {
            return FileOutcome::YamlLoadError { message: format!("could not read the file: {err}"), location: None };
        }
    };
    let document = match LoadedYamlDocument::load(&text, &path) {
        Ok(document) => document,
        Err(err) => {
            let location = match &err {
                SchemaError::SchemaDocument { span: Some(span), .. } => Some(simplified_schema::line_col_of_offset(&text, span.start)),
                _ => None,
            };
            let message = match err {
                SchemaError::SchemaDocument { message, .. } => message,
                other => other.to_string(),
            };
            return FileOutcome::YamlLoadError { message, location };
        }
    };
    let context = match request.document_context(Some(opened.reference()), &path) {
        Ok(context) => context,
        Err(err) => return FileOutcome::YamlLoadError { message: format!("{err:#}"), location: None },
    };
    match api.clone().with_file_resolution_context(context).validate_loaded_yaml_document(&document) {
        Ok(YamlOutcome::NoSchema) => FileOutcome::Validated {
            report_valid: true,
            problems: Vec::new(),
            advisories: Vec::new(),
            schema_label: None,
            layers: AppliedLayers::default(),
        },
        Ok(YamlOutcome::Validated(validation)) => FileOutcome::Validated {
            report_valid: validation.report.valid,
            problems: validation.report.problems,
            advisories: validation.report.advisories,
            layers: AppliedLayers {
                document: Some(DocumentSchema::Definition(validation.definition.clone())),
                ..AppliedLayers::default()
            },
            schema_label: Some(validation.definition),
        },
        Err(err) => FileOutcome::SchemaError(Box::new(err)),
    }
}

/// Writes one document's validation stage timings to stderr.
fn emit_timings(file: &Path, timings: &ValidationTimings, terminal: &Terminal) {
    let triggers = match timings.matched_triggers() {
        [] => "none".to_string(),
        names => escape_prose(&names.join(", ")),
    };
    let summary = format!(
        "<b>Validation timings</b> for {}\\\nmatched triggers: {triggers}\\\ncompile cache: <b>{}</b>",
        escape_prose(&file.display().to_string()),
        timings.compile_cache().name(),
    );
    eprintln!("{}", Prose::new(summary).render(terminal));

    let mut rows: Vec<Vec<TableCellContent>> = timings
        .stages()
        .map(|(stage, elapsed)| vec![stage.name().into(), milliseconds(elapsed).into()])
        .collect();
    rows.push(vec!["unassigned".into(), milliseconds(timings.unassigned()).into()]);
    rows.push(vec!["total".into(), milliseconds(timings.total()).into()]);
    let table = Table::new()
        .with_columns(vec![TableColumn::new("stage"), TableColumn::new("time")])
        .with_data(rows);
    eprintln!("{}", table.render(terminal));
}

fn milliseconds(elapsed: Duration) -> String {
    format!("{:.2} ms", elapsed.as_secs_f64() * 1000.0)
}

/// Splits positional inputs into file paths and parsed assignments. Returns
/// an error message describing the first invalid assignment encountered.
fn split_inputs(inputs: &[String]) -> Result<(Vec<PathBuf>, Vec<Assignment>), String> {
    let mut files = Vec::new();
    let mut assignments = Vec::new();
    for raw in inputs {
        match assignment::classify(raw) {
            PositionalKind::File(p) => files.push(PathBuf::from(p)),
            PositionalKind::Assignment(a) => assignments.push(a),
            PositionalKind::Invalid { raw, message } => {
                return Err(format!("invalid assignment `{raw}`: {message}"));
            }
        }
    }
    Ok((files, assignments))
}

fn schema_label_from(md: &Markdown) -> Option<String> {
    md.frontmatter()
        .as_map()
        .get("$schema")
        .and_then(|value| value.as_str())
        .map(String::from)
}

fn push_row(out: &mut String, row: &str) {
    out.push_str(row);
    out.push('\n');
}

fn render_pretty(
    out: &mut String,
    file: &Path,
    document: Option<&Path>,
    outcome: &FileOutcome,
    quiet: bool,
    verbose: u8,
    terminal: &Terminal,
) {
    let link = document_link(file, document);

    match outcome {
        FileOutcome::Validated {
            report_valid: true,
            advisories,
            layers,
            ..
        } => {
            if quiet {
                return;
            }
            let tail = layers.success_tail();
            let line =
                format!("- <green>✔</green> _<dim>the document</dim>_ {link} _<dim>{tail}</dim>_");
            push_row(out, &Prose::new(line).with_verbosity(verbose).render(terminal));
            for advisory in advisories {
                push_advisory_bullet(out, advisory, terminal);
            }
        }
        FileOutcome::Validated {
            report_valid: false,
            problems,
            advisories,
            ..
        } => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>failed schema validation:</dim>_"
            );
            push_row(out, &Prose::new(header).with_verbosity(verbose).render(terminal));
            for problem in problems {
                push_problem_bullet(out, problem, terminal);
            }
            for advisory in advisories {
                push_advisory_bullet(out, advisory, terminal);
            }
        }
        FileOutcome::Unopened(err) => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>could not be opened:</dim>_"
            );
            push_row(out, &Prose::new(header).with_verbosity(verbose).render(terminal));
            match unopened_argument(err) {
                Some(argument) => push_row(out, &argument.status_block(terminal).render(terminal)),
                None => {
                    let bullet = format!("    - {}", escape_prose(&format!("{err:#}")));
                    push_row(out, &Prose::new(bullet).render(terminal));
                }
            }
        }
        FileOutcome::ParseError(message) => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>has unparseable frontmatter:</dim>_"
            );
            push_row(out, &Prose::new(header).with_verbosity(verbose).render(terminal));
            let bullet = format!("    - {}", escape_prose(message));
            push_row(out, &Prose::new(bullet).render(terminal));
        }
        FileOutcome::YamlLoadError { message, location } => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>is not a valid YAML document:</dim>_"
            );
            push_row(out, &Prose::new(header).with_verbosity(verbose).render(terminal));
            let suffix = match location {
                Some((line, column)) => format!(" <dim>(at line {line}, column {column})</dim>"),
                None => String::new(),
            };
            let bullet = format!("    - {}{suffix}", escape_prose(message));
            push_row(out, &Prose::new(bullet).render(terminal));
        }
        FileOutcome::SchemaError(err) => {
            let header = format!(
                "- <red>✗</red> _<dim>the document</dim>_ {link} _<dim>has a schema definition error:</dim>_"
            );
            push_row(out, &Prose::new(header).with_verbosity(verbose).render(terminal));
            let block = err.status_block(terminal);
            push_row(out, &block.render(terminal));
        }
    }
}

fn push_advisory_bullet(out: &mut String, advisory: &SchemaAdvisory, terminal: &Terminal) {
    let bullet = format!(
        "    - <yellow>warning</yellow> <dim>[{}/{}]</dim> {}",
        advisory.source(),
        advisory.code(),
        escape_prose(&advisory.message()),
    );
    push_row(out, &Prose::new(bullet).render(terminal));
}

/// Renders one `ValidationProblem` as an indented Prose list item, with the
/// offending property name (or root-union arm) prefix and an optional
/// source-location suffix.
fn push_problem_bullet(out: &mut String, problem: &ValidationProblem, terminal: &Terminal) {
    let mut prefix_parts: Vec<String> = Vec::new();
    if let Some(arm_index) = problem.arm_index {
        prefix_parts.push(format!("arm[{arm_index}]:"));
    }
    if !problem.path.is_empty() {
        prefix_parts.push(format!(
            "<inverse>{}</inverse>",
            escape_prose(strip_pointer_prefix(&problem.path))
        ));
    } else if let Some(property) = problem.property.as_deref() {
        prefix_parts.push(format!("<inverse>{}</inverse>", escape_prose(property)));
    } else if problem.arm_index.is_none() {
        prefix_parts.push("<inverse>root</inverse>".to_string());
    }
    let prefix = if prefix_parts.is_empty() {
        String::new()
    } else {
        format!("{} ", prefix_parts.join(" "))
    };

    let location = format_location(problem);
    let location_suffix = if location.is_empty() {
        String::new()
    } else {
        format!(" <dim>(at {location})</dim>")
    };

    let message = trim_redundant_property_prefix(&problem.message, problem.property.as_deref());
    // A message's own `hint:` row stays a row, indented under its bullet.
    let message = escape_prose(message).replace('\n', "\n      ");
    let bullet = format!("    - {prefix}{message}{location_suffix}");
    push_row(out, &Prose::new(bullet).with_line_breaks(LineBreaks::Hard).render(terminal));

    // Surface the declared property description on its own dimmed sub-line,
    // one indent level beneath the bullet (Decision #7). Enrichment already
    // suppressed empty / message-equal descriptions, so a `Some` here renders.
    if let Some(description) = problem.description.as_deref() {
        let sub_line = format!("        <dim>{}</dim>", escape_prose(description));
        push_row(out, &Prose::new(sub_line).render(terminal));
    }
    // The stable class of a failed `file` value (R5), on its own sub-line.
    if let Some(reference) = problem.file_reference.as_ref() {
        let sub_line = format!(
            "        <dim>failure:</dim> {}",
            darkmatter::markdown::errors::resolution_failure_name(reference.resolution_failure())
        );
        push_row(out, &Prose::new(sub_line).render(terminal));
    }
}

/// When `jsonschema` reports a `Required` failure, the message already starts
/// with the quoted property name (e.g. `"name" is a required property`). Since
/// we now surface that name as the bullet prefix, strip the duplicate from the
/// message so it reads naturally: `name is a required property`.
fn trim_redundant_property_prefix<'a>(message: &'a str, property: Option<&str>) -> &'a str {
    let Some(name) = property else {
        return message;
    };
    let quoted = format!("\"{name}\" ");
    message.strip_prefix(&quoted).unwrap_or(message)
}

/// Strips the leading `/` from a JSON pointer so the prefix reads as a
/// human-friendly property reference (`age` rather than `/age`,
/// `tags/2` rather than `/tags/2`).
fn strip_pointer_prefix(pointer: &str) -> &str {
    pointer.strip_prefix('/').unwrap_or(pointer)
}

/// Builds the styled, OSC8-hyperlinked Prose markup for a document. The
/// visible label is the argument as the user spelled it; the underlying
/// `file://` href is the absolute file it opened (`document`), so `@doc.md`
/// links to the file it found. An argument that opened nothing links to its
/// own absolute spelling. Without OSC 8 support the row shows the label
/// alone, and `label <file://…>` at `-v`.
fn document_link(file: &Path, document: Option<&Path>) -> String {
    let target = document.unwrap_or(file);
    let absolute = std::path::absolute(target).unwrap_or_else(|_| target.to_path_buf());
    format!(
        "<blue>[{label}](file://{href})</blue>",
        label = file.display(),
        href = absolute.display(),
    )
}

/// One file's data record. `warnings` holds the discovery findings first
/// (discovery runs before validation), then the schema advisories, and is
/// present on any outcome that has one.
fn file_record(file: &Path, outcome: &FileOutcome, findings: &[InvalidTriggerFinding]) -> serde_json::Value {
    use serde_json::json;

    let mut warnings_json: Vec<serde_json::Value> = findings
        .iter()
        .map(|finding| serde_json::to_value(FindingRow::from(finding)).expect("a finding row is JSON"))
        .collect();
    let file_str = file.to_string_lossy().into_owned();
    let mut value = match outcome {
        FileOutcome::Validated {
            report_valid,
            problems,
            advisories,
            schema_label,
            layers,
        } => {
            let problems_json: Vec<serde_json::Value> = problems
                .iter()
                .map(|p| {
                    json!({
                        "path": p.path,
                        "property": p.property,
                        "message": p.message,
                        "kind": kind_str(p),
                        "line": p.line,
                        "column": p.column,
                        "arm_index": p.arm_index,
                        "description": p.description,
                    })
                })
                .collect();
            warnings_json.extend(advisories.iter().map(|advisory| {
                json!({
                    "source": advisory.source(),
                    "code": advisory.code(),
                    "path": advisory.path(),
                    "message": advisory.message(),
                })
            }));
            json!({
                "file": file_str,
                "valid": report_valid,
                "schema": schema_label,
                "layers": layers.to_json(),
                "problems": problems_json,
            })
        }
        FileOutcome::Unopened(err) => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "file_reference",
            "failure": unopened_argument(err)
                .map(|argument| resolution_failure_name(argument.resolution_failure())),
            "message": format!("{err:#}"),
            "problems": [],
        }),
        FileOutcome::ParseError(message) => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "frontmatter_parse",
            "message": message,
            "problems": [],
        }),
        FileOutcome::YamlLoadError { message, location } => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "yaml_parse",
            "message": message,
            "line": location.map(|(line, _)| line),
            "column": location.map(|(_, column)| column),
            "problems": [],
        }),
        FileOutcome::SchemaError(err) => json!({
            "file": file_str,
            "valid": false,
            "schema": serde_json::Value::Null,
            "error": "schema",
            "message": err.to_string(),
            "problems": [],
        }),
    };
    if !warnings_json.is_empty() {
        value
            .as_object_mut()
            .expect("every outcome is a JSON object")
            .insert("warnings".to_string(), warnings_json.into());
    }
    // The record's keys are alphabetical, a published contract that predates
    // `serde_json/preserve_order`.
    value.sort_all_objects();
    value
}

/// Stable lowercase token for a problem's kind, used in a data output; a YAML
/// document kind's own check is `kind_check` whatever its underlying kind.
fn kind_str(problem: &ValidationProblem) -> &'static str {
    if problem.code == ValidationProblemCode::KindCheck {
        return "kind_check";
    }
    match problem.kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

/// The problem's position in the complete document; the column is a 1-based
/// UTF-8 byte column.
fn format_location(problem: &ValidationProblem) -> String {
    match (problem.line, problem.column) {
        (Some(line), Some(column)) => format!("line {line}, column {column}"),
        (Some(line), None) => format!("line {line}"),
        (None, _) => String::new(),
    }
}

/// Escape text so it renders exactly as written inside Prose markup; code
/// spans it marks with backticks stay literal.
fn escape_prose(input: &str) -> String {
    Prose::escape_text_outside_code_spans(input)
}

/// The typed argument error inside an [`FileOutcome::Unopened`] report.
fn unopened_argument(err: &color_eyre::eyre::Report) -> Option<&DocumentArgumentError> {
    err.chain().find_map(|cause| cause.downcast_ref::<DocumentArgumentError>())
}

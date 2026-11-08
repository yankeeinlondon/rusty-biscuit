//! The render command's CLI style inputs: `--style FILE` arguments and
//! `style…=VALUE` setters, read into [`StyleLayer`]s for
//! [`prepare_style_layers`].
//!
//! Nothing here prints. Every failure is a run failure (exit 1, never a usage
//! error) whose records [`crate::output::report::failure_diagnostics`] finds:
//! a file reference that does not resolve keeps its
//! [`DocumentArgumentError`] (and so its `failure:` class) in the chain, and
//! every other failure is a [`ClassifiedFailure`] carrying its records. In
//! [`prepare_cli_style_inputs`] and [`prepare_render_style`] those records are
//! [located](ClassifiedFailure::located): a record about a style file or a
//! setter never takes the Markdown document's path.
//!
//! ## Reading a `--style` argument
//!
//! The whole argument is classified with [`FileReference`] first, with no
//! I/O. Only an implicit relative reference whose first `@` is not leading,
//! with no `/` or `\` before it (a `\.` or `\\` escape of the offset is
//! not a separator), is `offset@reference`: `theme.dark@style.yaml`
//! selects `theme.dark` of `style.yaml`. Every explicit form (`./`, `../`,
//! absolute, `~`, `@` magic, `&`, `^`, `vault:`, a URL, a native Windows
//! path) keeps every `@`, and so does `themes/user@host.yaml`. The offset is
//! a [`PropertyPath`]; an empty offset or file part fails, with no fallback
//! to another reading.
//!
//! A file is JSON, JSON5, YAML (`.yaml`/`.yml`), or TOML by extension, read
//! by the data readers `load_data` uses
//! ([`parse_local_document`]), and opened through [`open_argument`] in the
//! launch context. Its layer's context is the file's own
//! ([`OpenedArgument::document_context`]), so a `page.stylesheet` it supplies
//! resolves beside it. The document's schema is selected from the Markdown
//! document's context, never a style file's.

use std::collections::HashMap;
use std::ffi::{OsStr, OsString};
use std::path::{Path, PathBuf};

use biscuit_file::{DataFormat, FileReference, FileReferenceError, FileReferenceKind, FileResolutionContext, FileType};
use color_eyre::eyre::{Report, Result, WrapErr, eyre};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::data::{DataError, DataSourceIdentity, MALFORMED_CODE, parse_local_document};
use darkmatter::markdown::compose::expression::{DataLocation, Diagnostic, DiagnosticSource, ReferenceDetails, Severity};
use darkmatter::markdown::compose::policy::{STYLE_DEPRECATED_CODE, STYLE_UNKNOWN_KEY_CODE};
use darkmatter::markdown::compose::report_diagnostics::INPUT_READ_CODE;
use darkmatter::markdown::schemas::{EffectiveSchema, InvalidTriggerFinding};
use darkmatter::style::layer::{STYLE_INVALID_ROOT_CODE, STYLE_INVALID_SETTER_CODE};
use darkmatter::style::{
    PreparedStyle, StyleDiagnostic, build_yaml_position_map, StyleLayer, StyleOrigin, StylePreparationError, StylePreparationPolicy, StyleSetter,
    StyleSourceKind, StyleSpan, prepare_style_layers,
};
use serde_json::Value;
use simplified_schema::PropertyPath;
use simplified_schema::coercion::PathSegment;
use simplified_schema::completion::PathStep;

use crate::args::RawStyleSetter;
use crate::commands::schema::validate::select_document_schema;
use crate::io::{DocumentArgumentError, OpenedArgument, open_argument};
use crate::output::output_diagnostic;
use crate::output::report::ClassifiedFailure;
use crate::request::MdRequest;

/// One `--style` argument, classified passively.
#[derive(Debug, Clone)]
pub struct StyleFileArgument {
    /// The whole argument as written.
    pub argument: String,
    /// The mapping selected inside the file; `None` for the whole file.
    pub offset: Option<PropertyPath>,
    /// The file part, which [`open_argument`] opens.
    pub reference: FileReference,
    pub format: DataFormat,
}

/// Classifies one `--style` argument without touching the filesystem (see
/// the module docs).
///
/// ## Errors
///
/// A run failure for `-`, an argument that is not Unicode or not a file
/// reference, an inline object (`{ margin: 4 }`), an HTTP(S) URL, an empty
/// offset or file part, an offset that is not a property path, and an
/// extension other than `json`, `json5`, `yaml`, `yml`, and `toml`.
pub fn classify_style_argument(argument: &OsStr) -> Result<StyleFileArgument> {
    let Some(text) = argument.to_str() else {
        let written = argument.to_string_lossy().into_owned();
        let source = FileReferenceError::InvalidSyntax(format!("`{written}` is not valid Unicode"));
        return Err(Report::new(DocumentArgumentError::new(written, source)))
            .wrap_err("a `--style` argument must be valid Unicode");
    };
    if text == "-" {
        return Err(refusal(text, "`--style -` is not supported: stdin is the Markdown input".to_string()));
    }
    if text.trim_start().starts_with('{') {
        return Err(refusal(
            text,
            format!("`--style` takes a style file, not an inline value; to set a style inline, use the setter `style=\"{text}\"`"),
        ));
    }

    let whole = parse_reference(text, text)?;
    let (offset, reference) = match split_offset(text, &whole) {
        None => (None, whole),
        Some((offset, file)) => {
            if offset.is_empty() || file.is_empty() {
                return Err(refusal(
                    text,
                    format!("`--style {text}` needs both an offset and a file in `offset@file`; spell a file whose name holds `@` with `./`"),
                ));
            }
            let path = PropertyPath::parse(offset)
                .map_err(|error| refusal(text, format!("the offset of `--style {text}` is not a property path: {error}")))?;
            if file == "-" {
                return Err(refusal(text, "a `--style` file cannot be stdin, which is the Markdown input".to_string()));
            }
            (Some(path), parse_reference(text, file)?)
        }
    };
    if reference.class().kind == FileReferenceKind::Url {
        let source = FileReferenceError::RemoteNotLocal(reference.raw().to_string());
        return Err(Report::new(DocumentArgumentError::new(text.to_string(), source)))
            .wrap_err(format!("`--style {text}`: style files are local; a URL is never fetched"));
    }
    let format = format_of(Path::new(reference.raw())).ok_or_else(|| unsupported(text, reference.raw()))?;
    Ok(StyleFileArgument { argument: text.to_string(), offset, reference, format })
}

/// One `--style` file, read and selected at its offset.
#[derive(Debug, Clone)]
pub struct LoadedStyleFile {
    /// The layer, in the file's own context, with positions for its keys
    /// when the format's locator proves them.
    pub layer: StyleLayer,
    /// The file the argument opened.
    pub path: PathBuf,
}

/// Classifies, opens, reads, and selects one `--style` argument.
///
/// The layer's value is not validated here: a non-object whole-file root is
/// left for [`prepare_style_layers`] to report as `dm.style.invalid_root`.
///
/// ## Errors
///
/// The [`classify_style_argument`] refusals; a [`DocumentArgumentError`]
/// (kept in the chain) when the file does not resolve; a run failure when it
/// cannot be read, is not UTF-8 or not valid in its format
/// (`dm.data.malformed`, located), or has no mapping at the offset
/// (`dm.style.invalid_root`, naming the key and the file).
pub fn load_style_file(argument: &OsStr, request: &MdRequest) -> Result<LoadedStyleFile> {
    let classified = classify_style_argument(argument)?;
    let text = classified.argument.as_str();
    let opened = open_argument(Path::new(classified.reference.raw()), request)
        .wrap_err_with(|| format!("`--style {text}` did not open"))?;
    let path = opened.path().to_path_buf();
    // Resolution may land on another spelling of the file; its own extension decides.
    let format = format_of(&path).ok_or_else(|| unsupported(text, &biscuit_file::to_portable_string(&path)))?;
    let bytes = std::fs::read(&path).map_err(|error| {
        refusal(text, format!("`--style {text}`: could not read `{}`: {error}", biscuit_file::to_portable_string(&path)))
    })?;
    let document = parse_local_document(bytes, format, &DataSourceIdentity::local(&path))
        .map_err(|error| malformed(text, &path, &error))?;

    let (steps, value) = match &classified.offset {
        None => (Vec::new(), document.value().clone()),
        Some(offset) => {
            let resolved = offset.resolve(document.value());
            match resolved.value() {
                Some(value @ Value::Object(_)) => (resolved.steps().iter().map(path_segment).collect(), value.clone()),
                Some(other) => {
                    let message = format!(
                        "`{offset}` in the style file `{}` is {}, not a mapping",
                        biscuit_file::to_portable_string(&path),
                        kind_name(other)
                    );
                    return Err(located_failure(text, &path, message));
                }
                None => {
                    let message =
                        format!("the style file `{}` has no `{offset}`", biscuit_file::to_portable_string(&path));
                    return Err(located_failure(text, &path, message));
                }
            }
        }
    };

    let positions = key_positions(&document, &steps, &value);
    let context = opened.document_context(request)?;
    Ok(LoadedStyleFile { layer: StyleLayer::file(value, text, context, Some(positions.into_iter().collect())), path })
}

/// The style setters as one layer in the launch context, each key parsed as
/// a [`PropertyPath`]; their values are bound during preparation.
///
/// ## Errors
///
/// `dm.style.invalid_setter`, naming each argument whose key is not a
/// property path (`style..x=1`); a launch context that cannot be built.
pub fn setter_layer(setters: &[RawStyleSetter], request: &MdRequest) -> Result<StyleLayer> {
    let mut parsed = Vec::with_capacity(setters.len());
    let mut failures = Vec::new();
    for setter in setters {
        match PropertyPath::parse(&setter.key) {
            Ok(path) => parsed.push(StyleSetter::new(&setter.argument, path, &setter.value)),
            Err(error) => failures.push(output_diagnostic(
                STYLE_INVALID_SETTER_CODE,
                Severity::Error,
                &format!("the style setter `{}` does not name a property path: {error}", setter.argument),
            )),
        }
    }
    if !failures.is_empty() {
        let message = failures.iter().map(|failure| failure.message.as_str()).collect::<Vec<_>>().join("; ");
        return Err(ClassifiedFailure::located(eyre!(message), failures));
    }
    Ok(StyleLayer::setters(parsed, request.launch_context()?.clone()))
}

/// A render's CLI style inputs, read and ready to merge over a document
/// layer.
#[derive(Clone)]
pub struct CliStyleInputs {
    /// The Markdown document's effective schema, selected once.
    pub schema: Option<EffectiveSchema>,
    /// Each `--style` file in order, then the setters (when any).
    pub layers: Vec<StyleLayer>,
    /// The schema triggers the selection's discovery skipped as invalid; the
    /// caller reports them (`commands::trigger_findings`).
    pub trigger_findings: Vec<InvalidTriggerFinding>,
    /// The opened Markdown document, named by document-layer records.
    document: Option<PathBuf>,
    /// Each `--style` argument's opened file, for its records' `source`.
    files: HashMap<String, PathBuf>,
}

impl std::fmt::Debug for CliStyleInputs {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CliStyleInputs")
            .field("schema", &self.schema.is_some())
            .field("layers", &self.layers)
            .field("document", &self.document)
            .finish_non_exhaustive()
    }
}

/// Selects the document's schema and reads every `--style` file (in order)
/// and the setters.
///
/// `opened` is the Markdown argument (`None` for stdin, whose schema is
/// selected in the launch context).
///
/// ## Errors
///
/// [`ReadStyleInputs::failure`]; [`read_cli_style_inputs`] keeps what was
/// read before it.
pub fn prepare_cli_style_inputs(
    document: &Markdown,
    opened: Option<&OpenedArgument>,
    styles: &[OsString],
    setters: &[RawStyleSetter],
    request: &MdRequest,
) -> Result<CliStyleInputs> {
    let read = read_cli_style_inputs(document, opened, styles, setters, request);
    match read.failure {
        Some(failure) => Err(failure),
        None => Ok(read.inputs),
    }
}

/// A render's CLI style inputs as far as they could be read.
#[derive(Debug)]
pub struct ReadStyleInputs {
    /// The selected schema (`None` when selection failed), the trigger
    /// findings discovery made, and every layer read before the failure.
    pub inputs: CliStyleInputs,
    /// The first failure: the schema selection (only when a CLI layer needs
    /// it), a [`load_style_file`] failure, or a [`setter_layer`] failure.
    /// Nothing after it is read.
    pub failure: Option<Report>,
}

/// [`prepare_cli_style_inputs`], keeping the inputs read before a failure so
/// the run still reports their warnings, the trigger findings, and the
/// document's own style warnings beside it.
pub fn read_cli_style_inputs(
    document: &Markdown,
    opened: Option<&OpenedArgument>,
    styles: &[OsString],
    setters: &[RawStyleSetter],
    request: &MdRequest,
) -> ReadStyleInputs {
    let (trigger_findings, selected) = select_document_schema(document, opened, request);
    let mut inputs = CliStyleInputs {
        schema: None,
        layers: Vec::with_capacity(styles.len() + 1),
        trigger_findings,
        document: opened.map(|opened| opened.path().to_path_buf()),
        files: HashMap::new(),
    };
    let failed = |inputs, failure| ReadStyleInputs { inputs, failure: Some(failure) };
    match selected {
        Ok(schema) => inputs.schema = schema,
        // Only a CLI layer needs the schema's verdict. A document-only render
        // keeps rendering with the built-in style descriptor, as it did before
        // rendering selected a schema; `md schema validate` reports the schema.
        Err(error) if styles.is_empty() && setters.is_empty() => {
            tracing::debug!(error = %format!("{error:#}"), "no effective schema; using the built-in style descriptor");
        }
        Err(error) => return failed(inputs, error),
    }
    for style in styles {
        match load_style_file(style, request) {
            Ok(loaded) => {
                inputs.files.insert(loaded.layer.source().to_string(), loaded.path);
                inputs.layers.push(loaded.layer);
            }
            Err(error) => return failed(inputs, ClassifiedFailure::located_from(error)),
        }
    }
    if !setters.is_empty() {
        match setter_layer(setters, request) {
            Ok(layer) => inputs.layers.push(layer),
            Err(error) => return failed(inputs, ClassifiedFailure::located_from(error)),
        }
    }
    ReadStyleInputs { inputs, failure: None }
}

impl CliStyleInputs {
    /// Merges `document_layer` (the document's own `style:`) and these
    /// layers under `policy`.
    ///
    /// ## Errors
    ///
    /// A [`ClassifiedFailure`] whose records are every problem (an error,
    /// with its code: `dm.schema.*` or `dm.style.*`) followed by the warnings
    /// gathered on the way.
    pub fn prepare(&self, document_layer: Option<StyleLayer>, policy: StylePreparationPolicy) -> Result<PreparedStyle> {
        self.prepare_layers(document_layer, policy).map_err(|error| self.failure(&error))
    }

    fn prepare_layers(
        &self,
        document_layer: Option<StyleLayer>,
        policy: StylePreparationPolicy,
    ) -> std::result::Result<PreparedStyle, StylePreparationError> {
        let layers: Vec<StyleLayer> = document_layer.into_iter().chain(self.layers.iter().cloned()).collect();
        prepare_style_layers(&layers, self.schema.as_ref(), policy)
    }

    /// Whether any `--style` file or setter was given.
    pub fn has_cli_layers(&self) -> bool {
        !self.layers.is_empty()
    }

    /// [`Self::warnings`], each marked with whether the document's own
    /// `style:` raised it.
    fn warning_records(&self, diagnostics: &[StyleDiagnostic]) -> Vec<StyleWarningRecord> {
        diagnostics
            .iter()
            .filter_map(|diagnostic| {
                let record = self.warning(diagnostic)?;
                Some(StyleWarningRecord { from_document: diagnostic.origin.kind == StyleSourceKind::Document, record })
            })
            .collect()
    }

    /// The warnings of a prepared style as run records; a notice with no
    /// code (`KnownButInactive`) is left out.
    pub fn warnings(&self, prepared: &PreparedStyle) -> Vec<Diagnostic> {
        prepared.diagnostics().iter().filter_map(|diagnostic| self.warning(diagnostic)).collect()
    }

    fn failure(&self, error: &StylePreparationError) -> Report {
        let mut records: Vec<Diagnostic> = error
            .problems
            .iter()
            .map(|problem| {
                let mut record = output_diagnostic(problem.code, Severity::Error, &problem.to_string());
                record.source = problem.origin.as_ref().and_then(|origin| self.source_of(origin));
                record
            })
            .collect();
        records.extend(error.diagnostics.iter().filter_map(|diagnostic| self.warning(diagnostic)));
        ClassifiedFailure::located(eyre!("{error}"), records)
    }

    fn warning(&self, diagnostic: &StyleDiagnostic) -> Option<Diagnostic> {
        let code = diagnostic.code?;
        let message = format!("{}{}", diagnostic.message()?, origin_suffix(&diagnostic.origin));
        let mut record = output_diagnostic(code, Severity::Warning, &message);
        record.source = self.source_of(&diagnostic.origin);
        Some(record)
    }

    /// Where a layer's value was written: the style file or the document,
    /// with the key's position when known. A setter has no place in a file.
    fn source_of(&self, origin: &StyleOrigin) -> Option<DiagnosticSource> {
        let file = match origin.kind {
            StyleSourceKind::Setters => return None,
            StyleSourceKind::File => self.files.get(&origin.source)?,
            StyleSourceKind::Document => self.document.as_ref()?,
        };
        let file = Some(PathBuf::from(biscuit_file::to_portable_string(file)));
        Some(match &origin.span {
            Some(span) => DiagnosticSource::at(file, span.line as usize, span.column as usize),
            None => DiagnosticSource::file_only(file),
        })
    }
}

/// One style warning a render reports.
#[derive(Debug, Clone)]
pub struct StyleWarningRecord {
    /// The document's own `style:` raised it. A source output (Markdown, or
    /// a data output's `document`) builds no page, so only a data output
    /// reports these; a CLI layer's warnings are always reported.
    pub from_document: bool,
    pub record: Diagnostic,
}

/// A render's effective style: the document's `style:` under each `--style`
/// file and the setters.
#[derive(Debug, Clone)]
pub struct RenderStyle {
    pub style: PreparedStyle,
    /// The warnings, by layer in precedence order.
    pub warnings: Vec<StyleWarningRecord>,
}

/// The document's own `style:` as the bottom layer, named by `path` (the
/// opened file; `None` for stdin) and in `context` (the opened document's,
/// or the launch context for stdin), with its keys' positions as document
/// lines.
pub fn document_style_layer(document: &Markdown, path: Option<&Path>, context: FileResolutionContext) -> StyleLayer {
    let frontmatter = document.frontmatter();
    let value = frontmatter.as_map().get("style").cloned().unwrap_or(Value::Null);
    // The position map counts the YAML block's own lines; a frontmatter block
    // always opens with the `---` on the document's first line.
    let positions = frontmatter.raw_source().map(|raw| {
        build_yaml_position_map(raw)
            .into_iter()
            .map(|(key, span)| (key, StyleSpan { line: span.line + 1, ..span }))
            .collect()
    });
    let identity = path.map_or_else(|| "<stdin>".to_string(), biscuit_file::to_portable_string);
    StyleLayer::document(value, identity, context, positions)
}

/// Merges `document_layer` and `inputs` under `policy`, for a render that
/// `builds_page` (terminal, plain, HTML, MarkdownPlus) or prints its source
/// (Markdown and the data family).
///
/// A source output never reads the document's style, so a problem in the
/// document's own `style:` does not stop it; a denied warning (`--deny
/// style`) still does, and every CLI layer is checked on every target.
/// `KnownButInactive` notices go to the debug log.
///
/// ## Errors
///
/// A located [`ClassifiedFailure`] holding every problem.
pub fn prepare_render_style(
    inputs: &CliStyleInputs,
    document_layer: StyleLayer,
    policy: StylePreparationPolicy,
    builds_page: bool,
) -> Result<RenderStyle> {
    render_style(inputs, document_layer, policy, builds_page).map_err(|error| inputs.failure(&error))
}

/// A render style that was not prepared, and the warnings its run still
/// reports.
#[derive(Debug)]
pub struct RenderStyleFailure {
    /// The run's failure, as [`prepare_render_style`] or the read failure
    /// gives it.
    pub error: Report,
    /// The warnings of every layer that was read, in precedence order; a
    /// data output's failure records may repeat them.
    pub warnings: Vec<StyleWarningRecord>,
}

/// [`prepare_render_style`] over what [`read_cli_style_inputs`] read. When
/// reading failed, the layers it did read (the document's own `style:` and
/// the earlier CLI layers) are still merged, only for their warnings.
///
/// ## Errors
///
/// The read failure, or [`prepare_render_style`]'s, each with the warnings.
pub fn prepare_read_render_style(
    read: ReadStyleInputs,
    document_layer: StyleLayer,
    policy: StylePreparationPolicy,
    builds_page: bool,
) -> std::result::Result<RenderStyle, RenderStyleFailure> {
    let inputs = &read.inputs;
    let prepared = render_style(inputs, document_layer, policy, builds_page);
    match (read.failure, prepared) {
        (None, Ok(style)) => Ok(style),
        (None, Err(error)) => Err(RenderStyleFailure { warnings: inputs.warning_records(&error.diagnostics), error: inputs.failure(&error) }),
        (Some(error), Ok(style)) => Err(RenderStyleFailure { error, warnings: style.warnings }),
        (Some(error), Err(problems)) => Err(RenderStyleFailure { error, warnings: inputs.warning_records(&problems.diagnostics) }),
    }
}

fn render_style(
    inputs: &CliStyleInputs,
    document_layer: StyleLayer,
    policy: StylePreparationPolicy,
    builds_page: bool,
) -> std::result::Result<RenderStyle, StylePreparationError> {
    let (style, document_warnings) = match inputs.prepare_layers(Some(document_layer), policy) {
        Ok(style) => (style, Vec::new()),
        Err(error) if !builds_page && error.problems.iter().all(is_ignored_by_source_output) => {
            // The CLI layers alone, so their own problems and warnings are
            // still found; the document's warnings come from the first pass,
            // and a CLI layer's failure keeps them.
            let document: Vec<StyleDiagnostic> =
                error.diagnostics.into_iter().filter(|diagnostic| diagnostic.origin.kind == StyleSourceKind::Document).collect();
            match inputs.prepare_layers(None, policy) {
                Ok(style) => (style, document),
                Err(mut error) => {
                    error.diagnostics.splice(0..0, document);
                    return Err(error);
                }
            }
        }
        Err(error) => return Err(error),
    };
    let diagnostics: Vec<StyleDiagnostic> = document_warnings.into_iter().chain(style.diagnostics().iter().cloned()).collect();
    for diagnostic in diagnostics.iter().filter(|diagnostic| diagnostic.code.is_none()) {
        tracing::info!(path = %diagnostic.warning.path, "style key parsed but not yet wired");
    }
    Ok(RenderStyle { warnings: inputs.warning_records(&diagnostics), style })
}

/// A document's own problem that a source output, which never reads its
/// style, does not report; a warning `--deny` made an error is not one.
fn is_ignored_by_source_output(problem: &darkmatter::style::StyleProblem) -> bool {
    problem.origin.as_ref().is_some_and(|origin| origin.kind == StyleSourceKind::Document)
        && problem.code != STYLE_UNKNOWN_KEY_CODE
        && problem.code != STYLE_DEPRECATED_CODE
}

/// The text a warning adds to name its layer, as [`StyleProblem`]'s display
/// does for a problem.
///
/// [`StyleProblem`]: darkmatter::style::StyleProblem
fn origin_suffix(origin: &StyleOrigin) -> String {
    let at = origin.span.as_ref().map(|span| format!(" at line {}, column {}", span.line, span.column)).unwrap_or_default();
    match origin.kind {
        StyleSourceKind::Document => String::new(),
        StyleSourceKind::File => format!(" (in the style file `{}`{at})", origin.source),
        StyleSourceKind::Setters => format!(" (from the setter `{}`)", origin.source),
    }
}

fn parse_reference(argument: &str, file: &str) -> Result<FileReference> {
    FileReference::new(file).map_err(|source| {
        Report::new(DocumentArgumentError::new(argument.to_string(), source))
            .wrap_err(format!("`--style {argument}` is not a file reference"))
    })
}

/// The offset and file of `offset@file`, for an implicit relative reference
/// whose first `@` is not leading and has no separator before it.
fn split_offset<'a>(text: &'a str, whole: &FileReference) -> Option<(&'a str, &'a str)> {
    if whole.class().kind != FileReferenceKind::ImplicitRelative {
        return None;
    }
    let at = text.find('@')?;
    let offset = &text[..at];
    (at > 0 && !has_separator(offset)).then(|| (offset, &text[at + 1..]))
}

/// Whether `offset` holds `/` or a `\` that is not one of the property-path
/// escapes `\.` and `\\`, so `a\.b@x.yaml` is an offset and
/// `themes\user@host.yaml` a Windows path.
fn has_separator(offset: &str) -> bool {
    let mut characters = offset.chars();
    while let Some(character) = characters.next() {
        match character {
            '/' => return true,
            '\\' => match characters.next() {
                Some('.' | '\\') => {}
                _ => return true,
            },
            _ => {}
        }
    }
    false
}

/// The format a style file's extension selects; `None` for any other.
fn format_of(path: &Path) -> Option<DataFormat> {
    match biscuit_file::detect_from_extension(path) {
        FileType::Json => Some(DataFormat::Json),
        FileType::Json5 => Some(DataFormat::Json5),
        FileType::Yaml => Some(DataFormat::Yaml),
        FileType::Toml => Some(DataFormat::Toml),
        FileType::Jsonl | FileType::Csv | FileType::Tsv | FileType::Markdown | FileType::Pdf | FileType::Unknown => None,
    }
}

fn path_segment(step: &PathStep<'_>) -> PathSegment {
    match step {
        PathStep::Key(key) => PathSegment::Key((*key).to_string()),
        PathStep::Index(index) => PathSegment::Index(*index),
    }
}

/// The position of every mapping key below `value`, keyed relative to it
/// (`page.color`); `steps` lead from the file's root to `value`.
fn key_positions(
    document: &darkmatter::markdown::compose::expression::data::LocalDataDocument,
    steps: &[PathSegment],
    value: &Value,
) -> Vec<(String, StyleSpan)> {
    let mut keys: Vec<(String, Vec<PathSegment>)> = Vec::new();
    collect_keys(value, &mut Vec::new(), steps, &mut keys);
    let paths: Vec<Vec<PathSegment>> = keys.iter().map(|(_, path)| path.clone()).collect();
    keys.into_iter()
        .zip(document.positions(&paths))
        .filter_map(|((key, _), position)| {
            let (line, column) = position?;
            let span = StyleSpan { line: u32::try_from(line).ok()?, column: u32::try_from(column).ok()?, length: 0 };
            Some((key, span))
        })
        .collect()
}

fn collect_keys(value: &Value, relative: &mut Vec<String>, steps: &[PathSegment], out: &mut Vec<(String, Vec<PathSegment>)>) {
    let Value::Object(members) = value else { return };
    for (key, child) in members {
        relative.push(key.clone());
        let path = steps.iter().cloned().chain(relative.iter().cloned().map(PathSegment::Key)).collect();
        out.push((relative.join("."), path));
        collect_keys(child, relative, steps, out);
        relative.pop();
    }
}

fn kind_name(value: &Value) -> &'static str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "a boolean",
        Value::Number(_) => "a number",
        Value::String(_) => "a string",
        Value::Array(_) => "an array",
        Value::Object(_) => "a mapping",
    }
}

/// A refused `--style` argument: `dm.input.read_failure`, naming the argument.
fn refusal(argument: &str, message: String) -> Report {
    let mut record = output_diagnostic(INPUT_READ_CODE, Severity::Error, &message);
    record.reference_details = Some(ReferenceDetails { reference: Some(argument.to_string()), ..ReferenceDetails::default() });
    ClassifiedFailure::located(eyre!(message), vec![record])
}

fn unsupported(argument: &str, file: &str) -> Report {
    let message = format!(
        "`--style {argument}`: `{file}` is not a style file; a style file is JSON, JSON5, YAML (`.yaml`, `.yml`), or TOML"
    );
    refusal(argument, message)
}

/// A file whose selected value is missing or not a mapping:
/// `dm.style.invalid_root`, in that file.
fn located_failure(argument: &str, path: &Path, message: String) -> Report {
    let mut record = output_diagnostic(STYLE_INVALID_ROOT_CODE, Severity::Error, &message);
    record.source = Some(DiagnosticSource::file_only(Some(PathBuf::from(biscuit_file::to_portable_string(path)))));
    record.reference_details = Some(ReferenceDetails { reference: Some(argument.to_string()), ..ReferenceDetails::default() });
    ClassifiedFailure::located(eyre!(message), vec![record])
}

/// A file that is not UTF-8 or not valid in its format: `dm.data.malformed`,
/// located as `load_data` locates it.
fn malformed(argument: &str, path: &Path, error: &DataError) -> Report {
    let file = biscuit_file::to_portable_string(path);
    let (message, location) = match error {
        DataError::Malformed { message, location, .. } => (message.clone(), Some(location.clone())),
        other => (other.to_string(), None),
    };
    let message = format!("`--style {argument}`: `{file}` is not a valid style file: {message}");
    let code = if location.is_some() { MALFORMED_CODE } else { INPUT_READ_CODE };
    let mut record = output_diagnostic(code, Severity::Error, &message);
    if let Some(location) = location {
        let file_path = Some(PathBuf::from(&file));
        record.source = Some(match (location.line, location.column) {
            (Some(line), Some(column)) => DiagnosticSource::at(file_path, line, column),
            _ => DiagnosticSource::file_only(file_path),
        });
        record.data_location = Some(DataLocation {
            file,
            pointer: location.pointer,
            line: location.line,
            column: location.column,
            byte_offset: location.byte_offset,
        });
    }
    ClassifiedFailure::located(eyre!(message), vec![record])
}

#[cfg(test)]
mod tests {
    use super::*;
    use biscuit_file::ResolutionFailure;

    fn classify(text: &str) -> Result<StyleFileArgument> {
        classify_style_argument(OsStr::new(text))
    }

    fn split(text: &str) -> (Option<Vec<String>>, String) {
        let classified = classify(text).unwrap_or_else(|error| panic!("{text}: {error:#}"));
        (classified.offset.map(|offset| offset.segments().to_vec()), classified.reference.raw().to_string())
    }

    #[test]
    fn only_an_implicit_relative_reference_splits_at_its_first_at() {
        assert_eq!(split("style.yaml"), (None, "style.yaml".into()));
        assert_eq!(split("style@style.yaml"), (Some(vec!["style".into()]), "style.yaml".into()));
        assert_eq!(split("theme.dark@style.yaml"), (Some(vec!["theme".into(), "dark".into()]), "style.yaml".into()));
        assert_eq!(split(r"a\.b@style.yaml"), (Some(vec!["a.b".into()]), "style.yaml".into()));
        assert_eq!(split("style@@/shared.yaml"), (Some(vec!["style".into()]), "@/shared.yaml".into()));
        assert_eq!(split("@/shared.yaml"), (None, "@/shared.yaml".into()));
        assert_eq!(split("./user@host.yaml"), (None, "./user@host.yaml".into()));
        assert_eq!(split("themes/user@host.yaml"), (None, "themes/user@host.yaml".into()));
        assert_eq!(split(r"themes\user@host.yaml"), (None, r"themes\user@host.yaml".into()));
        assert_eq!(split("/abs/user@host.yaml"), (None, "/abs/user@host.yaml".into()));
        assert_eq!(split("~/user@host.yaml"), (None, "~/user@host.yaml".into()));
        assert_eq!(split(r"C:\styles\user@host.yaml"), (None, r"C:\styles\user@host.yaml".into()));
    }

    #[test]
    fn a_refused_argument_fails_before_any_lookup() {
        for text in ["-", "style@", "a..b@style.yaml", "{ margin: 4 }", "style.jsonl", "style.csv", "notes.md", "style"] {
            assert!(classify(text).is_err(), "{text} is refused");
        }
        let remote = classify("https://example.com/s.yaml").expect_err("a URL is refused");
        let argument = remote.chain().find_map(|cause| cause.downcast_ref::<DocumentArgumentError>()).expect("typed");
        assert_eq!(argument.resolution_failure(), ResolutionFailure::UnsupportedRemote);
        let offset_remote = classify("style@https://example.com/s.yaml").expect_err("a URL after an offset is refused");
        assert!(offset_remote.chain().any(|cause| cause.downcast_ref::<DocumentArgumentError>().is_some()));
    }

    #[test]
    fn an_inline_object_names_the_inline_setter() {
        let error = classify("{ margin: 4 }").expect_err("not a file");
        assert!(error.to_string().contains("style=\"{ margin: 4 }\""), "{error}");
    }

    #[cfg(unix)]
    #[test]
    fn a_non_unicode_argument_is_an_invalid_reference() {
        use std::os::unix::ffi::OsStrExt;
        let error = classify_style_argument(OsStr::from_bytes(b"style\xFF.yaml")).expect_err("refused");
        let argument = error.chain().find_map(|cause| cause.downcast_ref::<DocumentArgumentError>()).expect("typed");
        assert_eq!(argument.resolution_failure(), ResolutionFailure::InvalidReference);
    }
}

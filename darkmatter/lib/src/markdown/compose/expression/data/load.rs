//! The `load_data(file, as, options)` expression function.
//!
//! One call runs in this order, so a failure never depends on work a later
//! step would do: a `null` file returns `null`; read the options object; prepare
//! the contract, when one is given (an omitted and a `null` `as` both skip
//! it); resolve the file or URL, let a slot's gate judge it, and
//! record it in the read set; choose the effective format (the `format`
//! option, else the extension); check that the options apply to it; for
//! CSV/TSV choose the row mode (from the contract, else from the header
//! mode) and check the header mode allows it; fetch a URL's text (its host policy is checked here, before any memo
//! lookup); read and decode the source through the raw memo, parse it in the
//! effective format and delimiter through the parse memo, and bind it with the
//! effective header mode through the bind memo: rows built, the root
//! normalized to a container (a `null` root is `[]`, a scalar root a
//! one-element array), then converted to the contract when there is one.
//! Only a fully bound value is returned.
//!
//! A local file that resolves to nothing is judged by the same request checks
//! ([`plan_read`]) before its miss is returned, against the extension of every
//! file it could have been, so an invalid request fails alike whether or not
//! the file exists and never reaches `try` as a recoverable miss.
//!
//! Every outcome is observed in the data runtime's log: a missing file's
//! probed candidates, and once per call a read with the hash of the bytes
//! read (`None` when the read, or the parse in this call's format, failed).
//! The hash is kept on the raw entry, so a memo hit at any layer observes the
//! same read again. Local bytes come through
//! [`simplified_schema::resolve::buffers::read`], so an editor's open buffer
//! is what a resolution inside a buffer scope sees.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use biscuit_file::{DataFormat, DelimitedRecord, FileType};
use serde_json::Value;
use tracing::field::Empty;

use super::contract::{PreparedContract, RowClassification, RowMode, judge_written_contract, require_header_rows, written_contract};
use super::error::{DataError, TabularSource};
use super::options::{DataOptions, HeaderMode, effective_delimiter, effective_header};
use super::parse::{decode_bytes, located_at, parse_text, strip_bom, utf8_failure};
use super::runtime::{DataSourceIdentity, LoadedData, LoadedSource, RawSource, SourceContent, SourceKey};
use crate::markdown::compose::expression::call_binding::{BoundCall, CallBindingError};
use crate::markdown::compose::expression::functions::{
    file_reference_diagnostic, file_reference_error, parse_file_ref, record_probed_candidates, remote_text_error,
    resolve_arg,
};
use crate::markdown::compose::expression::functions::ReadKind;
use crate::markdown::compose::expression::read_gate::ReadLocation;
use crate::markdown::compose::expression::resolve_ctx::is_remote_url;
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, ResolutionContext, trace};

pub(crate) const FUNCTION: &str = "load_data";

/// `load_data(file: file | null, as?: type-definition(container) | null, options?: {format, delimiter, header} | null) -> any | null`.
///
/// A `null` file returns `null` before the options are read, the contract is
/// prepared, or anything is read; the binder has already refused a literal
/// mistake. An omitted and a `null` `as` are the same call: the data is
/// parsed, its rows built, and its root normalized, but nothing converts it. Each call opens a `load_data` debug span recording the source,
/// its effective format, delimiter, and header, its size and item count, and
/// whether each memo layer answered.
///
/// ## Errors
///
/// - The file-reference family for a missing, unreadable, not-yet-fetchable,
///   or policy-denied source, and [`ExpressionError::RemoteRead`] (worded as
///   a plain evaluation failure, typed by its kind) for a failed fetch.
/// - [`ExpressionError::Data`] for every `dm.data.*` failure, an invalid
///   contract, a contract import that cannot be read
///   ([`DataError::ContractImport`], projected like a file reference), and
///   options that cannot apply (`dm.data.invalid_options`, located at the
///   call: execution keeps no per-argument position). A missing local source
///   fails with the request's `dm.data.unsupported_format`,
///   `dm.data.invalid_options`, or `dm.data.row_shape_conflict` instead of a
///   miss when no file it could resolve to would accept the request (see
///   [`check_absent_request`]).
#[tracing::instrument(
    level = "debug",
    name = "load_data",
    skip_all,
    fields(
        identity = Empty,
        format = Empty,
        delimiter = Empty,
        header = Empty,
        bytes = Empty,
        items = Empty,
        raw_memo = Empty,
        source_memo = Empty,
        bind_memo = Empty
    )
)]
pub(crate) fn load_data_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    // Only a value this call read is a direct load; a `null` passthrough is not.
    trace::derive();
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let options = DataOptions::from_bound(call.args().get(2)).map_err(fail)?;
    let contract = match call.args().get(1) {
        None | Some(Value::Null) => None,
        Some(carrier) => {
            let written = written_contract(carrier).ok_or_else(|| ExpressionError::ContractViolation {
                function: FUNCTION.to_string(),
                message: "argument 1 did not bind to a type definition".to_string(),
            })?;
            Some(ctx.data.prepare_contract(&written, ctx)?)
        }
    };
    let contract = contract.as_deref();

    let remote = is_remote_url(raw);
    let identity = match url::Url::parse(raw) {
        Ok(url) if remote => DataSourceIdentity::Remote(Box::new(url)),
        _ => {
            let file_ref = parse_file_ref(FUNCTION, raw, ctx)?;
            // An editor observation asks its gate before each candidate is
            // probed, and observes exactly the candidates it probed.
            let resolved = match &ctx.editor {
                Some(editor) => super::super::editor_reads::resolve_local(FUNCTION, raw, &file_ref, ctx, editor)?,
                None => resolve_arg(FUNCTION, raw, ctx)?,
            };
            let Some(path) = resolved else {
                // An invalid request is invalid whether or not the file
                // exists, so it must not reach `try` as a miss.
                check_absent_request(&file_ref, &options, contract, ctx).map_err(fail)?;
                // Creating any probed candidate would change the answer.
                if ctx.editor.is_none() {
                    record_probed_candidates(&file_ref, ctx);
                }
                return Err(file_reference_error(FUNCTION, raw, ctx, FileRefFailure::NotFound, None));
            };
            DataSourceIdentity::local(&path)
        }
    };
    // A slot's gate decides before the target is read, recorded, or looked
    // up in the memo a document's own reads share.
    ctx.authorize_slot_read(FUNCTION, ReadKind::Content, || {
        Some(match &identity {
            DataSourceIdentity::Local(path) => ReadLocation::Local(path.clone()),
            DataSourceIdentity::Remote(url) => ReadLocation::Remote(url.as_ref().clone()),
        })
    })?;
    // Recorded before anything that can fail, so a failed read, a malformed
    // file, and a denied URL are all dependencies.
    ctx.data.record(&identity);
    // A failure before the read is observed as a read that produced nothing.
    let unread = |error: ExpressionError| {
        ctx.data.observe_read(&identity, None);
        error
    };
    let span = tracing::Span::current();
    span.record("identity", tracing::field::display(&identity));

    let ReadPlan { format, delimiter, header, row_mode } = plan_read(&options, contract, extension_format(&identity))
        .map_err(|refusal| refusal.into_error(|| identity.clone()))
        .map_err(fail)
        .map_err(unread)?;
    span.record("format", tracing::field::debug(format));
    if let Some(delimiter) = delimiter {
        span.record("delimiter", tracing::field::debug(char::from(delimiter)));
    }
    if let Some(header) = &header {
        span.record("header", tracing::field::debug(header));
    }

    // The fetch enforces host policy and is single-flight and cached by the
    // remote runtime itself, so authorization precedes every memo layer.
    let origin = match &identity {
        DataSourceIdentity::Local(path) => Origin::Local(path),
        DataSourceIdentity::Remote(_) => match ctx.fetch_remote_text(raw) {
            Ok(Some(body)) => Origin::Fetched(body),
            Ok(None) => {
                return Err(unread(file_reference_error(FUNCTION, raw, ctx, FileRefFailure::RemoteNotEnabled, None)));
            }
            Err(error) => return Err(unread(remote_text_error(FUNCTION, raw, ctx, error))),
        },
    };

    // An editor read that cannot complete is pending, not a data failure:
    // the loader leaves a deferral (which the memo never stores) and the
    // pending failure is returned in its place.
    let editor_pending: std::cell::RefCell<Option<ExpressionError>> = std::cell::RefCell::new(None);
    let raw_source = ctx.data.load_raw(&identity, || match origin {
        Origin::Fetched(body) => Ok(RawSource {
            text: Ok(Arc::from(strip_bom(&body))),
            len: body.len(),
            hash: biscuit_hash::blake3_hash_bytes(body.as_bytes()),
        }),
        Origin::Local(path) if ctx.editor.is_some() => {
            let editor = ctx.editor.as_deref().expect("checked by the guard");
            match super::super::editor_reads::read_local(FUNCTION, path, ctx, editor) {
                Ok(bytes) => {
                    Ok(RawSource { text: decode_bytes(&bytes), len: bytes.len(), hash: biscuit_hash::blake3_hash_bytes(&bytes) })
                }
                Err(pending) => {
                    *editor_pending.borrow_mut() = Some(pending);
                    Err(Arc::new(DataError::FileReference(file_reference_diagnostic(
                        FUNCTION,
                        raw,
                        ctx,
                        FileRefFailure::RemoteNotEnabled,
                        None,
                    ))))
                }
            }
        }
        Origin::Local(path) => {
            let bytes = simplified_schema::resolve::buffers::read(path).map_err(|error| {
                Arc::new(DataError::FileReference(file_reference_diagnostic(
                    FUNCTION,
                    raw,
                    ctx,
                    FileRefFailure::NotFound,
                    Some(biscuit_file::FileReferenceError::Io { path: path.to_path_buf(), source: error }),
                )))
            })?;
            Ok(RawSource { text: decode_bytes(&bytes), len: bytes.len(), hash: biscuit_hash::blake3_hash_bytes(&bytes) })
        }
    });
    let raw_source = match raw_source {
        Ok(entry) => entry,
        Err(error) => {
            let error = editor_pending.into_inner().unwrap_or_else(|| ExpressionError::from(error));
            return Err(unread(error));
        }
    };
    span.record("raw_memo", raw_source.answered_from_memo());

    let key = SourceKey { identity: identity.clone(), format, delimiter };
    let source = ctx.data.load_source(&key, &raw_source, |read| {
        let text = read.text.clone().map_err(|offset| Arc::new(utf8_failure(&identity, format, offset)))?;
        let content = parse_text(&text, format, delimiter, &identity)?;
        ctx.data.note_parse();
        Ok(LoadedSource { content, format, text, bytes: read.len })
    });
    // Observed once per call, whichever layer answers; a read or parse that
    // failed in this call's format is still a dependency, with no hash.
    ctx.data.observe_read(&identity, source.is_ok().then_some(raw_source.value().hash));
    let source = source.map_err(ExpressionError::from)?;
    span.record("bytes", source.value().bytes);
    span.record("source_memo", source.from_memo());

    let contract_key = contract.map_or("", |contract| contract.key.as_str());
    let bound = ctx
        .data
        .bind(&source, contract_key, header.clone(), |loaded| {
            bind_source(loaded, contract, &identity, row_mode, header.as_ref())
        })
        .map_err(ExpressionError::from)?;
    span.record("items", bound.value().items);
    span.record("bind_memo", bound.from_memo());
    trace::record_load(|| trace::LoadOrigin {
        identity: identity.clone(),
        format,
        text: source.value().text.clone(),
        table: bound.value().table.clone(),
        wrapped: bound.value().wrapped,
    });
    if super::super::editor_reads::is_collecting_calls() {
        super::super::editor_reads::note_read_mode(contract.is_none() && header == Some(HeaderMode::None));
    }
    Ok(bound.value().value.clone())
}

/// Where a source's text comes from: a local file still to be read, or a
/// URL's text the remote runtime already fetched.
enum Origin<'a> {
    Local(&'a Path),
    Fetched(String),
}

fn fail(error: DataError) -> ExpressionError {
    ExpressionError::from(Arc::new(error))
}

/// How a request reads its source once the request checks pass.
#[derive(Debug, Clone, PartialEq, Eq)]
struct ReadPlan {
    format: DataFormat,
    delimiter: Option<u8>,
    /// The effective header mode; `None` for a format that is not tabular.
    header: Option<HeaderMode>,
    /// The CSV/TSV row mode; `None` for a format that is not tabular.
    row_mode: Option<RowMode>,
}

/// Why [`plan_read`] refused a request.
#[derive(Debug)]
enum Refusal {
    /// No `format` option, and the source's extension names no data format.
    /// The error is built by the caller, which alone knows what to name.
    UnsupportedExtension(Option<String>),
    /// The options or contract cannot apply to the effective format. Boxed:
    /// a `DataError` with this enum's tag crosses the workspace's
    /// `large-error-threshold` on Windows, where `PathBuf` is wider.
    Invalid(Box<DataError>),
}

impl Refusal {
    /// The error this refusal reports, naming `identity` when the extension
    /// was the problem.
    fn into_error(self, identity: impl FnOnce() -> DataSourceIdentity) -> DataError {
        match self {
            Self::UnsupportedExtension(extension) => DataError::UnsupportedFormat { identity: identity(), extension },
            Self::Invalid(error) => *error,
        }
    }
}

/// The request checks every `load_data` read passes before its source is
/// read, and that a missing source passes before its miss is reported:
/// choose the effective format (the `format` option, else `extension`'s),
/// check that the options apply to it, and for CSV/TSV choose the row mode
/// ([`select_row_mode`]). It reads nothing, so a present and an absent
/// source are judged alike.
///
/// `extension` is the source's extension and the format it names (`Err`
/// carrying the unreadable extension); it is ignored when a `format` option
/// is given.
fn plan_read(
    options: &DataOptions,
    contract: Option<&PreparedContract>,
    extension: Result<DataFormat, Option<String>>,
) -> Result<ReadPlan, Refusal> {
    // An explicit format skips the extension entirely, so an extensionless
    // URL or an unsupported extension reads as the format chosen.
    let format = match options.format {
        Some(format) => format,
        None => extension.map_err(Refusal::UnsupportedExtension)?,
    };
    options.check_applicability(format).map_err(|error| Refusal::Invalid(Box::new(error)))?;
    let delimiter = effective_delimiter(format, options.delimiter);
    let header = effective_header(format, options.header.clone());
    let row_mode = match &header {
        Some(header) => Some(select_row_mode(contract, header).map_err(|error| Refusal::Invalid(Box::new(error)))?),
        None => None,
    };
    Ok(ReadPlan { format, delimiter, header, row_mode })
}

/// The row shape of a CSV/TSV read: the contract's, which the header mode
/// must allow ([`require_header_rows`]), or without a contract the header
/// mode's own (object rows for a header record or explicit names, lists of
/// cells for `header: false`).
///
/// ## Errors
///
/// [`DataError::RowShapeConflict`] for a contract that selects no row shape,
/// or whose shape the header mode cannot read.
fn select_row_mode(contract: Option<&PreparedContract>, header: &HeaderMode) -> Result<RowMode, DataError> {
    let Some(contract) = contract else {
        return Ok(match header {
            HeaderMode::FirstRecord | HeaderMode::Names(_) => RowMode::Object,
            HeaderMode::None => RowMode::List,
        });
    };
    let row_shape = |conflict| DataError::RowShapeConflict { written: contract.written.clone(), conflict };
    let mode = contract.row_mode().map_err(row_shape)?;
    require_header_rows(header, RowClassification::Known(mode)).map_err(row_shape)?;
    Ok(mode)
}

/// Judges the request of a local source that resolved to nothing by every
/// file it could have resolved to ([`absent_candidates`]): `Ok` when any of
/// them would accept the request, so creating that file would load it and
/// the miss is the honest answer; otherwise the first candidate's refusal,
/// in resolution order. Nothing is read, probed, or canonicalized except to
/// name the file in a `dm.data.unsupported_format` error.
///
/// ## Errors
///
/// `dm.data.unsupported_format`, `dm.data.invalid_options`, or
/// `dm.data.row_shape_conflict`, as [`plan_read`] reports them.
fn check_absent_request(
    file_ref: &biscuit_file::FileReference,
    options: &DataOptions,
    contract: Option<&PreparedContract>,
    ctx: &ResolutionContext,
) -> Result<(), DataError> {
    judge_absent(absent_candidates(file_ref, ctx), options, contract)
}

/// [`check_absent_request`]'s verdict over `candidates`.
fn judge_absent(
    candidates: Vec<AbsentCandidate>,
    options: &DataOptions,
    contract: Option<&PreparedContract>,
) -> Result<(), DataError> {
    let mut first_refusal = None;
    for candidate in candidates {
        match plan_read(options, contract, data_format_of(&candidate.path).ok_or_else(|| extension_text(&candidate.path))) {
            Ok(_) => return Ok(()),
            Err(refusal) => {
                first_refusal.get_or_insert((candidate, refusal));
            }
        }
    }
    match first_refusal {
        None => Ok(()),
        Some((candidate, refusal)) => Err(refusal.into_error(|| candidate.identity())),
    }
}

/// A file a missing source could have been.
struct AbsentCandidate {
    /// The path whose extension names the format the file would be read as.
    path: std::path::PathBuf,
    /// The authored reference, when `path` is not a resolution-plan path.
    authored: Option<String>,
}

impl AbsentCandidate {
    /// The identity an unsupported-format error names: a planned path's,
    /// or the authored reference itself, which a search has not placed at
    /// any path (never an invented resolved one).
    fn identity(&self) -> DataSourceIdentity {
        match &self.authored {
            Some(authored) => DataSourceIdentity::Local(std::path::PathBuf::from(authored)),
            None => DataSourceIdentity::local(&self.path),
        }
    }
}

/// The files a missing local source could have been: every path of a
/// nonrecursive reference's resolution plan, in resolution order. A
/// recursive (`%`) reference's plan holds search roots rather than files, so
/// it, like a reference whose plan cannot be listed, is judged by its
/// payload after environment interpolation (the relative path any match
/// would end in, so its extension is that match's) and named by its authored
/// text. Never empty.
fn absent_candidates(file_ref: &biscuit_file::FileReference, ctx: &ResolutionContext) -> Vec<AbsentCandidate> {
    let file_context = ctx.file_context();
    let planned = (!file_ref.class().recursive)
        .then(|| file_ref.candidate_plan_with_order(&file_context, biscuit_file::CandidatePlanOrder::Resolution).ok())
        .flatten()
        .filter(|candidates| !candidates.is_empty());
    match planned {
        Some(candidates) => candidates
            .into_iter()
            .map(|candidate| AbsentCandidate { path: candidate.path().to_path_buf(), authored: None })
            .collect(),
        None => {
            // An interpolation failure already failed resolution with its own
            // error before a miss could reach here, so the authored text is
            // only a defensive stand-in.
            let payload = file_ref.interpolated_payload(&file_context).unwrap_or_else(|_| file_ref.raw().to_string());
            vec![AbsentCandidate { path: payload.into(), authored: Some(file_ref.raw().to_string()) }]
        }
    }
}

/// The format named by the extension of a local path or of a URL's path (its
/// query and fragment ignored), or the unreadable extension it has.
fn extension_format(identity: &DataSourceIdentity) -> Result<DataFormat, Option<String>> {
    let path = match identity {
        DataSourceIdentity::Local(path) => Cow::Borrowed(path.as_path()),
        DataSourceIdentity::Remote(url) => Cow::Owned(Path::new(url.path()).to_path_buf()),
    };
    data_format_of(&path).ok_or_else(|| extension_text(&path))
}

fn extension_text(path: &Path) -> Option<String> {
    path.extension().map(|extension| extension.to_string_lossy().into_owned())
}

/// The data format `path`'s extension names, compared case-insensitively;
/// `None` for an extension `load_data` cannot read. Shared with the passive
/// call check, so both choose a format from an extension alike.
pub(crate) fn data_format_of(path: &Path) -> Option<DataFormat> {
    match biscuit_file::detect_from_extension(path) {
        FileType::Yaml => Some(DataFormat::Yaml),
        FileType::Json => Some(DataFormat::Json),
        FileType::Jsonl => Some(DataFormat::Jsonl),
        FileType::Csv => Some(DataFormat::Csv),
        FileType::Tsv => Some(DataFormat::Tsv),
        FileType::Toml => Some(DataFormat::Toml),
        FileType::Json5 => Some(DataFormat::Json5),
        FileType::Markdown | FileType::Pdf | FileType::Unknown => None,
    }
}

/// Binds a parsed source: a CSV/TSV table first becomes rows in `row_mode`
/// under the header mode `header`; the root is then normalized to a
/// container (arrays and objects kept, `null` read as `[]`, any other scalar
/// wrapped in a one-element array) and, when there is a `contract`,
/// converted to it. A failure returns no value at all.
fn bind_source(
    source: &LoadedSource,
    contract: Option<&PreparedContract>,
    identity: &DataSourceIdentity,
    row_mode: Option<RowMode>,
    header: Option<&HeaderMode>,
) -> Result<LoadedData, Arc<DataError>> {
    let (value, table, wrapped) = match &source.content {
        SourceContent::Tree(Value::Null) => (Cow::Owned(Value::Array(Vec::new())), None, false),
        SourceContent::Tree(value @ (Value::Array(_) | Value::Object(_))) => (Cow::Borrowed(value), None, false),
        SourceContent::Tree(scalar) => (Cow::Owned(Value::Array(vec![scalar.clone()])), None, true),
        SourceContent::Table(data) => {
            let header = header.cloned().unwrap_or(HeaderMode::FirstRecord);
            let mode = match row_mode {
                Some(mode) => mode,
                None => select_row_mode(contract, &header).map_err(Arc::new)?,
            };
            let table = tabular_source(data, mode, &header, source, identity)?;
            (Cow::Owned(table_rows(&table)), Some(table), false)
        }
    };
    let Some(contract) = contract else {
        let value = value.into_owned();
        return Ok(LoadedData { items: item_count(&value), value, wrapped, table });
    };
    match simplified_schema::coercion::bind(&contract.shape, &value) {
        Ok(bound) => {
            let value = bound.into_value();
            Ok(LoadedData { items: item_count(&value), value, wrapped, table })
        }
        Err(failure) => Err(Arc::new(DataError::ContractMismatch {
            identity: identity.clone(),
            format: source.format,
            written: contract.written.clone(),
            failure,
            text: source.text.clone(),
            table,
            wrapped,
        })),
    }
}

/// The number of top-level items: array elements or object members.
fn item_count(value: &Value) -> usize {
    match value {
        Value::Array(items) => items.len(),
        Value::Object(members) => members.len(),
        _ => 0,
    }
}

/// The logical view of a delimited table read in `mode` under `header`: a
/// header record is skipped only for [`HeaderMode::FirstRecord`], and object
/// rows are keyed by the header's camelCase names ([`header_keys`]) or by an
/// explicit name list, which must be as wide as the first record. Object
/// rows under `header: false` were refused before the read.
///
/// ## Errors
///
/// [`DataError::Malformed`] for header names that cannot key objects, and
/// for an explicit name list whose length is not the first record's width.
fn tabular_source(
    data: &Arc<biscuit_file::DelimitedData>,
    mode: RowMode,
    header: &HeaderMode,
    source: &LoadedSource,
    identity: &DataSourceIdentity,
) -> Result<TabularSource, Arc<DataError>> {
    let keys = match (mode, header, &data.headers) {
        (RowMode::List, ..) | (RowMode::Object, _, None) => None,
        (RowMode::Object, HeaderMode::Names(names), Some(first)) => {
            if first.fields.len() != names.len() {
                return Err(malformed(
                    source,
                    identity,
                    format!(
                        "the `header` list names {} column{}, but the first record on line {} has {} field{}",
                        names.len(),
                        plural(names.len()),
                        first.line,
                        first.fields.len(),
                        plural(first.fields.len())
                    ),
                    first.offset,
                ));
            }
            Some(names.clone())
        }
        (RowMode::Object, HeaderMode::FirstRecord | HeaderMode::None, Some(first)) => {
            Some(header_keys(first, source, identity)?)
        }
    };
    Ok(TabularSource { mode, header: header.skips_first_record(), keys, data: data.clone() })
}

fn plural(count: usize) -> &'static str {
    if count == 1 { "" } else { "s" }
}

/// The rows of a delimited table as values, read through its logical row
/// view: objects keyed by [`TabularSource::keys`] in object mode, lists of
/// cells in list mode. A table with no records, or with a header alone, has
/// no rows.
fn table_rows(table: &TabularSource) -> Value {
    let cells = |row: &DelimitedRecord| row.fields.iter().map(|field| field.text.clone()).collect::<Vec<_>>();
    let rows = (0..table.row_count()).filter_map(|index| table.row(index));
    Value::Array(match (table.mode, &table.keys) {
        (RowMode::List, _) => rows.map(|row| Value::from(cells(row))).collect(),
        (RowMode::Object, None) => Vec::new(),
        (RowMode::Object, Some(keys)) => rows
            .map(|row| {
                let members = keys.iter().zip(&row.fields);
                Value::Object(members.map(|(key, cell)| (key.clone(), Value::from(cell.text.clone()))).collect())
            })
            .collect(),
    })
}

/// The object keys a header record gives its columns: each name in camelCase,
/// by the `camel_case` expression function's rule. Every key must be
/// non-empty and distinct.
///
/// ## Errors
///
/// [`DataError::Malformed`] naming every column whose name has no usable
/// key and every pair whose keys collide, by their original header text,
/// located at the first offending header field.
fn header_keys(header: &DelimitedRecord, source: &LoadedSource, identity: &DataSourceIdentity) -> Result<Arc<[String]>, Arc<DataError>> {
    let mut seen: HashMap<String, (usize, &str)> = HashMap::new();
    let mut keys = Vec::with_capacity(header.fields.len());
    let mut problems = Vec::new();
    let mut first_offset = None;
    for (index, field) in header.fields.iter().enumerate() {
        let column = index + 1;
        let key = crate::markdown::compose::expression::functions::strings::to_camel_case(&field.text);
        let problem = if field.text.is_empty() {
            Some(format!("column {column} has no name"))
        } else if key.is_empty() {
            Some(format!("column {column} (`{}`) has no letters or digits to name a field", field.text))
        } else if let Some((earlier, original)) = seen.get(&key) {
            Some(format!("columns {earlier} (`{original}`) and {column} (`{}`) both name the field `{key}`", field.text))
        } else {
            seen.insert(key.clone(), (column, &field.text));
            None
        };
        if let Some(problem) = problem {
            first_offset.get_or_insert(field.offset);
            problems.push(problem);
        }
        keys.push(key);
    }
    let Some(offset) = first_offset else {
        return Ok(keys.into());
    };
    Err(malformed(
        source,
        identity,
        format!(
            "the header on line {} cannot name object fields: {}; name the columns with a `header` list, or read rows by position with `header: false` or a `tuple([…])[]` contract",
            header.line,
            problems.join("; ")
        ),
        offset,
    ))
}

fn malformed(source: &LoadedSource, identity: &DataSourceIdentity, message: String, offset: usize) -> Arc<DataError> {
    Arc::new(DataError::Malformed {
        identity: identity.clone(),
        format: source.format,
        message,
        location: located_at(&source.text, offset),
    })
}

/// Refines a failed binding of `load_data`'s `as` argument: a contract the
/// text alone proves cannot be one (a scalar type, a union of scalars, or a
/// `Name@this` reference), carried as a string or as a list or mapping,
/// fails with its `dm.data.*` error rather than a generic type error, the
/// same error a literal contract gets passively. Any other binding failure,
/// including contract text that does not parse, keeps the binder's error.
pub(crate) fn refine_binding_error(error: &CallBindingError, args: &[Value]) -> Option<ExpressionError> {
    let mut failures = error.argument_errors().peekable();
    failures.peek()?;
    if !failures.all(|failure| failure.index == 1) {
        return None;
    }
    let written = written_contract(args.get(1)?)?;
    match judge_written_contract(&written) {
        Err(rejected @ (DataError::ThisImportRejected { .. } | DataError::NotAContainer { .. })) => {
            Some(fail(rejected))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::markdown::compose::expression::call_binding::{
        ArgumentError, ArgumentProblem, CandidateFailure, SignatureProblem,
    };
    use serde_json::json;

    fn failed_at(index: usize, received: Value) -> CallBindingError {
        CallBindingError::Type {
            function: FUNCTION.to_string(),
            candidates: vec![CandidateFailure {
                signature: "load_data(file: file | null, as?: type-definition(container) | null, options?: {format: enum(yaml, yml, json, jsonl, ndjson, csv, tsv, toml, json5), delimiter: string, header: boolean | string[]} | null)".to_string(),
                problem: SignatureProblem::Arguments(vec![ArgumentError {
                    parameter: if index == 1 { "as" } else { "file" }.to_string(),
                    index,
                    expected: "type-definition(container)".to_string(),
                    received,
                    problem: ArgumentProblem::SourceExcluded,
                }]),
            }],
        }
    }

    fn refined_code(error: &CallBindingError, args: &[Value]) -> Option<&'static str> {
        match refine_binding_error(error, args)? {
            ExpressionError::Data(data) => data.code(),
            other => panic!("not a data error: {other}"),
        }
    }

    fn contract(written: &str) -> PreparedContract {
        let dir = tempfile::TempDir::new().unwrap();
        super::super::contract::prepare_contract(written, &biscuit_file::FileResolutionContext::new(dir.path())).unwrap()
    }

    fn planned(paths: &[&str]) -> Vec<AbsentCandidate> {
        paths.iter().map(|path| AbsentCandidate { path: path.into(), authored: None }).collect()
    }

    fn semicolon() -> DataOptions {
        DataOptions { delimiter: Some(b';'), ..DataOptions::default() }
    }

    /// Creating `b.csv` would load it, so the miss is the honest answer even
    /// though `a.yaml` would refuse the delimiter.
    #[test]
    fn an_absent_source_is_a_miss_when_any_candidate_accepts_the_request() {
        let verdict = judge_absent(planned(&["/d/a.yaml", "/d/b.csv"]), &semicolon(), Some(&contract("tuple([string, string])[]")));
        assert!(verdict.is_ok(), "{verdict:?}");
        let unreadable_first = judge_absent(planned(&["/d/a.xyz", "/d/b.yaml"]), &DataOptions::default(), Some(&contract("string[]")));
        assert!(unreadable_first.is_ok(), "{unreadable_first:?}");
    }

    #[test]
    fn an_absent_source_reports_the_first_refusal_when_every_candidate_refuses() {
        let error = judge_absent(planned(&["/d/a.xyz", "/d/b.json"]), &semicolon(), Some(&contract("string[]"))).unwrap_err();
        match error {
            DataError::UnsupportedFormat { identity, extension } => {
                assert_eq!(extension.as_deref(), Some("xyz"));
                assert!(identity.to_string().ends_with("a.xyz"), "{identity}");
            }
            other => panic!("unsupported format first: {other:?}"),
        }
        let error = judge_absent(planned(&["/d/a.json", "/d/b.xyz"]), &semicolon(), Some(&contract("string[]"))).unwrap_err();
        assert_eq!(error.code(), Some("dm.data.invalid_options"));
    }

    /// The authored text of a recursive reference is named as written, never
    /// as an invented resolved path.
    #[test]
    fn an_authored_candidate_names_the_reference_as_written() {
        let candidates = vec![AbsentCandidate { path: "optional.xyz".into(), authored: Some("%optional.xyz".to_string()) }];
        match judge_absent(candidates, &DataOptions::default(), Some(&contract("string[]"))).unwrap_err() {
            DataError::UnsupportedFormat { identity, .. } => assert_eq!(identity.to_string(), "%optional.xyz"),
            other => panic!("unsupported format: {other:?}"),
        }
    }

    #[test]
    fn a_format_option_judges_every_candidate_by_that_format() {
        let options = DataOptions { format: Some(DataFormat::Yaml), ..semicolon() };
        let error = judge_absent(planned(&["/d/a.csv", "/d/b.xyz"]), &options, Some(&contract("string[]"))).unwrap_err();
        assert_eq!(error.code(), Some("dm.data.invalid_options"));
    }

    #[test]
    fn a_scalar_contract_refines_to_not_a_container() {
        let args = [json!("os.yaml"), json!("string")];
        assert_eq!(refined_code(&failed_at(1, json!("string")), &args), Some("dm.data.not_a_container"));
    }

    /// A bracket list, quoted or as a list value, is the union of its items.
    #[test]
    fn a_scalar_union_list_refines_to_not_a_container_from_either_carrier() {
        for carrier in [json!("[string, number]"), json!(["string", "number"])] {
            let args = [json!("os.yaml"), carrier.clone()];
            assert_eq!(refined_code(&failed_at(1, carrier), &args), Some("dm.data.not_a_container"));
        }
    }

    #[test]
    fn a_this_import_refines_to_this_import_rejected() {
        let args = [json!("os.yaml"), json!("Os@this")];
        assert_eq!(refined_code(&failed_at(1, json!("Os@this")), &args), Some("dm.data.this_import_rejected"));
    }

    #[test]
    fn a_syntax_error_or_a_file_failure_keeps_the_binders_error() {
        let syntax = [json!("os.yaml"), json!("string[")];
        assert!(refine_binding_error(&failed_at(1, json!("string[")), &syntax).is_none());
        let file = [json!([]), json!("string")];
        assert!(refine_binding_error(&failed_at(0, json!([])), &file).is_none());
        let number = [json!("os.yaml"), json!(5)];
        assert!(refine_binding_error(&failed_at(1, json!(5)), &number).is_none());
    }
}

//! The `load_data(file, as, options)` expression function.
//!
//! One call runs in this order, so a failure never depends on work a later
//! step would do: a `null` file returns `null`; read the options object; prepare
//! the contract; resolve the file or URL, let a slot's gate judge it, and
//! record it in the read set; choose the effective format (the `format`
//! option, else the extension); check that the options apply to it; for
//! CSV/TSV choose the row mode and, without a header, require positional
//! rows; fetch a URL's text (its host policy is checked here, before any memo
//! lookup); read and decode the source through the raw memo, parse it in the
//! effective format and delimiter through the parse memo, and bind it to the
//! contract with the effective header through the bind memo. Only a fully
//! converted value is returned.
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

use super::contract::{PreparedContract, RowClassification, RowMode, judge_written_contract, require_positional_rows, written_contract};
use super::error::{DataError, TabularSource};
use super::options::{DataOptions, effective_delimiter, effective_header};
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
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, ResolutionContext};

pub(super) const FUNCTION: &str = "load_data";

/// `load_data(file: file | null, as: type-definition(container), options?: {format, delimiter, header} | null) -> any | null`.
///
/// A `null` file returns `null` before the options are read, the contract is
/// prepared, or anything is read; the binder has already refused a literal
/// mistake. Each call opens a `load_data` debug span recording the source,
/// its effective format, delimiter, and header, its size and item count, and
/// whether each memo layer answered.
///
/// ## Errors
///
/// - The file-reference family for a missing, unreadable, not-yet-fetchable,
///   or policy-denied source, and [`ExpressionError::RemoteRead`] (worded as
///   a plain evaluation failure, typed by its kind) for a failed fetch.
/// - [`ExpressionError::Data`] for every `dm.data.*` failure (including
///   `dm.data.unsupported_format` for a missing source, with no `format`
///   option, that no candidate extension could read), an invalid contract, a contract import that cannot be read
///   ([`DataError::ContractImport`], projected like a file reference), and
///   options that cannot apply (`dm.data.invalid_options`, located at the
///   call: execution keeps no per-argument position).
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
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let options = DataOptions::from_bound(call.args().get(2)).map_err(fail)?;
    let written = call.args().get(1).and_then(written_contract).ok_or_else(|| ExpressionError::ContractViolation {
        function: FUNCTION.to_string(),
        message: "argument 1 did not bind to a type definition".to_string(),
    })?;
    let contract = ctx.data.prepare_contract(&written, ctx)?;

    let remote = is_remote_url(raw);
    let identity = match url::Url::parse(raw) {
        Ok(url) if remote => DataSourceIdentity::Remote(Box::new(url)),
        _ => {
            let file_ref = parse_file_ref(FUNCTION, raw, ctx)?;
            let Some(path) = resolve_arg(FUNCTION, raw, ctx)? else {
                // A request no format can read is invalid whether or not the
                // file exists, so it must not reach `try` as a miss.
                if options.format.is_none()
                    && let Some(error) = unreadable_candidates(&file_ref, ctx)
                {
                    return Err(fail(error));
                }
                // Creating any probed candidate would change the answer.
                record_probed_candidates(&file_ref, ctx);
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

    // An explicit format skips the extension entirely, so an extensionless
    // URL or an unsupported extension reads as the format chosen.
    let format = match options.format {
        Some(format) => format,
        None => format_of(&identity).map_err(fail).map_err(unread)?,
    };
    span.record("format", tracing::field::debug(format));
    options.check_applicability(format).map_err(fail).map_err(unread)?;
    let delimiter = effective_delimiter(format, options.delimiter);
    let header = effective_header(format, options.header);
    if let Some(delimiter) = delimiter {
        span.record("delimiter", tracing::field::debug(char::from(delimiter)));
    }
    if let Some(header) = header {
        span.record("header", header);
    }
    let row_shape = |conflict| fail(DataError::RowShapeConflict { written: contract.written.clone(), conflict });
    let row_mode = match header {
        Some(header) => {
            let mode = contract.row_mode().map_err(row_shape).map_err(unread)?;
            if !header {
                require_positional_rows(RowClassification::Known(mode)).map_err(row_shape).map_err(unread)?;
            }
            Some(mode)
        }
        None => None,
    };

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

    let raw_source = ctx.data.load_raw(&identity, || match origin {
        Origin::Fetched(body) => Ok(RawSource {
            text: Ok(Arc::from(strip_bom(&body))),
            len: body.len(),
            hash: biscuit_hash::blake3_hash_bytes(body.as_bytes()),
        }),
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
        Err(error) => return Err(unread(ExpressionError::from(error))),
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

    let bound = ctx
        .data
        .bind(&source, &contract.key, header, |loaded| bind_source(loaded, &contract, &identity, row_mode, header))
        .map_err(ExpressionError::from)?;
    span.record("items", bound.value().items);
    span.record("bind_memo", bound.from_memo());
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

/// `dm.data.unsupported_format` for a missing source when no file it could
/// resolve to has an extension `load_data` reads; `None` when some candidate
/// does, or when the candidates cannot be listed.
fn unreadable_candidates(file_ref: &biscuit_file::FileReference, ctx: &ResolutionContext) -> Option<DataError> {
    if file_ref.class().recursive {
        return None;
    }
    let candidates = file_ref
        .candidate_plan_with_order(&ctx.file_context(), biscuit_file::CandidatePlanOrder::Resolution)
        .ok()?;
    let first = candidates.first()?.path().to_path_buf();
    if candidates.iter().any(|candidate| data_format_of(candidate.path()).is_some()) {
        return None;
    }
    format_of(&DataSourceIdentity::local(&first)).err()
}

/// The format named by the extension of a local path or of a URL's path (its
/// query and fragment ignored), compared case-insensitively.
fn format_of(identity: &DataSourceIdentity) -> Result<DataFormat, DataError> {
    let path = match identity {
        DataSourceIdentity::Local(path) => Cow::Borrowed(path.as_path()),
        DataSourceIdentity::Remote(url) => Cow::Owned(Path::new(url.path()).to_path_buf()),
    };
    data_format_of(&path).ok_or_else(|| DataError::UnsupportedFormat {
        identity: identity.clone(),
        extension: path.extension().map(|extension| extension.to_string_lossy().into_owned()),
    })
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

/// Binds a parsed source to `contract`; a CSV/TSV table first becomes rows in
/// `row_mode`, its first record a header unless `header` is `Some(false)`. A
/// failure returns no value at all.
fn bind_source(
    source: &LoadedSource,
    contract: &PreparedContract,
    identity: &DataSourceIdentity,
    row_mode: Option<RowMode>,
    header: Option<bool>,
) -> Result<LoadedData, Arc<DataError>> {
    let (value, table) = match &source.content {
        SourceContent::Tree(value) => (Cow::Borrowed(value), None),
        SourceContent::Table(data) => {
            let mode = row_mode.map_or_else(|| contract.row_mode(), Ok).map_err(|conflict| {
                Arc::new(DataError::RowShapeConflict { written: contract.written.clone(), conflict })
            })?;
            let table = TabularSource { mode, header: header.unwrap_or(true), data: data.clone() };
            let rows = table_rows(&table, source, identity)?;
            (Cow::Owned(rows), Some(table))
        }
    };
    match simplified_schema::coercion::bind(&contract.shape, &value) {
        Ok(bound) => {
            let value = bound.into_value();
            let items = match &value {
                Value::Array(items) => items.len(),
                Value::Object(members) => members.len(),
                _ => 0,
            };
            Ok(LoadedData { value, items })
        }
        Err(failure) => Err(Arc::new(DataError::ContractMismatch {
            identity: identity.clone(),
            format: source.format,
            written: contract.written.clone(),
            failure,
            text: source.text.clone(),
            table,
        })),
    }
}

/// The rows of a delimited table as values, read through its logical row
/// view: objects keyed by the header in object mode, lists of cells in list
/// mode (a header skipped; without one the first record is a row). A table
/// with no records, or with a header alone, has no rows.
fn table_rows(table: &TabularSource, source: &LoadedSource, identity: &DataSourceIdentity) -> Result<Value, Arc<DataError>> {
    let cells = |row: &DelimitedRecord| row.fields.iter().map(|field| field.text.clone()).collect::<Vec<_>>();
    let rows = (0..table.row_count()).filter_map(|index| table.row(index));
    let rows = match table.mode {
        RowMode::List => rows.map(|row| Value::from(cells(row))).collect(),
        RowMode::Object => {
            // Object rows exist only with a header (`header: false` refused
            // them before the read).
            let Some(header) = &table.data.headers else {
                return Ok(Value::Array(Vec::new()));
            };
            check_header(header, source, identity)?;
            rows.map(|row| {
                let members = header.fields.iter().zip(&row.fields);
                Value::Object(members.map(|(name, cell)| (name.text.clone(), Value::from(cell.text.clone()))).collect())
            })
            .collect()
        }
    };
    Ok(Value::Array(rows))
}

/// Object rows need a distinct, non-empty name for every column.
///
/// ## Errors
///
/// [`DataError::Malformed`] naming every empty or repeated column, located at
/// the first offending header field.
fn check_header(header: &DelimitedRecord, source: &LoadedSource, identity: &DataSourceIdentity) -> Result<(), Arc<DataError>> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut problems = Vec::new();
    let mut first_offset = None;
    for (index, field) in header.fields.iter().enumerate() {
        let column = index + 1;
        let problem = if field.text.is_empty() {
            Some(format!("column {column} has no name"))
        } else if let Some(earlier) = seen.get(field.text.as_str()) {
            Some(format!("column {column} repeats `{}` from column {earlier}", field.text))
        } else {
            seen.insert(&field.text, column);
            None
        };
        if let Some(problem) = problem {
            first_offset.get_or_insert(field.offset);
            problems.push(problem);
        }
    }
    let Some(offset) = first_offset else {
        return Ok(());
    };
    Err(Arc::new(DataError::Malformed {
        identity: identity.clone(),
        format: source.format,
        message: format!(
            "the header on line {} cannot name object fields: {}; read rows by position with a `tuple([…])[]` contract instead",
            header.line,
            problems.join("; ")
        ),
        location: located_at(&source.text, offset),
    }))
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
                signature: "load_data(file: file | null, as: type-definition(container), options?: {format: enum(yaml, yml, json, jsonl, ndjson, csv, tsv, toml, json5), delimiter: string, header: boolean} | null)".to_string(),
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

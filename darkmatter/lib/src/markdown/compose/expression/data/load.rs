//! The `load_data(file, as)` expression function.
//!
//! One call runs in this order, so a failure never depends on work a later
//! step would do: prepare the contract; resolve the file or URL and record it
//! in the read set; choose the format from the extension; for CSV/TSV choose
//! the row mode; fetch a URL's text (its host policy is checked here, before
//! any memo lookup); read, decode, and parse the source through the source
//! memo; bind the value to the contract through the bind memo. Only a fully
//! converted value is returned.

use std::borrow::Cow;
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use biscuit_file::{DataFormat, DelimitedData, DelimitedRecord, FileType};
use serde_json::Value;
use tracing::field::Empty;

use super::contract::{PreparedContract, RowMode, judge_written_contract};
use super::error::{DataError, TabularSource};
use super::parse::{decode_local, located_at, parse_text, strip_bom};
use super::runtime::{DataSourceIdentity, LoadedData, LoadedSource, SourceContent};
use crate::markdown::compose::expression::call_binding::{BoundCall, CallBindingError};
use crate::markdown::compose::expression::functions::{
    file_reference_diagnostic, file_reference_error, remote_text_error, resolve_arg,
};
use crate::markdown::compose::expression::resolve_ctx::is_remote_url;
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, ResolutionContext};

const FUNCTION: &str = "load_data";

/// `load_data(file: file | null, as: type-definition(container)) -> any | null`.
///
/// A `null` file returns `null` before the contract is prepared or anything
/// is read. Each call opens a `load_data` debug span recording the source,
/// its format, size, item count, and whether each memo level answered.
///
/// ## Errors
///
/// - The file-reference family for a missing, unreadable, not-yet-fetchable,
///   or policy-denied source, and a plain evaluation failure for a failed
///   fetch.
/// - [`ExpressionError::Data`] for every `dm.data.*` failure and an invalid
///   contract.
#[tracing::instrument(
    level = "debug",
    name = "load_data",
    skip_all,
    fields(identity = Empty, format = Empty, bytes = Empty, items = Empty, source_memo = Empty, bind_memo = Empty)
)]
pub(crate) fn load_data_fn(call: &BoundCall<'_>, ctx: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(raw) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    let written = call.args().text(1).ok_or_else(|| ExpressionError::ContractViolation {
        function: FUNCTION.to_string(),
        message: "argument 1 did not bind to text".to_string(),
    })?;
    let contract = ctx.data.prepare_contract(written, &ctx.file_context())?;

    let remote = is_remote_url(raw);
    let identity = match url::Url::parse(raw) {
        Ok(url) if remote => DataSourceIdentity::Remote(Box::new(url)),
        _ => {
            let path = resolve_arg(FUNCTION, raw, ctx)?
                .ok_or_else(|| file_reference_error(FUNCTION, raw, ctx, FileRefFailure::NotFound, None))?;
            DataSourceIdentity::local(&path)
        }
    };
    // Recorded before anything that can fail, so a failed read, a malformed
    // file, and a denied URL are all dependencies.
    ctx.data.record(&identity);
    let span = tracing::Span::current();
    span.record("identity", tracing::field::display(&identity));

    let format = format_of(&identity).map_err(fail)?;
    span.record("format", tracing::field::debug(format));
    let row_mode = match format {
        DataFormat::Csv | DataFormat::Tsv => Some(contract.row_mode().map_err(|conflict| {
            fail(DataError::RowShapeConflict { written: contract.written.clone(), conflict })
        })?),
        _ => None,
    };

    // The fetch enforces host policy and is single-flight and cached by the
    // remote runtime itself, so authorization precedes the memo lookup.
    let origin = match &identity {
        DataSourceIdentity::Local(path) => Origin::Local(path),
        DataSourceIdentity::Remote(_) => match ctx.fetch_remote_text(raw) {
            Ok(Some(body)) => Origin::Fetched(body),
            Ok(None) => return Err(file_reference_error(FUNCTION, raw, ctx, FileRefFailure::RemoteNotEnabled, None)),
            Err(error) => return Err(remote_text_error(FUNCTION, raw, ctx, error)),
        },
    };

    let source = ctx
        .data
        .load_source(&identity, || {
            let (text, bytes) = match origin {
                Origin::Fetched(body) => (Arc::from(strip_bom(&body)), body.len()),
                Origin::Local(path) => {
                    let bytes = std::fs::read(path).map_err(|error| {
                        Arc::new(DataError::FileReference(file_reference_diagnostic(
                            FUNCTION,
                            raw,
                            ctx,
                            FileRefFailure::NotFound,
                            Some(biscuit_file::FileReferenceError::Io { path: path.to_path_buf(), source: error }),
                        )))
                    })?;
                    let count = bytes.len();
                    (decode_local(bytes, &identity, format)?, count)
                }
            };
            let content = parse_text(&text, format, &identity)?;
            ctx.data.note_parse();
            Ok(LoadedSource { content, format, text, bytes })
        })
        .map_err(ExpressionError::from)?;
    span.record("bytes", source.value().bytes);
    span.record("source_memo", source.from_memo());

    let bound = ctx
        .data
        .bind(&source, &contract.key, |loaded| bind_source(loaded, &contract, &identity, row_mode))
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

/// The format named by the extension of a local path or of a URL's path (its
/// query and fragment ignored), compared case-insensitively.
fn format_of(identity: &DataSourceIdentity) -> Result<DataFormat, DataError> {
    let path = match identity {
        DataSourceIdentity::Local(path) => Cow::Borrowed(path.as_path()),
        DataSourceIdentity::Remote(url) => Cow::Owned(Path::new(url.path()).to_path_buf()),
    };
    match biscuit_file::detect_from_extension(&path) {
        FileType::Yaml => Ok(DataFormat::Yaml),
        FileType::Json => Ok(DataFormat::Json),
        FileType::Jsonl => Ok(DataFormat::Jsonl),
        FileType::Csv => Ok(DataFormat::Csv),
        FileType::Tsv => Ok(DataFormat::Tsv),
        FileType::Toml => Ok(DataFormat::Toml),
        FileType::Json5 => Ok(DataFormat::Json5),
        FileType::Markdown | FileType::Pdf | FileType::Unknown => Err(DataError::UnsupportedFormat {
            identity: identity.clone(),
            extension: path.extension().map(|extension| extension.to_string_lossy().into_owned()),
        }),
    }
}

/// Binds a parsed source to `contract`; a CSV/TSV table first becomes rows in
/// `row_mode`. A failure returns no value at all.
fn bind_source(
    source: &LoadedSource,
    contract: &PreparedContract,
    identity: &DataSourceIdentity,
    row_mode: Option<RowMode>,
) -> Result<LoadedData, Arc<DataError>> {
    let (value, table) = match &source.content {
        SourceContent::Tree(value) => (Cow::Borrowed(value), None),
        SourceContent::Table(data) => {
            let mode = row_mode.map_or_else(|| contract.row_mode(), Ok).map_err(|conflict| {
                Arc::new(DataError::RowShapeConflict { written: contract.written.clone(), conflict })
            })?;
            let rows = table_rows(data, mode, source, identity)?;
            (Cow::Owned(rows), Some(TabularSource { mode, data: data.clone() }))
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

/// The rows of a delimited table as values: objects keyed by the header in
/// object mode, lists of cells (the header skipped) in list mode. A table
/// with no header, or with a header alone, has no rows.
fn table_rows(
    data: &DelimitedData,
    mode: RowMode,
    source: &LoadedSource,
    identity: &DataSourceIdentity,
) -> Result<Value, Arc<DataError>> {
    let Some(header) = &data.headers else {
        return Ok(Value::Array(Vec::new()));
    };
    let cells = |row: &DelimitedRecord| row.fields.iter().map(|field| field.text.clone()).collect::<Vec<_>>();
    let rows = match mode {
        RowMode::List => data.rows.iter().map(|row| Value::from(cells(row))).collect(),
        RowMode::Object => {
            check_header(header, source, identity)?;
            data.rows
                .iter()
                .map(|row| {
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

/// Refines a failed binding of `load_data`'s `as` argument: a string the text
/// alone proves cannot be a contract (a scalar type, or a `Name@this`
/// reference) fails with its `dm.data.*` error rather than a generic type
/// error, the same error a literal contract gets passively. Any other
/// binding failure, including contract text that does not parse, keeps the
/// binder's error.
pub(crate) fn refine_binding_error(error: &CallBindingError, args: &[Value]) -> Option<ExpressionError> {
    let mut failures = error.argument_errors().peekable();
    failures.peek()?;
    if !failures.all(|failure| failure.index == 1) {
        return None;
    }
    let Some(Value::String(written)) = args.get(1) else {
        return None;
    };
    match judge_written_contract(written) {
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
                signature: "load_data(file: file | null, as: type-definition(container))".to_string(),
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

//! Typed `load_data` failures.
//!
//! A [`DataError`] is the cause a data read stores in the request memo, so it
//! carries only source-independent facts: the resolved identity, the format,
//! the written contract, the parser's location, and the coercion engine's
//! structured failure. A call site's own span, document, and iteration path
//! are attached when the error is projected to a diagnostic, never here, so
//! one memoized failure can be reported at every call site that reaches it.

use std::sync::Arc;

use biscuit_file::{DataFormat, DelimitedData, DelimitedRecord};
use simplified_schema::SchemaError;
use simplified_schema::coercion::ConversionError;

use super::contract::{RowMode, RowShapeConflict};
use super::runtime::DataSourceIdentity;
use crate::markdown::compose::expression::{ExpressionError, FileRefFailure, FileReferenceDiagnostic};

/// `as` does not denote a container (R2).
pub const NOT_A_CONTAINER_CODE: &str = "dm.data.not_a_container";
/// `as` cannot pick one CSV/TSV row shape (R4).
pub const ROW_SHAPE_CONFLICT_CODE: &str = "dm.data.row_shape_conflict";
/// `as` names a type of the writing document (`Name@this`) (R1).
pub const THIS_IMPORT_REJECTED_CODE: &str = "dm.data.this_import_rejected";
/// The data file's extension names no supported format (R3).
pub const UNSUPPORTED_FORMAT_CODE: &str = "dm.data.unsupported_format";
/// The data does not parse under its format's rules (R3, R4).
pub const MALFORMED_CODE: &str = "dm.data.malformed";
/// The parsed data does not satisfy `as` (R5).
pub const CONTRACT_MISMATCH_CODE: &str = "dm.data.contract_mismatch";
/// An options object (`load_data`'s third argument) binds but cannot apply:
/// an invalid delimiter, a non-tab TSV delimiter, or a tabular option on a
/// non-tabular format.
pub const INVALID_OPTIONS_CODE: &str = "dm.data.invalid_options";

/// Every `dm.data.*` code, each always an error. None is a warning, so none
/// belongs in the `--deny` registry of governed warnings.
pub const DATA_DIAGNOSTIC_CODES: &[&str] = &[
    NOT_A_CONTAINER_CODE,
    ROW_SHAPE_CONFLICT_CODE,
    THIS_IMPORT_REJECTED_CODE,
    UNSUPPORTED_FORMAT_CODE,
    MALFORMED_CODE,
    CONTRACT_MISMATCH_CODE,
    INVALID_OPTIONS_CODE,
];

/// Where a parser located a malformed data document.
///
/// Every field is a fact the parser proved; an unknown field is `None`, never
/// a placeholder. `line` and `column` are 1-based and count characters of the
/// BOM-stripped text. `byte_offset` is set only for local text that is not
/// valid UTF-8, where a line is meaningless: it is zero-based into the
/// file's original bytes (a byte-order mark included), and `line` and
/// `column` are then absent.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DataSourceLocation {
    /// Zero-based offset of the first invalid UTF-8 byte in the local file's
    /// original bytes.
    pub byte_offset: Option<usize>,
    /// RFC 6901 pointer to the value the failure belongs to.
    pub pointer: Option<String>,
    /// 1-based line, when the parser reported one.
    pub line: Option<usize>,
    /// 1-based character column, when the parser reported one.
    pub column: Option<usize>,
}

/// A `load_data` failure, kept typed until diagnostic projection.
#[derive(Debug, thiserror::Error)]
pub enum DataError {
    /// The data file could not be resolved, read, or fetched. Kept in the
    /// shared file-reference family so it projects like every other
    /// read-side function's failure.
    #[error("{}", ExpressionError::FileReference(.0.clone()))]
    FileReference(FileReferenceDiagnostic),

    /// A named-type import of the contract could not be resolved or read.
    /// Worded and projected like [`FileReference`](Self::FileReference), but
    /// it is a failure of the contract, not of the data source, so `try`
    /// never recovers it and it is never a deferral.
    #[error("{}", ExpressionError::FileReference(.0.clone()))]
    ContractImport(FileReferenceDiagnostic),

    /// The contract does not describe an array, tuple, object, or dictionary.
    #[error("load_data(): the contract `{written}` does not describe a container (an array, tuple, object, or dictionary)")]
    NotAContainer {
        /// The contract as written.
        written: String,
    },

    /// A CSV or TSV contract whose arms do not agree on one row shape.
    #[error("load_data(): the contract `{written}` cannot describe tabular rows: {conflict}")]
    RowShapeConflict {
        /// The contract as written.
        written: String,
        /// Which arms disagree, or which element is not a row.
        conflict: RowShapeConflict,
    },

    /// The contract names a type of the writing document (`Name` or
    /// `Name@this`), which an inline contract has none of.
    #[error("load_data(): the contract `{written}` names `{name}` from this document; an inline contract must import named types from a file (`{name}@./types.yaml`)")]
    ThisImportRejected {
        /// The contract as written.
        written: String,
        /// The local type name it referenced.
        name: String,
    },

    /// The data file's extension names no supported format.
    #[error("load_data(): `{identity}` has no supported data format{}", extension.as_deref().map(|ext| format!(" (`.{ext}`)")).unwrap_or_default())]
    UnsupportedFormat {
        /// The resolved data source.
        identity: DataSourceIdentity,
        /// The extension found, if any.
        extension: Option<String>,
    },

    /// The data document does not parse in its format.
    #[error("load_data(): `{identity}` is not valid {}: {message}", format_name(*format))]
    Malformed {
        /// The resolved data source.
        identity: DataSourceIdentity,
        /// The format the document was read as.
        format: DataFormat,
        /// The parser's message.
        message: String,
        /// Where the parser located the failure.
        location: DataSourceLocation,
    },

    /// The data parsed but does not satisfy the contract.
    #[error("load_data(): `{identity}` does not match `{written}`: {failure}")]
    ContractMismatch {
        /// The resolved data source.
        identity: DataSourceIdentity,
        /// The format the document was read as.
        format: DataFormat,
        /// The contract as written.
        written: String,
        /// The coercion engine's structured failure; flatten it with
        /// `ConversionError::located_failures`, never from display text.
        failure: ConversionError,
        /// The BOM-stripped text the value was parsed from, local or fetched,
        /// for locating pointers.
        text: Arc<str>,
        /// For CSV and TSV, the table the rows were built from and how: a
        /// pointer `/i/<column>` (object rows) or `/i/<n>` (list rows) names
        /// [`TabularSource::row`] `i`, whose fields carry their raw source
        /// offsets. `None` for every other format.
        table: Option<TabularSource>,
    },

    /// An option of the options object cannot apply to this call. It names no
    /// position in a data file: it is decided before the file is read.
    #[error("{}", super::options::invalid_options_message(key, reason))]
    InvalidOptions {
        /// The option key (`format`, `delimiter`, or `header`).
        key: &'static str,
        /// Why the value cannot apply, naming what is accepted.
        reason: String,
    },

    /// The contract is not a valid SimplifiedSchema type, or names a type its
    /// import does not export.
    #[error("load_data(): the contract `{written}` is invalid: {source}")]
    ContractSchema {
        /// The contract as written.
        written: String,
        /// The schema language's cause.
        #[source]
        source: Arc<SchemaError>,
    },
}

/// The delimited table a CSV or TSV value was built from, its row mode, and
/// whether its first record was a header.
///
/// Rows are read only through [`Self::row_count`] and [`Self::row`], the one
/// logical view binding and diagnostic lookup share: row `i` of the value is
/// `row(i)`, whose fields keep their raw source offsets. The parsed table is
/// never rewritten for a headerless read.
#[derive(Debug, Clone)]
pub struct TabularSource {
    /// How each row became a value.
    pub mode: RowMode,
    /// Whether the first record is a header (and so not a row). Object rows
    /// always have one.
    pub header: bool,
    /// The first record and the records after it, with their raw field
    /// offsets.
    pub data: Arc<DelimitedData>,
}

impl TabularSource {
    /// The number of rows the value has.
    pub fn row_count(&self) -> usize {
        let first = usize::from(!self.header && self.data.headers.is_some());
        first + self.data.rows.len()
    }

    /// Row `index` of the value: with a header, the record after it; without
    /// one, the first record is row 0.
    pub fn row(&self, index: usize) -> Option<&DelimitedRecord> {
        match (&self.data.headers, self.header) {
            (Some(first), false) => match index.checked_sub(1) {
                None => Some(first),
                Some(index) => self.data.rows.get(index),
            },
            _ => self.data.rows.get(index),
        }
    }
}

pub(super) fn format_name(format: DataFormat) -> &'static str {
    match format {
        DataFormat::Toml => "TOML",
        DataFormat::Yaml => "YAML",
        DataFormat::Json => "JSON",
        DataFormat::Json5 => "JSON5",
        DataFormat::Jsonl => "JSON Lines",
        DataFormat::Csv => "CSV",
        DataFormat::Tsv => "TSV",
        DataFormat::Markdown => "Markdown",
        DataFormat::Pdf => "PDF",
        DataFormat::Text => "text",
    }
}

impl DataError {
    /// Whether this failure only means "not possible yet": a remote read in a
    /// context with no remote permission. The request memo never stores one,
    /// so a later pass that may read the URL still tries.
    pub fn is_deferral(&self) -> bool {
        matches!(self, Self::FileReference(diagnostic) if diagnostic.kind == FileRefFailure::RemoteNotEnabled)
    }

    /// The `dm.data.*` code of this failure; `None` for the file-reference
    /// family (a contract import included) and an invalid contract schema, which stay
    /// `dm.expression.evaluation_failure`.
    pub fn code(&self) -> Option<&'static str> {
        match self {
            Self::NotAContainer { .. } => Some(NOT_A_CONTAINER_CODE),
            Self::RowShapeConflict { .. } => Some(ROW_SHAPE_CONFLICT_CODE),
            Self::ThisImportRejected { .. } => Some(THIS_IMPORT_REJECTED_CODE),
            Self::UnsupportedFormat { .. } => Some(UNSUPPORTED_FORMAT_CODE),
            Self::Malformed { .. } => Some(MALFORMED_CODE),
            Self::ContractMismatch { .. } => Some(CONTRACT_MISMATCH_CODE),
            Self::InvalidOptions { .. } => Some(INVALID_OPTIONS_CODE),
            Self::FileReference(_) | Self::ContractImport(_) | Self::ContractSchema { .. } => None,
        }
    }

    /// The written contract this failure names, when it is about the contract.
    pub fn written_contract(&self) -> Option<&str> {
        match self {
            Self::NotAContainer { written }
            | Self::RowShapeConflict { written, .. }
            | Self::ThisImportRejected { written, .. }
            | Self::ContractMismatch { written, .. }
            | Self::ContractSchema { written, .. } => Some(written),
            Self::FileReference(_)
            | Self::ContractImport(_)
            | Self::UnsupportedFormat { .. }
            | Self::Malformed { .. }
            | Self::InvalidOptions { .. } => None,
        }
    }
}

impl From<Arc<DataError>> for ExpressionError {
    /// A data-source file failure stays [`ExpressionError::FileReference`], so
    /// its fatality and projection match `frontmatter()`'s; every other
    /// failure, a contract import's included, is [`ExpressionError::Data`].
    fn from(error: Arc<DataError>) -> Self {
        match &*error {
            DataError::FileReference(diagnostic) => ExpressionError::FileReference(diagnostic.clone()),
            _ => ExpressionError::Data(error),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn remote_not_enabled() -> DataError {
        DataError::FileReference(FileReferenceDiagnostic {
            function: "load_data",
            reference: "https://example.com/a.json".to_string(),
            kind: FileRefFailure::RemoteNotEnabled,
            cwd: PathBuf::from("/tmp"),
            fallback_dir: None,
            source: None,
            caller: None,
        })
    }

    #[test]
    fn only_a_remote_not_enabled_read_is_a_deferral() {
        assert!(remote_not_enabled().is_deferral());
        assert!(!DataError::NotAContainer { written: "string".into() }.is_deferral());
        let missing = DataError::FileReference(FileReferenceDiagnostic {
            kind: FileRefFailure::NotFound,
            ..match remote_not_enabled() {
                DataError::FileReference(diagnostic) => diagnostic,
                _ => unreachable!(),
            }
        });
        assert!(!missing.is_deferral());
    }

    #[test]
    fn a_data_error_is_authoring_fatal_and_survives_callback_wrappers() {
        let error = ExpressionError::from(Arc::new(DataError::NotAContainer { written: "string".into() }));
        assert!(matches!(error, ExpressionError::Data(_)));
        assert!(error.is_authoring_fatal());
        let wrapped = ExpressionError::Callback {
            function: "map".into(),
            position: crate::markdown::compose::expression::iteration::IterationPosition::Index(0),
            source: Box::new(error),
        };
        assert!(wrapped.is_authoring_fatal());
        assert!(matches!(wrapped.without_callbacks(), ExpressionError::Data(_)));
    }

    #[test]
    fn a_file_family_failure_converts_to_the_shared_file_reference_variant() {
        let error = ExpressionError::from(Arc::new(remote_not_enabled()));
        assert!(matches!(error, ExpressionError::FileReference(ref d) if d.kind == FileRefFailure::RemoteNotEnabled));
    }

    #[test]
    fn invalid_options_has_its_own_code_and_names_no_contract() {
        let error = DataError::InvalidOptions { key: "delimiter", reason: "must be one character".into() };
        assert_eq!(error.code(), Some(INVALID_OPTIONS_CODE));
        assert!(DATA_DIAGNOSTIC_CODES.contains(&INVALID_OPTIONS_CODE));
        assert_eq!(error.written_contract(), None);
        assert!(!error.is_deferral());
        assert_eq!(error.to_string(), "load_data(): option `delimiter` is invalid: must be one character");
        assert!(matches!(ExpressionError::from(Arc::new(error)), ExpressionError::Data(_)));
    }

    fn missing_import() -> FileReferenceDiagnostic {
        FileReferenceDiagnostic {
            function: "load_data",
            reference: "./types.yaml".to_string(),
            kind: FileRefFailure::NotFound,
            cwd: PathBuf::from("/tmp"),
            fallback_dir: None,
            source: None,
            caller: None,
        }
    }

    #[test]
    fn a_contract_import_reads_like_a_file_reference_but_has_its_own_identity() {
        let import = DataError::ContractImport(missing_import());
        let former = ExpressionError::FileReference(missing_import());
        assert_eq!(import.to_string(), former.to_string());
        assert_eq!(import.code(), None);
        assert_eq!(import.written_contract(), None);
        assert!(!import.is_deferral());
        let remote = DataError::ContractImport(FileReferenceDiagnostic {
            kind: FileRefFailure::RemoteNotEnabled,
            ..missing_import()
        });
        assert!(!remote.is_deferral(), "an import is never a deferral");

        let error = ExpressionError::from(Arc::new(import));
        assert!(matches!(&error, ExpressionError::Data(data) if matches!(**data, DataError::ContractImport(_))));
        assert_eq!(error.to_string(), former.to_string());
        assert!(error.is_authoring_fatal(), "graded like the file-reference failure it was");
        assert_eq!(error.file_reference_diagnostic().map(|d| d.reference.as_str()), Some("./types.yaml"));
    }

    #[test]
    fn contract_failures_name_the_written_contract() {
        let error = DataError::ContractSchema {
            written: "strng[]".into(),
            source: Arc::new(SchemaError::Convert { property: "as".into(), message: "unknown type".into() }),
        };
        assert_eq!(error.written_contract(), Some("strng[]"));
        assert!(error.to_string().contains("strng[]"), "{error}");
    }
}

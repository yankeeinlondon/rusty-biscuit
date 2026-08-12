//! Typed error surface for [`InputTableState`](super::table::InputTableState)
//! construction.
//!
//! [`InputTableError`] is returned by
//! [`InputTableState::try_new`](super::table::InputTableState::try_new) for
//! recoverable validation failures (row-shape mismatches, duplicate or unknown
//! column ids, typed cell mismatches). The panicking convenience constructor
//! [`InputTableState::new`](super::table::InputTableState::new) routes through
//! `try_new` and unwraps the result, preserving the historical panic-on-misuse
//! contract for static callers while exposing a `Result` API for embedders
//! building tables from user or config data.

use thiserror::Error;

/// Error returned by
/// [`InputTableState::try_new`](super::table::InputTableState::try_new) when
/// the supplied columns or rows fail validation.
///
/// Each variant carries enough context (row index, expected vs actual cell
/// count, offending column id, expected vs found cell kind) to produce a
/// readable diagnostic at the CLI boundary or in library logs.
#[derive(Debug, Clone, PartialEq, Error)]
pub enum InputTableError {
    /// A row's cell count does not match the configured column count.
    #[error("row {row} has {found} cells, expected {expected}")]
    RowShapeMismatch {
        /// 0-based index of the offending row.
        row: usize,
        /// Expected cell count, derived from the column schema length.
        expected: usize,
        /// Actual cell count observed in the row.
        found: usize,
    },

    /// A row contains two cells with the same column id.
    #[error("row {row} has duplicate column id '{id}'")]
    DuplicateColumnId {
        /// 0-based index of the offending row.
        row: usize,
        /// The repeated column id.
        id: String,
    },

    /// A row references a column id that is not in the schema.
    #[error("row {row} has unknown column id '{id}'")]
    UnknownColumnId {
        /// 0-based index of the offending row.
        row: usize,
        /// The unrecognized column id.
        id: String,
    },

    /// A row is missing a cell for a configured column id.
    #[error("row {row} is missing column '{id}'")]
    MissingColumnId {
        /// 0-based index of the offending row.
        row: usize,
        /// The absent column id.
        id: String,
    },

    /// A row cell's [`CellValue`](super::cell::CellValue) variant is not
    /// compatible with the cell kind derived from the column schema
    /// (for example, [`CellValue::Text`] supplied for a boolean column).
    #[error("row {row} column '{id}' has cell type '{found}', expected '{expected}'")]
    CellTypeMismatch {
        /// 0-based index of the offending row.
        row: usize,
        /// The configured column id.
        id: String,
        /// Cell kind the column expects (e.g. `"boolean"`, `"text"`).
        expected: &'static str,
        /// Cell kind the row cell value actually carries.
        found: &'static str,
    },
}

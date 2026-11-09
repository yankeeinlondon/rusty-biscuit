//! `load_data` support: contracts, typed failures, and the request runtime.
//!
//! - [`contract`] resolves an `as` contract in a document's context and
//!   chooses a CSV/TSV row mode from it.
//! - [`error`] holds [`DataError`], the typed failure kept until diagnostic
//!   projection.
//! - `options` reads the options object (`{format?, delimiter?, header?}`)
//!   and decides whether it can apply to the call's effective format.
//! - [`parse`] decodes and parses one data document in its format.
//! - `locate` places a contract violation in its source text and in file
//!   order, for diagnostic projection.
//! - [`load`] is the `load_data` function: read the options, resolve, choose
//!   the effective format, read and parse through the memo, and bind to the
//!   contract.
//! - [`runtime`] holds [`DataRuntime`], the command-wide memo and read set
//!   every expression context of one command shares.

pub mod contract;
pub mod error;
pub mod load;
pub(crate) mod locate;
pub(crate) mod options;
pub mod parse;
pub mod runtime;

#[cfg(test)]
mod observation_tests;
#[cfg(test)]
mod option_execution_tests;
#[cfg(test)]
mod provenance_tests;

pub use contract::{
    PreparedContract, RowClassification, RowMode, RowShapeConflict, classify_rows, judge_written_contract, prepare_contract,
    require_positional_rows, written_contract,
};
pub use error::{
    CONTRACT_MISMATCH_CODE, DATA_DIAGNOSTIC_CODES, DataError, DataSourceLocation, INVALID_OPTIONS_CODE, MALFORMED_CODE,
    NOT_A_CONTAINER_CODE, ROW_SHAPE_CONFLICT_CODE, TabularSource, THIS_IMPORT_REJECTED_CODE, UNSUPPORTED_FORMAT_CODE,
};
pub use parse::{LocalDataDocument, parse_local_document};
pub use runtime::{
    BindEntry, DataRuntime, DataRuntimeStats, DataSourceIdentity, LoadedData, LoadedSource, Observation, ObservationKind,
    ReadMark, SourceContent, SourceEntry,
};

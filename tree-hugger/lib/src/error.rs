use std::path::PathBuf;

use thiserror::Error;

use crate::queries::QueryKind;
use crate::shared::ProgrammingLanguage;

/// Errors emitted by tree-hugger operations.
#[derive(Debug, Error)]
pub enum TreeHuggerError {
    #[error("Failed to read `{path}`: {source}")]
    Io {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("Unsupported language for `{path}`")]
    UnsupportedLanguage { path: PathBuf },

    #[error("Failed to parse `{path}`")]
    ParseFailed { path: PathBuf },

    #[error("Missing query for {language} {kind}")]
    MissingQuery {
        language: ProgrammingLanguage,
        kind: QueryKind,
    },

    #[error("Missing vendor query for `{name}`")]
    MissingVendorQuery { name: String },

    #[error("Query error for {language} {kind}: {source}")]
    QueryError {
        language: ProgrammingLanguage,
        kind: QueryKind,
        #[source]
        source: tree_sitter::QueryError,
    },

    #[error("Query cache lock was poisoned")]
    QueryCachePoisoned,

    #[error("Directory `{path}` is not inside a git repository")]
    GitRootNotFound { path: PathBuf },

    #[error("No supported source files found in `{path}`")]
    NoSourceFiles { path: PathBuf },

    /// A declaration writes more generic parameters than
    /// `GenericParam::position` (`u16`) can number; positions are never
    /// truncated.
    #[error("`{symbol}` in `{path}` declares more than {max} generic parameters ({count})", max = u32::from(u16::MAX) + 1)]
    TooManyGenericParameters {
        path: PathBuf,
        symbol: String,
        count: usize,
    },

    #[error("Ignore error: {0}")]
    Ignore(#[from] ignore::Error),
}

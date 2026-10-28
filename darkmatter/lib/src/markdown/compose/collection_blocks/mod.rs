//! `::map` / `::filter` collection regions.
//!
//! Both directives are two modes ([`CollectionKind`]) of one region type that
//! the shared structural scanner pairs with its closer. This module owns the
//! passive region model, the body scope inventory ([`BodyScopes`]) every
//! consumer reads, the passive site analysis
//! ([`analyze_collection_blocks`]), and the typed failures; execution is
//! layered on top of these contracts.

pub mod diagnostics;
pub mod error;
pub mod analysis;
pub(crate) mod engine;
pub mod parser;
pub mod scopes;
pub mod types;

pub use error::{CollectionError, CollectionSite, IterationFailure};
pub use parser::{
    HeaderProblem, LenientCollectionHeader, LenientCollectionScan, parse_collection_regions,
    scan_collection_regions_lenient,
};
pub use analysis::{CollectionAnalysisReport, analyze_collection_blocks, analyze_collection_blocks_report};
pub use scopes::{BodyScopes, ExpressionRole, ExpressionSite};
pub use types::{
    CollectionAnalysis, CollectionAnalysisOptions, CollectionHeader, CollectionKind, CollectionRegion,
    QuotedExpression, RecoveredSource, SortOrder,
};

//! `::map` / `::filter` collection regions.
//!
//! Both directives are two modes ([`CollectionKind`]) of one region type that
//! the shared structural scanner pairs with its closer. This module owns the
//! passive region model, the body scope inventory ([`BodyScopes`]) every
//! consumer reads, and the typed failures; execution is layered on top of
//! these contracts.

pub mod diagnostics;
pub mod error;
pub mod types;

pub use error::{CollectionError, CollectionSite};
pub use types::{
    BodyScopes, CollectionAnalysis, CollectionAnalysisOptions, CollectionKind, CollectionRegion,
};

//! Shared leaf types for the generated Claudine provider catalog.
//!
//! This crate is the dependency floor shared by the `claudine` library
//! (whose generated `data.rs` references these types directly) and the
//! `claudine-gen` generator (which introspects them for
//! schema↔catalog compatibility checks without linking the library).
//! It must stay a leaf: serde + strum only.
//!
//! Design authority: `claudine/features/2026-07-02-provider-metadata/`
//! (`design/catalog-generation.md` F1, `design/signal-detection.md` shared
//! vocab, `design/render-components.md` DisplayPolicy ownership).

mod display_policy;
mod model_catalog_source;
mod vocab;

pub use display_policy::{DisplayPolicy, EventClass, ToolResultSummary};
pub use model_catalog_source::ModelCatalogSource;
pub use vocab::{Confidence, Unit, Zone};

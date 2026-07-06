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

mod acp_server_mode;
mod billing_model;
mod display_policy;
mod model_catalog_source;
mod platform_kind;
mod resume_support;
mod vocab;

pub use acp_server_mode::AcpServerMode;
pub use billing_model::BillingModel;
pub use display_policy::{DisplayPolicy, EventClass, ToolResultSummary};
pub use model_catalog_source::ModelCatalogSource;
pub use platform_kind::PlatformKind;
pub use resume_support::ResumeSupport;
pub use vocab::{Confidence, Unit, Zone};

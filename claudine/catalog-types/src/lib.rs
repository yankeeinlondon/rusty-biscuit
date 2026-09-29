//! Shared leaf types for the generated Claudine provider catalog.
//!
//! This crate is the dependency floor shared by the `claudine` library
//! (whose generated `data.rs` references these types directly) and the
//! `claudine-gen` generator (which introspects them for
//! schema↔catalog compatibility checks without linking the library).
//! It must stay a leaf: serde + strum + chrono (types and pure rules shared by
//! generator and library, no clock or I/O).
//!
//! Design authority: `claudine/features/2026-07-02-provider-metadata/`
//! (`design/catalog-generation.md` F1, `design/signal-detection.md` shared
//! vocab, `design/render-components.md` DisplayPolicy ownership).

mod acp_server_mode;
mod billing_model;
mod display_policy;
mod family;
mod model_catalog_source;
mod offering;
mod platform_kind;
mod provider_overlay;
mod resume_support;
mod signal;
mod signal_table;
pub mod steering;
mod vocab;

pub use acp_server_mode::AcpServerMode;
pub use billing_model::BillingModel;
pub use display_policy::{DisplayPolicy, EventClass, ToolResultSummary};
pub use family::{FamilyRow, family_key};
pub use model_catalog_source::ModelCatalogSource;
pub use offering::{
    ExpectedOffering, LocalRunnerIntegration, OfferingClass, OfferingSource, ResolvesVia,
};
pub use platform_kind::PlatformKind;
pub use provider_overlay::{
    OverlayCapability, OverlayReason, OverlayResourceClass, OverlaySelectorShape,
};
pub use resume_support::ResumeSupport;
pub use signal::{
    CapPolicy, CapScope, DetectionMode, DriftObservation, MatchOp, Quantity, QwenLoopType,
    SignalEvent, SignalKind, SignalSource,
};
pub use signal_table::{DetectionRecord, ExtractStrategy, ExtractionSpec, ProviderSignalTable};
pub use vocab::{Confidence, Unit, Zone};

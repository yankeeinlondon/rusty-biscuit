//! Typed [`DisplayPolicy`] descriptor for [`ProviderInfo`].
//!
//! The types live in the `claudine-catalog-types` leaf crate so the
//! catalog generator can introspect them without linking this library
//! (provider-metadata design F1); this re-export keeps the public path
//! `claudine::provider::DisplayPolicy` stable.
//!
//! [`ProviderInfo`]: super::ProviderInfo

pub use claudine_catalog_types::{DisplayPolicy, EventClass, ToolResultSummary};

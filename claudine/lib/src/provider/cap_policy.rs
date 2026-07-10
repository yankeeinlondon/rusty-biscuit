//! Typed [`CapPolicy`] catalog descriptor for [`ProviderInfo`].
//!
//! The types live in the `claudine-catalog-types` leaf crate so the catalog
//! generator can introspect them without linking this library
//! (provider-metadata design F1); this re-export keeps the public paths
//! stable and gives the generated `data.rs` one import for the whole
//! cap-policy shape (`CapPolicy` + its `CapScope`/`Quantity`/`Unit` parts).
//!
//! [`ProviderInfo`]: super::ProviderInfo
//! [`CapPolicy`]: claudine_catalog_types::CapPolicy

pub use claudine_catalog_types::{CapPolicy, CapScope, Quantity, Unit};

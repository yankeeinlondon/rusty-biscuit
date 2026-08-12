//! Typed [`ModelCatalogSource`] descriptor for [`ProviderInfo`].
//!
//! The type moved to the `claudine-catalog-types` leaf crate so the
//! catalog generator can introspect it without linking this library
//! (provider-metadata design F1); this re-export keeps the public path
//! `claudine::provider::ModelCatalogSource` stable.
//!
//! [`ProviderInfo`]: super::ProviderInfo

pub use claudine_catalog_types::ModelCatalogSource;

//! Typed [`PlatformKind`] descriptor for [`ProviderInfo`].
//!
//! The type lives in the `claudine-catalog-types` leaf crate so the
//! catalog generator can introspect it without linking this library
//! (provider-metadata design F1); this re-export keeps the public path
//! `claudine::provider::PlatformKind` stable.
//!
//! [`ProviderInfo`]: super::ProviderInfo

pub use claudine_catalog_types::PlatformKind;

//! Typed expected-offering rows for [`ProviderInfo`].
//!
//! The types live in the `claudine-catalog-types` leaf crate so the
//! catalog generator can introspect them without linking this library
//! (provider-metadata design F1); these re-exports keep the public paths
//! under `claudine::provider` stable.
//!
//! [`ProviderInfo`]: super::ProviderInfo

pub use claudine_catalog_types::{
    ExpectedOffering, LocalRunnerIntegration, OfferingClass, OfferingSource, ResolvesVia,
};

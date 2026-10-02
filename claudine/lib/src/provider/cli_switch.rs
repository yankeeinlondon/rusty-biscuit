//! Typed [`CliSwitchCatalog`] descriptor for [`ProviderInfo`].
//!
//! The types live in the `claudine-catalog-types` leaf crate so the catalog
//! generator can introspect them without linking this library
//! (provider-metadata design F1); this re-export gives the generated
//! `data.rs` one import path for the whole switch-metadata shape.
//!
//! [`ProviderInfo`]: super::ProviderInfo
//! [`CliSwitchCatalog`]: claudine_catalog_types::CliSwitchCatalog

pub use claudine_catalog_types::{
    CliSwitch, CliSwitchCatalog, SwitchAttachment, SwitchScope, SwitchValue, VariadicMin,
};

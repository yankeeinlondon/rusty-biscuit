//! Canonical [`Provider`] enum and ordering constants.
//!
//! `Provider` was previously defined here, then in [`crate::events::provider`]
//! (now a deprecated re-export). The canonical home is now
//! [`crate::provider_id`]. This module re-exports the types for backward
//! compatibility during the migration cycle.

// Re-export from the new leaf module so existing `use crate::provider::identity::Provider`
// paths continue to work.
pub use crate::provider_id::{PROVIDER_COUNT, PROVIDERS_DISPLAY_ORDER, Provider};

//! OpenAPI export file writing.
//!
//! This module is a thin backward-compatible shim re-exporting the implementation
//! from [`crate::export::openapi`]. New code should prefer `schematic_gen::export::openapi`.
//!
//! [`write_openapi_grouped`] is the preferred entry point for module-grouped
//! export (one OpenAPI document per resolved module path, even when multiple
//! `RestApi` definitions share that module). [`write_openapi`] remains for
//! the single-API case and now also derives its filename from the resolved
//! module name rather than from `api.name`.
//!
//! # Deprecated
//!
//! This module is deprecated. Use `schematic_gen::export::openapi` instead.

#[deprecated(since = "0.4.0", note = "Use `schematic_gen::export::openapi` instead")]
pub use crate::export::openapi::*;

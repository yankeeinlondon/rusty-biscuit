//! Postman collection export.
//!
//! Generates Postman Collection v2.1.0 JSON files from schematic RestApi definitions.
//!
//! This module is a thin backward-compatible shim re-exporting the implementation
//! from [`crate::export::postman`]. New code should prefer `schematic_gen::export::postman`.
//!
//! # Deprecated
//!
//! This module is deprecated. Use `schematic_gen::export::postman` instead.

#[deprecated(since = "0.4.0", note = "Use `schematic_gen::export::postman` instead")]
pub use crate::export::postman::*;

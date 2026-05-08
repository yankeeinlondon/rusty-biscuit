//! Postman collection export.
//!
//! Generates Postman Collection v2.1.0 JSON files from schematic RestApi definitions.
//!
//! This module is a thin backward-compatible shim re-exporting the implementation
//! from [`crate::export::postman`]. New code should prefer `schematic_gen::export::postman`.

pub use crate::export::postman::*;

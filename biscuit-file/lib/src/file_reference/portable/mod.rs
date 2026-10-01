//! Portable references: the most portable verified spelling of a target.
//!
//! This module holds the shared path identity ([`PathIdentity`]) and the
//! crate-internal seam that turns generated references into text.

mod path_identity;
// Consumed by `PortablePath` evaluation; until that lands only the tests call it.
#[cfg_attr(not(test), expect(dead_code, reason = "consumed by PortablePath evaluation"))]
mod text;

pub use path_identity::{PathIdentity, RelativeRoute};

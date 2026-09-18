//! Authored research DTOs.
//!
//! Each type mirrors schema version 1 (`docs/research/platforms/_types.yaml`)
//! with `deny_unknown_fields` and closed enums. These are authored records:
//! executable projections are built only by `research::validate`.

pub mod common;
pub mod document;
pub mod mappings;
pub mod overrides;
pub mod roster;

pub use common::*;
pub use document::*;
pub use mappings::*;
pub use overrides::*;
pub use roster::*;

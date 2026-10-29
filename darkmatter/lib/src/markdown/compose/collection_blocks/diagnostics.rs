//! Diagnostic codes for collection regions.
//!
//! Codes are defined once here. The shared iteration and callback codes are
//! re-exported from the expression modules that own them.

pub use crate::markdown::compose::expression::iteration::{EXPANSION_LIMIT_CODE, VISIT_LIMIT_CODE};
pub use crate::markdown::compose::expression::parser::INVALID_CALLBACK_CODE;

/// The producer identity of collection warnings in a compose report.
pub const COLLECTION_SOURCE: &str = "darkmatter.collection";

/// Invalid region, header, or options.
pub const MALFORMED_CODE: &str = "dm.collection.malformed";

/// A `::map` source with no iterable declaration.
pub const UNTYPED_MAP_SOURCE_CODE: &str = "dm.collection.untyped_map_source";

/// A `::filter` source with no iterable declaration.
pub const UNTYPED_FILTER_SOURCE_CODE: &str = "dm.collection.untyped_filter_source";

/// A known unsuitable source, or a reached scalar.
pub const INVALID_SOURCE_CODE: &str = "dm.collection.invalid_source";

/// Incompatible input or result destructuring.
pub const PATTERN_MISMATCH_CODE: &str = "dm.collection.pattern_mismatch";

/// Unsupported, mixed, or null ordering keys.
pub const INVALID_SORT_KEY_CODE: &str = "dm.collection.invalid_sort_key";

/// An unhandled callback failure, carrying the underlying typed cause.
pub const CALLBACK_FAILED_CODE: &str = "dm.collection.callback_failed";

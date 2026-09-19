//! The dependency both packages under test configure identically.
//!
//! It has no features, so `alpha`'s and `beta`'s separate archive invocations
//! resolve the same unit for it and the second invocation reuses the first
//! one's output from the shared owner target tree.

/// A value both packages read, so neither can be optimized into not linking it.
pub const MARK: &str = "shared-deps-common";

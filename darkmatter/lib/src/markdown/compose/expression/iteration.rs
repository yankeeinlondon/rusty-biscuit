//! Shared contracts for collection iteration: request limits, usage records,
//! original-item positions, and the typed budget failure.
//!
//! These types live in the expression layer so expression functions and the
//! rendering directives share one vocabulary without the expression layer
//! depending on region types. Nothing here iterates; the budget authority and
//! its charging rules are layered on top of these contracts.

use std::fmt;
use std::num::NonZeroU64;

/// Default number of source items one request may visit.
pub const DEFAULT_MAX_ITERATION_VISITS: NonZeroU64 = NonZeroU64::new(100_000).unwrap();

/// Default number of bytes one request may materialize through expansion.
pub const DEFAULT_MAX_EXPANSION_BYTES: NonZeroU64 = NonZeroU64::new(16_777_216).unwrap();

/// Diagnostic code for an exhausted visit allowance.
pub const VISIT_LIMIT_CODE: &str = "dm.iteration.visit_limit";

/// Diagnostic code for an exhausted expansion-byte allowance.
pub const EXPANSION_LIMIT_CODE: &str = "dm.iteration.expansion_limit";

/// The two request-wide allowances shared by every nested collection, child
/// composition, and cache reuse.
///
/// Both are positive: zero is never a valid allowance and is rejected where
/// the value enters (builders take [`NonZeroU64`]; CLI arguments are parsed
/// as `NonZeroU64`) rather than clamped.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct IterationLimits {
    max_visits: NonZeroU64,
    max_expansion_bytes: NonZeroU64,
}

impl IterationLimits {
    /// Limits with explicit allowances.
    pub const fn new(max_visits: NonZeroU64, max_expansion_bytes: NonZeroU64) -> Self {
        Self {
            max_visits,
            max_expansion_bytes,
        }
    }

    /// The visit allowance.
    pub const fn max_visits(&self) -> NonZeroU64 {
        self.max_visits
    }

    /// The expansion-byte allowance.
    pub const fn max_expansion_bytes(&self) -> NonZeroU64 {
        self.max_expansion_bytes
    }

    /// Returns these limits with a different visit allowance.
    #[must_use]
    pub const fn with_max_visits(mut self, max_visits: NonZeroU64) -> Self {
        self.max_visits = max_visits;
        self
    }

    /// Returns these limits with a different expansion-byte allowance.
    #[must_use]
    pub const fn with_max_expansion_bytes(mut self, max_expansion_bytes: NonZeroU64) -> Self {
        self.max_expansion_bytes = max_expansion_bytes;
        self
    }
}

impl Default for IterationLimits {
    fn default() -> Self {
        Self::new(DEFAULT_MAX_ITERATION_VISITS, DEFAULT_MAX_EXPANSION_BYTES)
    }
}

/// What one subtree consumed from the request allowances.
///
/// A cache entry records this for its subtree so a reuse reserves the same
/// amount instead of bypassing the budget.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
pub struct ExpansionUsage {
    /// Source items visited.
    pub visits: u64,
    /// Bytes materialized.
    pub bytes: u64,
}

impl ExpansionUsage {
    /// Adds `other`, or `None` on `u64` overflow.
    pub fn checked_add(self, other: Self) -> Option<Self> {
        Some(Self {
            visits: self.visits.checked_add(other.visits)?,
            bytes: self.bytes.checked_add(other.bytes)?,
        })
    }
}

/// The position of one item in its original, unsorted and unfiltered source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IterationPosition {
    /// A zero-based array or tuple index.
    Index(u64),
    /// An object entry's original key.
    Key(String),
}

impl fmt::Display for IterationPosition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Index(index) => write!(f, "{index}"),
            Self::Key(key) => write!(f, "{key}"),
        }
    }
}

/// One step of an [`IterationPath`]: the source reference as authored and the
/// original position of the item being processed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct IterationFrame {
    /// The source reference spelling, such as `teams`.
    pub collection: String,
    /// The item's original position.
    pub position: IterationPosition,
}

impl fmt::Display for IterationFrame {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}[{}]", self.collection, self.position)
    }
}

/// The nested original positions leading to a failure, outermost first.
///
/// Renders as `teams[2] → members[4]`.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct IterationPath(Vec<IterationFrame>);

impl IterationPath {
    /// A path with no frames: a static finding or an unscoped failure.
    pub const fn empty() -> Self {
        Self(Vec::new())
    }

    /// This path extended by one inner frame.
    #[must_use]
    pub fn pushed(&self, frame: IterationFrame) -> Self {
        let mut frames = self.0.clone();
        frames.push(frame);
        Self(frames)
    }

    /// The frames, outermost first.
    pub fn frames(&self) -> &[IterationFrame] {
        &self.0
    }

    /// Whether the path has no frames.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl fmt::Display for IterationPath {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (index, frame) in self.0.iter().enumerate() {
            if index > 0 {
                f.write_str(" → ")?;
            }
            write!(f, "{frame}")?;
        }
        Ok(())
    }
}

/// Which allowance ran out.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum IterationLimitKind {
    /// The visit allowance.
    Visits,
    /// The expansion-byte allowance.
    ExpansionBytes,
}

impl IterationLimitKind {
    /// The diagnostic code for this exhaustion.
    pub const fn code(self) -> &'static str {
        match self {
            Self::Visits => VISIT_LIMIT_CODE,
            Self::ExpansionBytes => EXPANSION_LIMIT_CODE,
        }
    }
}

/// A request-fatal budget exhaustion.
///
/// Exhaustion is sticky and unrecoverable: no expression handler, fallback, or
/// recovery option turns it into a value.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{} limit of {limit} exceeded ({used} used, {requested} requested)", kind_label(*.kind))]
pub struct IterationError {
    /// Which allowance ran out.
    pub kind: IterationLimitKind,
    /// The configured allowance.
    pub limit: u64,
    /// Consumed before the failing reservation.
    pub used: u64,
    /// The failing reservation.
    pub requested: u64,
    /// Where the reservation happened; empty outside a collection.
    pub path: IterationPath,
}

fn kind_label(kind: IterationLimitKind) -> &'static str {
    match kind {
        IterationLimitKind::Visits => "iteration visit",
        IterationLimitKind::ExpansionBytes => "expansion byte",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_the_specified_allowances() {
        let limits = IterationLimits::default();
        assert_eq!(limits.max_visits().get(), 100_000);
        assert_eq!(limits.max_expansion_bytes().get(), 16_777_216);
    }

    #[test]
    fn builders_replace_one_allowance_only() {
        let one = NonZeroU64::new(1).unwrap();
        let limits = IterationLimits::default().with_max_visits(one);
        assert_eq!(limits.max_visits(), one);
        assert_eq!(limits.max_expansion_bytes(), DEFAULT_MAX_EXPANSION_BYTES);
    }

    #[test]
    fn usage_addition_reports_overflow() {
        let a = ExpansionUsage { visits: 1, bytes: u64::MAX };
        assert_eq!(a.checked_add(ExpansionUsage { visits: 1, bytes: 0 }), Some(ExpansionUsage { visits: 2, bytes: u64::MAX }));
        assert_eq!(a.checked_add(ExpansionUsage { visits: 0, bytes: 1 }), None);
    }

    #[test]
    fn path_renders_original_positions_outermost_first() {
        let path = IterationPath::empty()
            .pushed(IterationFrame { collection: "teams".into(), position: IterationPosition::Index(2) })
            .pushed(IterationFrame { collection: "members".into(), position: IterationPosition::Key("ada".into()) });
        assert_eq!(path.to_string(), "teams[2] → members[ada]");
        assert!(IterationPath::empty().is_empty());
    }

    #[test]
    fn limit_kinds_map_to_the_registered_codes() {
        assert_eq!(IterationLimitKind::Visits.code(), "dm.iteration.visit_limit");
        assert_eq!(IterationLimitKind::ExpansionBytes.code(), "dm.iteration.expansion_limit");
    }
}

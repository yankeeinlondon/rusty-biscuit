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
///
/// Serializes as a bare number (an index) or string (a key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(untagged)]
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
#[derive(Debug, Clone, PartialEq, Eq, Hash, serde::Serialize)]
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
/// Renders as `teams[2] → members[4]`; serializes as the array of its
/// frames.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(transparent)]
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

    /// `self` followed by `inner`'s frames: the path of a failure that
    /// `inner` locates inside the item `self` names.
    #[must_use]
    pub fn joined(&self, inner: &IterationPath) -> Self {
        let mut frames = self.0.clone();
        frames.extend(inner.0.iter().cloned());
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

/// The request-wide allowance counters, shared by every nested collection,
/// child composition, and explicit nested composition of one request.
///
/// Clones share one counter. A reservation is checked before the work it
/// pays for, so an exactly full allowance succeeds and the failing
/// reservation charges nothing. Exhaustion is sticky: after the first failure
/// every later reservation fails with the same error.
///
/// A [`discovery`](Self::discovery) budget charges nothing, because preflight
/// does not consume execution allowances.
#[derive(Debug, Clone)]
pub struct IterationBudget {
    shared: std::sync::Arc<BudgetState>,
    ledger: Option<std::sync::Arc<Ledger>>,
}

/// What one subtree consumed, kept per node.
///
/// A global counter's before/after difference would include concurrent
/// siblings' work, so each subtree that must report its own consumption
/// (a cached child composition) holds a ledger. A reservation is recorded in
/// the ledger of the handle that made it and in every ancestor's.
#[derive(Debug)]
struct Ledger {
    visits: std::sync::atomic::AtomicU64,
    bytes: std::sync::atomic::AtomicU64,
    parent: Option<std::sync::Arc<Ledger>>,
}

impl Ledger {
    fn record(&self, kind: IterationLimitKind, amount: u64) {
        use std::sync::atomic::Ordering::Relaxed;
        let mut node = Some(self);
        while let Some(ledger) = node {
            let counter = match kind {
                IterationLimitKind::Visits => &ledger.visits,
                IterationLimitKind::ExpansionBytes => &ledger.bytes,
            };
            counter.fetch_add(amount, Relaxed);
            node = ledger.parent.as_deref();
        }
    }
}

#[derive(Debug)]
struct BudgetState {
    limits: IterationLimits,
    charging: bool,
    visits: std::sync::atomic::AtomicU64,
    bytes: std::sync::atomic::AtomicU64,
    exhausted: std::sync::Mutex<Option<IterationError>>,
}

impl IterationBudget {
    /// A budget with nothing consumed.
    pub fn new(limits: IterationLimits) -> Self {
        Self::build(limits, true)
    }

    /// A budget that never charges: preflight reads collections to discover
    /// children without consuming the execution allowances.
    pub fn discovery(limits: IterationLimits) -> Self {
        Self::build(limits, false)
    }

    fn build(limits: IterationLimits, charging: bool) -> Self {
        Self {
            shared: std::sync::Arc::new(BudgetState {
                limits,
                charging,
                visits: std::sync::atomic::AtomicU64::new(0),
                bytes: std::sync::atomic::AtomicU64::new(0),
                exhausted: std::sync::Mutex::new(None),
            }),
            ledger: None,
        }
    }

    /// A handle on the same allowance that also records what it reserves,
    /// and passes each reservation on to the ledger of the handle it came
    /// from. [`Self::ledger_usage`] is then the consumption of exactly the
    /// work done through this handle and its clones.
    #[must_use]
    pub fn with_ledger(&self) -> Self {
        let ledger = std::sync::Arc::new(Ledger {
            visits: std::sync::atomic::AtomicU64::new(0),
            bytes: std::sync::atomic::AtomicU64::new(0),
            parent: self.ledger.clone(),
        });
        Self { shared: std::sync::Arc::clone(&self.shared), ledger: Some(ledger) }
    }

    /// What the work done through this handle consumed, or nothing for a
    /// handle without a ledger.
    pub fn ledger_usage(&self) -> ExpansionUsage {
        use std::sync::atomic::Ordering::Acquire;
        self.ledger
            .as_ref()
            .map(|ledger| ExpansionUsage { visits: ledger.visits.load(Acquire), bytes: ledger.bytes.load(Acquire) })
            .unwrap_or_default()
    }

    /// The configured allowances.
    pub fn limits(&self) -> IterationLimits {
        self.shared.limits
    }

    /// Whether reservations are charged.
    pub fn is_charging(&self) -> bool {
        self.shared.charging
    }

    /// What has been consumed so far.
    pub fn usage(&self) -> ExpansionUsage {
        use std::sync::atomic::Ordering::Acquire;
        ExpansionUsage { visits: self.shared.visits.load(Acquire), bytes: self.shared.bytes.load(Acquire) }
    }

    /// The first exhaustion, once there was one.
    pub fn exhausted(&self) -> Option<IterationError> {
        self.shared.exhausted.lock().unwrap_or_else(std::sync::PoisonError::into_inner).clone()
    }

    /// Reserves one source-item visit.
    ///
    /// ## Errors
    ///
    /// [`IterationError`] when the allowance is spent; `path` names the item.
    pub fn reserve_visit(&self, path: &IterationPath) -> Result<(), IterationError> {
        self.reserve(IterationLimitKind::Visits, 1, path)
    }

    /// Reserves `bytes` of expansion work.
    ///
    /// ## Errors
    ///
    /// [`IterationError`] when the reservation would pass the allowance or
    /// overflow `u64`.
    pub fn reserve_bytes(&self, bytes: u64, path: &IterationPath) -> Result<(), IterationError> {
        self.reserve(IterationLimitKind::ExpansionBytes, bytes, path)
    }

    /// Reserves everything `usage` records, as a cache reuse replays a
    /// subtree's consumption.
    ///
    /// ## Errors
    ///
    /// [`IterationError`] for the first allowance that cannot cover it.
    pub fn reserve_usage(&self, usage: ExpansionUsage, path: &IterationPath) -> Result<(), IterationError> {
        self.reserve(IterationLimitKind::Visits, usage.visits, path)?;
        self.reserve(IterationLimitKind::ExpansionBytes, usage.bytes, path)
    }

    fn reserve(&self, kind: IterationLimitKind, amount: u64, path: &IterationPath) -> Result<(), IterationError> {
        use std::sync::atomic::Ordering::{Acquire, Release};
        if !self.shared.charging {
            return Ok(());
        }
        let mut exhausted = self.shared.exhausted.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        if let Some(error) = exhausted.as_ref() {
            return Err(error.clone());
        }
        let (counter, limit) = match kind {
            IterationLimitKind::Visits => (&self.shared.visits, self.shared.limits.max_visits().get()),
            IterationLimitKind::ExpansionBytes => (&self.shared.bytes, self.shared.limits.max_expansion_bytes().get()),
        };
        let used = counter.load(Acquire);
        match used.checked_add(amount).filter(|total| *total <= limit) {
            Some(total) => {
                counter.store(total, Release);
                if let Some(ledger) = &self.ledger {
                    ledger.record(kind, amount);
                }
                Ok(())
            }
            None => {
                let error = IterationError { kind, limit, used, requested: amount, path: path.clone() };
                *exhausted = Some(error.clone());
                Err(error)
            }
        }
    }
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

    fn budget(visits: u64, bytes: u64) -> IterationBudget {
        IterationBudget::new(IterationLimits::new(NonZeroU64::new(visits).unwrap(), NonZeroU64::new(bytes).unwrap()))
    }

    #[test]
    fn an_exactly_full_allowance_succeeds_and_the_next_reservation_fails() {
        let budget = budget(2, 10);
        let path = IterationPath::empty();
        assert!(budget.reserve_visit(&path).is_ok());
        assert!(budget.reserve_visit(&path).is_ok());
        let error = budget.reserve_visit(&path).unwrap_err();
        assert_eq!((error.kind, error.limit, error.used, error.requested), (IterationLimitKind::Visits, 2, 2, 1));
        assert_eq!(budget.usage().visits, 2, "the failing reservation charges nothing");
        assert!(budget.reserve_bytes(10, &path).is_err(), "exhaustion is sticky across allowances");
    }

    #[test]
    fn bytes_fail_before_the_append_that_would_exceed_them() {
        let budget = budget(5, 10);
        let path = IterationPath::empty();
        assert!(budget.reserve_bytes(6, &path).is_ok());
        let error = budget.reserve_bytes(5, &path).unwrap_err();
        assert_eq!((error.kind, error.used, error.requested), (IterationLimitKind::ExpansionBytes, 6, 5));
        assert_eq!(budget.usage().bytes, 6);
        assert_eq!(budget.exhausted(), Some(error));
    }

    #[test]
    fn a_reservation_that_overflows_u64_fails() {
        let budget = IterationBudget::new(IterationLimits::new(NonZeroU64::MAX, NonZeroU64::MAX));
        let path = IterationPath::empty();
        assert!(budget.reserve_bytes(u64::MAX, &path).is_ok());
        assert!(budget.reserve_bytes(1, &path).is_err());
    }

    #[test]
    fn clones_share_one_counter_and_discovery_charges_nothing() {
        let shared = budget(1, 1);
        let other = shared.clone();
        let path = IterationPath::empty();
        shared.reserve_visit(&path).unwrap();
        assert!(other.reserve_visit(&path).is_err());
        let discovery = IterationBudget::discovery(IterationLimits::default());
        for _ in 0..3 {
            discovery.reserve_usage(ExpansionUsage { visits: u64::MAX, bytes: u64::MAX }, &path).unwrap();
        }
        assert_eq!(discovery.usage(), ExpansionUsage::default());
    }

    #[test]
    fn usage_reservation_replays_both_allowances() {
        let budget = budget(3, 100);
        let path = IterationPath::empty();
        budget.reserve_usage(ExpansionUsage { visits: 3, bytes: 100 }, &path).unwrap();
        assert!(budget.reserve_usage(ExpansionUsage { visits: 1, bytes: 0 }, &path).is_err());
    }
}

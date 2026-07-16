//! Typed session-status model + precedence reducer.
//!
//! The active-sessions register carries a *displayed* status per session
//! that several independent producers contribute to (the wrapper's
//! stream sink, the `claudine handle` permission/idle hooks, a future
//! process monitor). A single unversioned `status` string is unsound the
//! moment a second producer appears: a weaker late report can hide a
//! stronger unresolved one, and two reordered reports from one producer
//! can land the session in the wrong terminal state.
//!
//! This module replaces that string with a per-producer *slot* model.
//! Each producer owns exactly one [`StatusContribution`] slot; the daemon
//! folds every live slot into one [`EffectiveStatus`] via [`effective`],
//! taking the highest [`SessionState::strength`] and breaking ties by the
//! newest [`StatusContribution::revision`]. Two ordering concerns stay
//! deliberately separate (see the design doc referenced by
//! `rendezvous/../session-state-design.md`):
//!
//! - **Same-producer reordering** is fixed by [`apply_contribution`]'s
//!   per-slot revision guard (a producer-captured unix-ns wall clock).
//! - **Cross-producer conflict** is fixed by precedence here; revisions
//!   are incomparable across producers and never used to order them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;

/// The displayable state of a session, ordered by intervention strength.
///
/// [`Unknown`](Self::Unknown) is the forward-compat sink for any wire
/// value a reader does not recognize; it has strength 0 so it never wins
/// the reducer over a state the reader understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionState {
    /// Running, no intervention needed.
    Active,
    /// Interactive turn complete, waiting on the user (weak).
    Idle,
    /// Permission ask / blocked, unresolved (strong).
    WaitingOnUser,
    /// An unrecognized wire value.
    #[serde(other)]
    Unknown,
}

impl SessionState {
    /// Precedence rank. Higher wins in the reducer.
    #[must_use]
    pub fn strength(self) -> u8 {
        match self {
            SessionState::WaitingOnUser => 30,
            SessionState::Idle => 20,
            SessionState::Active => 10,
            SessionState::Unknown => 0,
        }
    }

    /// Backward-compatible flat `status` string the daemon projects for
    /// plain consumers (the dashboard's `parse_sessions`).
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            SessionState::Active => "active",
            SessionState::Idle => "idle",
            SessionState::WaitingOnUser => "waiting_on_user",
            SessionState::Unknown => "unknown",
        }
    }
}

/// Which producer authored a contribution — its slot key in the entry.
///
/// `PermissionHook` and `IdleHook` are distinct slots even though both
/// originate from `claudine handle`: the permission-ask hook and the
/// idle/turn-complete hook must not clobber each other, so neither slot
/// doubles for the other's signal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ProducerId {
    /// The wrapper's STARTED lifecycle event (initial `Active`).
    Lifecycle,
    /// The wrapper's structured stream sink (fallback permission signal).
    Sink,
    /// `claudine handle`'s permission-ask hook.
    PermissionHook,
    /// `claudine handle`'s idle / turn-complete hook.
    IdleHook,
    /// A future out-of-band liveness reconciler.
    ProcessMonitor,
}

impl ProducerId {
    /// Slot key used in the stored `status_slots` object.
    #[must_use]
    pub fn as_slug(self) -> &'static str {
        match self {
            ProducerId::Lifecycle => "lifecycle",
            ProducerId::Sink => "sink",
            ProducerId::PermissionHook => "permission_hook",
            ProducerId::IdleHook => "idle_hook",
            ProducerId::ProcessMonitor => "process_monitor",
        }
    }

    /// Parse a slot key back into a producer. Unknown keys are dropped
    /// (returns `None`) rather than folded together, so a newer peer's
    /// extra producer slot never collides with a known one.
    #[must_use]
    pub fn from_slug(slug: &str) -> Option<Self> {
        match slug {
            "lifecycle" => Some(ProducerId::Lifecycle),
            "sink" => Some(ProducerId::Sink),
            "permission_hook" => Some(ProducerId::PermissionHook),
            "idle_hook" => Some(ProducerId::IdleHook),
            "process_monitor" => Some(ProducerId::ProcessMonitor),
            _ => None,
        }
    }
}

/// Why the producer believes the state — surfaced to the UI for honesty.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Basis {
    /// A permission ask is outstanding.
    PermissionAsk,
    /// An interactive turn completed; the agent idles on the user.
    InteractiveTurnComplete,
    /// Inferred from process/heuristic liveness, not a first-class event.
    ProcessHeuristic,
    /// The session lifecycle boundary (STARTED / clearing).
    Lifecycle,
    /// An unrecognized wire value.
    #[serde(other)]
    Unknown,
}

impl Basis {
    /// Flat `status_basis` string the daemon projects for consumers.
    #[must_use]
    pub fn as_wire(self) -> &'static str {
        match self {
            Basis::PermissionAsk => "permission_ask",
            Basis::InteractiveTurnComplete => "interactive_turn_complete",
            Basis::ProcessHeuristic => "process_heuristic",
            Basis::Lifecycle => "lifecycle",
            Basis::Unknown => "unknown",
        }
    }
}

/// One producer's current opinion about a session. Exactly one slot per
/// producer lives in an entry; a producer's newer opinion overwrites its
/// own slot (revision-guarded), never another producer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StatusContribution {
    /// The state this producer currently believes.
    pub state: SessionState,
    /// Which producer authored the opinion.
    pub producer: ProducerId,
    /// Why the producer believes it.
    pub basis: Basis,
    /// Producer-captured unix-ns wall clock; the daemon LWW-guards it
    /// per slot (a contribution whose `revision <= stored` is rejected).
    pub revision: i64,
}

/// The reducer's output: the winning contribution's state plus its
/// provenance, projected onto the flat register fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EffectiveStatus {
    /// The displayed state.
    pub state: SessionState,
    /// The winning contribution's basis.
    pub basis: Basis,
    /// The winning contribution's producer.
    pub producer: ProducerId,
}

impl EffectiveStatus {
    /// The default a session folds to before any producer has spoken
    /// beyond Lifecycle's initial `Active`.
    #[must_use]
    pub fn active() -> Self {
        Self {
            state: SessionState::Active,
            basis: Basis::Lifecycle,
            producer: ProducerId::Lifecycle,
        }
    }
}

/// Fold every producer's live contribution into the effective displayed
/// state. Highest strength wins; ties break by newest revision. Empty
/// (no slot) folds to [`EffectiveStatus::active`].
///
/// ## Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use rendezvous_core::session_status::{
///     effective, Basis, ProducerId, SessionState, StatusContribution,
/// };
///
/// let mut slots = BTreeMap::new();
/// slots.insert(ProducerId::Sink, StatusContribution {
///     state: SessionState::WaitingOnUser,
///     producer: ProducerId::Sink,
///     basis: Basis::PermissionAsk,
///     revision: 2,
/// });
/// slots.insert(ProducerId::IdleHook, StatusContribution {
///     state: SessionState::Idle,
///     producer: ProducerId::IdleHook,
///     basis: Basis::InteractiveTurnComplete,
///     revision: 1,
/// });
/// // The weaker idle slot cannot hide the stronger waiting slot.
/// assert_eq!(effective(&slots).state, SessionState::WaitingOnUser);
/// ```
#[must_use]
pub fn effective(slots: &BTreeMap<ProducerId, StatusContribution>) -> EffectiveStatus {
    slots
        .values()
        .max_by_key(|c| (c.state.strength(), c.revision))
        .map(|c| EffectiveStatus {
            state: c.state,
            basis: c.basis,
            producer: c.producer,
        })
        .unwrap_or_else(EffectiveStatus::active)
}

/// Apply one contribution to its producer's slot under the per-slot
/// revision guard. Returns `true` when the slot changed (the write was
/// accepted), `false` when the contribution was stale (`revision <=` the
/// slot's stored revision) and therefore rejected.
///
/// Only orders a single producer's own opinions — cross-producer
/// conflicts are resolved by [`effective`], never by revision.
pub fn apply_contribution(
    slots: &mut BTreeMap<ProducerId, StatusContribution>,
    contribution: StatusContribution,
) -> bool {
    match slots.get(&contribution.producer) {
        Some(existing) if contribution.revision <= existing.revision => false,
        _ => {
            slots.insert(contribution.producer, contribution);
            true
        }
    }
}

/// Serde shape of one stored slot value: the producer is the map key, so
/// the value carries only state / basis / revision.
#[derive(Serialize, Deserialize)]
struct SlotWire {
    state: SessionState,
    basis: Basis,
    revision: i64,
}

/// Parse a stored `status_slots` JSON object into the typed slot map.
/// Malformed or unknown-producer entries are dropped (best-effort, so a
/// newer peer's replica never wedges the reducer).
#[must_use]
pub fn slots_from_json(value: &JsonValue) -> BTreeMap<ProducerId, StatusContribution> {
    let mut slots = BTreeMap::new();
    let Some(object) = value.as_object() else {
        return slots;
    };
    for (key, raw) in object {
        let Some(producer) = ProducerId::from_slug(key) else {
            continue;
        };
        let Ok(wire) = serde_json::from_value::<SlotWire>(raw.clone()) else {
            continue;
        };
        slots.insert(producer, StatusContribution {
            state: wire.state,
            producer,
            basis: wire.basis,
            revision: wire.revision,
        });
    }
    slots
}

/// Serialize the typed slot map back into a `status_slots` JSON object.
#[must_use]
pub fn slots_to_json(slots: &BTreeMap<ProducerId, StatusContribution>) -> JsonValue {
    let object: serde_json::Map<String, JsonValue> = slots
        .values()
        .map(|c| {
            (
                c.producer.as_slug().to_string(),
                serde_json::json!({
                    "state": c.state,
                    "basis": c.basis,
                    "revision": c.revision,
                }),
            )
        })
        .collect();
    JsonValue::Object(object)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contribution(
        state: SessionState,
        producer: ProducerId,
        basis: Basis,
        revision: i64,
    ) -> StatusContribution {
        StatusContribution {
            state,
            producer,
            basis,
            revision,
        }
    }

    #[test]
    fn precedence_and_retraction_across_two_producers() {
        let mut slots = BTreeMap::new();
        apply_contribution(
            &mut slots,
            contribution(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                10,
            ),
        );
        apply_contribution(
            &mut slots,
            contribution(
                SessionState::Idle,
                ProducerId::IdleHook,
                Basis::InteractiveTurnComplete,
                5,
            ),
        );
        // Stronger sink WaitingOnUser dominates the weaker idle slot even
        // though the idle slot's revision is not comparable.
        let e = effective(&slots);
        assert_eq!(e.state, SessionState::WaitingOnUser);
        assert_eq!(e.producer, ProducerId::Sink);

        // Sink retracts by writing a lower-strength state into ITS slot.
        apply_contribution(
            &mut slots,
            contribution(SessionState::Active, ProducerId::Sink, Basis::Lifecycle, 20),
        );
        assert_eq!(effective(&slots).state, SessionState::Idle);

        // The idle hook then clears; only Active remains.
        apply_contribution(
            &mut slots,
            contribution(SessionState::Active, ProducerId::IdleHook, Basis::Lifecycle, 30),
        );
        assert_eq!(effective(&slots).state, SessionState::Active);
    }

    #[test]
    fn stale_revision_is_rejected_per_slot() {
        let mut slots = BTreeMap::new();
        // Newer active lands first...
        assert!(apply_contribution(
            &mut slots,
            contribution(SessionState::Active, ProducerId::Sink, Basis::Lifecycle, 2),
        ));
        // ...then a late (lower-revision) waiting_on_user for the SAME
        // slot is rejected; the slot stays active.
        assert!(!apply_contribution(
            &mut slots,
            contribution(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                1,
            ),
        ));
        assert_eq!(effective(&slots).state, SessionState::Active);
    }

    #[test]
    fn equal_revision_is_rejected() {
        let mut slots = BTreeMap::new();
        apply_contribution(
            &mut slots,
            contribution(SessionState::Active, ProducerId::Sink, Basis::Lifecycle, 7),
        );
        // `<=` guard: an equal revision is a no-op, not an overwrite.
        assert!(!apply_contribution(
            &mut slots,
            contribution(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                7,
            ),
        ));
        assert_eq!(effective(&slots).state, SessionState::Active);
    }

    #[test]
    fn empty_slots_fold_to_active() {
        let slots = BTreeMap::new();
        let e = effective(&slots);
        assert_eq!(e.state, SessionState::Active);
        assert_eq!(e.producer, ProducerId::Lifecycle);
    }

    #[test]
    fn unknown_wire_value_folds_to_strength_zero() {
        let state: SessionState = serde_json::from_value(serde_json::json!("teleporting")).unwrap();
        assert_eq!(state, SessionState::Unknown);
        assert_eq!(state.strength(), 0);

        // An Unknown-only slot still yields *some* effective status, but
        // any recognized state outranks it.
        let mut slots = BTreeMap::new();
        apply_contribution(
            &mut slots,
            contribution(SessionState::Unknown, ProducerId::Sink, Basis::Unknown, 1),
        );
        apply_contribution(
            &mut slots,
            contribution(SessionState::Active, ProducerId::IdleHook, Basis::Lifecycle, 1),
        );
        assert_eq!(effective(&slots).state, SessionState::Active);
    }

    #[test]
    fn slots_json_round_trip() {
        let mut slots = BTreeMap::new();
        apply_contribution(
            &mut slots,
            contribution(
                SessionState::WaitingOnUser,
                ProducerId::Sink,
                Basis::PermissionAsk,
                1_752_000_000_000_000_001,
            ),
        );
        apply_contribution(
            &mut slots,
            contribution(
                SessionState::Idle,
                ProducerId::IdleHook,
                Basis::InteractiveTurnComplete,
                1_752_000_000_000_000_000,
            ),
        );
        let json = slots_to_json(&slots);
        assert_eq!(json["sink"]["state"], serde_json::json!("waiting_on_user"));
        assert_eq!(json["idle_hook"]["basis"], serde_json::json!("interactive_turn_complete"));

        let parsed = slots_from_json(&json);
        assert_eq!(parsed, slots);
    }

    #[test]
    fn slots_from_json_drops_unknown_producers() {
        let json = serde_json::json!({
            "sink": { "state": "active", "basis": "lifecycle", "revision": 1 },
            "future_producer": { "state": "idle", "basis": "lifecycle", "revision": 2 },
        });
        let parsed = slots_from_json(&json);
        assert_eq!(parsed.len(), 1);
        assert!(parsed.contains_key(&ProducerId::Sink));
    }
}

//! Persisted budget-ledger records and their state transitions.
//!
//! Every transition takes the wall-clock time to charge as an explicit
//! `elapsed_ms` argument, so the arithmetic is testable without sleeping. The
//! live runner ([`super::run`]) measures that elapsed time with a monotonic
//! clock; timestamps written here are UTC and serve only as evidence.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::error::BudgetError;

/// Format tag written into every ledger; a different tag is refused on load.
pub(crate) const LEDGER_FORMAT: &str = "claudine-budget-ledger/1";

/// Where a ledger is in its lifecycle.
///
/// Only `Active` charges time. Every other state is an explicit stop that an
/// operator (or the end of a run) put the ledger in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LedgerState {
    /// Not running; the next budgeted run may start.
    Stopped,
    /// A budgeted Claudine run holds the ledger and its clock is charging.
    Active,
    /// Paused for human approval; `resume` is required before another run.
    Suspended,
    /// The previous runner died; `resume` is required before another run.
    Interrupted,
    /// Consumed allowance reached a limit; only a recorded grant reopens it.
    Exhausted,
}

impl LedgerState {
    pub(crate) const fn label(self) -> &'static str {
        match self {
            Self::Stopped => "stopped",
            Self::Active => "active",
            Self::Suspended => "suspended",
            Self::Interrupted => "interrupted",
            Self::Exhausted => "exhausted",
        }
    }
}

/// An invocation count paired with active wall-clock milliseconds.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Allowance {
    pub invocations: u64,
    pub active_ms: u64,
}

/// Extra allowance an operator recorded; the only way a limit grows.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Grant {
    pub operator: String,
    pub reason: String,
    pub invocations: u64,
    pub active_ms: u64,
    pub recorded_at: DateTime<Utc>,
}

/// The open charging window of the current run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Segment {
    pub opened_at: DateTime<Utc>,
    /// Time is persisted as charged through this instant; crash recovery
    /// charges one heartbeat interval past it.
    pub heartbeat_at: DateTime<Utc>,
}

/// An admitted agent launch that has not settled yet.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct InFlight {
    pub invocation: u64,
    pub run: u64,
    pub stage: Option<String>,
    pub attempt: u32,
    pub pid: Option<u32>,
    /// Process start time (seconds since the Unix epoch) captured at spawn.
    /// Crash recovery signals `pid` only when this still matches, because
    /// PIDs are reused.
    pub process_start: Option<u64>,
    pub admitted_at: DateTime<Utc>,
    /// The latest instant this launch may run: admission time plus the
    /// remaining active allowance.
    pub deadline: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum EventKind {
    Initialized,
    Opened,
    Stage,
    Admitted,
    Spawned,
    Settled,
    Refused,
    Exhausted,
    Closed,
    CrashRecovered,
    Suspended,
    Resumed,
    Granted,
}

/// One append-only ledger history entry.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Event {
    pub at: DateTime<Utc>,
    pub kind: EventKind,
    /// The budgeted run (1-based) the event belongs to; `0` outside a run.
    pub run: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invocation: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
}

/// What ended a budgeted run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CloseOutcome {
    /// Every step succeeded; the run now awaits human review.
    Completed,
    /// The sequence exited with a failing status.
    Failed(i32),
    /// The operator interrupted the run.
    Interrupted,
    /// The sequence returned an error before producing a status.
    Error(String),
}

/// What crash recovery found for an in-flight launch the dead runner owned.
#[derive(Debug)]
pub(crate) enum OrphanOutcome {
    /// No launch was in flight, or it recorded no process identity.
    NoneRecorded,
    /// The recorded process no longer exists.
    NotRunning,
    /// The PID now belongs to a different process; it was not signaled.
    IdentityMismatch,
    /// The verified orphan was still running and was terminated.
    Terminated,
    /// The verified orphan was still running and could not be terminated.
    TerminateFailed(std::io::Error),
}

impl OrphanOutcome {
    fn describe(&self) -> String {
        match self {
            Self::NoneRecorded => "no in-flight process recorded".to_string(),
            Self::NotRunning => "in-flight process no longer running".to_string(),
            Self::IdentityMismatch => {
                "recorded PID now names a different process; not signaled".to_string()
            }
            Self::Terminated => "orphaned process tree terminated".to_string(),
            Self::TerminateFailed(error) => format!("orphaned process could not be terminated: {error}"),
        }
    }
}

/// A successful admission: the invocation number and the remaining active
/// allowance the launch may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Admission {
    pub invocation: u64,
    pub remaining_ms: u64,
}

/// The persisted shared budget of one platform run.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct Ledger {
    pub format: String,
    pub run_id: String,
    pub platform: String,
    pub state: LedgerState,
    pub stop_reason: Option<String>,
    /// The most recent sequence step entered; after a stop, the incomplete one.
    pub stage: Option<String>,
    pub limits: Allowance,
    pub grants: Vec<Grant>,
    pub used: Allowance,
    pub heartbeat_ms: u64,
    /// A lock file every run sharing it must hold, so runs of different
    /// platforms execute one at a time. Relative paths resolve against the
    /// ledger's directory.
    pub exclusive_lock: Option<String>,
    /// How many budgeted runs have opened this ledger; restarts increment it.
    pub runs: u64,
    pub segment: Option<Segment>,
    pub in_flight: Vec<InFlight>,
    pub events: Vec<Event>,
}

impl Ledger {
    /// Create a ledger in the `Stopped` state.
    ///
    /// ## Errors
    ///
    /// Both limits must be positive and the identity fields non-empty; there
    /// are no defaults.
    pub(crate) fn new(
        run_id: &str,
        platform: &str,
        limits: Allowance,
        heartbeat_ms: u64,
        exclusive_lock: Option<String>,
        now: DateTime<Utc>,
    ) -> Result<Self, BudgetError> {
        let mut ledger = Self {
            format: LEDGER_FORMAT.to_string(),
            run_id: run_id.to_string(),
            platform: platform.to_string(),
            state: LedgerState::Stopped,
            stop_reason: Some("initialized".to_string()),
            stage: None,
            limits,
            grants: Vec::new(),
            used: Allowance::default(),
            heartbeat_ms,
            exclusive_lock,
            runs: 0,
            segment: None,
            in_flight: Vec::new(),
            events: Vec::new(),
        };
        ledger.validate()?;
        ledger.push(now, EventKind::Initialized, None, None);
        Ok(ledger)
    }

    /// Reject a ledger that could not have been produced by [`Ledger::new`].
    pub(crate) fn validate(&self) -> Result<(), BudgetError> {
        if self.format != LEDGER_FORMAT {
            return Err(BudgetError::Invalid(format!(
                "unsupported ledger format `{}` (expected `{LEDGER_FORMAT}`)",
                self.format
            )));
        }
        if self.run_id.trim().is_empty() || self.platform.trim().is_empty() {
            return Err(BudgetError::Invalid(
                "run id and platform must not be empty".to_string(),
            ));
        }
        if self.limits.invocations == 0 || self.limits.active_ms == 0 {
            return Err(BudgetError::Invalid(
                "both the invocation limit and the elapsed-time limit must be positive".to_string(),
            ));
        }
        if self.heartbeat_ms == 0 {
            return Err(BudgetError::Invalid("heartbeat interval must be positive".to_string()));
        }
        Ok(())
    }

    /// Limits plus every recorded grant.
    pub(crate) fn allowed(&self) -> Allowance {
        self.grants.iter().fold(self.limits, |acc, grant| Allowance {
            invocations: acc.invocations.saturating_add(grant.invocations),
            active_ms: acc.active_ms.saturating_add(grant.active_ms),
        })
    }

    pub(crate) fn remaining(&self) -> Allowance {
        let allowed = self.allowed();
        Allowance {
            invocations: allowed.invocations.saturating_sub(self.used.invocations),
            active_ms: allowed.active_ms.saturating_sub(self.used.active_ms),
        }
    }

    pub(crate) fn is_exhausted(&self) -> bool {
        let remaining = self.remaining();
        remaining.invocations == 0 || remaining.active_ms == 0
    }

    /// Whether active time ran out: the one limit that can be crossed while a
    /// launch or a step is still running. Invocation exhaustion is detected at
    /// the next admission or step boundary, where it names the step that
    /// could not run.
    pub(crate) fn is_time_exhausted(&self) -> bool {
        self.remaining().active_ms == 0
    }

    /// Whether the current run has already recorded its exhaustion.
    pub(crate) fn exhaustion_noted(&self) -> bool {
        self.events
            .iter()
            .any(|event| event.kind == EventKind::Exhausted && event.run == self.runs)
    }

    /// Start a budgeted run.
    ///
    /// ## Errors
    ///
    /// Refuses unless the ledger is `Stopped` with allowance remaining. An
    /// exhausted ledger is moved to `Exhausted` rather than opened.
    pub(crate) fn open(&mut self, now: DateTime<Utc>) -> Result<(), BudgetError> {
        if self.state != LedgerState::Stopped {
            return Err(BudgetError::NotRunnable {
                state: self.state,
                reason: self.stop_reason.clone(),
            });
        }
        if self.is_exhausted() {
            self.state = LedgerState::Exhausted;
            self.stop_reason = Some(self.exhaustion_reason());
            return Err(BudgetError::Exhausted(self.exhaustion_reason()));
        }
        self.runs += 1;
        self.state = LedgerState::Active;
        self.stop_reason = None;
        self.segment = Some(Segment {
            opened_at: now,
            heartbeat_at: now,
        });
        self.push(now, EventKind::Opened, None, None);
        Ok(())
    }

    /// Charge active time and mark the heartbeat.
    pub(crate) fn heartbeat(&mut self, elapsed_ms: u64, now: DateTime<Utc>) {
        self.used.active_ms = self.used.active_ms.saturating_add(elapsed_ms);
        if let Some(segment) = self.segment.as_mut() {
            segment.heartbeat_at = now;
        }
    }

    /// Record the step being entered. Returns `true` when the budget is
    /// exhausted and the run must stop before the step does any work.
    pub(crate) fn enter_stage(&mut self, stage: &str, elapsed_ms: u64, now: DateTime<Utc>) -> bool {
        self.heartbeat(elapsed_ms, now);
        self.stage = Some(stage.to_string());
        self.push(now, EventKind::Stage, None, Some(stage.to_string()));
        if self.is_exhausted() {
            self.note_exhaustion(now);
            return true;
        }
        false
    }

    /// Debit one invocation before an agent is spawned.
    ///
    /// ## Errors
    ///
    /// Returns [`BudgetError::Exhausted`] and charges nothing when either
    /// limit is already reached.
    pub(crate) fn admit(
        &mut self,
        attempt: u32,
        elapsed_ms: u64,
        now: DateTime<Utc>,
    ) -> Result<Admission, BudgetError> {
        self.heartbeat(elapsed_ms, now);
        if self.is_exhausted() {
            self.push(now, EventKind::Refused, None, self.stage.clone());
            self.note_exhaustion(now);
            return Err(BudgetError::Exhausted(self.exhaustion_reason()));
        }
        self.used.invocations += 1;
        let invocation = self.used.invocations;
        let remaining_ms = self.remaining().active_ms;
        self.in_flight.push(InFlight {
            invocation,
            run: self.runs,
            stage: self.stage.clone(),
            attempt,
            pid: None,
            process_start: None,
            admitted_at: now,
            deadline: now + chrono::Duration::milliseconds(i64::try_from(remaining_ms).unwrap_or(i64::MAX)),
        });
        self.push(
            now,
            EventKind::Admitted,
            Some(invocation),
            Some(format!("attempt {attempt}")),
        );
        Ok(Admission {
            invocation,
            remaining_ms,
        })
    }

    /// Attach the spawned child's identity to an admitted launch.
    pub(crate) fn record_child(
        &mut self,
        invocation: u64,
        pid: u32,
        process_start: Option<u64>,
        now: DateTime<Utc>,
    ) {
        if let Some(entry) = self.in_flight.iter_mut().find(|e| e.invocation == invocation) {
            entry.pid = Some(pid);
            entry.process_start = process_start;
        }
        self.push(now, EventKind::Spawned, Some(invocation), Some(format!("pid {pid}")));
    }

    /// Clear a finished launch. Its invocation stays charged. Returns `true`
    /// when active time ran out and the run must stop.
    pub(crate) fn settle(&mut self, invocation: u64, elapsed_ms: u64, now: DateTime<Utc>) -> bool {
        self.heartbeat(elapsed_ms, now);
        self.in_flight.retain(|entry| entry.invocation != invocation);
        let exhausted = self.is_time_exhausted();
        if exhausted {
            self.note_exhaustion(now);
        }
        let detail = self.exhaustion_noted().then(|| {
            "settled after exhaustion: local process exited; remote model work and billing unverified"
                .to_string()
        });
        self.push(now, EventKind::Settled, Some(invocation), detail);
        exhausted
    }

    /// Record exhaustion once per run. Returns `true` the first time.
    pub(crate) fn note_exhaustion(&mut self, now: DateTime<Utc>) -> bool {
        if self.exhaustion_noted() {
            return false;
        }
        let mut detail = self.exhaustion_reason();
        if !self.in_flight.is_empty() {
            detail.push_str(
                "; in-flight local work is being stopped; remote model work and billing may continue (unverified)",
            );
        }
        self.push(now, EventKind::Exhausted, None, Some(detail));
        true
    }

    /// End the run: fold the last time slice and choose the resting state.
    pub(crate) fn close(&mut self, outcome: &CloseOutcome, elapsed_ms: u64, now: DateTime<Utc>) {
        self.heartbeat(elapsed_ms, now);
        self.segment = None;
        // A launch still listed here never settled; its invocation stays charged.
        self.in_flight.clear();
        let (state, reason) = if self.is_exhausted() && outcome != &CloseOutcome::Completed {
            (LedgerState::Exhausted, self.exhaustion_reason())
        } else {
            match outcome {
                CloseOutcome::Completed => {
                    (LedgerState::Suspended, "awaiting human review".to_string())
                }
                CloseOutcome::Failed(code) => {
                    (LedgerState::Stopped, format!("sequence exited with status {code}"))
                }
                CloseOutcome::Interrupted => {
                    (LedgerState::Stopped, "interrupted by the operator".to_string())
                }
                CloseOutcome::Error(message) => (LedgerState::Stopped, message.clone()),
            }
        };
        self.state = state;
        self.stop_reason = Some(reason.clone());
        self.push(now, EventKind::Closed, None, Some(reason));
    }

    /// Account for a runner that died while `Active`.
    ///
    /// Charges one heartbeat interval past the last persisted heartbeat (the
    /// most the dead runner could have used unseen), plus the time a verified
    /// orphan kept working until it was terminated. The downtime otherwise
    /// counts as operator-resumption time, so the ledger rests `Interrupted`.
    pub(crate) fn recover_crash(&mut self, orphan: &OrphanOutcome, now: DateTime<Utc>) {
        let mut charged = self.heartbeat_ms;
        if let (OrphanOutcome::Terminated, Some(segment)) = (orphan, self.segment.as_ref()) {
            let orphan_ms = (now - segment.heartbeat_at).num_milliseconds().max(0) as u64;
            charged = charged.max(orphan_ms);
        }
        self.used.active_ms = self.used.active_ms.saturating_add(charged);
        self.segment = None;
        self.in_flight.clear();
        self.state = LedgerState::Interrupted;
        let reason = format!(
            "previous runner stopped unexpectedly; charged {charged} ms after its last heartbeat; {}",
            orphan.describe()
        );
        self.stop_reason = Some(reason.clone());
        self.push(now, EventKind::CrashRecovered, None, Some(reason));
    }

    /// Pause a stopped ledger for human approval.
    pub(crate) fn suspend(&mut self, reason: &str, now: DateTime<Utc>) -> Result<(), BudgetError> {
        if self.state != LedgerState::Stopped {
            return Err(self.transition_error("suspend"));
        }
        self.state = LedgerState::Suspended;
        self.stop_reason = Some(reason.to_string());
        self.push(now, EventKind::Suspended, None, Some(reason.to_string()));
        Ok(())
    }

    /// Return a suspended or interrupted ledger to `Stopped`.
    pub(crate) fn resume(&mut self, operator: &str, now: DateTime<Utc>) -> Result<(), BudgetError> {
        if !matches!(self.state, LedgerState::Suspended | LedgerState::Interrupted) {
            return Err(self.transition_error("resume"));
        }
        self.state = LedgerState::Stopped;
        self.stop_reason = Some(format!("resumed by {operator}"));
        self.push(now, EventKind::Resumed, None, Some(format!("operator {operator}")));
        Ok(())
    }

    /// Record extra allowance decided by an operator.
    pub(crate) fn grant(
        &mut self,
        operator: &str,
        reason: &str,
        extra: Allowance,
        now: DateTime<Utc>,
    ) -> Result<(), BudgetError> {
        if self.state == LedgerState::Active {
            return Err(self.transition_error("grant"));
        }
        if operator.trim().is_empty() || reason.trim().is_empty() {
            return Err(BudgetError::Invalid(
                "a grant needs a named operator and a reason".to_string(),
            ));
        }
        if extra.invocations == 0 && extra.active_ms == 0 {
            return Err(BudgetError::Invalid(
                "a grant must add invocations, time, or both".to_string(),
            ));
        }
        self.grants.push(Grant {
            operator: operator.to_string(),
            reason: reason.to_string(),
            invocations: extra.invocations,
            active_ms: extra.active_ms,
            recorded_at: now,
        });
        self.push(
            now,
            EventKind::Granted,
            None,
            Some(format!(
                "operator {operator}: +{} invocations, +{} ms",
                extra.invocations, extra.active_ms
            )),
        );
        if self.state == LedgerState::Exhausted && !self.is_exhausted() {
            self.state = LedgerState::Stopped;
            self.stop_reason = Some(format!("reopened by a grant from {operator}"));
        }
        Ok(())
    }

    fn exhaustion_reason(&self) -> String {
        let allowed = self.allowed();
        let stage = self.stage.as_deref().unwrap_or("before the first stage");
        format!(
            "budget exhausted at stage `{stage}`: {}/{} invocations, {}/{} ms",
            self.used.invocations, allowed.invocations, self.used.active_ms, allowed.active_ms
        )
    }

    fn transition_error(&self, action: &'static str) -> BudgetError {
        BudgetError::Transition {
            action,
            state: self.state,
        }
    }

    fn push(
        &mut self,
        at: DateTime<Utc>,
        kind: EventKind,
        invocation: Option<u64>,
        detail: Option<String>,
    ) {
        self.events.push(Event {
            at,
            kind,
            run: self.runs,
            invocation,
            detail,
        });
    }
}

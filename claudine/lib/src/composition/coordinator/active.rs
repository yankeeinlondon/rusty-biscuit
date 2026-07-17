//! Active-document execution state: a replaceable provider-attempt slice
//! inside a longer-lived document-iteration slice.
//!
//! The nesting is the point. Retry and resume replace the inner slice, so a
//! fresh attempt gets a fresh session and fresh per-attempt timing — but the
//! budgets that bound retry and resume live in the *outer* slice, where
//! replacing an attempt cannot reach them. Otherwise a retry would reset the
//! limit that is supposed to stop it.

use std::time::Instant;

use indexmap::IndexMap;

use crate::composition::lifecycle_control::control_budget_for;

/// Which control a budget bounds.
///
/// Retry and resume get separate, labeled homes rather than sharing one
/// counter: "3 attempts left" is unanswerable when two different controls
/// spend from it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ControlBudgetKind {
    #[allow(missing_docs)]
    Retry,
    #[allow(missing_docs)]
    Resume,
}

/// One control's attempt ceiling for the current document iteration.
///
/// A lifecycle `retry`/`resume` declares `max_attempts` relative to the
/// attempt at which it *first* fires, so the ceiling is computed once and
/// reused; recomputing it per firing would let it drift upward with the
/// attempt counter and never exhaust.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ControlBudget {
    kind: ControlBudgetKind,
    ceiling: Option<u32>,
}

impl ControlBudget {
    #[allow(missing_docs)]
    pub fn new(kind: ControlBudgetKind) -> Self {
        Self {
            kind,
            ceiling: None,
        }
    }

    #[allow(missing_docs)]
    pub fn kind(&self) -> ControlBudgetKind {
        self.kind
    }

    /// The established ceiling, or `None` before this control has fired.
    pub fn ceiling(&self) -> Option<u32> {
        self.ceiling
    }

    /// Return the ceiling for a control firing at `attempt`, establishing it
    /// on first call and reusing it thereafter.
    pub fn ceiling_for(&mut self, attempt: u32, max_attempts: u32) -> u32 {
        *self
            .ceiling
            .get_or_insert_with(|| control_budget_for(attempt, max_attempts))
    }

    /// Whether a control firing at `attempt` still has budget.
    pub fn permits(&self, attempt: u32) -> bool {
        self.ceiling.is_none_or(|ceiling| attempt < ceiling)
    }
}

/// How the previous provider attempt ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AttemptOutcome {
    #[allow(missing_docs)]
    Succeeded,
    #[allow(missing_docs)]
    Blocked,
    #[allow(missing_docs)]
    Failed,
}

/// One provider attempt at the active document.
///
/// Replaced wholesale by retry and resume.
#[derive(Debug, Clone)]
pub struct ProviderAttempt {
    number: u32,
    last_outcome: Option<AttemptOutcome>,
    session_id: Option<String>,
    resume_followup: Option<String>,
    started: Option<Instant>,
}

impl ProviderAttempt {
    fn first() -> Self {
        Self {
            number: 1,
            last_outcome: None,
            session_id: None,
            resume_followup: None,
            started: None,
        }
    }

    #[allow(missing_docs)]
    pub fn number(&self) -> u32 {
        self.number
    }

    #[allow(missing_docs)]
    pub fn last_outcome(&self) -> Option<AttemptOutcome> {
        self.last_outcome
    }

    /// The live provider session, when one exists.
    pub fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    /// The follow-up message that will be substituted as provider input.
    pub fn resume_followup(&self) -> Option<&str> {
        self.resume_followup.as_deref()
    }

    #[allow(missing_docs)]
    pub fn started(&self) -> Option<Instant> {
        self.started
    }

    #[allow(missing_docs)]
    pub fn mark_started(&mut self, at: Instant) {
        self.started = Some(at);
    }

    #[allow(missing_docs)]
    pub fn record_outcome(&mut self, outcome: AttemptOutcome) {
        self.last_outcome = Some(outcome);
    }

    /// Adopt the session the provider reported for this attempt.
    pub fn adopt_session(&mut self, session_id: String) {
        self.session_id = Some(session_id);
    }
}

/// One iteration of the active document's loop.
#[derive(Debug, Clone)]
pub struct DocumentIteration {
    number: u32,
    mutations: IndexMap<String, serde_json::Value>,
    retry: ControlBudget,
    resume: ControlBudget,
    attempt: ProviderAttempt,
}

impl DocumentIteration {
    fn first() -> Self {
        Self {
            number: 1,
            mutations: IndexMap::new(),
            retry: ControlBudget::new(ControlBudgetKind::Retry),
            resume: ControlBudget::new(ControlBudgetKind::Resume),
            attempt: ProviderAttempt::first(),
        }
    }

    #[allow(missing_docs)]
    pub fn number(&self) -> u32 {
        self.number
    }

    /// In-memory loop mutations accumulated by this iteration.
    pub fn mutations(&self) -> &IndexMap<String, serde_json::Value> {
        &self.mutations
    }

    #[allow(missing_docs)]
    pub fn mutations_mut(&mut self) -> &mut IndexMap<String, serde_json::Value> {
        &mut self.mutations
    }

    #[allow(missing_docs)]
    pub fn retry_budget(&self) -> &ControlBudget {
        &self.retry
    }

    #[allow(missing_docs)]
    pub fn retry_budget_mut(&mut self) -> &mut ControlBudget {
        &mut self.retry
    }

    #[allow(missing_docs)]
    pub fn resume_budget(&self) -> &ControlBudget {
        &self.resume
    }

    #[allow(missing_docs)]
    pub fn resume_budget_mut(&mut self) -> &mut ControlBudget {
        &mut self.resume
    }

    #[allow(missing_docs)]
    pub fn attempt(&self) -> &ProviderAttempt {
        &self.attempt
    }

    #[allow(missing_docs)]
    pub fn attempt_mut(&mut self) -> &mut ProviderAttempt {
        &mut self.attempt
    }

    /// Replace the provider-attempt slice for a retry: a fresh attempt number,
    /// no session, no follow-up, no per-attempt timing.
    ///
    /// The enclosing budgets are untouched — that is the whole reason they
    /// live out here.
    pub fn retry_attempt(&mut self) {
        let number = self.attempt.number.saturating_add(1);
        self.attempt = ProviderAttempt {
            number,
            last_outcome: None,
            session_id: None,
            resume_followup: None,
            started: None,
        };
    }

    /// Replace the provider-attempt slice for a resume, retaining `session`
    /// and substituting `message` as the next provider input.
    pub fn resume_attempt(&mut self, session: String, message: Option<String>) {
        let number = self.attempt.number.saturating_add(1);
        self.attempt = ProviderAttempt {
            number,
            last_outcome: None,
            session_id: Some(session),
            resume_followup: message,
            started: None,
        };
    }
}

/// Everything mutable within the current active document.
///
/// A proxy discards this whole layer — see [`Self::initial`], which is what
/// the coordinator builds for the target. Nothing here is carried across a
/// handoff; the chain, hop accounting, approval cache, and command-wide
/// timing live in the run ledger instead, precisely so they survive one.
#[derive(Debug, Clone)]
pub struct ActiveDocumentState {
    iteration: DocumentIteration,
}

impl ActiveDocumentState {
    /// Fresh state for a newly adopted document: iteration 1, attempt 1,
    /// unestablished budgets.
    pub fn initial() -> Self {
        Self {
            iteration: DocumentIteration::first(),
        }
    }

    #[allow(missing_docs)]
    pub fn iteration(&self) -> &DocumentIteration {
        &self.iteration
    }

    #[allow(missing_docs)]
    pub fn iteration_mut(&mut self) -> &mut DocumentIteration {
        &mut self.iteration
    }

    /// Advance the document loop: a fresh iteration with fresh budgets and a
    /// fresh attempt, keeping only the iteration counter's continuity.
    pub fn advance_iteration(&mut self) {
        let number = self.iteration.number.saturating_add(1);
        self.iteration = DocumentIteration {
            number,
            ..DocumentIteration::first()
        };
    }
}

impl Default for ActiveDocumentState {
    fn default() -> Self {
        Self::initial()
    }
}

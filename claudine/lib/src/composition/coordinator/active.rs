//! Active-document execution state: a replaceable provider-attempt slice
//! inside a longer-lived document-iteration slice.
//!
//! The nesting is the point. Retry and resume replace the inner slice, so a
//! fresh attempt gets a fresh session and fresh per-attempt timing — but the
//! budgets that bound retry and resume live in the *outer* slice, where
//! replacing an attempt cannot reach them. Otherwise a retry would reset the
//! limit that is supposed to stop it.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use indexmap::IndexMap;

use crate::composition::lifecycle_control::control_budget_for;

/// The launch properties a provider cannot renegotiate when a live session is
/// resumed (spec R8).
///
/// A resume reuses the provider session created by a previous attempt. Between
/// that attempt and the resume, the active document is refreshed through the
/// canonical preparation service; if that refresh changes any property the
/// provider fixed when the session was opened, feeding the new launch plan into
/// the old session would silently run under a stale contract. This key captures
/// those fixed properties so the runtime can compare "the key that made this
/// session" against "the key of the freshly refreshed plan" and refuse a
/// mismatch instead of mixing the two.
///
/// The listed facets are the R8 minimum; [`Self::extra`] is the extension point
/// through which a provider adapter contributes provider-specific identity
/// fields without changing this type.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SessionCompatibilityKey {
    /// Provider slug the session was created under.
    pub provider: String,
    /// Effective model, when one is pinned.
    pub model: Option<String>,
    /// Provider binary / profile identity.
    pub binary: String,
    /// Resume-protocol identity: the provider's resume style plus whether it
    /// supports resume at all.
    pub resume_protocol: String,
    /// Workspace / child working directory the provider ran in.
    pub workspace_cwd: String,
    /// Permission / tool mode (bypass vs prompt).
    pub permission_mode: String,
    /// Interactive vs non-interactive session shape.
    pub interactivity: String,
    /// Structured-output mode identity.
    pub structured_output: String,
    /// System-prompt delivery mode plus a digest of its content.
    pub system_prompt: String,
    /// Effective MCP server set, sorted for order-independent comparison.
    pub mcp_servers: Vec<String>,
    /// Provider-specific identity facets a provider adapter contributes. Empty
    /// until an adapter populates it; the generic facets above are always set.
    pub extra: BTreeMap<String, String>,
}

impl SessionCompatibilityKey {
    /// Names of the facets that differ between the key that created a live
    /// session (`self`) and a freshly prepared launch plan (`other`).
    ///
    /// An empty result means the refreshed plan can safely resume the session.
    /// A non-empty result is the list of properties the resume must refuse,
    /// named for the diagnostic.
    pub fn incompatibilities(&self, other: &Self) -> Vec<String> {
        let mut facets = Vec::new();
        if self.provider != other.provider {
            facets.push("provider".to_string());
        }
        if self.model != other.model {
            facets.push("model".to_string());
        }
        if self.binary != other.binary {
            facets.push("profile/binary".to_string());
        }
        if self.resume_protocol != other.resume_protocol {
            facets.push("resume protocol".to_string());
        }
        if self.workspace_cwd != other.workspace_cwd {
            facets.push("workspace CWD".to_string());
        }
        if self.permission_mode != other.permission_mode {
            facets.push("permission mode".to_string());
        }
        if self.interactivity != other.interactivity {
            facets.push("interactivity".to_string());
        }
        if self.structured_output != other.structured_output {
            facets.push("structured-output mode".to_string());
        }
        if self.system_prompt != other.system_prompt {
            facets.push("system prompt".to_string());
        }
        if self.mcp_servers != other.mcp_servers {
            facets.push("MCP server set".to_string());
        }
        let mut seen = BTreeSet::new();
        for key in self.extra.keys().chain(other.extra.keys()) {
            if !seen.insert(key) {
                continue;
            }
            if self.extra.get(key) != other.extra.get(key) {
                facets.push(format!("provider identity `{key}`"));
            }
        }
        facets
    }

    /// Whether a freshly prepared plan (`other`) can resume a session created
    /// under `self`.
    pub fn is_compatible(&self, other: &Self) -> bool {
        self.incompatibilities(other).is_empty()
    }
}

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
    compat_key: Option<SessionCompatibilityKey>,
}

impl ProviderAttempt {
    fn first() -> Self {
        Self {
            number: 1,
            last_outcome: None,
            session_id: None,
            resume_followup: None,
            started: None,
            compat_key: None,
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

    /// The compatibility key of the launch that opened this attempt's session,
    /// carried forward from the session-producing attempt across a resume.
    pub fn compat_key(&self) -> Option<&SessionCompatibilityKey> {
        self.compat_key.as_ref()
    }

    /// Record the compatibility key of the launch this attempt runs under, so a
    /// later resume can compare its refreshed plan against it.
    pub fn set_compat_key(&mut self, key: SessionCompatibilityKey) {
        self.compat_key = Some(key);
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
            // A retry opens a fresh session, so there is no prior-session key to
            // carry forward — the retried attempt records its own on launch.
            compat_key: None,
        };
    }

    /// Replace the provider-attempt slice for a resume, retaining `session`
    /// and substituting `message` as the next provider input.
    ///
    /// The compatibility key of the session-producing attempt travels forward
    /// with the session it belongs to, so the resumed attempt can compare its
    /// refreshed launch plan against the plan that opened the session.
    pub fn resume_attempt(&mut self, session: String, message: Option<String>) {
        let number = self.attempt.number.saturating_add(1);
        let compat_key = self.attempt.compat_key.clone();
        self.attempt = ProviderAttempt {
            number,
            last_outcome: None,
            session_id: Some(session),
            resume_followup: message,
            started: None,
            compat_key,
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

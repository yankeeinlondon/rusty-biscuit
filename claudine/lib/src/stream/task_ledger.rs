//! Provider-agnostic, attempt-scoped ledger of sub-agent task outcomes.
//!
//! A provider process exiting with code 0 is not proof that its sub-agent work
//! finished. Claude Code, for example, can force-stop background tasks, emit
//! `task_notification` records carrying `status: "stopped"`, complete the
//! parent turn, and exit 0. This ledger is the authoritative record that lets
//! stream finalization turn that shape into a real failure.
//!
//! ## Notes
//!
//! Identity is the **nonempty provider task ID and nothing else**. Names,
//! descriptions, and apparent purpose are display metadata: Claudine has no
//! sound way to prove two prose labels describe the same unit of work, so a
//! completed task named `commit` never clears a stopped task named `commit`.
//! An observation without a provider ID is enrolled under a distinct internal
//! identity, so one anonymous event can never clear another; those synthetic
//! identities stay inside this module and are never written into machine
//! output as if the provider had supplied them.
//!
//! The ledger is unbounded and lives for exactly one provider attempt. It is
//! **not** the CLI watchdog's `recent_subagents` ring, which is a five-entry
//! diagnostic aid and deliberately lossy.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::summary::StreamExecutionSummary;

/// Raw provider statuses that count as a successful terminal state.
///
/// This is an explicit allowlist, never a fallback: an unrecognized status is
/// finalized as [`TaskOutcome::UnknownStatus`] and poisons success until the
/// provider's vocabulary is extended here with fixtures.
const SUCCESS_STATUSES: &[&str] = &["completed", "success", "succeeded"];

/// Raw provider statuses that are terminal but unsuccessful.
const UNSUCCESSFUL_STATUSES: &[&str] = &[
    "stopped",
    "failed",
    "failure",
    "error",
    "errored",
    "cancelled",
    "canceled",
    "aborted",
    "timed_out",
    "timeout",
    "interrupted",
    "killed",
];

/// Raw provider statuses that describe work still in flight.
///
/// The third vocabulary exists so that "not terminal" is something the provider
/// says, rather than something Claudine infers from its own ignorance. Every
/// entry is a word whose plain meaning is *still going*, so reading it as an
/// abandoned or failed task would be wrong; anything outside all three lists is
/// a word Claudine cannot interpret at all, and is failed closed. Extend this
/// with fixtures when a provider is observed emitting a new in-flight word —
/// never to silence a status whose meaning is actually unknown.
const PROGRESS_STATUSES: &[&str] = &[
    "thinking",
    "running",
    "in_progress",
    "progress",
    "started",
    "starting",
    "pending",
    "queued",
    "working",
    "active",
    "resumed",
];

/// `error_kind` stamped on a summary poisoned by incomplete sub-agent work.
pub const INCOMPLETE_SUBAGENTS_ERROR_KIND: &str = "incomplete_subagents";

/// Normalized terminal state of one provider sub-agent task.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskOutcome {
    /// A terminal observation whose raw status is in [`SUCCESS_STATUSES`].
    Succeeded,
    /// A terminal observation whose raw status is a recognized unsuccessful
    /// state (`stopped` and friends).
    Stopped,
    /// A terminal observation whose raw status is absent or outside every
    /// recognized vocabulary. Deliberately not success — see the module note.
    UnknownStatus,
    /// The task started and the session ended with no terminal observation.
    Unfinished,
}

impl TaskOutcome {
    /// True for every state that is not a recognized success.
    pub const fn is_incomplete(self) -> bool {
        !matches!(self, Self::Succeeded)
    }
}

impl std::fmt::Display for TaskOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Succeeded => write!(f, "succeeded"),
            Self::Stopped => write!(f, "stopped"),
            Self::UnknownStatus => write!(f, "unknown status"),
            Self::Unfinished => write!(f, "never finished"),
        }
    }
}

/// One normalized, provider-agnostic sub-agent task fact.
///
/// `task_id` is the provider's own identifier and is `None` when the provider
/// did not supply one — the ledger's internal identity for anonymous
/// observations is never leaked here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SubagentOutcome {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub outcome: TaskOutcome,
    /// The provider's raw status string, preserved byte-for-byte — padding and
    /// case included, and a whitespace-only string kept as-is — so an
    /// unrecognized vocabulary can be diagnosed from machine data alone.
    /// `None` only when the provider supplied no status. Normalization belongs
    /// to classification and display, never to this field.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub raw_status: Option<String>,
}

impl SubagentOutcome {
    /// Best available operator-facing label: provider name, else provider task
    /// ID, else a placeholder. Never the internal anonymous identity.
    pub fn label(&self) -> String {
        self.name
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .or_else(|| {
                self.task_id
                    .as_deref()
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
            })
            .map_or_else(|| "unnamed task".to_string(), str::to_string)
    }

    /// One list-ready line: label plus the reason it is incomplete.
    pub fn describe(&self) -> String {
        match self.raw_status.as_deref().map(str::trim) {
            Some(status) if !status.is_empty() => format!("{} ({status})", self.label()),
            _ => format!("{} ({})", self.label(), self.outcome),
        }
    }
}

/// How a status observed on an event kind without inherent terminality
/// (`task_notification`) must be handled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NotificationRouting {
    /// The event is a progress report and carries nothing worth preserving as
    /// an outcome.
    Progress,
    /// The event is a terminal observation with this outcome.
    Terminal(TaskOutcome),
}

/// Route a `task_notification` status through the three status vocabularies.
///
/// An absent or blank status is progress because the provider said nothing that
/// could be preserved; a status in the success or unsuccessful vocabulary is
/// terminal; a status in [`PROGRESS_STATUSES`] is progress; and a status that is
/// present but in none of the three is terminal with
/// [`TaskOutcome::UnknownStatus`]. That last branch is the fail-closed residue:
/// Claudine cannot read a future provider vocabulary, so an uninterpretable word
/// must poison success rather than quietly resolve the task.
pub fn route_notification_status(status: Option<&str>) -> NotificationRouting {
    let Some(normalized) = normalized_status(status) else {
        return NotificationRouting::Progress;
    };
    if let Some(outcome) = recognized_terminal_outcome(&normalized) {
        return NotificationRouting::Terminal(outcome);
    }
    if PROGRESS_STATUSES.contains(&normalized.as_str()) {
        return NotificationRouting::Progress;
    }
    NotificationRouting::Terminal(TaskOutcome::UnknownStatus)
}

/// Trim-normalized, case-folded status, or `None` when the provider supplied
/// nothing to interpret.
fn normalized_status(status: Option<&str>) -> Option<String> {
    let normalized = status?.trim().to_ascii_lowercase();
    (!normalized.is_empty()).then_some(normalized)
}

/// Classify an already-normalized status against the two terminal vocabularies.
fn recognized_terminal_outcome(normalized: &str) -> Option<TaskOutcome> {
    if SUCCESS_STATUSES.contains(&normalized) {
        return Some(TaskOutcome::Succeeded);
    }
    if UNSUCCESSFUL_STATUSES.contains(&normalized) {
        return Some(TaskOutcome::Stopped);
    }
    None
}

/// Classify a status observed on an event kind that is *inherently* terminal
/// (`task_completed`). Every such event yields a terminal outcome; an
/// unrecognized status becomes [`TaskOutcome::UnknownStatus`] rather than being
/// optimistically read as success.
fn forced_terminal_outcome(status: Option<&str>) -> TaskOutcome {
    normalized_status(status)
        .as_deref()
        .and_then(recognized_terminal_outcome)
        .unwrap_or(TaskOutcome::UnknownStatus)
}

/// The ledger's internal identity for one task.
#[derive(Debug, Clone, PartialEq, Eq)]
enum TaskIdentity {
    /// A nonempty provider task ID — the only identity that reconciles.
    Provider(String),
    /// A synthetic per-observation identity for a task the provider did not
    /// name. Distinct by construction, so it can never be cleared.
    Anonymous(u64),
}

#[derive(Debug, Clone, PartialEq)]
enum EntryState {
    InFlight,
    Terminal {
        outcome: TaskOutcome,
        raw_status: Option<String>,
    },
}

#[derive(Debug, Clone)]
struct LedgerEntry {
    identity: TaskIdentity,
    name: Option<String>,
    state: EntryState,
}

/// Unbounded, attempt-scoped record of every sub-agent task the stream
/// mentioned. Never caps, never evicts.
#[derive(Debug, Default)]
pub struct TaskLedger {
    entries: Vec<LedgerEntry>,
    by_provider_id: HashMap<String, usize>,
    next_anonymous: u64,
}

impl TaskLedger {
    pub fn new() -> Self {
        Self::default()
    }

    /// True when the stream mentioned no sub-agent task at all.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Record a task start or resume.
    ///
    /// A start for an ID that already has a terminal observation puts that ID
    /// back in flight: the provider re-opened the work, so its earlier outcome
    /// no longer describes the session.
    pub fn record_start(&mut self, task_id: Option<&str>, name: Option<&str>) {
        let index = self.entry_index(task_id, name);
        self.entries[index].state = EntryState::InFlight;
    }

    /// Record a terminal observation. The most recent terminal observation for
    /// an ID wins, which is what lets a later success clear an earlier stop.
    ///
    /// Classification trim- and case-normalizes the status; the stored
    /// `raw_status` is the authored string, untouched.
    pub fn record_terminal(
        &mut self,
        task_id: Option<&str>,
        name: Option<&str>,
        raw_status: Option<&str>,
    ) {
        let outcome = forced_terminal_outcome(raw_status);
        let index = self.entry_index(task_id, name);
        self.entries[index].state = EntryState::Terminal {
            outcome,
            raw_status: raw_status.map(str::to_string),
        };
    }

    /// Facts for every task that did **not** reach a recognized successful
    /// terminal state, in first-observation order.
    pub fn incomplete_outcomes(&self) -> Vec<SubagentOutcome> {
        self.entries
            .iter()
            .filter_map(|entry| {
                let (outcome, raw_status) = match &entry.state {
                    EntryState::InFlight => (TaskOutcome::Unfinished, None),
                    EntryState::Terminal {
                        outcome,
                        raw_status,
                    } => (*outcome, raw_status.clone()),
                };
                outcome.is_incomplete().then(|| SubagentOutcome {
                    task_id: match &entry.identity {
                        TaskIdentity::Provider(id) => Some(id.clone()),
                        TaskIdentity::Anonymous(_) => None,
                    },
                    name: entry.name.clone(),
                    outcome,
                    raw_status,
                })
            })
            .collect()
    }

    /// Project the ledger onto a finished summary.
    ///
    /// Incomplete work poisons success: `is_error` flips true, the facts land
    /// on `subagent_outcomes`, and `error_kind` becomes
    /// [`INCOMPLETE_SUBAGENTS_ERROR_KIND`] **unconditionally** — that label is
    /// the stable machine-facing exit reason for this shape, so a parser error
    /// kind recorded earlier in the attempt may not displace it. The provider's
    /// real `exit_code` and the process termination are left untouched —
    /// Claudine did not kill the child, and manufacturing a nonzero exit would
    /// lie about what happened.
    ///
    /// A displaced provider failure is not discarded: it is retained inside the
    /// composed `error_message`, which is the only place that text still has.
    /// A failure counts whenever *either* a kind or a message is present — a
    /// Claude `result.is_error` carries text with no kind, and that text is
    /// the actionable cause.
    pub fn apply_to_summary(&self, summary: &mut StreamExecutionSummary) {
        let incomplete = self.incomplete_outcomes();
        if incomplete.is_empty() {
            return;
        }
        let prior = prior_failure_clause(
            summary.error_kind.as_deref(),
            summary.error_message.as_deref(),
        );
        summary.error_message = Some(crate::harness::concise_message(&compose_message(
            &incomplete,
            prior.as_deref(),
        )));
        summary.subagent_outcomes = incomplete;
        summary.is_error = true;
        summary.error_kind = Some(INCOMPLETE_SUBAGENTS_ERROR_KIND.to_string());
    }

    fn entry_index(&mut self, task_id: Option<&str>, name: Option<&str>) -> usize {
        let name = name
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        let provider_id = task_id
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);

        if let Some(id) = provider_id {
            if let Some(&index) = self.by_provider_id.get(&id) {
                if name.is_some() {
                    self.entries[index].name = name;
                }
                return index;
            }
            let index = self.entries.len();
            self.by_provider_id.insert(id.clone(), index);
            self.entries.push(LedgerEntry {
                identity: TaskIdentity::Provider(id),
                name,
                state: EntryState::InFlight,
            });
            return index;
        }

        let index = self.entries.len();
        self.entries.push(LedgerEntry {
            identity: TaskIdentity::Anonymous(self.next_anonymous),
            name,
            state: EntryState::InFlight,
        });
        self.next_anonymous += 1;
        index
    }
}

/// Operator-facing headline for a run poisoned by incomplete sub-agent work.
///
/// Names as many tasks as the shared 240-character failure-message budget can
/// hold once [`crate::harness::concise_message`] clamps it; the complete list
/// always remains in `subagent_outcomes`, which is why truncating here is safe.
pub fn incomplete_message(incomplete: &[SubagentOutcome]) -> String {
    compose_message(incomplete, None)
}

/// Character budget reserved for a displaced provider failure.
///
/// The composed message is clamped from the tail, so this reservation is what
/// keeps the displaced failure — the one fact with no other home once
/// `error_kind`/`error_message` are overwritten — from being the part that
/// truncates. Well under the 240-character message budget by construction, so
/// the task list is always the tail that gives way.
const PRIOR_FAILURE_CLAUSE_MAX_CHARS: usize = 96;

/// Build the operator headline, optionally naming a displaced provider failure
/// between the count and the (truncatable) task list.
fn compose_message(incomplete: &[SubagentOutcome], prior: Option<&str>) -> String {
    let count = incomplete.len();
    let noun = if count == 1 { "task" } else { "tasks" };
    let listed = incomplete
        .iter()
        .map(SubagentOutcome::describe)
        .collect::<Vec<_>>()
        .join(", ");
    match prior {
        Some(clause) => format!("{count} sub-agent {noun} did not complete; {clause}; incomplete: {listed}"),
        None => format!("{count} sub-agent {noun} did not complete: {listed}"),
    }
}

/// Render the provider failure this finalization displaces, bounded so it
/// cannot crowd every task name out of the composed headline.
///
/// The provider's text is hygiened *before* the clamp, so the cut can never
/// land inside an escape sequence and leave its tail as visible garbage.
///
/// Returns `None` when neither a non-blank kind nor a non-blank message
/// exists — there is nothing to displace, so the headline carries no clause.
fn prior_failure_clause(kind: Option<&str>, message: Option<&str>) -> Option<String> {
    fn present(value: Option<&str>) -> Option<&str> {
        value.map(str::trim).filter(|text| !text.is_empty())
    }
    let clause = match (present(kind), present(message)) {
        (Some(kind), Some(text)) => format!("after provider failure {kind}: {text}"),
        (Some(kind), None) => format!("after provider failure {kind}"),
        (None, Some(text)) => format!("after provider failure: {text}"),
        (None, None) => return None,
    };
    Some(clamp_chars(
        &crate::harness::concise_message(&clause),
        PRIOR_FAILURE_CLAUSE_MAX_CHARS,
    ))
}

/// Clamp to `budget` characters, reserving one for the ellipsis so the result
/// never exceeds it.
fn clamp_chars(text: &str, budget: usize) -> String {
    if text.chars().count() <= budget {
        return text.to_string();
    }
    let mut clamped: String = text.chars().take(budget.saturating_sub(1)).collect();
    clamped.push('…');
    clamped
}

#[cfg(test)]
mod tests;

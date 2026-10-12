//! What a contention-aware removal did, as library types: the typed
//! [`RemovalOutcome`] and the [`OperationReport`] saved as `report.json` in
//! the operation's recovery directory (see [`super::recovery`]).
//!
//! The outcome is deliberately not an exit code. The CLI maps it, together
//! with the branch results, to the exit-code table of
//! `2026-10-05-graceful-file-contention`; a stop after the move is never
//! reduced to a refusal or to an unexplained Git failure.

use serde::{Deserialize, Serialize};
use sniff::filesystem::query::{FileIdentity, NativeString, PathUsageReport};

use super::recovery::OperationId;

/// Version of the [`OperationReport`] serialization. Bump it with any change
/// to the persisted shape.
pub const REPORT_VERSION: u32 = 1;

/// The most activity observations a report lists. Later ones are counted in
/// [`Truncation::activity_omitted`]; the deletion journal itself is never
/// truncated.
pub const MAX_REPORTED_ACTIVITY: usize = 200;

/// The most retained entries a report names. The count in
/// [`QuarantineReport::retained_entries`] stays exact.
pub const MAX_REPORTED_RETAINED: usize = 200;

/// The operation-level result of one removal attempt.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemovalOutcome {
    /// Discovery found another process using the target; nothing was moved.
    UsageRefused,
    /// The checkout and its registration are verified gone.
    Removed,
    /// Activity at either location stopped the removal after the move.
    ActivityDetected,
    /// A filesystem or Git step failed after the move.
    CleanupFailed,
    /// Observation coverage was lost, or the state could not be confirmed.
    StateUnverified,
}

/// What is at the path the checkout was removed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OriginalState {
    /// Never moved; the checkout is still there.
    Kept,
    /// Moved to the quarantine; nothing is at the original path.
    Moved,
    /// Something appeared at the original path after the move. It is never
    /// deleted or traversed by this operation.
    Recreated,
    /// Could not be inspected.
    Unknown,
}

/// What is at the quarantine path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QuarantineState {
    /// Holds content that was not deleted.
    Present,
    /// Verified absent after deletion.
    Removed,
    /// Could not be inspected.
    Unknown,
}

/// The moved checkout, once a quarantine exists.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QuarantineReport {
    pub path: NativeString,
    /// The checkout directory's identity, unchanged by the move.
    pub identity: FileIdentity,
    pub state: QuarantineState,
    /// Exact number of entries still in the quarantine.
    pub retained_entries: u64,
    /// Up to [`MAX_REPORTED_RETAINED`] of them, relative to the quarantine.
    pub retained: Vec<NativeString>,
}

/// Where Git's registration of the checkout points.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RegistrationState {
    AtOriginal,
    AtQuarantine,
    Removed,
    Unknown,
}

/// A step of the operation, for the timeline.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Step {
    Discovery,
    RecordCreated,
    ObservationStarted,
    InventoryCaptured,
    Moved,
    AssociationUpdated,
    DeletionBatch,
    ContentRemoved,
    Unregistered,
    BranchAction,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StepResult {
    Succeeded,
    Failed,
    /// Attempted, but its result could not be confirmed.
    Uncertain,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimelineEvent {
    /// Milliseconds since the Unix epoch.
    pub at_ms: u64,
    pub step: Step,
    pub result: StepResult,
    pub detail: Option<String>,
}

/// Why a Sniff query ran.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryPurpose {
    /// The decisive query immediately before the move.
    Preflight,
    /// A query after a stop, to name processes at either location.
    Diagnostic,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryResult {
    Report(Box<PathUsageReport>),
    /// The query returned an error. `kind` is `PathUsageError::kind()`.
    Failed { kind: String, message: String },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DiscoveryRun {
    pub at_ms: u64,
    pub purpose: DiscoveryPurpose,
    pub path: NativeString,
    pub result: DiscoveryResult,
}

/// Which watched location an activity observation came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityLocation {
    Original,
    Quarantine,
    /// The worktree's record in the shared Git directory (index, `HEAD`).
    Admin,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ActivityKind {
    Created,
    Modified,
    Replaced,
    /// An approved entry vanished in a way the operation cannot explain.
    Vanished,
    /// The observer lost events or a watch; nothing after this is covered.
    CoverageLost,
}

/// One unexplained change. `received_at_ms` is when the notification
/// arrived, not when the process wrote; no PID is attributed to it.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservedActivity {
    pub location: ActivityLocation,
    pub path: NativeString,
    pub kind: ActivityKind,
    pub received_at_ms: u64,
}

/// Directory entries by type.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EntryCounts {
    pub files: u64,
    pub directories: u64,
    pub links: u64,
}

impl EntryCounts {
    pub fn total(&self) -> u64 {
        self.files + self.directories + self.links
    }
}

/// The deletion journal in counts, by the state recovery resolved.
///
/// The counts are of directory entries removed, not of data destroyed: a
/// hard-linked or still-open file's bytes can outlive its entry.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalSummary {
    /// Removed by this operation, confirmed in the journal.
    pub deleted: EntryCounts,
    /// Already gone when this operation reached them.
    pub externally_absent: EntryCounts,
    /// Reached and deliberately left in place.
    pub kept: EntryCounts,
    /// Planned and still present.
    pub planned: EntryCounts,
    /// Planned and now absent, with no confirmed result: the operation may or
    /// may not have removed them.
    pub uncertain: EntryCounts,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchTarget {
    Local { name: String },
    Remote { remote: String, name: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchState {
    Kept,
    Deleted,
    /// Not run because an earlier step stopped the operation.
    NotAttempted,
    Failed { error: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcome {
    pub target: BranchTarget,
    pub state: BranchState,
}

/// Lists cut short in this report. The journal and the counts stay complete.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Truncation {
    pub activity_omitted: u64,
    pub retained_omitted: u64,
}

/// Everything known about one removal attempt; persisted as `report.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct OperationReport {
    pub report_version: u32,
    /// `None` when the attempt stopped before a recovery record existed.
    pub operation_id: Option<OperationId>,
    pub outcome: RemovalOutcome,
    pub original: NativeString,
    pub original_state: OriginalState,
    pub quarantine: Option<QuarantineReport>,
    pub registration: RegistrationState,
    pub timeline: Vec<TimelineEvent>,
    pub discovery: Vec<DiscoveryRun>,
    pub activity: Vec<ObservedActivity>,
    pub journal: JournalSummary,
    pub branches: Vec<BranchOutcome>,
    pub errors: Vec<String>,
    pub truncation: Truncation,
}

impl OperationReport {
    /// An empty report for `original`, which has not been moved.
    pub fn new(operation_id: Option<OperationId>, outcome: RemovalOutcome, original: NativeString) -> Self {
        Self {
            report_version: REPORT_VERSION,
            operation_id,
            outcome,
            original,
            original_state: OriginalState::Kept,
            quarantine: None,
            registration: RegistrationState::AtOriginal,
            timeline: Vec::new(),
            discovery: Vec::new(),
            activity: Vec::new(),
            journal: JournalSummary::default(),
            branches: Vec::new(),
            errors: Vec::new(),
            truncation: Truncation::default(),
        }
    }

    /// Records `activity`, counting it as omitted past
    /// [`MAX_REPORTED_ACTIVITY`].
    pub fn push_activity(&mut self, activity: ObservedActivity) {
        if self.activity.len() < MAX_REPORTED_ACTIVITY {
            self.activity.push(activity);
        } else {
            self.truncation.activity_omitted += 1;
        }
    }

    /// Records what is left in the quarantine: the exact count, and the
    /// first [`MAX_REPORTED_RETAINED`] entries by name.
    pub fn set_retained(&mut self, entries: Vec<NativeString>) {
        let Some(quarantine) = self.quarantine.as_mut() else { return };
        quarantine.retained_entries = entries.len() as u64;
        self.truncation.retained_omitted = entries.len().saturating_sub(MAX_REPORTED_RETAINED) as u64;
        quarantine.retained = entries.into_iter().take(MAX_REPORTED_RETAINED).collect();
    }

    /// Whether any branch action was attempted and failed.
    pub fn branch_failed(&self) -> bool {
        self.branches.iter().any(|branch| matches!(branch.state, BranchState::Failed { .. }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> FileIdentity {
        FileIdentity::Unix { device: 1, inode: 2 }
    }

    fn stopped_report() -> OperationReport {
        let id = OperationId::parse("0123456789abcdef0123456789abcdef").unwrap();
        let mut report = OperationReport::new(Some(id), RemovalOutcome::ActivityDetected, NativeString::from("/work/fix"));
        report.original_state = OriginalState::Recreated;
        report.registration = RegistrationState::AtQuarantine;
        report.quarantine = Some(QuarantineReport {
            path: NativeString::from("/work/.wt-remove-fix-a91c"),
            identity: identity(),
            state: QuarantineState::Present,
            retained_entries: 0,
            retained: Vec::new(),
        });
        report.timeline.push(TimelineEvent { at_ms: 1, step: Step::Moved, result: StepResult::Succeeded, detail: None });
        report.discovery.push(DiscoveryRun {
            at_ms: 2,
            purpose: DiscoveryPurpose::Diagnostic,
            path: NativeString::from("/work/fix"),
            result: DiscoveryResult::Failed { kind: "root_not_found".into(), message: "gone".into() },
        });
        report.journal.deleted.files = 124;
        report.journal.uncertain.files = 1;
        report.branches.push(BranchOutcome {
            target: BranchTarget::Local { name: "fix/path".into() },
            state: BranchState::NotAttempted,
        });
        report
    }

    #[test]
    fn a_stopped_report_round_trips_through_json() {
        let mut report = stopped_report();
        report.push_activity(ObservedActivity {
            location: ActivityLocation::Original,
            path: NativeString::from("/work/fix/.gitnexus/index.db"),
            kind: ActivityKind::Created,
            received_at_ms: 3,
        });
        let bytes = serde_json::to_vec(&report).unwrap();
        let back: OperationReport = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(back, report);
        let text = String::from_utf8(bytes).unwrap();
        assert!(text.contains("\"outcome\":\"activity_detected\""), "{text}");
        assert!(text.contains("\"registration\":\"at_quarantine\""), "{text}");
    }

    #[cfg(unix)]
    #[test]
    fn a_non_utf8_path_survives_the_report_losslessly() {
        use std::ffi::OsString;
        use std::os::unix::ffi::OsStringExt;
        let mut report = stopped_report();
        let raw = OsString::from_vec(b"/work/fix/\xff\xfe".to_vec());
        report.original = NativeString::from(raw.clone());
        let back: OperationReport = serde_json::from_slice(&serde_json::to_vec(&report).unwrap()).unwrap();
        assert_eq!(back.original.as_os_str(), raw.as_os_str());
    }

    #[test]
    fn activity_past_the_cap_is_counted_not_listed() {
        let mut report = stopped_report();
        for n in 0..MAX_REPORTED_ACTIVITY + 3 {
            report.push_activity(ObservedActivity {
                location: ActivityLocation::Quarantine,
                path: NativeString::from(format!("/q/{n}").as_str()),
                kind: ActivityKind::Modified,
                received_at_ms: n as u64,
            });
        }
        assert_eq!(report.activity.len(), MAX_REPORTED_ACTIVITY);
        assert_eq!(report.truncation.activity_omitted, 3);
    }

    #[test]
    fn retained_entries_keep_an_exact_count_past_the_listing_cap() {
        let mut report = stopped_report();
        let entries: Vec<NativeString> =
            (0..MAX_REPORTED_RETAINED + 5).map(|n| NativeString::from(format!("f{n}").as_str())).collect();
        report.set_retained(entries);
        let quarantine = report.quarantine.as_ref().unwrap();
        assert_eq!(quarantine.retained_entries, (MAX_REPORTED_RETAINED + 5) as u64);
        assert_eq!(quarantine.retained.len(), MAX_REPORTED_RETAINED);
        assert_eq!(report.truncation.retained_omitted, 5);
    }

    #[test]
    fn an_unknown_report_field_is_refused() {
        let mut value = serde_json::to_value(stopped_report()).unwrap();
        value["surprise"] = serde_json::json!(true);
        assert!(serde_json::from_value::<OperationReport>(value).is_err());
    }

    #[test]
    fn a_failed_branch_action_is_visible_on_a_removed_checkout() {
        let mut report = OperationReport::new(None, RemovalOutcome::Removed, NativeString::from("/work/fix"));
        assert!(!report.branch_failed());
        report.branches.push(BranchOutcome {
            target: BranchTarget::Remote { remote: "origin".into(), name: "fix".into() },
            state: BranchState::Failed { error: "rejected".into() },
        });
        assert!(report.branch_failed());
        assert_eq!(report.outcome, RemovalOutcome::Removed);
    }
}

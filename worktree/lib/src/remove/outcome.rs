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

use super::observation::ObservationDiagnostics;
use super::recovery::OperationId;
use super::recovery::journal::JournalEntry;

/// Version of the [`OperationReport`] serialization. Bump it with any change
/// to the persisted shape.
pub const REPORT_VERSION: u32 = 5;

/// The most activity observations a report lists. Later ones are counted in
/// [`Truncation::activity_omitted`]; the deletion journal itself is never
/// truncated.
pub const MAX_REPORTED_ACTIVITY: usize = 200;

/// The most deleted entries [`OperationReport::deleted`] names. The journal
/// summary's counts stay exact, and [`OperationReport::journal_entries`]
/// lists every entry.
pub const MAX_REPORTED_DELETED: usize = 200;

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
    /// A first preflight query that found only Windows Search holding files;
    /// the `preflight` run after it is the one recheck that decided.
    WindowsSearchHold,
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

/// Lists cut short in this report. The counts and
/// [`OperationReport::journal_entries`] stay complete.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Truncation {
    pub activity_omitted: u64,
    pub retained_omitted: u64,
    pub deleted_omitted: u64,
}

/// A statement the report makes about its own limits.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportNote {
    /// Deletion checked each entry and both locations, but a write that lands
    /// between an entry's last check and its unlink is deleted with it, and a
    /// writer can start after the final check. The report never claims that
    /// no writer was active.
    ConcurrentWriteLimit,
}

/// What an earlier attempt of the same operation reported when it stopped,
/// carried into every later report so a retry never replaces it. Its
/// deletions are in [`OperationReport::journal_entries`], not here.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EarlierAttempt {
    pub outcome: RemovalOutcome,
    pub original_state: OriginalState,
    pub quarantine: Option<QuarantineReport>,
    pub registration: RegistrationState,
    pub timeline: Vec<TimelineEvent>,
    pub discovery: Vec<DiscoveryRun>,
    pub activity: Vec<ObservedActivity>,
    /// Activity past [`MAX_REPORTED_ACTIVITY`] that attempt counted.
    pub activity_omitted: u64,
    pub branches: Vec<BranchOutcome>,
    pub errors: Vec<String>,
    /// That attempt's [`OperationReport::observation`].
    pub observation: Option<ObservationDiagnostics>,
}

/// Everything known about one removal operation; persisted as
/// `report.json`. The top-level fields describe the latest attempt; earlier
/// attempts are in [`OperationReport::earlier_attempts`], and the journal
/// counts and entries cover every attempt.
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
    /// Every entry any attempt's journal planned, with its resolved state;
    /// never truncated. The journals themselves are deleted once completion
    /// saves this list.
    pub journal_entries: Vec<JournalEntry>,
    /// Up to [`MAX_REPORTED_DELETED`] entries this operation deleted, as
    /// absolute paths.
    pub deleted: Vec<NativeString>,
    pub notes: Vec<ReportNote>,
    pub branches: Vec<BranchOutcome>,
    pub errors: Vec<String>,
    pub truncation: Truncation,
    /// Earlier attempts that saved a report, oldest first.
    pub earlier_attempts: Vec<EarlierAttempt>,
    /// What this attempt's activity observation measured. `None` means
    /// unknown: the attempt started no observer (it had nothing to delete),
    /// or the report predates version 5, which lacks the field. An explicit
    /// `null` reads the same way; neither is an error.
    pub observation: Option<ObservationDiagnostics>,
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
            journal_entries: Vec::new(),
            deleted: Vec::new(),
            notes: Vec::new(),
            branches: Vec::new(),
            errors: Vec::new(),
            truncation: Truncation::default(),
            earlier_attempts: Vec::new(),
            observation: None,
        }
    }

    /// Carries `earlier`, the report of the attempt this one resumes, into
    /// [`OperationReport::earlier_attempts`] after the attempts it carried.
    pub fn continue_from(&mut self, earlier: OperationReport) {
        self.earlier_attempts.extend(earlier.earlier_attempts);
        self.earlier_attempts.push(EarlierAttempt {
            outcome: earlier.outcome,
            original_state: earlier.original_state,
            quarantine: earlier.quarantine,
            registration: earlier.registration,
            timeline: earlier.timeline,
            discovery: earlier.discovery,
            activity: earlier.activity,
            activity_omitted: earlier.truncation.activity_omitted,
            branches: earlier.branches,
            errors: earlier.errors,
            observation: earlier.observation,
        });
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

    /// Records the entries this operation deleted: the first
    /// [`MAX_REPORTED_DELETED`] by name, the rest counted.
    pub fn set_deleted(&mut self, entries: Vec<NativeString>) {
        self.truncation.deleted_omitted = entries.len().saturating_sub(MAX_REPORTED_DELETED) as u64;
        self.deleted = entries.into_iter().take(MAX_REPORTED_DELETED).collect();
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

    /// A report a version-4 `wt` saved: one earlier attempt, and no
    /// `observation` field anywhere.
    const VERSION_4_REPORT: &str = concat!(
        r#"{"activity":[],"branches":[{"state":"not_attempted","target":{"local":{"name":"fix/path"}}}],"#,
        r#""deleted":[],"discovery":[{"at_ms":2,"path":{"display":"/work/fix"},"purpose":"diagnostic","#,
        r#""result":{"failed":{"kind":"root_not_found","message":"gone"}}}],"earlier_attempts":[{"activity":[],"#,
        r#""activity_omitted":0,"branches":[{"state":"not_attempted","target":{"local":{"name":"fix/path"}}}],"#,
        r#""discovery":[{"at_ms":2,"path":{"display":"/work/fix"},"purpose":"diagnostic","#,
        r#""result":{"failed":{"kind":"root_not_found","message":"gone"}}}],"errors":[],"#,
        r#""original_state":"recreated","outcome":"activity_detected","quarantine":{"identity":{"device":"1","#,
        r#""inode":"2","kind":"unix"},"path":{"display":"/work/.wt-remove-fix-a91c"},"retained":[],"#,
        r#""retained_entries":0,"state":"present"},"registration":"at_quarantine","timeline":[{"at_ms":1,"#,
        r#""detail":null,"result":"succeeded","step":"moved"}]}],"errors":[],"#,
        r#""journal":{"deleted":{"directories":0,"files":124,"links":0},"externally_absent":{"directories":0,"#,
        r#""files":0,"links":0},"kept":{"directories":0,"files":0,"links":0},"planned":{"directories":0,"#,
        r#""files":0,"links":0},"uncertain":{"directories":0,"files":1,"links":0}},"journal_entries":[],"#,
        r#""notes":[],"operation_id":"0123456789abcdef0123456789abcdef","original":{"display":"/work/fix"},"#,
        r#""original_state":"recreated","outcome":"activity_detected","quarantine":{"identity":{"device":"1","#,
        r#""inode":"2","kind":"unix"},"path":{"display":"/work/.wt-remove-fix-a91c"},"retained":[],"#,
        r#""retained_entries":0,"state":"present"},"registration":"at_quarantine","report_version":4,"#,
        r#""timeline":[{"at_ms":1,"detail":null,"result":"succeeded","step":"moved"}],"#,
        r#""truncation":{"activity_omitted":0,"deleted_omitted":0,"retained_omitted":0}}"#,
    );

    #[test]
    fn a_version_4_report_reads_with_unknown_diagnostics_and_keeps_its_earlier_attempt() {
        let report: OperationReport = serde_json::from_str(VERSION_4_REPORT).unwrap();

        assert_eq!((report.report_version, report.observation.as_ref()), (4, None));
        let [earlier] = report.earlier_attempts.as_slice() else { panic!("{:?}", report.earlier_attempts) };
        assert_eq!((earlier.outcome, earlier.observation.as_ref()), (RemovalOutcome::ActivityDetected, None));
        assert_eq!(earlier.discovery, report.discovery);

        // A retry of it carries the attempt it read, still with unknown
        // diagnostics, before its own.
        let mut retry = OperationReport::new(report.operation_id.clone(), RemovalOutcome::Removed, report.original.clone());
        retry.observation = Some(diagnostics());
        retry.continue_from(report);
        let observations: Vec<_> = retry.earlier_attempts.iter().map(|attempt| attempt.observation.clone()).collect();
        assert_eq!(observations, vec![None, None]);
        let back: OperationReport = serde_json::from_slice(&serde_json::to_vec(&retry).unwrap()).unwrap();
        assert_eq!(back, retry);
        assert_eq!(back.report_version, REPORT_VERSION);
    }

    fn diagnostics() -> ObservationDiagnostics {
        serde_json::from_value(serde_json::json!({
            "backend": "fs_events",
            "startup_ms": 1_250,
            "last_round_trip_ms": 40,
            "longest_round_trip_ms": 12_000,
            "loss": {
                "stage": "deletion",
                "reason": { "kind": "fs_events_dropped", "raw": 0x6, "flags": ["kernel_dropped", "user_dropped"] },
                "entries_since_drain": 530,
            },
        }))
        .unwrap()
    }

    /// The `observation` field through the report reader: present (the
    /// control), absent or `null` (both unknown), and the wrong type
    /// (refused). Earlier attempts read it the same way.
    #[test]
    fn the_observation_field_reads_present_absent_null_and_refuses_the_wrong_type() {
        let mut report = stopped_report();
        report.observation = Some(diagnostics());
        let mut earlier = stopped_report();
        earlier.observation = Some(diagnostics());
        report.continue_from(earlier);
        let control = serde_json::to_value(&report).unwrap();
        assert_eq!(serde_json::from_value::<OperationReport>(control.clone()).unwrap(), report);

        for (field, edit) in [
            ("absent", None),
            ("null", Some(serde_json::Value::Null)),
        ] {
            for at in ["top", "earlier"] {
                let mut value = control.clone();
                let object = if at == "top" { &mut value } else { &mut value["earlier_attempts"][0] };
                match &edit {
                    None => drop(object.as_object_mut().unwrap().remove("observation")),
                    Some(null) => object["observation"] = null.clone(),
                }
                let read: OperationReport = serde_json::from_value(value).unwrap();
                let observation = if at == "top" { read.observation } else { read.earlier_attempts[0].observation.clone() };
                assert_eq!(observation, None, "{field} at {at} is unknown");
            }
        }
        for wrong in [serde_json::json!("fs_events"), serde_json::json!({ "backend": 3 })] {
            let mut value = control.clone();
            value["observation"] = wrong.clone();
            assert!(serde_json::from_value::<OperationReport>(value).is_err(), "{wrong}");
        }
        let mut value = control.clone();
        value["observation"]["loss"]["reason"]["surprise"] = serde_json::json!(1);
        assert!(serde_json::from_value::<OperationReport>(value).is_err(), "an unknown loss field is refused");
    }

    /// Every loss reason keeps its details through the report, durations as
    /// whole milliseconds; the generic overflow and the full queue stay
    /// distinct from the macOS payload.
    #[test]
    fn every_loss_reason_round_trips_through_the_report() {
        use crate::remove::observation::{CoverageLost, FsEventsLossFlag, Root};
        use std::time::Duration;
        let reasons = [
            (CoverageLost::Overflow, serde_json::json!({ "kind": "overflow" })),
            (
                CoverageLost::FsEventsDropped { raw: 0x1, flags: vec![FsEventsLossFlag::MustScanSubDirs] },
                serde_json::json!({ "kind": "fs_events_dropped", "raw": 1, "flags": ["must_scan_sub_dirs"] }),
            ),
            (CoverageLost::QueueFull { capacity: 16_384 }, serde_json::json!({ "kind": "queue_full", "capacity": 16_384 })),
            (CoverageLost::WatchLost { root: Root::Admin }, serde_json::json!({ "kind": "watch_lost", "root": "admin" })),
            (CoverageLost::ReadFailed { message: "denied".into() }, serde_json::json!({ "kind": "read_failed", "message": "denied" })),
            (
                CoverageLost::DeliveryStalled { waited: Duration::from_millis(30_000), watch: Some(Root::Parent) },
                serde_json::json!({ "kind": "delivery_stalled", "waited_ms": 30_000, "watch": "parent" }),
            ),
            (
                CoverageLost::DrainCeiling { waited: Duration::from_secs(300), watch: None },
                serde_json::json!({ "kind": "drain_ceiling", "waited_ms": 300_000, "watch": null }),
            ),
        ];
        for (reason, json) in reasons {
            let mut report = stopped_report();
            let mut observation = diagnostics();
            observation.loss.as_mut().unwrap().reason = reason.clone();
            report.observation = Some(observation);
            let value = serde_json::to_value(&report).unwrap();
            assert_eq!(value["observation"]["loss"]["reason"], json, "{reason:?}");
            let back: OperationReport = serde_json::from_value(value).unwrap();
            assert_eq!(back.observation.unwrap().loss.unwrap().reason, reason);
        }
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

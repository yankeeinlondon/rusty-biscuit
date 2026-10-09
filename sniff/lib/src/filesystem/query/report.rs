//! The serialized usage report.
//!
//! Every `Option` field is written as an explicit `null` when unavailable and
//! must be present when read back, so a consumer can tell "unavailable" from
//! a field a writer never knew about.

use super::identity::FileIdentity;
use super::native::{NativeString, decimal, nullable};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Version of the serialized report shape.
pub const SCHEMA_VERSION: u32 = 1;

/// One observation of the processes associated with a file or directory tree.
///
/// Observations are not atomic: a process can start, exit, or change what it
/// holds while the query runs. An empty `processes` list does not establish
/// that the target is unused; read `outcome` and `coverage` first.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PathUsageReport {
    pub schema_version: u32,
    pub target: QueryTarget,
    pub outcome: Outcome,
    pub started_at: DateTime<Utc>,
    pub finished_at: DateTime<Utc>,
    pub budget_ms: u64,
    /// Monotonic duration of the query, independent of wall-clock changes.
    /// An integer so a retained report reads back exactly.
    pub elapsed_us: u64,
    pub budget_exhausted: bool,
    /// Sorted by PID, then creation token.
    pub processes: Vec<ProcessRecord>,
    /// Prerequisites first (process enumeration, tree identity), then each
    /// platform mechanism, including the ones this OS cannot inventory.
    pub coverage: Vec<Coverage>,
    /// Report-wide limitations; mechanism-specific ones live on their
    /// coverage record.
    pub limitations: Vec<Limitation>,
}

/// The queried target as requested and as resolved.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryTarget {
    /// The path exactly as the caller supplied it.
    pub requested: NativeString,
    /// Absolute path with every alias (including a root symlink) resolved.
    pub resolved: NativeString,
    pub kind: TargetKind,
    pub identity: FileIdentity,
    /// Whether a directory's descendants are in scope.
    pub recursive: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetKind {
    File,
    Directory,
}

/// The library-owned verdict on whether the report can be relied on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    /// At least one usage mechanism completed, or finished partially with a
    /// successful inspection or retained evidence. Matches may still be empty.
    Usable,
    /// Usage mechanisms are supported here but every one failed, was not
    /// attempted, or finished partially without a successful inspection.
    Unavailable,
    /// No usage mechanism is supported on this OS or build.
    Unsupported,
}

/// One process lifetime and the evidence linking it to the target.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProcessRecord {
    pub pid: u32,
    /// Native start token at full precision; the only lifetime key that
    /// distinguishes a reused PID.
    #[serde(with = "decimal::option")]
    pub creation_token: Option<u64>,
    /// Presentation start time; never an identity key.
    #[serde(deserialize_with = "nullable")]
    pub start_time: Option<DateTime<Utc>>,
    /// True when no creation token or retained handle ties this record to one
    /// process lifetime, so it is never merged with another observation.
    pub identity_uncertain: bool,
    #[serde(deserialize_with = "nullable")]
    pub name: Option<NativeString>,
    #[serde(deserialize_with = "nullable")]
    pub executable: Option<NativeString>,
    /// Native user identifier (Unix UID or Windows SID), not a user name.
    #[serde(deserialize_with = "nullable")]
    pub user: Option<String>,
    pub evidence: Vec<Evidence>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceKind {
    WatchRegistration,
    OpenHandle,
    WorkingDirectory,
    LoadedModule,
}

/// How an observed object was tied to an in-scope path.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MatchBasis {
    /// The backend supplied the object's identity and it is in the tree.
    Identity,
    /// The backend supplied only a path; it was looked up now. If the object
    /// was replaced since, the lookup names the replacement.
    PathLookup,
}

/// One link between a process and an in-scope object.
///
/// A handle or registration identifies usage; it does not establish that the
/// process blocks deletion or writes to the object.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Evidence {
    pub kind: EvidenceKind,
    pub mechanism: Mechanism,
    /// Every in-scope path naming the object (several for hard links), sorted;
    /// none is preferred.
    pub matched_paths: Vec<NativeString>,
    /// The spelling the backend reported, which may be an alias outside the
    /// tree or descriptive text such as an unlinked file's name.
    #[serde(deserialize_with = "nullable")]
    pub observed_path: Option<NativeString>,
    pub match_basis: MatchBasis,
    #[serde(deserialize_with = "nullable")]
    pub identity: Option<FileIdentity>,
    /// File descriptor or handle value, so distinct registrations are not
    /// collapsed into one path match.
    #[serde(with = "decimal::option")]
    pub descriptor: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub watch: Option<WatchInfo>,
    #[serde(deserialize_with = "nullable")]
    pub access: Option<Access>,
    /// macOS `O_EVTONLY`: a descriptor opened only for event notification.
    /// It remains an open handle, not a proven subscription.
    #[serde(deserialize_with = "nullable")]
    pub event_only: Option<bool>,
}

/// A kernel watch registration.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WatchInfo {
    #[serde(deserialize_with = "nullable")]
    pub watch_id: Option<i64>,
    #[serde(with = "decimal::option")]
    pub mask: Option<u64>,
    /// `Some(false)` for a per-object watch such as inotify; `None` when the
    /// mechanism cannot say. Several per-directory watches do not make a
    /// recursive subscription.
    #[serde(deserialize_with = "nullable")]
    pub recursive: Option<bool>,
}

/// Observed open access. Absence of write access does not prove the process
/// cannot write through another path or handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Access {
    pub read: bool,
    pub write: bool,
}

/// A unit of discovery whose coverage is reported separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Mechanism {
    /// Prerequisite: listing visible processes.
    ProcessEnumeration,
    /// Prerequisite: collecting identities of the target's descendants.
    TreeIdentity,
    /// Open file descriptors (Unix) or handles (Windows).
    OpenHandles,
    WorkingDirectories,
    Inotify,
    Fanotify,
    Fsevents,
    LoadedModules,
    /// Windows `ReadDirectoryChangesW` subscriptions.
    DirectoryChangeSubscriptions,
    /// Watchers that poll and hold no kernel registration.
    Polling,
}

impl Mechanism {
    /// Prerequisites never make an outcome usable on their own.
    pub fn is_prerequisite(self) -> bool {
        matches!(self, Self::ProcessEnumeration | Self::TreeIdentity)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageScope {
    /// Processes the OS exposes to the caller; never a systemwide snapshot.
    VisibleProcesses,
    /// The target object only.
    Target,
    /// The target and its descendants.
    TargetTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageStatus {
    /// Finished over its declared visible-process and path scope with no
    /// known omission. A complete descriptor scan is not complete watcher
    /// discovery.
    Complete,
    /// Started, but permissions, races, malformed records, an incomplete tree,
    /// or the budget left known gaps.
    Partial,
    /// This OS or backend cannot inventory the mechanism; `reason` says why.
    Unsupported,
    /// Attempted, but no inspection succeeded.
    Failed,
    /// Supported but not started; `reason` says why.
    NotAttempted,
}

/// Coverage of one mechanism or prerequisite.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Coverage {
    pub mechanism: Mechanism,
    pub scope: CoverageScope,
    pub status: CoverageStatus,
    /// Inspections attempted (processes for usage mechanisms, entries for the
    /// tree), or `null` when not observable.
    #[serde(deserialize_with = "nullable")]
    pub attempted: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub succeeded: Option<u64>,
    #[serde(deserialize_with = "nullable")]
    pub reason: Option<String>,
    #[serde(deserialize_with = "nullable")]
    pub truncation: Option<Truncation>,
    pub limitations: Vec<Limitation>,
}

/// A retained-evidence cap that was reached.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Truncation {
    pub per_process_limit: u64,
    pub total_limit: u64,
    pub omitted: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LimitationKind {
    PermissionDenied,
    ProcessDisappeared,
    /// Name, executable, or user could not be read or verified.
    IdentityUnavailable,
    /// A record has no lifetime key, so it is kept apart from others.
    IdentityUncertain,
    UnreadableTreeEntry,
    MalformedRecord,
    InspectionFailed,
    BudgetExhausted,
    /// A prerequisite this mechanism depends on was incomplete.
    PrerequisiteIncomplete,
    /// The root was missing or replaced when rechecked.
    RootChanged,
    /// The root's continued identity could not be rechecked.
    RootUnverified,
    Truncated,
    /// A watch registration names an ancestor of the target. It can observe
    /// some events involving the target but is outside the query.
    AncestorWatch,
    /// A descriptor named a different object when re-read during inspection.
    DescriptorReplaced,
    /// A watch names an in-scope inode on a different device; the device
    /// encodings may not be comparable on this filesystem.
    DeviceIdentityUnreliable,
}

/// One kind of gap, aggregated with a count and a few representative examples.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Limitation {
    pub kind: LimitationKind,
    pub message: String,
    pub count: u64,
    pub examples: Vec<LimitationExample>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LimitationExample {
    #[serde(deserialize_with = "nullable")]
    pub pid: Option<u32>,
    #[serde(deserialize_with = "nullable")]
    pub path: Option<NativeString>,
    #[serde(deserialize_with = "nullable")]
    pub detail: Option<String>,
}

impl LimitationExample {
    pub fn pid(pid: u32) -> Self {
        Self {
            pid: Some(pid),
            path: None,
            detail: None,
        }
    }

    pub fn path(path: impl Into<NativeString>) -> Self {
        Self {
            pid: None,
            path: Some(path.into()),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Examples retained per aggregated limitation.
pub(crate) const MAX_LIMITATION_EXAMPLES: usize = 3;

impl Limitation {
    pub(crate) fn new(kind: LimitationKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            count: 0,
            examples: Vec::new(),
        }
    }
}

/// Aggregates limitations sharing a kind and message.
#[derive(Debug, Default)]
pub(crate) struct Limitations(Vec<Limitation>);

impl Limitations {
    pub(crate) fn record(
        &mut self,
        kind: LimitationKind,
        message: impl Into<String>,
        count: u64,
        example: Option<LimitationExample>,
    ) {
        let message = message.into();
        let index = match self
            .0
            .iter()
            .position(|l| l.kind == kind && l.message == message)
        {
            Some(index) => index,
            None => {
                self.0.push(Limitation::new(kind, message));
                self.0.len() - 1
            }
        };
        let entry = &mut self.0[index];
        entry.count += count;
        if let Some(example) = example
            && entry.examples.len() < MAX_LIMITATION_EXAMPLES
        {
            entry.examples.push(example);
        }
    }

    pub(crate) fn merge(&mut self, limitation: Limitation) {
        let Limitation {
            kind,
            message,
            count,
            examples,
        } = limitation;
        let key = message.clone();
        self.record(kind, message, count, None);
        let entry = self
            .0
            .iter_mut()
            .find(|l| l.kind == kind && l.message == key)
            .expect("recorded");
        for example in examples {
            if entry.examples.len() < MAX_LIMITATION_EXAMPLES {
                entry.examples.push(example);
            }
        }
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether any limitation leaves a known gap. A watch on an ancestor is a
    /// scope note: the query's own scope was still inspected completely.
    pub(crate) fn has_gaps(&self) -> bool {
        self.0.iter().any(|l| l.kind != LimitationKind::AncestorWatch)
    }

    pub(crate) fn into_vec(self) -> Vec<Limitation> {
        self.0
    }
}

/// Applies the outcome rule to finished coverage and retained evidence.
pub(crate) fn outcome(coverage: &[Coverage], processes: &[ProcessRecord]) -> Outcome {
    let mut supported = false;
    for record in coverage.iter().filter(|c| !c.mechanism.is_prerequisite()) {
        let usable = match record.status {
            CoverageStatus::Unsupported => continue,
            CoverageStatus::Complete => true,
            CoverageStatus::Partial => {
                record.succeeded.unwrap_or(0) > 0
                    || processes
                        .iter()
                        .flat_map(|p| &p.evidence)
                        .any(|e| e.mechanism == record.mechanism)
            }
            CoverageStatus::Failed | CoverageStatus::NotAttempted => false,
        };
        supported = true;
        if usable {
            return Outcome::Usable;
        }
    }
    if supported {
        Outcome::Unavailable
    } else {
        Outcome::Unsupported
    }
}

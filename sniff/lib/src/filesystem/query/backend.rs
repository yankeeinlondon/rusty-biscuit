//! The seam between the platform-neutral query and an OS backend.
//!
//! A backend lists processes and inspects each one; the query owns matching,
//! merging, enrichment ordering, coverage, and the outcome. Tests drive the
//! query through a fake backend that replays injected observations.

// Until a native backend is built, only the tests' fake backend constructs
// the inspection side of this seam.
#![cfg_attr(not(test), allow(dead_code))]

use super::budget::Budget;
use super::matching::{self, ObjectMatch, ObservedObject};
use super::report::{Access, EvidenceKind, Limitation, Mechanism, WatchInfo};
use super::root::Root;
use super::tree::TreeIndex;
use chrono::{DateTime, Utc};
use std::ffi::OsString;
use std::path::PathBuf;

/// Whether a backend can inventory a mechanism.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Support {
    Supported,
    /// Names the absent capability.
    Unsupported(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct MechanismSupport {
    pub(crate) mechanism: Mechanism,
    pub(crate) support: Support,
}

impl MechanismSupport {
    pub(crate) fn supported(mechanism: Mechanism) -> Self {
        Self {
            mechanism,
            support: Support::Supported,
        }
    }

    pub(crate) fn unsupported(mechanism: Mechanism, reason: impl Into<String>) -> Self {
        Self {
            mechanism,
            support: Support::Unsupported(reason.into()),
        }
    }
}

/// Identity fields a backend read in the same acquisition as the process
/// itself, or that enrichment read later.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct ProcessDetails {
    pub(crate) name: Option<OsString>,
    pub(crate) executable: Option<PathBuf>,
    pub(crate) user: Option<String>,
    pub(crate) start_time: Option<DateTime<Utc>>,
}

/// One enumerated process, as one acquisition.
#[derive(Debug, Clone)]
pub(crate) struct ProcessCandidate<P> {
    pub(crate) pid: u32,
    /// Native creation token at full precision.
    pub(crate) creation_token: Option<u64>,
    /// The backend holds a handle to this exact process lifetime (a process
    /// handle or `/proc/<pid>` directory), so enrichment through it cannot
    /// reach a successor that reused the PID.
    pub(crate) retained_handle: bool,
    pub(crate) details: ProcessDetails,
    pub(crate) payload: P,
}

#[derive(Debug)]
pub(crate) struct Enumeration<P> {
    pub(crate) processes: Vec<ProcessCandidate<P>>,
    /// False when some processes are known to be missing from the list.
    pub(crate) complete: bool,
    pub(crate) limitations: Vec<Limitation>,
}

/// The result of inspecting one mechanism of one process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum InspectionResult {
    Complete,
    /// Some records were usable and some were not; `limitations` says why.
    Partial,
    Denied,
    /// The process exited, or its PID now names a different lifetime.
    Vanished,
    Failed(String),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RawEvidence {
    pub(crate) kind: EvidenceKind,
    pub(crate) object: ObservedObject,
    pub(crate) descriptor: Option<u64>,
    pub(crate) watch: Option<WatchInfo>,
    pub(crate) access: Option<Access>,
    pub(crate) event_only: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MechanismInspection {
    pub(crate) mechanism: Mechanism,
    pub(crate) result: InspectionResult,
    /// Evidence that is not in scope is dropped by the query; a backend may
    /// pre-filter with [`InspectionContext::match_object`] to bound memory.
    pub(crate) evidence: Vec<RawEvidence>,
    pub(crate) limitations: Vec<Limitation>,
}

/// What a backend may consult while inspecting a process.
pub(crate) struct InspectionContext<'a> {
    pub(crate) budget: &'a Budget,
    pub(crate) root: &'a Root,
    pub(crate) index: &'a TreeIndex,
}

impl InspectionContext<'_> {
    pub(crate) fn match_object(&self, object: &ObservedObject) -> Option<ObjectMatch> {
        matching::match_object(object, self.root, self.index, self.budget)
    }
}

/// An OS-specific source of process usage observations.
///
/// ## Notes
///
/// When a backend inspects the querying process itself it must omit exactly
/// the handles it opened for that inspection, and nothing else: the query's
/// genuine usage, including a caller's own open files and cwd, stays in the
/// report.
pub(crate) trait UsageBackend {
    /// Backend-private state carried from enumeration to inspection.
    type Payload;

    /// Every usage mechanism for this OS, including permanently unsupported
    /// ones, in report order. Prerequisites are not listed.
    fn mechanisms(&self) -> Vec<MechanismSupport>;

    /// Lists visible processes once. `Err` names the operational failure.
    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<Self::Payload>, String>;

    /// Inspects one process for every supported mechanism.
    fn inspect(
        &mut self,
        candidate: &ProcessCandidate<Self::Payload>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection>;

    /// Re-reads `pid`'s creation token so enrichment can prove the PID still
    /// names the inspected lifetime.
    fn creation_token(&mut self, pid: u32) -> Option<u64>;

    /// Reads identity fields for a matching process.
    fn details(&mut self, pid: u32) -> Result<ProcessDetails, String> {
        sysinfo_details(pid)
    }
}

/// Reads name, executable, and user for one PID through `sysinfo`, without a
/// full refresh, thread listing, command line, or environment.
pub(crate) fn sysinfo_details(pid: u32) -> Result<ProcessDetails, String> {
    use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};

    let pid = Pid::from_u32(pid);
    let mut system = System::new();
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&[pid]),
        false,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_exe(UpdateKind::Always)
            .with_user(UpdateKind::Always),
    );
    let process = system
        .process(pid)
        .ok_or_else(|| "the process is not visible".to_string())?;
    Ok(ProcessDetails {
        name: Some(process.name().to_owned()),
        executable: process.exe().map(PathBuf::from),
        user: process.user_id().map(|uid| (**uid).to_string()),
        start_time: DateTime::from_timestamp(process.start_time() as i64, 0),
    })
}

#[cfg(any(target_os = "linux", all(test, unix)))]
pub(crate) const FANOTIFY_UNSUPPORTED: &str =
    "fanotify marks of other processes cannot be enumerated in this release";
pub(crate) const POLLING_UNSUPPORTED: &str =
    "polling watchers hold no kernel registration to inventory";

/// The backend for this OS.
#[cfg(target_os = "linux")]
pub(crate) fn platform() -> super::linux::ProcBackend {
    super::linux::ProcBackend::new()
}

/// The backend for this OS.
///
/// No native mechanism is implemented here yet, so every usage mechanism
/// reports `unsupported` and a query's outcome is `unsupported`.
#[cfg(not(target_os = "linux"))]
pub(crate) fn platform() -> PlatformBackend {
    PlatformBackend
}

#[cfg(not(target_os = "linux"))]
pub(crate) struct PlatformBackend;

#[cfg(not(target_os = "linux"))]
const NOT_IMPLEMENTED: &str = "this build of sniff does not implement this mechanism yet";

#[cfg(not(target_os = "linux"))]
impl UsageBackend for PlatformBackend {
    type Payload = ();

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        platform_mechanisms()
    }

    fn enumerate(&mut self, _budget: &Budget) -> Result<Enumeration<()>, String> {
        Err(NOT_IMPLEMENTED.to_string())
    }

    fn inspect(
        &mut self,
        _candidate: &ProcessCandidate<()>,
        _context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        Vec::new()
    }

    fn creation_token(&mut self, _pid: u32) -> Option<u64> {
        None
    }
}

#[cfg(not(target_os = "linux"))]
fn platform_mechanisms() -> Vec<MechanismSupport> {
    use Mechanism::*;
    let polling = MechanismSupport::unsupported(Polling, POLLING_UNSUPPORTED);
    if cfg!(target_os = "macos") {
        vec![
            MechanismSupport::unsupported(OpenHandles, NOT_IMPLEMENTED),
            MechanismSupport::unsupported(WorkingDirectories, NOT_IMPLEMENTED),
            MechanismSupport::unsupported(
                Fsevents,
                "macOS offers no supported systemwide inventory of FSEvents subscriptions",
            ),
            polling,
        ]
    } else if cfg!(windows) {
        vec![
            MechanismSupport::unsupported(OpenHandles, NOT_IMPLEMENTED),
            MechanismSupport::unsupported(LoadedModules, NOT_IMPLEMENTED),
            MechanismSupport::unsupported(
                WorkingDirectories,
                "a working directory is indistinguishable from other directory handles without \
                 reading another process's memory",
            ),
            MechanismSupport::unsupported(
                DirectoryChangeSubscriptions,
                "handle ownership does not identify a ReadDirectoryChangesW subscription",
            ),
            polling,
        ]
    } else {
        vec![polling]
    }
}

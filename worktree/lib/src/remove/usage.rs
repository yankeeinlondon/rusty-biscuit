//! The decisive usage check before a checkout is moved: one fresh
//! [`query_path_usage`] of the checkout, and the removal policy applied to
//! what it found.
//!
//! Any retained evidence of another process (a watch registration, an open
//! handle, a working directory, or a loaded module) refuses with a typed
//! [`UsageRefusal`]; there is no override. No match continues, whatever the
//! coverage: a partial, unavailable, or unsupported report only limits what
//! "no usage observed" means, and [`UsageClearance`] keeps it for the
//! operation report. A query that could not verify the checkout at all
//! refuses before anything is moved.
//!
//! The query uses Sniff's default options: one shared two-second budget, no
//! retry, with one exception. When every holder is one of Windows Search's
//! own processes, identified by its full image path in the system directory
//! ([`is_windows_search`]), the check waits [`SEARCH_RECHECK_WAIT`] and
//! queries once more; that second query decides, whatever it finds. Search
//! opens freshly written files for a few milliseconds to index them, so a
//! query that lands inside such a hold would otherwise refuse a removal no
//! user process is involved in. The recheck is kept in [`SearchRecheck`] for
//! the operation report. A missing checkout has nothing to query and is
//! removed from its record alone; callers never send it here.

use std::collections::BTreeSet;
use std::ffi::OsStr;
use std::path::Path;
use std::time::{Duration, SystemTime};

use sniff::filesystem::query::{
    CoverageStatus, Evidence, EvidenceKind, FileIdentity, Mechanism, NativeString, Outcome,
    PathUsageError, PathUsageOptions, PathUsageReport, ProcessRecord, query_path_usage,
};

/// Where usage reports come from. [`SniffUsage`] queries the host; policy
/// tests inject reports instead, so they never depend on what the host is
/// running.
pub trait UsageSource: Sync {
    fn query(&self, path: &Path) -> Result<PathUsageReport, PathUsageError>;

    /// The system directory (`%SystemRoot%\System32`) that a Windows Search
    /// holder's image must be in for [`check_usage`] to recheck; `None`
    /// never rechecks. The host's on Windows, `None` elsewhere.
    fn system_directory(&self) -> Option<NativeString> {
        host_system_directory()
    }

    /// Waits `wait` before the one recheck.
    fn pause(&self, wait: Duration) {
        std::thread::sleep(wait);
    }
}

/// How long [`check_usage`] waits before its one recheck of a checkout that
/// only Windows Search was holding. Measured on a Windows host on
/// 2026-10-13: Search opened 253 of 3,200 freshly written files, and every
/// hold was gone by the next poll (17 ms at most at 15 ms resolution).
pub const SEARCH_RECHECK_WAIT: Duration = Duration::from_millis(500);

/// Windows Search's processes, by image file name. Only an image in the
/// system directory counts ([`is_windows_search`]).
pub const WINDOWS_SEARCH_IMAGES: [&str; 3] = ["SearchProtocolHost.exe", "SearchFilterHost.exe", "SearchIndexer.exe"];

#[cfg(windows)]
fn host_system_directory() -> Option<NativeString> {
    use std::ffi::OsString;
    use std::os::windows::ffi::OsStringExt;

    use windows_sys::Win32::System::SystemInformation::GetSystemDirectoryW;

    let mut buffer = [0u16; 1024];
    // SAFETY: the buffer is `buffer.len()` writable UTF-16 units.
    let written = unsafe { GetSystemDirectoryW(buffer.as_mut_ptr(), buffer.len() as u32) } as usize;
    // 0 is a failure; a result as long as the buffer is the size it needs.
    (written > 0 && written < buffer.len()).then(|| NativeString::from(OsString::from_wide(&buffer[..written])))
}

#[cfg(not(windows))]
fn host_system_directory() -> Option<NativeString> {
    None
}

/// [`UsageSource`] through Sniff with its default options.
#[derive(Debug, Clone, Copy, Default)]
pub struct SniffUsage;

impl UsageSource for SniffUsage {
    fn query(&self, path: &Path) -> Result<PathUsageReport, PathUsageError> {
        query_path_usage(path, &PathUsageOptions::default())
    }
}

/// Handles this removal opened itself, such as its watcher's descriptors.
///
/// Only a registered handle is excluded from the check, matched by kind and
/// descriptor value under this process's PID. Other evidence for this PID
/// still refuses: if `wt` itself runs from inside the checkout, its loaded
/// executable is real usage.
#[derive(Debug, Clone)]
pub struct OwnedHandles {
    pid: u32,
    handles: BTreeSet<(EvidenceKind, u64)>,
}

impl Default for OwnedHandles {
    fn default() -> Self {
        Self::for_process(std::process::id())
    }
}

impl OwnedHandles {
    /// An empty registry for the process `pid`; [`Default`] uses the current
    /// process.
    pub fn for_process(pid: u32) -> Self {
        Self { pid, handles: BTreeSet::new() }
    }

    /// Registers a handle when the operation creates it, before the query.
    pub fn register(&mut self, kind: EvidenceKind, descriptor: u64) {
        self.handles.insert((kind, descriptor));
    }

    #[cfg(test)]
    pub(crate) fn is_registered(&self, kind: EvidenceKind, descriptor: u64) -> bool {
        self.handles.contains(&(kind, descriptor))
    }

    /// Whether `evidence` is a registered handle of `record`.
    ///
    /// A live process's PID cannot be reused, so a record with this PID is
    /// this process, with two exceptions that are never excluded: a record
    /// Sniff could not tie to one process lifetime, and any record when the
    /// records for this PID disagree on their creation token (`own_token` is
    /// then `Err`).
    fn excludes(&self, record: &ProcessRecord, own_token: Result<Option<u64>, ()>, evidence: &Evidence) -> bool {
        record.pid == self.pid
            && !record.identity_uncertain
            && own_token.is_ok_and(|token| token == record.creation_token)
            && evidence.descriptor.is_some_and(|descriptor| self.handles.contains(&(evidence.kind, descriptor)))
    }

    /// The creation token shared by every certain record for this PID, or
    /// `Err` when they disagree.
    fn own_token(&self, report: &PathUsageReport) -> Result<Option<u64>, ()> {
        let tokens: BTreeSet<Option<u64>> = report
            .processes
            .iter()
            .filter(|record| record.pid == self.pid && !record.identity_uncertain)
            .map(|record| record.creation_token)
            .collect();
        match tokens.len() {
            0 | 1 => Ok(tokens.into_iter().next().flatten()),
            _ => Err(()),
        }
    }
}

/// No other process was observed using the checkout; the removal may move it.
#[derive(Debug, Clone, PartialEq)]
pub struct UsageClearance {
    /// The full report, for the operation report.
    pub report: PathUsageReport,
    /// Evidence records dropped because they were this operation's own
    /// registered handles.
    pub own_handles_excluded: u64,
    pub coverage: CoverageSummary,
    /// Set when the first query found only Windows Search; `report` is then
    /// the recheck's.
    pub search_recheck: Option<SearchRecheck>,
}

/// The one recheck [`check_usage`] made because only Windows Search was
/// holding files in the checkout.
#[derive(Debug, Clone, PartialEq)]
pub struct SearchRecheck {
    /// The first query's report, which found only Windows Search.
    pub first: PathUsageReport,
    /// Evidence records Windows Search held in the first query, one per file
    /// or directory.
    pub held: u64,
    pub waited: Duration,
    /// When the recheck's query started.
    pub rechecked_at: SystemTime,
}

impl SearchRecheck {
    /// One line for the operation report's timeline: what was held, the
    /// wait, and whether the recheck `released` the checkout.
    pub fn summary(&self, released: bool) -> String {
        format!(
            "Windows Search held {} file(s); rechecked after {} ms; {}",
            self.held,
            self.waited.as_millis(),
            if released { "released" } else { "still in use" }
        )
    }
}

/// Why the checkout was not moved. Nothing was changed.
#[derive(Debug, thiserror::Error)]
pub enum UsageRefusal {
    /// Other processes were observed using the checkout.
    #[error("{} is in use by {} process(es)", .0.target.display(), .0.holders.len())]
    InUse(Box<InUse>),
    /// The query could not verify the checkout, so nothing it reports could
    /// be trusted to describe it.
    #[error("the checkout could not be verified before removal: {0}")]
    Unverified(#[source] PathUsageError),
}

/// The processes using the checkout.
#[derive(Debug, Clone, PartialEq)]
pub struct InUse {
    /// The checkout as Sniff resolved it.
    pub target: NativeString,
    /// In report order (by PID, then creation token).
    pub holders: Vec<Holder>,
    pub coverage: CoverageSummary,
    pub report: PathUsageReport,
    /// Set when this refusal is the recheck's, after a first query found
    /// only Windows Search.
    pub search_recheck: Option<SearchRecheck>,
}

/// One process lifetime using the checkout.
#[derive(Debug, Clone, PartialEq)]
pub struct Holder {
    pub pid: u32,
    pub creation_token: Option<u64>,
    /// Sniff could not tie this record to one process lifetime. Its name, if
    /// any, came with the evidence itself; none is looked up afterward, so a
    /// reused PID never lends its name.
    pub identity_uncertain: bool,
    pub name: Option<NativeString>,
    pub executable: Option<NativeString>,
    pub evidence: Vec<HolderEvidence>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HolderEvidence {
    pub kind: EvidenceKind,
    pub location: EvidenceLocation,
}

/// Which object in the checkout the evidence is about.
#[derive(Debug, Clone, PartialEq)]
pub enum EvidenceLocation {
    /// Exactly one path in the checkout names it.
    Path(NativeString),
    /// Several paths in the checkout name it (hard links); none is the
    /// preferred one.
    Aliases(Vec<NativeString>),
    /// No path in the checkout names it any longer: it is known only by its
    /// identity, and by the spelling the process used when Sniff had one.
    Unnamed { identity: Option<FileIdentity>, observed: Option<NativeString> },
}

/// How much of the host the query could see.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageSummary {
    pub outcome: Outcome,
    pub budget_exhausted: bool,
    /// Every mechanism and prerequisite that did not complete, in report
    /// order.
    pub gaps: Vec<CoverageGap>,
    /// Evidence records Sniff counted but did not retain.
    pub evidence_omitted: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CoverageGap {
    pub mechanism: Mechanism,
    pub status: CoverageStatus,
    pub reason: Option<String>,
}

impl CoverageSummary {
    pub fn of(report: &PathUsageReport) -> Self {
        Self {
            outcome: report.outcome,
            budget_exhausted: report.budget_exhausted,
            gaps: report
                .coverage
                .iter()
                .filter(|coverage| coverage.status != CoverageStatus::Complete)
                .map(|coverage| CoverageGap {
                    mechanism: coverage.mechanism,
                    status: coverage.status,
                    reason: coverage.reason.clone(),
                })
                .collect(),
            evidence_omitted: report.coverage.iter().filter_map(|c| c.truncation.as_ref()).map(|t| t.omitted).sum(),
        }
    }

    /// Every mechanism completed and nothing was omitted.
    pub fn is_complete(&self) -> bool {
        self.outcome == Outcome::Usable && !self.budget_exhausted && self.gaps.is_empty() && self.evidence_omitted == 0
    }
}

/// Queries `checkout` through `source` and applies [`assess`]. When every
/// holder [`is_windows_search`], it waits [`SEARCH_RECHECK_WAIT`] and
/// queries exactly once more, and that query decides; any other holder in
/// the first query refuses at once.
///
/// ## Errors
///
/// [`UsageRefusal::Unverified`] for any [`PathUsageError`];
/// [`UsageRefusal::InUse`] as [`assess`] decides.
pub fn check_usage(source: &dyn UsageSource, checkout: &Path, own: &OwnedHandles) -> Result<UsageClearance, UsageRefusal> {
    let report = source.query(checkout).map_err(UsageRefusal::Unverified)?;
    let in_use = match assess(report, own) {
        Err(UsageRefusal::InUse(in_use)) => in_use,
        decided => return decided,
    };
    let Some(held) = source.system_directory().and_then(|system| windows_search_only(&in_use.holders, &system)) else {
        return Err(UsageRefusal::InUse(in_use));
    };
    source.pause(SEARCH_RECHECK_WAIT);
    let recheck = SearchRecheck { first: in_use.report, held, waited: SEARCH_RECHECK_WAIT, rechecked_at: SystemTime::now() };
    let report = source.query(checkout).map_err(UsageRefusal::Unverified)?;
    match assess(report, own) {
        Ok(mut clearance) => {
            clearance.search_recheck = Some(recheck);
            Ok(clearance)
        }
        Err(UsageRefusal::InUse(mut in_use)) => {
            in_use.search_recheck = Some(recheck);
            Err(UsageRefusal::InUse(in_use))
        }
        Err(unverified) => Err(unverified),
    }
}

/// The evidence count when `holders` is not empty and every one
/// [`is_windows_search`] in `system_directory`.
fn windows_search_only(holders: &[Holder], system_directory: &NativeString) -> Option<u64> {
    let all = !holders.is_empty() && holders.iter().all(|holder| is_windows_search(holder, system_directory));
    all.then(|| holders.iter().map(|holder| holder.evidence.len() as u64).sum())
}

/// Whether `holder` is one of [`WINDOWS_SEARCH_IMAGES`] running from
/// `system_directory` itself, by the full image path Sniff read from the
/// process. A holder without an image path, one Sniff could not tie to one
/// process lifetime, or a same-named image anywhere else is an ordinary
/// holder. Compared ASCII-case-insensitively, as Windows spells the system
/// directory in either case (`C:\WINDOWS\System32`).
pub fn is_windows_search(holder: &Holder, system_directory: &NativeString) -> bool {
    !holder.identity_uncertain
        && holder.executable.as_ref().is_some_and(|image| is_search_image(image.as_os_str(), system_directory.as_os_str()))
}

fn is_search_image(image: &OsStr, system_directory: &OsStr) -> bool {
    // A path that is not Unicode cannot be one of these images.
    let (Some(image), Some(directory)) = (image.to_str(), system_directory.to_str()) else {
        return false;
    };
    let directory = without_verbatim(directory).trim_end_matches('\\');
    let Some((parent, file)) = without_verbatim(image).rsplit_once('\\') else {
        return false;
    };
    !directory.is_empty()
        && parent.eq_ignore_ascii_case(directory)
        && WINDOWS_SEARCH_IMAGES.iter().any(|name| file.eq_ignore_ascii_case(name))
}

fn without_verbatim(path: &str) -> &str {
    path.strip_prefix(r"\\?\").unwrap_or(path)
}

/// The removal policy for one report: refuse on any evidence that is not one
/// of `own`'s registered handles, otherwise clear, whatever the coverage.
///
/// ## Errors
///
/// [`UsageRefusal::InUse`] naming every process with remaining evidence.
pub fn assess(report: PathUsageReport, own: &OwnedHandles) -> Result<UsageClearance, UsageRefusal> {
    let own_token = own.own_token(&report);
    let mut excluded = 0;
    let mut holders = Vec::new();
    for record in &report.processes {
        let mut evidence = Vec::new();
        for item in &record.evidence {
            if own.excludes(record, own_token, item) {
                excluded += 1;
            } else {
                evidence.push(HolderEvidence { kind: item.kind, location: location(item) });
            }
        }
        if !evidence.is_empty() {
            holders.push(Holder {
                pid: record.pid,
                creation_token: record.creation_token,
                identity_uncertain: record.identity_uncertain,
                name: record.name.clone(),
                executable: record.executable.clone(),
                evidence,
            });
        }
    }
    let coverage = CoverageSummary::of(&report);
    if holders.is_empty() {
        return Ok(UsageClearance { report, own_handles_excluded: excluded, coverage, search_recheck: None });
    }
    Err(UsageRefusal::InUse(Box::new(InUse {
        target: report.target.resolved.clone(),
        holders,
        coverage,
        report,
        search_recheck: None,
    })))
}

fn location(evidence: &Evidence) -> EvidenceLocation {
    match evidence.matched_paths.as_slice() {
        [] => EvidenceLocation::Unnamed { identity: evidence.identity, observed: evidence.observed_path.clone() },
        [path] => EvidenceLocation::Path(path.clone()),
        paths => EvidenceLocation::Aliases(paths.to_vec()),
    }
}

#[cfg(test)]
mod tests;

//! Which processes are using a file or directory tree.
//!
//! [`query_path_usage`] is a focused, read-only, request-scoped query. It
//! performs no host or repository inventory, network request, privilege
//! elevation, process termination, or caching, and it never leaves a worker
//! running after it returns. Discovery is advisory: an open handle, working
//! directory, or watch registration does not prove that deletion will fail,
//! and an empty result does not prove that nothing is using the target.
//!
//! See `sniff/docs/topics/filesystem-query.md` for the report contract.

mod backend;
mod budget;
mod identity;
// The `/proc` reader is pure `std`, so Unix test builds also exercise it
// against a synthetic `/proc` tree.
#[cfg(any(target_os = "linux", all(test, unix)))]
mod linux;
// The classification logic runs against a scripted source in Unix test
// builds; the `libproc` calls themselves are macOS-only.
#[cfg(any(target_os = "macos", all(test, unix)))]
mod macos;
mod matching;
mod native;
mod options;
mod process;
mod report;
mod root;
mod tree;
// The module classification runs against a scripted source in every test
// build; the Win32 calls themselves are Windows-only.
#[cfg(any(windows, test))]
mod win32;
#[cfg(any(windows, test))]
mod win32_paths;

#[cfg(test)]
mod tests;
#[cfg(all(test, unix))]
mod linux_tests;
#[cfg(all(test, unix))]
mod macos_tests;
#[cfg(test)]
mod win32_tests;

pub use identity::FileIdentity;
pub use native::NativeString;
pub use options::{DEFAULT_DEADLINE, PathUsageError, PathUsageOptions};
pub use report::{
    Access, Coverage, CoverageScope, CoverageStatus, Evidence, EvidenceKind, IndexProvenance,
    Limitation, LimitationExample, LimitationKind, MatchBasis, Mechanism, Outcome,
    PathUsageReport, ProcessRecord, QueryTarget, SCHEMA_VERSION, TargetKind, Truncation,
    WatchInfo,
};
pub use tree::{TreeEntryKind, TreeIndex, TreeIndexBuilder, TreeIndexError};

use backend::{
    InspectionContext, InspectionResult, MechanismSupport, ProcessCandidate, Support,
    UsageBackend,
};
use budget::Budget;
use chrono::{DateTime, Utc};
use process::Observation;
use report::Limitations;
use root::{Recheck, Root};
use std::path::Path;

use crate::performance::{self, counters};

/// Evidence records retained per process before the rest are counted as
/// omitted.
pub const MAX_EVIDENCE_PER_PROCESS: u64 = 1_000;

/// Evidence records retained per query before the rest are counted as omitted.
pub const MAX_EVIDENCE_TOTAL: u64 = 10_000;

/// Reports the processes associated with `path` and, for a directory, its
/// descendants (unless [`PathUsageOptions::target_only`] is set).
///
/// The options' deadline is one shared budget for root resolution, the tree
/// walk, process enumeration, inspection, and identity enrichment. A caller
/// that already walked the tree can supply its index with
/// [`PathUsageOptions::with_tree_index`]; the query then skips the walk,
/// still verifying the root, and spends the budget on processes. When it
/// expires the query stops scheduling work and returns what it collected,
/// with incomplete coverage; a native call already running can overrun it.
///
/// ## Examples
///
/// ```no_run
/// use sniff::filesystem::query::{Outcome, PathUsageOptions, query_path_usage};
/// use std::path::Path;
///
/// let report = query_path_usage(Path::new("."), &PathUsageOptions::default())?;
/// if report.outcome == Outcome::Usable {
///     for process in &report.processes {
///         println!("{} holds {} object(s)", process.pid, process.evidence.len());
///     }
/// }
/// # Ok::<(), sniff::filesystem::query::PathUsageError>(())
/// ```
///
/// ## Errors
///
/// Returns [`PathUsageError`] only when no report about a verified target can
/// be built: invalid options (including a supplied tree index combined with
/// `target_only`, given for a file, or describing a different root), a
/// missing target, a socket/device/FIFO target, a
/// root whose identity cannot be read, or a budget that expired before the
/// root identity was captured. Every later failure is reported in coverage.
pub fn query_path_usage(
    path: &Path,
    options: &PathUsageOptions,
) -> Result<PathUsageReport, PathUsageError> {
    let started_at = Utc::now();
    let budget = Budget::start(options.deadline())?;
    run(path, options, &mut backend::platform(), &budget, started_at)
}

/// Per-mechanism tallies across every inspected process.
struct Tally {
    support: MechanismSupport,
    attempted: u64,
    succeeded: u64,
    partial: u64,
    omitted: u64,
    /// Inspected processes for which the budget expired before this
    /// mechanism started.
    not_attempted: u64,
    limitations: Limitations,
}

/// Why usage mechanisms could not run at all, when they could not.
enum NotStarted {
    Budget,
    NoSupportedMechanism,
    EnumerationFailed,
}

pub(crate) fn run<B: UsageBackend>(
    path: &Path,
    options: &PathUsageOptions,
    backend: &mut B,
    budget: &Budget,
    started_at: DateTime<Utc>,
) -> Result<PathUsageReport, PathUsageError> {
    if options.tree_index().is_some() && !options.recursive() {
        return Err(PathUsageError::InvalidOption {
            message: "a tree index describes descendants, so it cannot be combined with target_only"
                .to_string(),
        });
    }
    let root = root::resolve(path, budget)?;
    let tree = match options.tree_index() {
        Some(index) => tree::supplied(&root, index)?,
        None => tree::collect(&root, options.recursive(), budget),
    };
    let tree_partial = tree
        .coverage
        .as_ref()
        .is_some_and(|c| c.status != CoverageStatus::Complete);

    let mut tallies: Vec<Tally> = backend
        .mechanisms()
        .into_iter()
        .map(|support| Tally {
            support,
            attempted: 0,
            succeeded: 0,
            partial: 0,
            omitted: 0,
            not_attempted: 0,
            limitations: Limitations::default(),
        })
        .collect();
    let any_supported = tallies.iter().any(|t| t.support.support == Support::Supported);

    let (enumeration_coverage, candidates, enumeration_complete, not_started) =
        enumerate(backend, budget, any_supported);

    let mut observations = Vec::new();
    let mut skipped = 0u64;
    let mut retained_total = 0u64;
    let context = InspectionContext {
        budget,
        root: &root,
        index: &tree.index,
    };
    let mut retained_by_lifetime = std::collections::HashMap::new();
    let path_inspection = backend.inspect_paths(&context, &candidates);
    let path_coverage = path_inspection.map(|(path_observations, coverage)| {
        for observation in &path_observations {
            let count = observation.evidence.len() as u64;
            retained_total += count;
            if let Some(token) = observation.creation_token {
                *retained_by_lifetime.entry((observation.pid, token)).or_insert(0) += count;
            }
        }
        observations.extend(path_observations);
        coverage
    });
    for (position, candidate) in candidates.iter().enumerate() {
        if budget.expired() {
            skipped = (candidates.len() - position) as u64;
            break;
        }
        performance::increment_counter(counters::QUERY_PROCESS_INSPECTIONS, 1);
        let retained_for_process = candidate.creation_token
            .and_then(|token| retained_by_lifetime.get(&(candidate.pid, token)).copied())
            .unwrap_or(0);
        let evidence = inspect(backend, candidate, &context, &mut tallies, &mut retained_total, retained_for_process);
        if !evidence.is_empty() {
            if let Some(token) = candidate.creation_token {
                *retained_by_lifetime.entry((candidate.pid, token)).or_insert(0) += evidence.len() as u64;
            }
            observations.push(Observation {
                pid: candidate.pid,
                creation_token: candidate.creation_token,
                retained_handle: candidate.retained_handle,
                details: candidate.details.clone(),
                evidence,
            });
        }
    }
    drop(candidates);

    let mut report_limitations = Limitations::default();
    let root_note = match root::recheck(&root, budget) {
        Recheck::Verified => None,
        Recheck::Changed(detail) => {
            report_limitations.record(
                LimitationKind::RootChanged,
                "the target was missing or replaced when rechecked; evidence was kept",
                1,
                Some(LimitationExample::path(root.resolved.as_path()).with_detail(detail)),
            );
            Some("the target changed during the query")
        }
        Recheck::Unverified(detail) => {
            report_limitations.record(
                LimitationKind::RootUnverified,
                "the target's continued identity could not be verified",
                1,
                Some(LimitationExample::path(root.resolved.as_path()).with_detail(detail)),
            );
            Some("the target's continued identity could not be verified")
        }
    };

    let mut observations = process::merge(observations);
    process::enrich(&mut observations, backend, budget, &mut report_limitations);
    let processes = process::into_records(observations);

    let scope = if root.kind == TargetKind::Directory && options.recursive() {
        CoverageScope::TargetTree
    } else {
        CoverageScope::Target
    };
    let gaps = PrerequisiteGaps {
        enumeration: !enumeration_complete,
        tree: tree_partial,
        root: root_note,
    };
    let mut coverage = vec![enumeration_coverage];
    coverage.extend(tree.coverage);
    for tally in tallies {
        if path_coverage.as_ref().is_some_and(|c| c.mechanism == tally.support.mechanism) {
            let mut paths = path_coverage.clone().expect("path coverage");
            paths.scope = scope;
            if paths.status != CoverageStatus::NotAttempted && (gaps.tree || gaps.root.is_some()) {
                if paths.status == CoverageStatus::Complete {
                    paths.status = CoverageStatus::Partial;
                }
                paths.limitations.push(Limitation {
                    kind: LimitationKind::PrerequisiteIncomplete,
                    message: "tree collection or root verification was incomplete".to_string(),
                    count: 1, examples: Vec::new(),
                });
            }
            coverage.push(paths);
        } else {
            coverage.push(finish(tally, scope, &not_started, skipped, &gaps));
        }
    }

    let outcome = report::outcome(&coverage, &processes);
    Ok(PathUsageReport {
        schema_version: SCHEMA_VERSION,
        target: target(&root, options.recursive()),
        index_provenance: tree.provenance,
        outcome,
        started_at,
        finished_at: Utc::now(),
        budget_ms: u64::try_from(budget.limit().as_millis()).unwrap_or(u64::MAX),
        elapsed_us: u64::try_from(budget.elapsed().as_micros()).unwrap_or(u64::MAX),
        budget_exhausted: budget.exhausted(),
        processes,
        coverage,
        limitations: report_limitations.into_vec(),
    })
}

fn target(root: &Root, recursive: bool) -> QueryTarget {
    QueryTarget {
        requested: root.requested.as_path().into(),
        resolved: root.resolved.as_path().into(),
        kind: root.kind,
        identity: root.identity,
        recursive: root.kind == TargetKind::Directory && recursive,
    }
}

type Enumerated<P> = (
    Coverage,
    Vec<ProcessCandidate<P>>,
    bool,
    Option<NotStarted>,
);

fn enumerate<B: UsageBackend>(
    backend: &mut B,
    budget: &Budget,
    any_supported: bool,
) -> Enumerated<B::Payload> {
    let coverage = |status, attempted, reason: Option<&str>, limitations| Coverage {
        mechanism: Mechanism::ProcessEnumeration,
        scope: CoverageScope::VisibleProcesses,
        status,
        attempted,
        succeeded: attempted,
        reason: reason.map(str::to_string),
        truncation: None,
        limitations,
    };
    if !any_supported {
        let reason = "no usage mechanism is supported on this OS or build";
        return (
            coverage(CoverageStatus::NotAttempted, None, Some(reason), Vec::new()),
            Vec::new(),
            false,
            Some(NotStarted::NoSupportedMechanism),
        );
    }
    if budget.expired() {
        let reason = "the budget expired before processes were enumerated";
        return (
            coverage(CoverageStatus::NotAttempted, None, Some(reason), Vec::new()),
            Vec::new(),
            false,
            Some(NotStarted::Budget),
        );
    }
    performance::increment_counter(counters::QUERY_PROCESS_ENUMERATIONS, 1);
    match backend.enumerate(budget) {
        Ok(enumeration) => {
            let status = if enumeration.complete && enumeration.limitations.is_empty() {
                CoverageStatus::Complete
            } else {
                CoverageStatus::Partial
            };
            let complete = status == CoverageStatus::Complete;
            let count = Some(enumeration.processes.len() as u64);
            (
                coverage(status, count, None, enumeration.limitations),
                enumeration.processes,
                complete,
                None,
            )
        }
        Err(reason) => (
            coverage(CoverageStatus::Failed, None, Some(&reason), Vec::new()),
            Vec::new(),
            false,
            Some(NotStarted::EnumerationFailed),
        ),
    }
}

/// Runs one process inspection, tallying every result and keeping only
/// in-scope evidence within the retention caps.
fn inspect<B: UsageBackend>(
    backend: &mut B,
    candidate: &ProcessCandidate<B::Payload>,
    context: &InspectionContext<'_>,
    tallies: &mut [Tally],
    retained_total: &mut u64,
    retained_for_process: u64,
) -> Vec<Evidence> {
    let mut evidence = Vec::new();
    for inspection in backend.inspect(candidate, context) {
        let Some(tally) = tallies.iter_mut().find(|t| {
            t.support.mechanism == inspection.mechanism && t.support.support == Support::Supported
        }) else {
            continue;
        };
        let example = LimitationExample::pid(candidate.pid);
        match inspection.result {
            // Carries no evidence or limitations.
            InspectionResult::NotAttempted => {
                tally.not_attempted += 1;
                continue;
            }
            InspectionResult::Complete => tally.succeeded += 1,
            InspectionResult::Partial => {
                tally.succeeded += 1;
                tally.partial += 1;
            }
            InspectionResult::Denied => tally.limitations.record(
                LimitationKind::PermissionDenied,
                "permission was denied for some processes",
                1,
                Some(example),
            ),
            InspectionResult::Vanished => tally.limitations.record(
                LimitationKind::ProcessDisappeared,
                "some processes exited before they were inspected",
                1,
                Some(example),
            ),
            InspectionResult::Failed(detail) => tally.limitations.record(
                LimitationKind::InspectionFailed,
                "inspection failed for some processes",
                1,
                Some(example.with_detail(detail)),
            ),
        }
        tally.attempted += 1;
        for limitation in inspection.limitations {
            tally.limitations.merge(limitation);
        }
        for mut raw in inspection.evidence {
            debug_assert!(
                raw.matched.is_some() || raw.object.identity.is_some(),
                "a backend must validate a path-only observation while its budget remains"
            );
            // Never a path lookup here: the budget may have expired since the
            // backend read this, and a refused lookup would discard it.
            let found = match raw.matched.take() {
                Some(found) => found,
                None if raw.object.identity.is_some() => {
                    match context.match_object(&raw.object) {
                        Some(found) => found,
                        None => continue,
                    }
                }
                None => continue,
            };
            if evidence.len() as u64 + retained_for_process >= MAX_EVIDENCE_PER_PROCESS
                || *retained_total >= MAX_EVIDENCE_TOTAL
            {
                tally.omitted += 1;
                continue;
            }
            *retained_total += 1;
            evidence.push(Evidence {
                kind: raw.kind,
                mechanism: inspection.mechanism,
                matched_paths: found.matched_paths.into_iter().map(Into::into).collect(),
                observed_path: raw.object.path.map(Into::into),
                match_basis: found.basis,
                identity: Some(found.identity),
                descriptor: raw.descriptor,
                watch: raw.watch,
                access: raw.access,
                event_only: raw.event_only,
            });
        }
    }
    evidence
}

struct PrerequisiteGaps {
    enumeration: bool,
    tree: bool,
    root: Option<&'static str>,
}

/// Decides one mechanism's status from its tallies and the shared gaps.
fn finish(
    tally: Tally,
    scope: CoverageScope,
    not_started: &Option<NotStarted>,
    skipped: u64,
    gaps: &PrerequisiteGaps,
) -> Coverage {
    let Tally {
        support,
        attempted,
        succeeded,
        partial,
        omitted,
        not_attempted,
        mut limitations,
    } = tally;
    let mut coverage = Coverage {
        mechanism: support.mechanism,
        scope,
        status: CoverageStatus::Complete,
        attempted: None,
        succeeded: None,
        reason: None,
        truncation: None,
        limitations: Vec::new(),
    };
    if let Support::Unsupported(reason) = support.support {
        coverage.status = CoverageStatus::Unsupported;
        coverage.reason = Some(reason);
        return coverage;
    }
    if let Some(not_started) = not_started {
        coverage.status = CoverageStatus::NotAttempted;
        coverage.reason = Some(
            match not_started {
                NotStarted::Budget => "the budget expired before inspection started",
                NotStarted::NoSupportedMechanism => "no usage mechanism is supported",
                NotStarted::EnumerationFailed => "process enumeration failed",
            }
            .to_string(),
        );
        return coverage;
    }
    coverage.attempted = Some(attempted);
    coverage.succeeded = Some(succeeded);
    if not_attempted > 0 {
        limitations.record(
            LimitationKind::BudgetExhausted,
            "the budget expired before this mechanism was started for some processes",
            not_attempted,
            None,
        );
    }
    if attempted == 0 && (skipped > 0 || not_attempted > 0) {
        coverage.status = CoverageStatus::NotAttempted;
        coverage.reason = Some("the budget expired before inspection started".to_string());
        coverage.limitations = limitations.into_vec();
        return coverage;
    }
    if skipped > 0 {
        limitations.record(
            LimitationKind::BudgetExhausted,
            "the budget expired before every process was inspected",
            skipped,
            None,
        );
    }
    if omitted > 0 {
        limitations.record(
            LimitationKind::Truncated,
            "evidence beyond the retention cap was omitted",
            omitted,
            None,
        );
        coverage.truncation = Some(Truncation {
            per_process_limit: MAX_EVIDENCE_PER_PROCESS,
            total_limit: MAX_EVIDENCE_TOTAL,
            omitted,
        });
    }
    if gaps.enumeration {
        limitations.record(
            LimitationKind::PrerequisiteIncomplete,
            "process enumeration was partial",
            1,
            None,
        );
    }
    if gaps.tree {
        limitations.record(
            LimitationKind::PrerequisiteIncomplete,
            "tree identity collection was partial",
            1,
            None,
        );
    }
    if let Some(note) = gaps.root {
        limitations.record(LimitationKind::PrerequisiteIncomplete, note, 1, None);
    }
    coverage.status = if attempted > 0 && succeeded == 0 {
        CoverageStatus::Failed
    } else if partial > 0 || succeeded < attempted || limitations.has_gaps() {
        CoverageStatus::Partial
    } else {
        CoverageStatus::Complete
    };
    coverage.limitations = limitations.into_vec();
    coverage
}

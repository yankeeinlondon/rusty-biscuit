//! Platform-neutral query behavior, driven through a fake backend that
//! replays injected observations against real temporary trees.

use super::backend::{
    Enumeration, InspectionContext, InspectionResult, MechanismInspection, MechanismSupport,
    ProcessCandidate, ProcessDetails, RawEvidence, UsageBackend,
};
use super::budget::Budget;
use super::identity::{self, FileIdentity};
use super::matching::{ObjectMatch, ObservedObject};
use super::*;
use crate::performance::testing::measure;
use std::collections::HashMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;
use tempfile::TempDir;

type Script = Vec<MechanismInspection>;
type Hook = Box<dyn FnMut(u32, &InspectionContext<'_>)>;
type EnumerateHook = Box<dyn FnMut(&Budget)>;

struct Fake {
    mechanisms: Vec<MechanismSupport>,
    enumeration: Option<Result<Enumeration<Script>, String>>,
    /// Creation token returned on re-read; absent means the PID is gone.
    tokens: HashMap<u32, u64>,
    details: HashMap<u32, Result<ProcessDetails, String>>,
    details_calls: Vec<u32>,
    inspected: Vec<u32>,
    on_inspect: Option<Hook>,
    on_enumerate: Option<EnumerateHook>,
}

impl Fake {
    fn new(processes: Vec<ProcessCandidate<Script>>) -> Self {
        let tokens = processes
            .iter()
            .filter_map(|p| p.creation_token.map(|t| (p.pid, t)))
            .collect();
        Self {
            mechanisms: vec![
                MechanismSupport::supported(Mechanism::OpenHandles),
                MechanismSupport::supported(Mechanism::WorkingDirectories),
                MechanismSupport::unsupported(Mechanism::Fsevents, "no inventory API"),
            ],
            enumeration: Some(Ok(Enumeration {
                processes,
                complete: true,
                limitations: Vec::new(),
            })),
            tokens,
            details: HashMap::new(),
            details_calls: Vec::new(),
            inspected: Vec::new(),
            on_inspect: None,
            on_enumerate: None,
        }
    }

    fn failing_enumeration(reason: &str) -> Self {
        let mut fake = Self::new(Vec::new());
        fake.enumeration = Some(Err(reason.to_string()));
        fake
    }
}

impl UsageBackend for Fake {
    type Payload = Script;

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        self.mechanisms.clone()
    }

    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<Script>, String> {
        if let Some(hook) = self.on_enumerate.as_mut() {
            hook(budget);
        }
        self.enumeration.take().expect("enumerated once")
    }

    fn inspect(
        &mut self,
        candidate: &ProcessCandidate<Script>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        self.inspected.push(candidate.pid);
        if let Some(hook) = self.on_inspect.as_mut() {
            hook(candidate.pid, context);
        }
        // Like a real backend, validate path-only evidence during inspection;
        // the query makes no path lookup of its own.
        let mut inspections = candidate.payload.clone();
        for inspection in &mut inspections {
            inspection.evidence.retain_mut(|raw| {
                if raw.object.identity.is_some() || raw.matched.is_some() {
                    return true;
                }
                raw.matched = context.match_object(&raw.object);
                raw.matched.is_some()
            });
        }
        inspections
    }

    fn creation_token(&mut self, pid: u32) -> Option<u64> {
        self.tokens.get(&pid).copied()
    }

    fn details(&mut self, pid: u32) -> Result<ProcessDetails, String> {
        self.details_calls.push(pid);
        self.details
            .get(&pid)
            .cloned()
            .unwrap_or_else(|| Ok(named(&format!("proc-{pid}"))))
    }
}

/// Delegates to a real backend, letting the first process inspection make
/// `checks` budget checks before the budget expires, so a full query can
/// expire partway through one process's inspection.
pub(super) struct ExpiringDuringInspection<B> {
    pub(super) inner: B,
    checks: Option<i64>,
}

impl<B> ExpiringDuringInspection<B> {
    pub(super) fn new(inner: B, checks: i64) -> Self {
        Self {
            inner,
            checks: Some(checks),
        }
    }
}

impl<B: UsageBackend> UsageBackend for ExpiringDuringInspection<B> {
    type Payload = B::Payload;

    fn mechanisms(&self) -> Vec<MechanismSupport> {
        self.inner.mechanisms()
    }

    fn enumerate(&mut self, budget: &Budget) -> Result<Enumeration<B::Payload>, String> {
        self.inner.enumerate(budget)
    }

    fn inspect(
        &mut self,
        candidate: &ProcessCandidate<B::Payload>,
        context: &InspectionContext<'_>,
    ) -> Vec<MechanismInspection> {
        if let Some(checks) = self.checks.take() {
            context.budget.expire_after_further_checks(checks);
        }
        self.inner.inspect(candidate, context)
    }

    fn inspect_paths(
        &mut self,
        context: &InspectionContext<'_>,
        candidates: &[ProcessCandidate<B::Payload>],
    ) -> Option<(Vec<super::process::Observation>, Coverage)> {
        self.inner.inspect_paths(context, candidates)
    }

    fn creation_token(&mut self, pid: u32) -> Option<u64> {
        self.inner.creation_token(pid)
    }

    fn details(&mut self, pid: u32) -> Result<ProcessDetails, String> {
        self.inner.details(pid)
    }
}

fn named(name: &str) -> ProcessDetails {
    ProcessDetails {
        name: Some(OsString::from(name)),
        executable: Some(PathBuf::from(format!("/bin/{name}"))),
        user: Some("501".to_string()),
        start_time: None,
    }
}

fn candidate(pid: u32, token: Option<u64>, script: Script) -> ProcessCandidate<Script> {
    ProcessCandidate {
        pid,
        creation_token: token,
        retained_handle: false,
        details: ProcessDetails::default(),
        payload: script,
    }
}

fn id(path: &Path) -> FileIdentity {
    identity::of_path(path, false).expect("identity")
}

fn handle(path: &Path, descriptor: u64) -> RawEvidence {
    RawEvidence {
        kind: EvidenceKind::OpenHandle,
        object: ObservedObject {
            identity: Some(id(path)),
            path: Some(path.to_path_buf()),
            deleted: false,
        },
        descriptor: Some(descriptor),
        watch: None,
        access: None,
        event_only: None,
        matched: None,
    }
}

fn path_only(path: &Path) -> RawEvidence {
    RawEvidence {
        kind: EvidenceKind::LoadedModule,
        object: ObservedObject {
            identity: None,
            path: Some(path.to_path_buf()),
            deleted: false,
        },
        descriptor: None,
        watch: None,
        access: None,
        event_only: None,
        matched: None,
    }
}

fn found(mechanism: Mechanism, evidence: Vec<RawEvidence>) -> MechanismInspection {
    MechanismInspection {
        mechanism,
        result: InspectionResult::Complete,
        evidence,
        limitations: Vec::new(),
    }
}

fn result(mechanism: Mechanism, result: InspectionResult) -> MechanismInspection {
    MechanismInspection {
        mechanism,
        result,
        evidence: Vec::new(),
        limitations: Vec::new(),
    }
}

fn both_clean() -> Script {
    vec![
        found(Mechanism::OpenHandles, Vec::new()),
        found(Mechanism::WorkingDirectories, Vec::new()),
    ]
}

/// `root/` with `a.txt`, `sub/b.txt`, and a sibling `root-copy/c.txt`.
struct Tree {
    _temp: TempDir,
    root: PathBuf,
    sibling: PathBuf,
}

fn tree() -> Tree {
    let temp = TempDir::new().expect("tempdir");
    let base = temp.path().canonicalize().expect("canonical tempdir");
    let root = base.join("app");
    let sibling = base.join("app-copy");
    std::fs::create_dir_all(root.join("sub")).unwrap();
    std::fs::create_dir_all(&sibling).unwrap();
    std::fs::write(root.join("a.txt"), "a").unwrap();
    std::fs::write(root.join("sub/b.txt"), "b").unwrap();
    std::fs::write(sibling.join("c.txt"), "c").unwrap();
    Tree {
        _temp: temp,
        root,
        sibling,
    }
}

fn query(path: &Path, options: &PathUsageOptions, fake: &mut Fake) -> PathUsageReport {
    let budget = Budget::start(options.deadline()).unwrap();
    run(path, options, fake, &budget, chrono::Utc::now()).expect("report")
}

fn query_with(path: &Path, fake: &mut Fake, budget: &Budget) -> Result<PathUsageReport, PathUsageError> {
    run(path, &PathUsageOptions::default(), fake, budget, chrono::Utc::now())
}

fn coverage(report: &PathUsageReport, mechanism: Mechanism) -> &Coverage {
    report
        .coverage
        .iter()
        .find(|c| c.mechanism == mechanism)
        .unwrap_or_else(|| panic!("no {mechanism:?} coverage in {:#?}", report.coverage))
}

fn has_limitation(limitations: &[Limitation], kind: LimitationKind) -> bool {
    limitations.iter().any(|l| l.kind == kind)
}

fn paths(evidence: &Evidence) -> Vec<PathBuf> {
    evidence
        .matched_paths
        .iter()
        .map(|p| p.as_path().to_path_buf())
        .collect()
}

/// `expired()` calls root resolution makes for `path` before it succeeds.
fn root_checks(path: &Path) -> i64 {
    let budget = Budget::expiring_after_checks(i64::MAX);
    root::resolve(path, &budget).expect("root");
    budget.checks_made()
}

// ---------------------------------------------------------------------------
// Options and typed errors
// ---------------------------------------------------------------------------

#[test]
fn default_options_are_recursive_with_a_two_second_budget() {
    let options = PathUsageOptions::default();
    assert!(options.recursive());
    assert_eq!(options.deadline(), Duration::from_secs(2));
    assert!(!options.target_only().recursive());
}

#[test]
fn zero_and_unrepresentable_deadlines_are_invalid_options() {
    let tree = tree();
    for deadline in [Duration::ZERO, Duration::MAX] {
        let options = PathUsageOptions::default().with_deadline(deadline);
        let error = query_path_usage(&tree.root, &options).expect_err("invalid deadline");
        assert_eq!(error.kind(), "invalid_option", "{deadline:?}: {error}");
    }
}

#[test]
fn a_missing_target_is_a_typed_error_not_an_empty_report() {
    let tree = tree();
    let missing = tree.root.join("nope");
    let error = query_path_usage(&missing, &PathUsageOptions::default()).expect_err("missing");
    assert!(matches!(&error, PathUsageError::MissingTarget { path } if *path == missing));
    assert_eq!(error.kind(), "missing_target");
}

#[cfg(unix)]
#[test]
fn a_dangling_root_symlink_is_a_missing_target() {
    let tree = tree();
    let link = tree.root.join("dangling");
    std::os::unix::fs::symlink(tree.root.join("absent"), &link).unwrap();
    let error = query_path_usage(&link, &PathUsageOptions::default()).expect_err("dangling");
    assert_eq!(error.kind(), "missing_target");
}

#[cfg(unix)]
#[test]
fn a_fifo_target_is_rejected_before_it_is_opened() {
    let tree = tree();
    let fifo = tree.root.join("pipe");
    let c_path = std::ffi::CString::new(fifo.as_os_str().as_encoded_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(c_path.as_ptr(), 0o600) }, 0);
    // Opening a FIFO with no writer blocks, so returning at all proves no open.
    let error = query_path_usage(&fifo, &PathUsageOptions::default()).expect_err("fifo");
    assert!(matches!(
        error,
        PathUsageError::UnsupportedTargetKind { file_type: "fifo", .. }
    ));
    assert_eq!(error.kind(), "unsupported_target_kind");

    let link = tree.root.join("pipe-link");
    std::os::unix::fs::symlink(&fifo, &link).unwrap();
    let error = query_path_usage(&link, &PathUsageOptions::default()).expect_err("fifo link");
    assert_eq!(error.kind(), "unsupported_target_kind");
}

#[test]
fn every_error_kind_is_stable() {
    let path = PathBuf::from("x");
    let errors = [
        (PathUsageError::InvalidOption { message: String::new() }, "invalid_option"),
        (PathUsageError::MissingTarget { path: path.clone() }, "missing_target"),
        (
            PathUsageError::UnsupportedTargetKind {
                path: path.clone(),
                file_type: "socket",
            },
            "unsupported_target_kind",
        ),
        (
            PathUsageError::RootValidationTimeout {
                path: path.clone(),
                budget: Duration::from_secs(1),
            },
            "root_validation_timeout",
        ),
        (
            PathUsageError::RootIdentity {
                path,
                source: std::io::Error::other("x"),
            },
            "root_identity",
        ),
    ];
    for (error, kind) in errors {
        assert_eq!(error.kind(), kind);
    }
}

// ---------------------------------------------------------------------------
// Root resolution
// ---------------------------------------------------------------------------

#[cfg(unix)]
#[test]
fn a_root_symlink_queries_its_target_and_reports_both_spellings() {
    let tree = tree();
    let alias = tree.sibling.join("alias");
    std::os::unix::fs::symlink(&tree.root, &alias).unwrap();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        10,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])],
    )]);
    let report = query(&alias, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.target.requested.as_path(), alias);
    assert_eq!(report.target.resolved.as_path(), tree.root);
    assert_eq!(report.target.identity, id(&tree.root));
    assert_eq!(report.target.kind, TargetKind::Directory);
    assert_eq!(paths(&report.processes[0].evidence[0]), vec![a]);
}

#[test]
fn a_relative_target_keeps_its_requested_spelling() {
    let tree = tree();
    let cwd = std::env::current_dir().unwrap();
    let relative = pathdiff(&tree.root, &cwd);
    let mut fake = Fake::new(Vec::new());
    let report = query(&relative, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.target.requested.as_path(), relative);
    assert_eq!(report.target.resolved.as_path(), tree.root);
}

/// `target` relative to `base` through `..` components.
fn pathdiff(target: &Path, base: &Path) -> PathBuf {
    let base = base.canonicalize().unwrap();
    let common = target
        .components()
        .zip(base.components())
        .take_while(|(a, b)| a == b)
        .count();
    let mut relative = PathBuf::new();
    for _ in base.components().skip(common) {
        relative.push("..");
    }
    for part in target.components().skip(common) {
        relative.push(part);
    }
    relative
}

// ---------------------------------------------------------------------------
// Coverage statuses and outcome
// ---------------------------------------------------------------------------

#[test]
fn a_clean_inspection_is_complete_and_usable_with_no_matches() {
    let tree = tree();
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.outcome, Outcome::Usable);
    assert!(report.processes.is_empty());
    assert_eq!(coverage(&report, Mechanism::ProcessEnumeration).status, CoverageStatus::Complete);
    assert_eq!(coverage(&report, Mechanism::TreeIdentity).status, CoverageStatus::Complete);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Complete);
    assert_eq!((handles.attempted, handles.succeeded), (Some(1), Some(1)));
    let fsevents = coverage(&report, Mechanism::Fsevents);
    assert_eq!(fsevents.status, CoverageStatus::Unsupported);
    assert_eq!(fsevents.reason.as_deref(), Some("no inventory API"));
    assert!(!report.budget_exhausted);
}

#[test]
fn an_empty_visible_process_set_is_complete() {
    let tree = tree();
    let mut fake = Fake::new(Vec::new());
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Complete);
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn a_denied_process_makes_coverage_partial_and_stays_visible() {
    let tree = tree();
    let mut fake = Fake::new(vec![
        candidate(1, Some(1), both_clean()),
        candidate(
            2,
            Some(2),
            vec![
                result(Mechanism::OpenHandles, InspectionResult::Denied),
                result(Mechanism::WorkingDirectories, InspectionResult::Denied),
            ],
        ),
    ]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    assert_eq!((handles.attempted, handles.succeeded), (Some(2), Some(1)));
    let denied = handles
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::PermissionDenied)
        .expect("denial is reported, not converted to no matches");
    assert_eq!(denied.count, 1);
    assert_eq!(denied.examples[0].pid, Some(2));
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn a_process_that_exits_before_inspection_is_a_visible_gap() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![
        candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]),
        candidate(2, Some(2), vec![result(Mechanism::OpenHandles, InspectionResult::Vanished)]),
    ]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    let gone = handles
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::ProcessDisappeared)
        .expect("disappearance is reported");
    assert_eq!(gone.examples[0].pid, Some(2));
    assert_eq!(report.processes.len(), 1);
}

#[test]
fn every_inspection_failing_is_failed_and_unavailable() {
    let tree = tree();
    let failing = || {
        vec![
            result(Mechanism::OpenHandles, InspectionResult::Failed("EIO".into())),
            result(Mechanism::WorkingDirectories, InspectionResult::Denied),
        ]
    };
    let mut fake = Fake::new(vec![candidate(1, Some(1), failing()), candidate(2, Some(2), failing())]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Failed);
    assert_eq!(coverage(&report, Mechanism::WorkingDirectories).status, CoverageStatus::Failed);
    assert_eq!(report.outcome, Outcome::Unavailable);
    let failed = &coverage(&report, Mechanism::OpenHandles).limitations[0];
    assert_eq!(failed.kind, LimitationKind::InspectionFailed);
    assert_eq!(failed.examples[0].detail.as_deref(), Some("EIO"));
}

#[test]
fn a_failed_enumeration_leaves_usage_mechanisms_not_attempted() {
    let tree = tree();
    let mut fake = Fake::failing_enumeration("cannot list /proc");
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::Failed);
    assert_eq!(enumeration.reason.as_deref(), Some("cannot list /proc"));
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::NotAttempted);
    assert_eq!(handles.reason.as_deref(), Some("process enumeration failed"));
    assert_eq!(coverage(&report, Mechanism::Fsevents).status, CoverageStatus::Unsupported);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

#[test]
fn a_partial_enumeration_propagates_into_every_dependent_mechanism() {
    let tree = tree();
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    if let Some(Ok(enumeration)) = fake.enumeration.as_mut() {
        enumeration.complete = false;
    }
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(coverage(&report, Mechanism::ProcessEnumeration).status, CoverageStatus::Partial);
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        let record = coverage(&report, mechanism);
        assert_eq!(record.status, CoverageStatus::Partial, "{mechanism:?}");
        assert!(has_limitation(&record.limitations, LimitationKind::PrerequisiteIncomplete));
    }
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn a_partial_enumeration_with_nothing_inspected_is_unavailable() {
    let tree = tree();
    let mut fake = Fake::new(Vec::new());
    if let Some(Ok(enumeration)) = fake.enumeration.as_mut() {
        enumeration.complete = false;
    }
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Partial);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

fn record(mechanism: Mechanism, status: CoverageStatus, succeeded: Option<u64>) -> Coverage {
    Coverage {
        mechanism,
        scope: CoverageScope::TargetTree,
        status,
        attempted: succeeded,
        succeeded,
        reason: None,
        truncation: None,
        limitations: Vec::new(),
    }
}

#[test]
fn the_outcome_rule_ignores_prerequisites_and_fixed_unsupported_records() {
    use CoverageStatus::*;
    use Mechanism::*;
    let enumeration = record(ProcessEnumeration, Complete, Some(5));
    let tree = record(TreeIdentity, Complete, Some(5));
    let cases = [
        (vec![enumeration.clone(), tree.clone()], Outcome::Unsupported),
        (vec![enumeration.clone(), record(Fsevents, Unsupported, None)], Outcome::Unsupported),
        (vec![enumeration.clone(), record(OpenHandles, Complete, Some(0))], Outcome::Usable),
        (vec![record(OpenHandles, Partial, Some(1))], Outcome::Usable),
        (vec![enumeration.clone(), record(OpenHandles, Partial, Some(0))], Outcome::Unavailable),
        (vec![record(OpenHandles, Failed, Some(0))], Outcome::Unavailable),
        (vec![record(OpenHandles, NotAttempted, None)], Outcome::Unavailable),
        (
            vec![record(OpenHandles, Failed, None), record(Fsevents, Unsupported, None)],
            Outcome::Unavailable,
        ),
        (
            vec![record(OpenHandles, Failed, None), record(WorkingDirectories, Complete, Some(1))],
            Outcome::Usable,
        ),
    ];
    for (coverage, expected) in cases {
        assert_eq!(report::outcome(&coverage, &[]), expected, "{coverage:#?}");
    }
}

#[test]
fn a_partial_mechanism_with_retained_evidence_is_usable() {
    let evidence = Evidence {
        kind: EvidenceKind::OpenHandle,
        mechanism: Mechanism::OpenHandles,
        matched_paths: vec!["/x".into()],
        observed_path: None,
        match_basis: MatchBasis::Identity,
        identity: None,
        descriptor: None,
        watch: None,
        access: None,
        event_only: None,
    };
    let process = ProcessRecord {
        pid: 1,
        creation_token: None,
        start_time: None,
        identity_uncertain: true,
        name: None,
        executable: None,
        user: None,
        evidence: vec![evidence],
    };
    let coverage = [record(Mechanism::OpenHandles, CoverageStatus::Partial, Some(0))];
    assert_eq!(report::outcome(&coverage, &[process]), Outcome::Usable);
}

// ---------------------------------------------------------------------------
// Shared budget, phase by phase
// ---------------------------------------------------------------------------

#[test]
fn a_budget_expired_on_entry_is_a_root_validation_timeout() {
    let tree = tree();
    let mut fake = Fake::new(Vec::new());
    let error = query_with(&tree.root, &mut fake, &Budget::expiring_after_checks(0)).expect_err("timeout");
    assert_eq!(error.kind(), "root_validation_timeout");
    assert!(fake.enumeration.is_some(), "no enumeration after a root timeout");
}

#[test]
fn a_budget_expiring_during_root_validation_is_a_root_validation_timeout() {
    let tree = tree();
    let checks = root_checks(&tree.root);
    assert!(checks >= 2, "root resolution checks the budget between steps");
    for allowed in 1..checks {
        let mut fake = Fake::new(Vec::new());
        let error = query_with(&tree.root, &mut fake, &Budget::expiring_after_checks(allowed))
            .expect_err("timeout");
        assert_eq!(error.kind(), "root_validation_timeout", "after {allowed} check(s)");
    }
}

#[test]
fn a_budget_expiring_during_the_tree_walk_keeps_a_report_with_a_partial_tree() {
    let tree = tree();
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    // Root resolution, then one descendant before expiry.
    let budget = Budget::expiring_after_checks(root_checks(&tree.root) + 1);
    let report = query_with(&tree.root, &mut fake, &budget).expect("report, not an error");
    let tree_coverage = coverage(&report, Mechanism::TreeIdentity);
    assert_eq!(tree_coverage.status, CoverageStatus::Partial);
    assert_eq!(tree_coverage.succeeded, Some(1));
    assert!(has_limitation(&tree_coverage.limitations, LimitationKind::BudgetExhausted));
    assert_eq!(coverage(&report, Mechanism::ProcessEnumeration).status, CoverageStatus::NotAttempted);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::NotAttempted);
    assert!(report.budget_exhausted);
    assert_eq!(report.outcome, Outcome::Unavailable);
    assert!(fake.inspected.is_empty());
    assert!(has_limitation(&report.limitations, LimitationKind::RootUnverified));
}

#[test]
fn a_budget_expiring_before_enumeration_leaves_mechanisms_not_attempted() {
    let tree = tree();
    let file = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    let budget = Budget::expiring_after_checks(root_checks(&file));
    let report = query_with(&file, &mut fake, &budget).expect("report");
    let enumeration = coverage(&report, Mechanism::ProcessEnumeration);
    assert_eq!(enumeration.status, CoverageStatus::NotAttempted);
    assert!(enumeration.reason.as_deref().unwrap().contains("budget"));
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::NotAttempted);
    assert!(handles.reason.as_deref().unwrap().contains("budget"));
    assert_eq!(report.outcome, Outcome::Unavailable);
}

#[test]
fn a_budget_expiring_during_enumeration_inspects_nothing() {
    let tree = tree();
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    fake.on_enumerate = Some(Box::new(|budget: &Budget| budget.expire()));
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert!(fake.inspected.is_empty());
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::NotAttempted);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

#[test]
fn a_budget_expiring_during_inspection_keeps_collected_evidence() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let processes = (1..=4)
        .map(|pid| {
            candidate(
                pid,
                Some(u64::from(pid)),
                vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])],
            )
        })
        .collect();
    let mut fake = Fake::new(processes);
    fake.on_inspect = Some(Box::new(|pid, context| {
        if pid == 2 {
            context.budget.expire();
        }
    }));
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(fake.inspected, vec![1, 2]);
    assert_eq!(report.processes.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![1, 2]);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    let budget = handles
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::BudgetExhausted)
        .expect("budget limitation");
    assert_eq!(budget.count, 2, "two processes were never inspected");
    assert_eq!(report.outcome, Outcome::Usable);
    assert!(report.budget_exhausted);
    // Enrichment and the root recheck are skipped, never overrun.
    assert!(fake.details_calls.is_empty());
    assert!(has_limitation(&report.limitations, LimitationKind::RootUnverified));
    assert!(has_limitation(&report.limitations, LimitationKind::IdentityUnavailable));
    assert!(report.processes.iter().all(|p| p.name.is_none() && !p.evidence.is_empty()));
}

#[test]
fn a_mechanism_the_budget_stopped_for_one_process_is_partial() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![
        candidate(1, Some(1), both_clean()),
        candidate(
            2,
            Some(2),
            vec![
                found(Mechanism::OpenHandles, vec![handle(&a, 3)]),
                result(Mechanism::WorkingDirectories, InspectionResult::NotAttempted),
            ],
        ),
    ]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let cwd = coverage(&report, Mechanism::WorkingDirectories);
    assert_eq!(cwd.status, CoverageStatus::Partial, "{cwd:#?}");
    assert_eq!((cwd.attempted, cwd.succeeded), (Some(1), Some(1)));
    let budget = cwd
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::BudgetExhausted)
        .expect("budget limitation");
    assert_eq!(budget.count, 1);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Complete);
    assert_eq!(report.processes.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![2]);
}

#[test]
fn a_mechanism_the_budget_stopped_for_every_process_is_not_attempted() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![
            found(Mechanism::OpenHandles, vec![handle(&a, 3)]),
            result(Mechanism::WorkingDirectories, InspectionResult::NotAttempted),
        ],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let cwd = coverage(&report, Mechanism::WorkingDirectories);
    assert_eq!(cwd.status, CoverageStatus::NotAttempted, "{cwd:#?}");
    assert_eq!(cwd.attempted, Some(0));
    assert!(cwd.reason.as_deref().is_some_and(|r| r.contains("budget")));
    assert!(has_limitation(&cwd.limitations, LimitationKind::BudgetExhausted));
    assert_eq!(report.processes.len(), 1);
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn a_budget_expiring_before_any_inspection_is_not_attempted() {
    let tree = tree();
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    let budget = Budget::expiring_after_checks(root_checks(&tree.root) + 4 + 1);
    let report = query_with(&tree.root, &mut fake, &budget).expect("report");
    // root + 4 walk checks (3 entries and the end) + the enumeration check.
    assert_eq!(coverage(&report, Mechanism::TreeIdentity).status, CoverageStatus::Complete);
    assert_eq!(coverage(&report, Mechanism::ProcessEnumeration).status, CoverageStatus::Complete);
    assert!(fake.inspected.is_empty());
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::NotAttempted);
    assert_eq!(report.outcome, Outcome::Unavailable);
}

// ---------------------------------------------------------------------------
// Root recheck
// ---------------------------------------------------------------------------

#[test]
fn a_root_replaced_during_the_query_keeps_evidence_and_reports_the_change() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        7,
        Some(7),
        vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])],
    )]);
    let root = tree.root.clone();
    let moved = tree.sibling.join("moved");
    fake.on_inspect = Some(Box::new(move |_, _| {
        std::fs::rename(&root, &moved).unwrap();
        std::fs::create_dir(&root).unwrap();
    }));
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.processes.len(), 1, "prior evidence is kept");
    assert_eq!(paths(&report.processes[0].evidence[0]), vec![a]);
    assert!(has_limitation(&report.limitations, LimitationKind::RootChanged));
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    assert!(has_limitation(&handles.limitations, LimitationKind::PrerequisiteIncomplete));
}

#[test]
fn a_root_removed_during_the_query_is_a_root_change() {
    let tree = tree();
    let file = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    let target = file.clone();
    fake.on_inspect = Some(Box::new(move |_, _| std::fs::remove_file(&target).unwrap()));
    let report = query(&file, &PathUsageOptions::default(), &mut fake);
    assert!(
        has_limitation(&report.limitations, LimitationKind::RootChanged),
        "{:#?}",
        report.limitations
    );
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Partial);
}

// ---------------------------------------------------------------------------
// Process lifetimes
// ---------------------------------------------------------------------------

#[test]
fn the_same_pid_with_different_start_tokens_stays_two_processes() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let b = tree.root.join("sub/b.txt");
    let mut fake = Fake::new(vec![
        candidate(40, Some(100), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]),
        candidate(40, Some(200), vec![found(Mechanism::OpenHandles, vec![handle(&b, 4)])]),
        candidate(40, Some(100), vec![found(Mechanism::WorkingDirectories, vec![handle(&b, 5)])]),
    ]);
    fake.tokens.insert(40, 200);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let tokens: Vec<_> = report.processes.iter().map(|p| (p.pid, p.creation_token)).collect();
    assert_eq!(tokens, vec![(40, Some(100)), (40, Some(200))]);
    assert_eq!(report.processes[0].evidence.len(), 2, "same lifetime merged");
    assert_eq!(report.processes[1].evidence.len(), 1);
    assert!(report.processes.iter().all(|p| !p.identity_uncertain));
}

#[test]
fn enrichment_after_pid_reuse_never_mislabels_prior_evidence() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        40,
        Some(100),
        vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])],
    )]);
    // The PID now names a new lifetime started after inspection.
    fake.tokens.insert(40, 101);
    fake.details.insert(40, Ok(named("newcomer")));
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let process = &report.processes[0];
    assert_eq!(process.name, None);
    assert_eq!(process.executable, None);
    assert_eq!(process.evidence.len(), 1, "evidence survives");
    assert!(has_limitation(&report.limitations, LimitationKind::ProcessDisappeared));
}

#[test]
fn enrichment_after_the_process_exits_keeps_its_evidence() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        41,
        Some(5),
        vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])],
    )]);
    fake.details.insert(41, Err("no such process".to_string()));
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.processes[0].evidence.len(), 1);
    assert_eq!(report.processes[0].name, None);
    let unavailable = report
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::IdentityUnavailable)
        .expect("enrichment failure is visible");
    assert_eq!(unavailable.examples[0].detail.as_deref(), Some("no such process"));
}

#[test]
fn verified_enrichment_fills_identity_without_overriding_backend_fields() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut process = candidate(9, Some(9), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]);
    process.details.name = Some(OsString::from("from-backend"));
    let mut fake = Fake::new(vec![process]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let record = &report.processes[0];
    assert_eq!(record.name.as_ref().map(|n| n.display().into_owned()).as_deref(), Some("from-backend"));
    assert_eq!(
        record.executable.as_ref().map(|n| n.as_path().to_path_buf()),
        Some(PathBuf::from("/bin/proc-9"))
    );
    assert_eq!(record.user.as_deref(), Some("501"));
    assert_eq!(fake.details_calls, vec![9]);
}

#[test]
fn missing_start_tokens_never_merge_and_are_not_enriched() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let script = || vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])];
    let mut fake = Fake::new(vec![candidate(50, None, script()), candidate(50, None, script())]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.processes.len(), 2, "unverifiable acquisitions stay separate");
    assert!(report.processes.iter().all(|p| p.identity_uncertain && p.name.is_none()));
    assert!(fake.details_calls.is_empty(), "no name from a possibly different lifetime");
    let unavailable = report
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::IdentityUnavailable)
        .unwrap();
    assert_eq!(unavailable.count, 2);
}

#[test]
fn a_retained_handle_without_a_token_allows_enrichment() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut process = candidate(60, None, vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]);
    process.retained_handle = true;
    let mut fake = Fake::new(vec![process]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert!(!report.processes[0].identity_uncertain);
    assert!(report.processes[0].name.is_some());
}

#[test]
fn processes_and_evidence_are_sorted_for_presentation() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let b = tree.root.join("sub/b.txt");
    let mut fake = Fake::new(vec![
        candidate(30, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&b, 9), handle(&a, 8)])]),
        candidate(
            20,
            Some(1),
            vec![
                found(Mechanism::WorkingDirectories, vec![RawEvidence {
                    kind: EvidenceKind::WorkingDirectory,
                    ..handle(&a, 0)
                }]),
                found(Mechanism::OpenHandles, vec![handle(&a, 5), handle(&a, 4)]),
            ],
        ),
    ]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.processes.iter().map(|p| p.pid).collect::<Vec<_>>(), vec![20, 30]);
    let first: Vec<_> = report.processes[0]
        .evidence
        .iter()
        .map(|e| (e.kind, e.descriptor))
        .collect();
    assert_eq!(
        first,
        vec![
            (EvidenceKind::OpenHandle, Some(4)),
            (EvidenceKind::OpenHandle, Some(5)),
            (EvidenceKind::WorkingDirectory, Some(0)),
        ]
    );
    let second: Vec<_> = report.processes[1].evidence.iter().map(paths).collect();
    assert_eq!(second, vec![vec![a], vec![b]]);
}

// ---------------------------------------------------------------------------
// Tree scope and matching
// ---------------------------------------------------------------------------

#[test]
fn hard_links_report_every_in_scope_path_and_the_observed_alias() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let twin = tree.root.join("sub/twin.txt");
    let outside = tree.sibling.join("alias.txt");
    std::fs::hard_link(&a, &twin).unwrap();
    std::fs::hard_link(&a, &outside).unwrap();
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&outside, 3)])],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let evidence = &report.processes[0].evidence[0];
    let mut expected = vec![a, twin];
    expected.sort();
    assert_eq!(paths(evidence), expected, "no preferred path");
    assert_eq!(evidence.observed_path.as_ref().unwrap().as_path(), outside);
    assert_eq!(evidence.match_basis, MatchBasis::Identity);
}

#[test]
fn a_similarly_prefixed_sibling_is_outside_the_tree() {
    let tree = tree();
    let sibling_file = tree.sibling.join("c.txt");
    assert!(sibling_file.to_string_lossy().starts_with(&*tree.root.to_string_lossy()));
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(
            Mechanism::OpenHandles,
            vec![handle(&sibling_file, 3), handle(&tree.sibling, 4), path_only(&sibling_file)],
        )],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert!(report.processes.is_empty(), "{:#?}", report.processes);
    assert!(!matching::may_contain(&tree.root, &sibling_file));
    assert!(matching::may_contain(&tree.root, &tree.root.join("a.txt")));
}

#[cfg(windows)]
#[test]
fn windows_spellings_are_compared_by_component_not_by_string() {
    let contains = |root: &str, path: &str| matching::may_contain(Path::new(root), Path::new(path));
    assert!(contains(r"\\?\C:\work\app", r"C:\work\app\a.txt"));
    assert!(contains(r"C:\work\app", r"\\?\c:\WORK\App\a.txt"));
    assert!(contains(r"\\?\UNC\server\share\app", r"\\server\share\app\x"));
    assert!(contains(r"C:\Program Files\app", r"C:\PROGRA~1\app\x"), "8.3 alias left to lookup");
    assert!(!contains(r"C:\work\app", r"C:\work\app-copy\a.txt"));
    assert!(!contains(r"C:\work\app", r"D:\work\app\a.txt"));
    assert!(!contains(r"C:\work\app", r"\\server\share\work\app"));
}

#[cfg(windows)]
#[test]
fn a_case_variant_path_lookup_matches_only_the_object_it_names() {
    let tree = tree();
    let b = tree.root.join("sub/b.txt");
    let variant = PathBuf::from(b.to_string_lossy().to_uppercase());
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![path_only(&variant)])],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    // A default (case-insensitive) directory resolves the variant to `b.txt`.
    assert_eq!(paths(&report.processes[0].evidence[0]), vec![b]);
}

#[test]
fn a_path_only_observation_inside_the_tree_matches_by_lookup() {
    let tree = tree();
    let b = tree.root.join("sub/b.txt");
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![path_only(&b)])],
    )]);
    let (report, counts) = measure(|| query(&tree.root, &PathUsageOptions::default(), &mut fake));
    let evidence = &report.processes[0].evidence[0];
    assert_eq!(paths(evidence), vec![b.clone()]);
    assert_eq!(evidence.match_basis, MatchBasis::PathLookup);
    assert_eq!(evidence.identity, Some(id(&b)));
    assert_eq!(counts.get(crate::performance::counters::QUERY_PATH_LOOKUPS), 1);
}

#[test]
fn a_path_match_the_backend_validated_survives_a_budget_that_expires_before_projection() {
    let tree = tree();
    let b = tree.root.join("sub/b.txt");
    let mut validated = path_only(&b);
    validated.matched = Some(ObjectMatch {
        matched_paths: vec![b.clone()],
        basis: MatchBasis::PathLookup,
        identity: id(&b),
    });
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![validated])],
    )]);
    // Expires after the backend's lookup and before the query projects it.
    fake.on_inspect = Some(Box::new(|_, context| context.budget.expire()));
    let (report, counts) = measure(|| query(&tree.root, &PathUsageOptions::default(), &mut fake));

    assert!(report.budget_exhausted);
    assert_eq!(report.processes.len(), 1, "{:#?}", report.processes);
    let evidence = &report.processes[0].evidence[0];
    assert_eq!(paths(evidence), vec![b.clone()]);
    assert_eq!(evidence.match_basis, MatchBasis::PathLookup);
    assert_eq!(evidence.identity, Some(id(&b)));
    // The query made no lookup of its own.
    assert_eq!(counts.get(crate::performance::counters::QUERY_PATH_LOOKUPS), 0);
}

#[test]
fn a_deleted_object_is_never_matched_by_its_former_path() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut deleted = path_only(&a);
    deleted.object.deleted = true;
    // The former path exists and names an in-scope object, but not the one held.
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![deleted])])]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert!(report.processes.is_empty());
}

#[test]
fn an_object_with_a_known_identity_is_never_matched_by_path_text() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut stale = handle(&tree.sibling.join("c.txt"), 3);
    stale.object.path = Some(a);
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![stale])])]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert!(report.processes.is_empty());
}

#[cfg(unix)]
#[test]
fn descendant_symlinks_are_indexed_as_links_and_never_expanded() {
    let tree = tree();
    let link = tree.root.join("out");
    std::os::unix::fs::symlink(&tree.sibling, &link).unwrap();
    std::os::unix::fs::symlink(&tree.root, tree.root.join("sub/loop")).unwrap();
    let outside = tree.sibling.join("c.txt");
    let link_identity = identity::of_path(&link, false).unwrap();
    let link_object = RawEvidence {
        object: ObservedObject {
            identity: Some(link_identity),
            path: Some(link.clone()),
            deleted: false,
        },
        ..handle(&outside, 4)
    };
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&outside, 3), link_object])],
    )]);
    let (report, counts) = measure(|| query(&tree.root, &PathUsageOptions::default(), &mut fake));
    let evidence = &report.processes[0].evidence;
    assert_eq!(evidence.len(), 1, "the link's target is out of scope: {evidence:#?}");
    assert_eq!(paths(&evidence[0]), vec![link]);
    // a.txt, sub, sub/b.txt, out, sub/loop: the loop link is not followed.
    assert_eq!(counts.get(crate::performance::counters::QUERY_TREE_IDENTITY_READS), 5);
    assert_eq!(coverage(&report, Mechanism::TreeIdentity).status, CoverageStatus::Complete);
}

#[test]
fn hidden_and_git_ignored_entries_are_in_scope() {
    let tree = tree();
    std::fs::write(tree.root.join(".gitignore"), "ignored.txt\nnode_modules/\n").unwrap();
    std::fs::create_dir_all(tree.root.join(".hidden")).unwrap();
    std::fs::create_dir_all(tree.root.join("node_modules/pkg")).unwrap();
    let hidden = tree.root.join(".hidden/secret");
    let ignored = tree.root.join("ignored.txt");
    let module = tree.root.join("node_modules/pkg/index.js");
    for file in [&hidden, &ignored, &module] {
        std::fs::write(file, "x").unwrap();
    }
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(
            Mechanism::OpenHandles,
            vec![handle(&hidden, 3), handle(&ignored, 4), handle(&module, 5)],
        )],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let matched: Vec<_> = report.processes[0].evidence.iter().flat_map(paths).collect();
    assert_eq!(matched.len(), 3, "{matched:#?}");
    for file in [hidden, ignored, module] {
        assert!(matched.contains(&file), "{} missing", file.display());
    }
}

#[test]
fn target_only_matches_the_directory_itself_and_not_descendants() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let cwd = RawEvidence {
        kind: EvidenceKind::WorkingDirectory,
        ..handle(&tree.root, 0)
    };
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![
            found(Mechanism::OpenHandles, vec![handle(&a, 3)]),
            found(Mechanism::WorkingDirectories, vec![cwd]),
        ],
    )]);
    let (report, counts) = measure(|| query(&tree.root, &PathUsageOptions::default().target_only(), &mut fake));
    assert!(!report.target.recursive);
    assert!(report.coverage.iter().all(|c| c.mechanism != Mechanism::TreeIdentity));
    assert_eq!(coverage(&report, Mechanism::OpenHandles).scope, CoverageScope::Target);
    let evidence = &report.processes[0].evidence;
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].kind, EvidenceKind::WorkingDirectory);
    assert_eq!(paths(&evidence[0]), vec![tree.root.clone()]);
    assert_eq!(counts.get(crate::performance::counters::QUERY_TREE_WALKS), 0);
}

#[cfg(unix)]
#[test]
fn an_unreadable_descendant_makes_dependent_coverage_partial() {
    use std::os::unix::fs::PermissionsExt;
    if unsafe { libc::geteuid() } == 0 {
        eprintln!("skipping: root can read a mode-000 directory");
        return;
    }
    let tree = tree();
    let locked = tree.root.join("locked");
    std::fs::create_dir(&locked).unwrap();
    std::fs::write(locked.join("inner"), "x").unwrap();
    std::fs::set_permissions(&locked, std::fs::Permissions::from_mode(0o000)).unwrap();
    struct Restore(PathBuf);
    impl Drop for Restore {
        fn drop(&mut self) {
            let _ = std::fs::set_permissions(&self.0, std::fs::Permissions::from_mode(0o755));
        }
    }
    let _restore = Restore(locked.clone());

    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)]), found(Mechanism::WorkingDirectories, vec![])],
    )]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let tree_coverage = coverage(&report, Mechanism::TreeIdentity);
    assert_eq!(tree_coverage.status, CoverageStatus::Partial);
    let unreadable = tree_coverage
        .limitations
        .iter()
        .find(|l| l.kind == LimitationKind::UnreadableTreeEntry)
        .expect("unreadable entry limitation");
    assert_eq!(unreadable.count, 1);
    assert_eq!(unreadable.examples[0].path.as_ref().unwrap().as_path(), locked);
    for mechanism in [Mechanism::OpenHandles, Mechanism::WorkingDirectories] {
        assert_eq!(coverage(&report, mechanism).status, CoverageStatus::Partial);
    }
    assert_eq!(report.processes[0].evidence.len(), 1, "readable matches survive");
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn malformed_records_keep_valid_siblings_and_make_coverage_partial() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut inspection = found(Mechanism::OpenHandles, vec![handle(&a, 3)]);
    inspection.result = InspectionResult::Partial;
    let mut malformed = Limitation::new(LimitationKind::MalformedRecord, "an fdinfo line was malformed");
    malformed.count = 2;
    malformed.examples.push(LimitationExample::pid(1).with_detail("inotify wd:x"));
    inspection.limitations.push(malformed);
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![inspection])]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    let recorded = &handles.limitations[0];
    assert_eq!((recorded.kind, recorded.count), (LimitationKind::MalformedRecord, 2));
    assert_eq!(report.processes[0].evidence.len(), 1);
}

#[test]
fn evidence_beyond_the_per_process_cap_is_counted_as_omitted() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let many = (0..MAX_EVIDENCE_PER_PROCESS + 3).map(|fd| handle(&a, fd)).collect();
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![found(Mechanism::OpenHandles, many)])]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    assert_eq!(report.processes[0].evidence.len() as u64, MAX_EVIDENCE_PER_PROCESS);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    assert_eq!(
        handles.truncation,
        Some(Truncation {
            per_process_limit: MAX_EVIDENCE_PER_PROCESS,
            total_limit: MAX_EVIDENCE_TOTAL,
            omitted: 3,
        })
    );
    assert!(has_limitation(&handles.limitations, LimitationKind::Truncated));
}

#[test]
fn one_tree_walk_serves_every_process_and_no_inventory_runs() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let processes = (1..=50)
        .map(|pid| candidate(pid, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]))
        .collect();
    let mut fake = Fake::new(processes);
    let (report, counts) = measure(|| query(&tree.root, &PathUsageOptions::default(), &mut fake));
    use crate::performance::counters::*;
    assert_eq!(report.processes.len(), 50);
    assert_eq!(counts.get(QUERY_TREE_WALKS), 1, "{:#?}", counts.all());
    assert_eq!(counts.get(QUERY_TREE_IDENTITY_READS), 3);
    assert_eq!(counts.get(QUERY_PROCESS_ENUMERATIONS), 1);
    assert_eq!(counts.get(QUERY_PROCESS_INSPECTIONS), 50);
    assert_eq!(counts.get(QUERY_IDENTITY_ENRICHMENTS), 50);
    let allowed = [
        QUERY_TREE_WALKS,
        QUERY_TREE_IDENTITY_READS,
        QUERY_PROCESS_ENUMERATIONS,
        QUERY_PROCESS_INSPECTIONS,
        QUERY_IDENTITY_ENRICHMENTS,
        FS_METADATA_PROBES,
        FS_CANONICALIZATIONS,
    ];
    for name in counts.all().keys() {
        assert!(allowed.contains(&name.as_str()), "unrelated work recorded: {name}");
    }
    assert!(counts.get(FS_METADATA_PROBES) <= 3, "root resolution and recheck only");
}

// ---------------------------------------------------------------------------
// A caller-supplied tree index
// ---------------------------------------------------------------------------

fn kind_of(file_type: std::fs::FileType) -> TreeEntryKind {
    if file_type.is_symlink() {
        TreeEntryKind::Link
    } else if file_type.is_dir() {
        TreeEntryKind::Directory
    } else if file_type.is_file() {
        TreeEntryKind::File
    } else {
        TreeEntryKind::Other
    }
}

/// The index a caller that already walked `root` would supply.
fn caller_builder(root: &Path) -> TreeIndexBuilder {
    let mut builder = TreeIndex::builder(id(root));
    for entry in walkdir::WalkDir::new(root).min_depth(1).follow_links(false) {
        let entry = entry.unwrap();
        builder.entry(
            entry.path().strip_prefix(root).unwrap(),
            kind_of(entry.file_type()),
            id(entry.path()),
        );
    }
    builder
}

fn caller_index(root: &Path) -> std::sync::Arc<TreeIndex> {
    std::sync::Arc::new(caller_builder(root).build().expect("valid index"))
}

fn supplied(root: &Path) -> PathUsageOptions {
    PathUsageOptions::default().with_tree_index(caller_index(root))
}

#[test]
fn a_supplied_index_replaces_the_walk_but_the_root_is_still_verified() {
    let tree = tree();
    let b = tree.root.join("sub/b.txt");
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&b, 3)])],
    )]);
    let options = supplied(&tree.root);
    let (report, counts) = measure(|| query(&tree.root, &options, &mut fake));
    use crate::performance::counters::*;
    assert_eq!(counts.get(QUERY_TREE_WALKS), 0, "{:#?}", counts.all());
    assert_eq!(counts.get(QUERY_TREE_IDENTITY_READS), 0);
    assert_eq!(counts.get(QUERY_PROCESS_ENUMERATIONS), 1, "the budget reached processes");
    assert_eq!(counts.get(QUERY_PROCESS_INSPECTIONS), 1);
    // Root resolution and the final recheck read the root's metadata.
    assert!(counts.get(FS_METADATA_PROBES) >= 2, "{:#?}", counts.all());
    assert!(!has_limitation(&report.limitations, LimitationKind::RootUnverified));

    assert_eq!(report.index_provenance, IndexProvenance::Supplied);
    assert_eq!(paths(&report.processes[0].evidence[0]), vec![b]);
    let tree_coverage = coverage(&report, Mechanism::TreeIdentity);
    assert_eq!(tree_coverage.status, CoverageStatus::Complete);
    assert_eq!(tree_coverage.scope, CoverageScope::TargetTree);
    assert_eq!((tree_coverage.attempted, tree_coverage.succeeded), (None, None), "no descendant was read");
    assert!(tree_coverage.reason.as_deref().unwrap().contains("supplied"), "{tree_coverage:?}");
    assert_eq!(coverage(&report, Mechanism::Fsevents).status, CoverageStatus::Unsupported);
    assert_eq!(coverage(&report, Mechanism::OpenHandles).status, CoverageStatus::Complete);
    assert_eq!(report.outcome, Outcome::Usable);
}

#[test]
fn walked_and_supplied_indexes_find_the_same_holders_and_hard_link_aliases() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let b = tree.root.join("sub/b.txt");
    let twin = tree.root.join("sub/twin.txt");
    let outside = tree.sibling.join("alias.txt");
    std::fs::hard_link(&a, &twin).unwrap();
    std::fs::hard_link(&a, &outside).unwrap();
    let processes = || {
        vec![
            candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&outside, 3), handle(&tree.sibling.join("c.txt"), 4)])]),
            candidate(2, Some(2), vec![found(Mechanism::OpenHandles, vec![path_only(&b)])]),
            candidate(3, Some(3), vec![found(Mechanism::WorkingDirectories, vec![handle(&tree.root.join("sub"), 5)])]),
        ]
    };
    let walked = query(&tree.root, &PathUsageOptions::default(), &mut Fake::new(processes()));
    let supplied = query(&tree.root, &supplied(&tree.root), &mut Fake::new(processes()));
    assert_eq!(walked.index_provenance, IndexProvenance::Walked);
    assert_eq!(supplied.index_provenance, IndexProvenance::Supplied);
    assert_eq!(supplied.processes, walked.processes);
    let mut aliases = vec![a, twin];
    aliases.sort();
    assert_eq!(paths(&supplied.processes[0].evidence[0]), aliases, "every alias, none preferred");
    assert_eq!(supplied.processes[0].evidence.len(), 1, "the sibling is out of scope");
}

#[test]
fn an_index_for_a_different_root_is_an_invalid_option_and_nothing_runs() {
    let tree = tree();
    let options = PathUsageOptions::default().with_tree_index(caller_index(&tree.sibling));
    let mut fake = Fake::new(vec![candidate(1, Some(1), both_clean())]);
    let budget = Budget::start(options.deadline()).unwrap();
    let error = run(&tree.root, &options, &mut fake, &budget, chrono::Utc::now()).unwrap_err();
    assert!(matches!(&error, PathUsageError::InvalidOption { message } if message.contains("different root")), "{error}");
    assert!(fake.enumeration.is_some(), "processes were never enumerated");
}

#[test]
fn an_index_with_target_only_or_for_a_file_is_an_invalid_option() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let index = caller_index(&tree.root);
    let target_only = PathUsageOptions::default().with_tree_index(index.clone()).target_only();
    let file = PathUsageOptions::default()
        .with_tree_index(std::sync::Arc::new(TreeIndex::builder(id(&a)).build().unwrap()));
    for (path, options, needle) in [(&tree.root, target_only, "target_only"), (&a, file, "file")] {
        let mut fake = Fake::new(Vec::new());
        let budget = Budget::start(options.deadline()).unwrap();
        let error = run(path, &options, &mut fake, &budget, chrono::Utc::now()).unwrap_err();
        assert_eq!(error.kind(), "invalid_option", "{error}");
        assert!(error.to_string().contains(needle), "{error}");
        assert!(fake.enumeration.is_some());
    }
}

#[test]
fn a_partial_index_is_reported_as_the_callers_partial_coverage() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut builder = caller_builder(&tree.root);
    builder.partial("3 entries could not be read during the caller's walk");
    let options = PathUsageOptions::default().with_tree_index(std::sync::Arc::new(builder.build().unwrap()));
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])])]);
    let report = query(&tree.root, &options, &mut fake);
    let tree_coverage = coverage(&report, Mechanism::TreeIdentity);
    assert_eq!(tree_coverage.status, CoverageStatus::Partial);
    let declared = &tree_coverage.limitations[0];
    assert_eq!(declared.kind, LimitationKind::IncompleteTreeIndex);
    assert_eq!(declared.message, "3 entries could not be read during the caller's walk");
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial, "a dependent mechanism inherits the gap");
    assert!(has_limitation(&handles.limitations, LimitationKind::PrerequisiteIncomplete));
    assert_eq!(report.index_provenance, IndexProvenance::Supplied);
}

#[test]
fn an_empty_index_matches_only_the_root() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let index = TreeIndex::builder(id(&tree.root)).build().expect("[] is a root with no descendants");
    assert!(index.is_empty());
    let options = PathUsageOptions::default().with_tree_index(std::sync::Arc::new(index));
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::WorkingDirectories, vec![handle(&tree.root, 3), handle(&a, 4)])],
    )]);
    let report = query(&tree.root, &options, &mut fake);
    let evidence = &report.processes[0].evidence;
    assert_eq!(evidence.len(), 1, "{evidence:#?}");
    assert_eq!(paths(&evidence[0]), vec![tree.root.clone()], "the root keeps its spelling");
    assert_eq!(coverage(&report, Mechanism::TreeIdentity).status, CoverageStatus::Complete);
}

#[test]
fn a_budget_expiring_during_inspection_stays_visible_with_a_supplied_index() {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let processes = (1..=3)
        .map(|pid| candidate(pid, Some(u64::from(pid)), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]))
        .collect();
    let mut fake = Fake::new(processes);
    fake.on_inspect = Some(Box::new(|pid, context| {
        if pid == 1 {
            context.budget.expire();
        }
    }));
    let report = query(&tree.root, &supplied(&tree.root), &mut fake);
    assert_eq!(fake.inspected, vec![1]);
    let handles = coverage(&report, Mechanism::OpenHandles);
    assert_eq!(handles.status, CoverageStatus::Partial);
    assert!(has_limitation(&handles.limitations, LimitationKind::BudgetExhausted));
    assert!(report.budget_exhausted);
    assert!(has_limitation(&report.limitations, LimitationKind::RootUnverified));
}

#[cfg(unix)]
#[test]
fn a_supplied_link_is_matched_as_itself_and_never_probed_as_its_target() {
    let tree = tree();
    let link = tree.root.join("out");
    std::os::unix::fs::symlink(&tree.sibling, &link).unwrap();
    let outside = tree.sibling.join("c.txt");
    let link_object = RawEvidence {
        object: ObservedObject { identity: Some(id(&link)), path: Some(link.clone()), deleted: false },
        ..handle(&outside, 4)
    };
    let mut fake = Fake::new(vec![candidate(
        1,
        Some(1),
        vec![found(Mechanism::OpenHandles, vec![handle(&outside, 3), link_object])],
    )]);
    let index = caller_index(&tree.root);
    let report = query(&tree.root, &PathUsageOptions::default().with_tree_index(index.clone()), &mut fake);
    let evidence = &report.processes[0].evidence;
    assert_eq!(evidence.len(), 1, "the link's target is out of scope: {evidence:#?}");
    assert_eq!(paths(&evidence[0]), vec![link.clone()]);
    let probed: Vec<_> = index.probe_entries(&tree.root).into_iter().map(|e| e.path).collect();
    assert!(!probed.contains(&link), "{probed:?}");
}

/// The Windows file-side holder scan opens these entries; both indexes must
/// hand it the same regular files and directories, joined onto the root.
#[test]
fn walked_and_supplied_indexes_hand_the_holder_scan_the_same_probe_entries() {
    let tree = tree();
    let budget = Budget::start(Duration::from_secs(60)).unwrap();
    let root = root::resolve(&tree.root, &budget).unwrap();
    let walked = tree::collect(&root, true, &budget).index;
    let mut builder = caller_builder(&tree.root);
    builder
        .entry("link", TreeEntryKind::Link, FileIdentity::Unix { device: 0, inode: 1 })
        .entry("socket", TreeEntryKind::Other, FileIdentity::Unix { device: 0, inode: 2 });
    let supplied = builder.build().unwrap();
    let entries = |index: &TreeIndex| {
        let mut entries: Vec<_> = index
            .probe_entries(&root.resolved)
            .into_iter()
            .map(|e| (e.path, e.identity, e.directory))
            .collect();
        entries.sort();
        entries
    };
    let expected = entries(&walked);
    assert_eq!(entries(&supplied), expected, "links and special files are never probed");
    assert!(expected.contains(&(root.resolved.clone(), root.identity, true)), "the root is probed");
    assert_eq!(expected.len(), 4, "root, a.txt, sub, sub/b.txt: {expected:#?}");
}

#[test]
fn the_index_builder_rejects_each_invalid_entry_by_name() {
    let identity = |inode| FileIdentity::Unix { device: 1, inode };
    let build = |entries: &[(&str, TreeEntryKind)]| {
        let mut builder = TreeIndex::builder(identity(0));
        for (position, (path, kind)) in entries.iter().enumerate() {
            builder.entry(*path, *kind, identity(position as u64 + 1));
        }
        builder.build()
    };
    let file = TreeEntryKind::File;
    let link = TreeEntryKind::Link;
    let absolute = std::env::temp_dir().join("x");
    let absolute = absolute.to_str().unwrap();
    let cases: Vec<(Vec<(&str, TreeEntryKind)>, TreeIndexError)> = vec![
        (vec![("a", file), (absolute, file)], TreeIndexError::AbsolutePath { path: absolute.into() }),
        (vec![("a/../../x", file)], TreeIndexError::ParentTraversal { path: "a/../../x".into() }),
        (vec![("./a", file)], TreeIndexError::NotNormalized { path: "./a".into() }),
        (vec![("", file)], TreeIndexError::NotNormalized { path: "".into() }),
        (vec![("a/b", file), ("a//b/", file)], TreeIndexError::Duplicate { path: "a/b".into() }),
        (vec![("l", link), ("l/inside", file)], TreeIndexError::BelowLink { path: "l/inside".into(), link: "l".into() }),
        (vec![("l/deep/inside", file), ("l", link)], TreeIndexError::BelowLink { path: "l/deep/inside".into(), link: "l".into() }),
    ];
    for (entries, expected) in cases {
        let error = build(&entries).unwrap_err();
        assert_eq!(error, expected, "{entries:?}");
        assert!(!error.to_string().is_empty());
    }

    // Controls: a sibling whose name only starts with the link's, and a
    // name that is a link's prefix, are not below it; `a/./b` is `a/b`.
    let index = build(&[("l", link), ("l.txt", file), ("lx/y", file), ("a/./b", file)]).unwrap();
    assert_eq!(index.len(), 4);
    assert_eq!(index.root(), identity(0));
    assert_eq!(index.partial_reason(), None);
    assert_eq!(index.paths(&identity(4), Path::new("/r")), Some(vec![PathBuf::from("/r/a/b")]));
}

#[test]
fn the_index_keeps_every_hard_link_alias_and_sorts_them_under_the_root() {
    let shared = FileIdentity::Unix { device: 1, inode: 9 };
    let mut builder = TreeIndex::builder(FileIdentity::Unix { device: 1, inode: 1 });
    builder
        .entry("z/copy", TreeEntryKind::File, shared)
        .entry("a", TreeEntryKind::File, shared);
    let index = builder.build().unwrap();
    let root = Path::new("/work/app");
    assert_eq!(index.paths(&shared, root), Some(vec![root.join("a"), root.join("z/copy")]));
}

#[cfg(unix)]
#[test]
fn the_index_keeps_native_names_without_lossy_conversion() {
    use std::os::unix::ffi::OsStrExt;
    let name = non_utf8();
    let identity = FileIdentity::Unix { device: 1, inode: 2 };
    let mut builder = TreeIndex::builder(FileIdentity::Unix { device: 1, inode: 1 });
    builder.entry(Path::new("dir").join(&name), TreeEntryKind::File, identity);
    let index = builder.build().unwrap();
    let paths = index.paths(&identity, Path::new("/r")).unwrap();
    assert!(paths[0].as_os_str().as_bytes().ends_with(name.as_bytes()));
    assert!(paths[0].to_str().is_none(), "never converted through UTF-8");
}

// ---------------------------------------------------------------------------
// Serialization
// ---------------------------------------------------------------------------

#[cfg(unix)]
fn non_utf8() -> OsString {
    use std::os::unix::ffi::OsStringExt;
    OsString::from_vec(b"caf\xe9/\xff".to_vec())
}

#[test]
fn a_unicode_native_string_serializes_as_display_only() {
    let value = NativeString::from("naïve/日本語");
    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(json, serde_json::json!({ "display": "naïve/日本語" }));
    assert_eq!(serde_json::from_value::<NativeString>(json).unwrap(), value);
}

#[cfg(unix)]
#[test]
fn a_non_utf8_unix_path_round_trips_losslessly() {
    let value = NativeString::from(non_utf8());
    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "display": "caf\u{fffd}/\u{fffd}",
            "native": { "encoding": "unix_bytes", "units": [99, 97, 102, 233, 47, 255] }
        })
    );
    let text = serde_json::to_string(&value).unwrap();
    let back: NativeString = serde_json::from_str(&text).unwrap();
    assert_eq!(back, value);
    let again: NativeString = serde_json::from_str(&serde_json::to_string(&back).unwrap()).unwrap();
    assert_eq!(again, value);
}

#[cfg(windows)]
#[test]
fn an_unpaired_windows_surrogate_round_trips_losslessly() {
    use std::os::windows::ffi::OsStringExt;
    let value = NativeString::from(OsString::from_wide(&[0x61, 0xD800, 0x62]));
    let json = serde_json::to_value(&value).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "display": "a\u{fffd}b",
            "native": { "encoding": "windows_utf16", "units": [97, 55296, 98] }
        })
    );
    let back: NativeString = serde_json::from_value(json).unwrap();
    assert_eq!(back, value);
}

#[test]
fn foreign_native_encodings_decode_only_when_representable() {
    #[cfg(unix)]
    let (foreign, valid, invalid) = ("windows_utf16", "[104, 105]", "[55296]");
    #[cfg(windows)]
    let (foreign, valid, invalid) = ("unix_bytes", "[104, 105]", "[255]");
    let parse = |units: &str| {
        serde_json::from_str::<NativeString>(&format!(
            r#"{{"display": "?", "native": {{"encoding": "{foreign}", "units": {units}}}}}"#
        ))
    };
    assert_eq!(parse(valid).unwrap(), NativeString::from("hi"));
    assert!(parse(invalid).is_err());
}

#[cfg(target_os = "linux")]
#[test]
fn a_non_utf8_file_in_the_tree_matches_and_its_report_round_trips() {
    let tree = tree();
    let file = tree.root.join(non_utf8_name());
    std::fs::write(&file, "x").unwrap();
    let mut fake = Fake::new(vec![candidate(1, Some(1), vec![found(Mechanism::OpenHandles, vec![handle(&file, 3)])])]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    let evidence = &report.processes[0].evidence[0];
    assert_eq!(paths(evidence), vec![file]);
    assert!(!evidence.matched_paths[0].is_unicode());
    let back: PathUsageReport = serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(back, report);
}

#[cfg(target_os = "linux")]
fn non_utf8_name() -> PathBuf {
    use std::os::unix::ffi::OsStringExt;
    PathBuf::from(OsString::from_vec(b"caf\xe9".to_vec()))
}

#[test]
fn identifiers_serialize_as_decimal_strings() {
    let identity = FileIdentity::Windows {
        volume_serial: u64::MAX,
        file_id: u128::MAX,
    };
    let json = serde_json::to_value(identity).unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "kind": "windows",
            "volume_serial": "18446744073709551615",
            "file_id": "340282366920938463463374607431768211455"
        })
    );
    assert_eq!(serde_json::from_value::<FileIdentity>(json).unwrap(), identity);
}

/// A report from a real query, with every nullable field exercised.
fn fixture_report() -> (Tree, PathUsageReport) {
    let tree = tree();
    let a = tree.root.join("a.txt");
    let mut fake = Fake::new(vec![
        candidate(1, Some(u64::MAX), vec![found(Mechanism::OpenHandles, vec![handle(&a, 3)])]),
        candidate(2, None, vec![found(Mechanism::OpenHandles, vec![handle(&a, 4)])]),
    ]);
    let report = query(&tree.root, &PathUsageOptions::default(), &mut fake);
    (tree, report)
}

#[test]
fn unavailable_fields_serialize_as_explicit_nulls() {
    let (_tree, report) = fixture_report();
    let json = serde_json::to_value(&report).unwrap();
    let uncertain = &json["processes"][1];
    for key in ["creation_token", "start_time", "name", "executable", "user"] {
        assert!(uncertain.as_object().unwrap().contains_key(key), "{key} omitted");
        assert!(uncertain[key].is_null(), "{key}");
    }
    assert_eq!(json["processes"][0]["creation_token"], "18446744073709551615");
    assert_eq!(json["schema_version"], SCHEMA_VERSION);
    assert_eq!(json["index_provenance"], "walked");
    let evidence = json["processes"][0]["evidence"][0].as_object().unwrap();
    for key in ["watch", "access", "event_only"] {
        assert!(evidence[key].is_null(), "{key}");
    }
}

fn edit(document: &mut serde_json::Value, pointer: &str, change: &Change) {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let parent = document.pointer_mut(parent).unwrap_or_else(|| panic!("{parent}"));
    match change {
        Change::Remove => {
            parent.as_object_mut().unwrap().remove(key).expect("field present");
        }
        Change::Set(value) => {
            *parent.get_mut(key).unwrap_or_else(|| panic!("{pointer}")) = value.clone();
        }
        Change::FirstElement(value) => {
            parent[key].as_array_mut().unwrap()[0] = value.clone();
        }
        Change::EveryElement(value) => {
            for element in parent[key].as_array_mut().unwrap() {
                *element = value.clone();
            }
        }
    }
}

enum Change {
    Remove,
    Set(serde_json::Value),
    FirstElement(serde_json::Value),
    EveryElement(serde_json::Value),
}

/// Duplicates `"key": value` inside the object at `pointer` in `text`.
fn duplicate_key(document: &serde_json::Value, pointer: &str) -> String {
    let (parent, key) = pointer.rsplit_once('/').unwrap();
    let value = document.pointer(pointer).unwrap();
    let marker = serde_json::json!("__DUPLICATE__");
    let mut copy = document.clone();
    copy.pointer_mut(parent)
        .unwrap()
        .as_object_mut()
        .unwrap()
        .insert("__dup__".to_string(), marker);
    serde_json::to_string(&copy)
        .unwrap()
        .replace(r#""__dup__":"__DUPLICATE__""#, &format!(r#""{key}":{value}"#))
}

/// Walks the input-robustness matrix for the report reader. Each row edits one
/// load-bearing field of a real serialized report; the expected outcome for
/// every cell is the table in the implementation log (Phase 2): the reader
/// rejects every edit except `[]` for an array, which stays a present, empty
/// collection.
#[test]
fn the_report_reader_rejects_every_malformed_shape_of_its_load_bearing_fields() {
    use serde_json::json;
    let (_tree, report) = fixture_report();
    let fixture = serde_json::to_value(&report).unwrap();

    // Control: the unedited fixture is read back exactly.
    let control: PathUsageReport = serde_json::from_value(fixture.clone()).expect("control");
    assert_eq!(control, report);

    let fields = [
        "/outcome",
        "/index_provenance",
        "/target/resolved/display",
        "/processes",
        "/processes/0/creation_token",
        "/processes/0/name",
        "/processes/0/evidence/0/matched_paths",
        "/coverage/2/status",
    ];
    let arrays = ["/processes", "/processes/0/evidence/0/matched_paths"];
    for field in fields {
        let is_array = arrays.contains(&field);
        let nullable = matches!(field, "/processes/0/creation_token" | "/processes/0/name");
        let mut cells: Vec<(&str, Change, bool)> = vec![
            ("absent", Change::Remove, false),
            ("null", Change::Set(json!(null)), nullable),
            ("wrong type", Change::Set(json!(123)), false),
        ];
        if is_array {
            cells.push(("one element wrong", Change::FirstElement(json!(123)), false));
            cells.push(("every element wrong", Change::EveryElement(json!(123)), false));
            cells.push(("empty", Change::Set(json!([])), true));
        } else {
            // An empty display string is a valid (empty) native value.
            let accepted = field == "/target/resolved/display";
            cells.push(("empty string", Change::Set(json!("")), accepted));
        }
        for (cell, change, accepted) in cells {
            let mut document = fixture.clone();
            edit(&mut document, field, &change);
            let parsed = serde_json::from_value::<PathUsageReport>(document);
            assert_eq!(parsed.is_ok(), accepted, "{field} / {cell}: {parsed:?}");
            if let (Ok(parsed), "null") = (&parsed, cell) {
                assert_ne!(parsed, &report, "{field}: null is read as unavailable");
            }
        }
        let duplicated = duplicate_key(&fixture, field);
        assert!(
            serde_json::from_str::<PathUsageReport>(&duplicated).is_err(),
            "{field} / duplicate key"
        );
    }

    let text = serde_json::to_string(&fixture).unwrap();
    assert!(serde_json::from_str::<PathUsageReport>(&format!("{text} {{}}")).is_err(), "trailing");
    let mut unknown = fixture.clone();
    unknown["surprise"] = json!(true);
    assert!(serde_json::from_value::<PathUsageReport>(unknown).is_err(), "unknown field");
}

/// The same matrix for the path object's own fields, which a hand-written
/// visitor reads.
#[test]
fn the_native_string_reader_rejects_every_malformed_shape() {
    let parse = |text: &str| serde_json::from_str::<NativeString>(text);
    // Control rows.
    assert_eq!(parse(r#"{"display": "a"}"#).unwrap(), NativeString::from("a"));
    let native = r#"{"display": "?", "native": {"encoding": "unix_bytes", "units": [104]}}"#;
    #[cfg(unix)]
    assert_eq!(parse(native).unwrap(), NativeString::from("h"));
    #[cfg(windows)]
    assert_eq!(parse(native).unwrap(), NativeString::from("h"));

    let rejected = [
        ("display absent", r#"{}"#),
        ("display null", r#"{"display": null}"#),
        ("display wrong type", r#"{"display": 1}"#),
        ("display duplicate", r#"{"display": "a", "display": "b"}"#),
        ("native null", r#"{"display": "a", "native": null}"#),
        ("native wrong type", r#"{"display": "a", "native": [1]}"#),
        ("native duplicate", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": [97]}, "native": {"encoding": "unix_bytes", "units": [97]}}"#),
        ("encoding absent", r#"{"display": "a", "native": {"units": [97]}}"#),
        ("encoding null", r#"{"display": "a", "native": {"encoding": null, "units": [97]}}"#),
        ("encoding unknown", r#"{"display": "a", "native": {"encoding": "ebcdic", "units": [97]}}"#),
        ("units absent", r#"{"display": "a", "native": {"encoding": "unix_bytes"}}"#),
        ("units null", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": null}}"#),
        ("units wrong type", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": "a"}}"#),
        ("one unit wrong type", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": [97, "b"]}}"#),
        ("every unit wrong type", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": ["a"]}}"#),
        ("byte out of range", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": [256]}}"#),
        ("utf16 out of range", r#"{"display": "a", "native": {"encoding": "windows_utf16", "units": [65536]}}"#),
        ("units duplicate", r#"{"display": "a", "native": {"encoding": "unix_bytes", "units": [97], "units": [98]}}"#),
        ("unknown key", r#"{"display": "a", "extra": 1}"#),
        ("trailing content", r#"{"display": "a"} x"#),
    ];
    for (cell, text) in rejected {
        assert!(parse(text).is_err(), "{cell} was accepted");
    }
    // `[]` is a present, empty value, not an absent one.
    let empty = parse(r#"{"display": "ignored", "native": {"encoding": "unix_bytes", "units": []}}"#);
    assert_eq!(empty.unwrap(), NativeString::from(""));
}

#[test]
fn decimal_identifiers_reject_non_canonical_strings() {
    for text in ["", "+1", " 1", "01", "1e3", "-1", "18446744073709551616"] {
        assert!(native::decimal::parse::<u64>(text).is_err(), "{text:?}");
    }
    assert_eq!(native::decimal::parse::<u64>("0"), Ok(0));
    assert_eq!(native::decimal::parse::<u64>("18446744073709551615"), Ok(u64::MAX));
}

/// The loader's exceptions to "every invalid shape is an error" for
/// `index_provenance`, which the matrix above walks for a current report.
#[test]
fn old_reports_load_with_unknown_provenance_and_round_trip() {
    use serde_json::json;
    let (_tree, report) = fixture_report();
    let current = serde_json::to_value(&report).unwrap();
    let mut old = current.clone();
    old["schema_version"] = json!(1);
    old.as_object_mut().unwrap().remove("index_provenance");

    let loaded: PathUsageReport = serde_json::from_value(old.clone()).expect("a version 1 report loads");
    assert_eq!(loaded.schema_version, 1);
    assert_eq!(loaded.index_provenance, IndexProvenance::Unknown, "never inferred as walked");
    assert_eq!(loaded.processes, report.processes);

    // Read, write, read: the old report keeps its version and its unknown
    // provenance, now written explicitly.
    let written = serde_json::to_string(&loaded).unwrap();
    let again: PathUsageReport = serde_json::from_str(&written).expect("rewritten old report loads");
    assert_eq!(again, loaded);
    let rewritten = serde_json::to_string(&again).unwrap();
    assert_eq!(rewritten, written);

    for (cell, value, accepted) in [
        ("explicit unknown", json!("unknown"), true),
        ("walked", json!("walked"), false),
        ("supplied", json!("supplied"), false),
        ("null", json!(null), false),
    ] {
        let mut document = old.clone();
        document["index_provenance"] = value;
        let parsed = serde_json::from_value::<PathUsageReport>(document);
        assert_eq!(parsed.is_ok(), accepted, "schema 1 / {cell}: {parsed:?}");
    }

    let mut missing = current.clone();
    missing.as_object_mut().unwrap().remove("index_provenance");
    let error = serde_json::from_value::<PathUsageReport>(missing).unwrap_err();
    assert!(error.to_string().contains("index_provenance"), "{error}");

    let mut supplied = report.clone();
    supplied.index_provenance = IndexProvenance::Supplied;
    let back: PathUsageReport = serde_json::from_str(&serde_json::to_string(&supplied).unwrap()).unwrap();
    assert_eq!(back.index_provenance, IndexProvenance::Supplied);
}

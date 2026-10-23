use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use sniff::filesystem::query::{
    Coverage, CoverageScope, MatchBasis, Truncation, WatchInfo,
};

use super::*;

const TARGET: &str = "/work/fix-path-spelling";
const OWN_PID: u32 = 4242;

/// A report about [`TARGET`] with no processes and no coverage records.
fn report(outcome: &str) -> PathUsageReport {
    serde_json::from_value(json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": TARGET},
            "resolved": {"display": TARGET},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": outcome,
        "started_at": "2026-10-12T00:00:00Z",
        "finished_at": "2026-10-12T00:00:01Z",
        "budget_ms": 2000,
        "elapsed_us": 1000,
        "budget_exhausted": false,
        "processes": [],
        "coverage": [],
        "limitations": []
    }))
    .unwrap()
}

fn coverage(mechanism: Mechanism, status: CoverageStatus, reason: Option<&str>) -> Coverage {
    Coverage {
        mechanism,
        scope: CoverageScope::TargetTree,
        status,
        attempted: None,
        succeeded: None,
        reason: reason.map(str::to_string),
        truncation: None,
        limitations: Vec::new(),
    }
}

fn evidence(kind: EvidenceKind, paths: &[&str], descriptor: Option<u64>) -> Evidence {
    Evidence {
        kind,
        mechanism: match kind {
            EvidenceKind::WatchRegistration => Mechanism::Inotify,
            EvidenceKind::OpenHandle => Mechanism::OpenHandles,
            EvidenceKind::WorkingDirectory => Mechanism::WorkingDirectories,
            EvidenceKind::LoadedModule => Mechanism::LoadedModules,
        },
        matched_paths: paths.iter().map(|path| NativeString::from(*path)).collect(),
        observed_path: paths.first().map(|path| NativeString::from(*path)),
        match_basis: MatchBasis::Identity,
        identity: Some(FileIdentity::Unix { device: 7, inode: 9 }),
        descriptor,
        watch: (kind == EvidenceKind::WatchRegistration)
            .then_some(WatchInfo { watch_id: Some(1), mask: Some(0x2), recursive: Some(false) }),
        access: None,
        event_only: None,
    }
}

fn process(pid: u32, token: Option<u64>, name: Option<&str>, evidence: Vec<Evidence>) -> ProcessRecord {
    ProcessRecord {
        pid,
        creation_token: token,
        start_time: None,
        identity_uncertain: token.is_none(),
        name: name.map(NativeString::from),
        executable: name.map(|name| NativeString::from(format!("/usr/bin/{name}").as_str())),
        user: Some("501".to_string()),
        evidence,
    }
}

fn with(mut report: PathUsageReport, processes: Vec<ProcessRecord>) -> PathUsageReport {
    report.processes = processes;
    report
}

fn own() -> OwnedHandles {
    let mut own = OwnedHandles::for_process(OWN_PID);
    own.register(EvidenceKind::WatchRegistration, 5);
    own
}

/// Each evidence kind from another process is a holder on its own, naming
/// the PID, its name and executable, the kind, and the exact matched path.
#[test]
fn every_evidence_kind_from_another_process_is_a_holder() {
    let path = format!("{TARGET}/.gitnexus/index.db");
    for kind in [
        EvidenceKind::WatchRegistration,
        EvidenceKind::OpenHandle,
        EvidenceKind::WorkingDirectory,
        EvidenceKind::LoadedModule,
    ] {
        let found = with(report("usable"), vec![process(8124, Some(77), Some("node"), vec![evidence(kind, &[&path], Some(5))])]);

        let found = assess(found, &own());

        assert_eq!(found.report.target.resolved, NativeString::from(TARGET), "{kind:?}");
        assert_eq!(found.holders.len(), 1, "{kind:?}");
        let holder = &found.holders[0];
        assert_eq!((holder.pid, holder.creation_token), (8124, Some(77)));
        assert_eq!(holder.name, Some(NativeString::from("node")));
        assert_eq!(holder.executable, Some(NativeString::from("/usr/bin/node")));
        assert_eq!(
            holder.evidence,
            [HolderEvidence { kind, location: EvidenceLocation::Path(NativeString::from(path.as_str())) }]
        );
    }
}

/// No match finds no holder whatever the coverage, and the coverage is kept
/// for the operation report.
#[test]
fn no_match_has_no_holder_whatever_the_coverage() {
    let complete = {
        let mut r = report("usable");
        r.coverage = vec![coverage(Mechanism::OpenHandles, CoverageStatus::Complete, None)];
        r
    };
    let partial = {
        let mut r = report("usable");
        r.budget_exhausted = true;
        r.coverage = vec![
            coverage(Mechanism::OpenHandles, CoverageStatus::Partial, Some("budget")),
            coverage(Mechanism::Fsevents, CoverageStatus::Unsupported, Some("FSEvents subscriptions cannot be enumerated")),
        ];
        r
    };
    let unavailable = {
        let mut r = report("unavailable");
        r.coverage = vec![coverage(Mechanism::OpenHandles, CoverageStatus::Failed, Some("permission denied"))];
        r
    };
    let unsupported = {
        let mut r = report("unsupported");
        r.coverage = vec![coverage(Mechanism::OpenHandles, CoverageStatus::Unsupported, None)];
        r
    };

    let cases = [
        (complete, Outcome::Usable, 0, true),
        (partial, Outcome::Usable, 2, false),
        (unavailable, Outcome::Unavailable, 1, false),
        (unsupported, Outcome::Unsupported, 1, false),
    ];
    for (found, outcome, gaps, is_complete) in cases {
        let cleared = assess(found.clone(), &own());
        assert!(cleared.holders.is_empty(), "{outcome:?}");
        assert_eq!(cleared.report, found, "the whole report is kept");
        assert_eq!(cleared.own_handles_excluded, 0);
        assert_eq!(cleared.coverage.outcome, outcome);
        assert_eq!(cleared.coverage.gaps.len(), gaps, "{outcome:?}");
        assert_eq!(cleared.coverage.is_complete(), is_complete, "{outcome:?}");
    }
}

/// Partial coverage never weakens a match, and the coverage is kept so it
/// can be described.
#[test]
fn a_match_in_a_partial_report_is_a_holder_with_its_coverage() {
    let mut found = with(
        report("usable"),
        vec![process(8124, Some(77), Some("node"), vec![evidence(EvidenceKind::OpenHandle, &[TARGET], Some(9))])],
    );
    found.budget_exhausted = true;
    let mut fsevents = coverage(Mechanism::Fsevents, CoverageStatus::Unsupported, Some("not enumerable"));
    fsevents.scope = CoverageScope::VisibleProcesses;
    let mut handles = coverage(Mechanism::OpenHandles, CoverageStatus::Partial, None);
    handles.truncation = Some(Truncation { per_process_limit: 1000, total_limit: 10_000, omitted: 12 });
    found.coverage = vec![handles, fsevents];

    let refusal = assess(found, &own());

    assert_eq!(refusal.holders.len(), 1);
    assert!(refusal.coverage.budget_exhausted);
    assert_eq!(refusal.coverage.evidence_omitted, 12);
    assert_eq!(
        refusal.coverage.gaps,
        [
            CoverageGap { mechanism: Mechanism::OpenHandles, status: CoverageStatus::Partial, reason: None },
            CoverageGap {
                mechanism: Mechanism::Fsevents,
                status: CoverageStatus::Unsupported,
                reason: Some("not enumerable".to_string()),
            },
        ]
    );
}

/// Exclusion: only a registered handle, under this process's PID and one
/// certain lifetime, is dropped; every near miss is a holder.
#[test]
fn only_this_operations_registered_handles_are_excluded() {
    let watch = |descriptor| evidence(EvidenceKind::WatchRegistration, &[TARGET], Some(descriptor));
    // Ok: evidence records excluded; Err: the PIDs found holding it.
    type Expected = Result<u64, Vec<u32>>;
    let cases: Vec<(&str, Vec<ProcessRecord>, Expected)> = vec![
        ("registered handle of this process", vec![process(OWN_PID, Some(1), Some("wt"), vec![watch(5)])], Ok(1)),
        (
            "the same descriptor in another process",
            vec![process(8124, Some(1), Some("node"), vec![watch(5)])],
            Err(vec![8124]),
        ),
        (
            "an unregistered descriptor of this process",
            vec![process(OWN_PID, Some(1), Some("wt"), vec![watch(6)])],
            Err(vec![OWN_PID]),
        ),
        (
            "a registered descriptor of another kind",
            vec![process(OWN_PID, Some(1), Some("wt"), vec![evidence(EvidenceKind::OpenHandle, &[TARGET], Some(5))])],
            Err(vec![OWN_PID]),
        ),
        (
            "evidence with no descriptor",
            vec![process(OWN_PID, Some(1), Some("wt"), vec![evidence(EvidenceKind::WorkingDirectory, &[TARGET], None)])],
            Err(vec![OWN_PID]),
        ),
        (
            "a record not tied to one lifetime",
            vec![process(OWN_PID, None, None, vec![watch(5)])],
            Err(vec![OWN_PID]),
        ),
        (
            "two lifetimes claiming this PID",
            vec![
                process(OWN_PID, Some(1), Some("wt"), vec![watch(5)]),
                process(OWN_PID, Some(2), Some("other"), vec![watch(5)]),
            ],
            Err(vec![OWN_PID, OWN_PID]),
        ),
    ];
    for (case, processes, expected) in cases {
        let found = assess(with(report("usable"), processes), &own());
        match expected {
            Ok(excluded) => {
                assert!(found.holders.is_empty(), "{case}: {:?}", found.holders);
                assert_eq!(found.own_handles_excluded, excluded, "{case}");
            }
            Err(pids) => assert_eq!(found.holders.iter().map(|h| h.pid).collect::<Vec<_>>(), pids, "{case}"),
        }
    }
}

/// A registered handle is dropped, while the same process's other evidence,
/// such as `wt` running from inside the checkout, is still reported.
#[test]
fn this_processes_other_evidence_is_still_reported() {
    let found = with(
        report("usable"),
        vec![process(
            OWN_PID,
            Some(1),
            Some("wt"),
            vec![
                evidence(EvidenceKind::WatchRegistration, &[TARGET], Some(5)),
                evidence(EvidenceKind::LoadedModule, &[&format!("{TARGET}/target/debug/wt")], None),
            ],
        )],
    );

    let refusal = assess(found, &own());

    assert_eq!(refusal.holders.len(), 1);
    let kinds: Vec<_> = refusal.holders[0].evidence.iter().map(|e| e.kind).collect();
    assert_eq!(kinds, [EvidenceKind::LoadedModule]);
}

/// Several in-scope names and identity-only evidence are reported as they
/// are; no single path is chosen or invented, and a record without a proven
/// lifetime gets no name.
#[test]
fn locations_report_aliases_and_identity_only_evidence_as_facts() {
    let first = format!("{TARGET}/a.txt");
    let second = format!("{TARGET}/b.txt");
    let aliases = evidence(EvidenceKind::OpenHandle, &[&first, &second], Some(3));
    let mut unnamed = evidence(EvidenceKind::OpenHandle, &[], Some(4));
    unnamed.observed_path = Some(NativeString::from("/elsewhere/link.txt"));
    let mut bare = evidence(EvidenceKind::WatchRegistration, &[], Some(5));
    bare.identity = None;
    bare.observed_path = None;
    let found = with(report("usable"), vec![process(8124, None, None, vec![aliases, unnamed, bare])]);

    let refusal = assess(found, &own());

    let holder = &refusal.holders[0];
    assert!(holder.identity_uncertain);
    assert_eq!((holder.name.as_ref(), holder.executable.as_ref()), (None, None));
    let locations: Vec<_> = holder.evidence.iter().map(|e| e.location.clone()).collect();
    assert_eq!(
        locations,
        [
            EvidenceLocation::Aliases(vec![NativeString::from(first.as_str()), NativeString::from(second.as_str())]),
            EvidenceLocation::Unnamed {
                identity: Some(FileIdentity::Unix { device: 7, inode: 9 }),
                observed: Some(NativeString::from("/elsewhere/link.txt")),
            },
            EvidenceLocation::Unnamed { identity: None, observed: None },
        ]
    );
}

/// A source that answers every query the same way and counts the calls.
struct Scripted<F: Fn() -> Result<PathUsageReport, PathUsageError> + Sync> {
    answer: F,
    calls: AtomicUsize,
}

impl<F: Fn() -> Result<PathUsageReport, PathUsageError> + Sync> UsageSource for Scripted<F> {
    fn query(&self, _path: &Path) -> Result<PathUsageReport, PathUsageError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        (self.answer)()
    }
}

/// Every typed query error refuses as unverified, matched by variant, after
/// exactly one query: an exhausted or failed scan is never retried.
#[test]
fn every_query_error_refuses_as_unverified_without_a_retry() {
    let path = PathBuf::from(TARGET);
    let errors: Vec<fn(PathBuf) -> PathUsageError> = vec![
        |_| PathUsageError::InvalidOption { message: "zero deadline".to_string() },
        |path| PathUsageError::MissingTarget { path },
        |path| PathUsageError::UnsupportedTargetKind { path, file_type: "fifo" },
        |path| PathUsageError::RootValidationTimeout { path, budget: Duration::from_secs(2) },
        |path| PathUsageError::RootIdentity { path, source: std::io::Error::from(std::io::ErrorKind::PermissionDenied) },
    ];
    for make in errors {
        let expected = make(path.clone()).kind();
        let source = Scripted { answer: || Err(make(path.clone())), calls: AtomicUsize::new(0) };

        let result = check_usage(&source, &path, &own());

        assert!(
            matches!(&result, Err(UsageRefusal::Unverified(error)) if error.kind() == expected),
            "{expected}: {result:?}"
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 1, "{expected}");
    }

    let source = Scripted { answer: || Ok(report("unsupported")), calls: AtomicUsize::new(0) };
    assert!(check_usage(&source, &path, &own()).is_ok());
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
}

/// Through the real Sniff query: a process standing in a subdirectory of the
/// checkout is named by PID; the empty tree beside it has no holder.
#[test]
fn a_real_query_names_a_process_working_in_the_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let held = dir.path().join("held");
    let inside = held.join("sub");
    std::fs::create_dir_all(&inside).unwrap();
    let free = dir.path().join("free");
    std::fs::create_dir(&free).unwrap();
    // Spawned directly, not through `cmd /C`: killing `cmd` would leave its
    // `ping` child holding the directory.
    #[cfg(windows)]
    let mut command = std::process::Command::new("ping");
    #[cfg(windows)]
    command.args(["-n", "60", "127.0.0.1"]);
    #[cfg(unix)]
    let mut command = std::process::Command::new("sleep");
    #[cfg(unix)]
    command.arg("60");
    // On Unix the child's working directory is set before `spawn` returns.
    // On Windows the child opens its directory handle while it starts, after
    // `CreateProcess` returns, so wait for `ping`'s first output: by then it
    // runs its own code and holds the handle.
    let mut holder = command
        .current_dir(&inside)
        .stdout(if cfg!(windows) { std::process::Stdio::piped() } else { std::process::Stdio::null() })
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    #[cfg(windows)]
    {
        use std::io::Read as _;
        let mut first = [0_u8; 1];
        holder.stdout.as_mut().unwrap().read_exact(&mut first).unwrap();
    }

    let refused = check_usage(&SniffUsage, &held, &OwnedHandles::default());
    let cleared = check_usage(&SniffUsage, &free, &OwnedHandles::default());

    let _ = holder.kill();
    let _ = holder.wait();
    let refusal = refused.unwrap();
    let found = refusal.holders.iter().find(|h| h.pid == holder.id()).unwrap_or_else(|| panic!("{refusal:#?}"));
    // Windows sees the directory handle a working directory holds; Unix sees
    // the working directory itself.
    #[cfg(unix)]
    assert!(found.evidence.iter().any(|e| e.kind == EvidenceKind::WorkingDirectory), "{found:#?}");
    #[cfg(windows)]
    assert!(!found.evidence.is_empty(), "{found:#?}");
    let cleared = cleared.unwrap_or_else(|refusal| panic!("{refusal:?}"));
    assert!(cleared.holders.is_empty(), "{:?}", cleared.holders);
    assert_eq!(cleared.report.target.kind, sniff::filesystem::query::TargetKind::Directory);
}

/// Through the real Sniff query: a checkout that is gone by the time of the
/// query refuses as unverified rather than clearing.
#[test]
fn a_real_query_of_a_vanished_checkout_refuses() {
    let dir = tempfile::tempdir().unwrap();

    let result = check_usage(&SniffUsage, &dir.path().join("gone"), &OwnedHandles::default());

    assert!(matches!(result, Err(UsageRefusal::Unverified(PathUsageError::MissingTarget { .. }))), "{result:?}");
}

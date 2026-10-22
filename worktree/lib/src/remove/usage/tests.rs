use std::path::PathBuf;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use serde_json::json;
use sniff::filesystem::query::{
    Coverage, CoverageScope, IndexProvenance, MatchBasis, Truncation, WatchInfo,
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

fn in_use(result: Result<UsageClearance, UsageRefusal>) -> InUse {
    match result {
        Err(UsageRefusal::InUse(in_use)) => *in_use,
        other => panic!("expected InUse, got {other:?}"),
    }
}

/// Each evidence kind from another process refuses on its own, naming the
/// PID, its name and executable, the kind, and the exact matched path.
#[test]
fn every_evidence_kind_from_another_process_refuses() {
    let path = format!("{TARGET}/.gitnexus/index.db");
    for kind in [
        EvidenceKind::WatchRegistration,
        EvidenceKind::OpenHandle,
        EvidenceKind::WorkingDirectory,
        EvidenceKind::LoadedModule,
    ] {
        let found = with(report("usable"), vec![process(8124, Some(77), Some("node"), vec![evidence(kind, &[&path], Some(5))])]);

        let refusal = in_use(assess(found, &own()));

        assert_eq!(refusal.target, NativeString::from(TARGET), "{kind:?}");
        assert_eq!(refusal.holders.len(), 1, "{kind:?}");
        let holder = &refusal.holders[0];
        assert_eq!((holder.pid, holder.creation_token), (8124, Some(77)));
        assert_eq!(holder.name, Some(NativeString::from("node")));
        assert_eq!(holder.executable, Some(NativeString::from("/usr/bin/node")));
        assert_eq!(
            holder.evidence,
            [HolderEvidence { kind, location: EvidenceLocation::Path(NativeString::from(path.as_str())) }]
        );
    }
}

/// No match continues whatever the coverage, and the coverage is kept for
/// the operation report.
#[test]
fn no_match_clears_whatever_the_coverage() {
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
        let cleared = assess(found.clone(), &own()).unwrap_or_else(|refusal| panic!("{outcome:?}: {refusal:?}"));
        assert_eq!(cleared.report, found, "the whole report is kept");
        assert_eq!(cleared.own_handles_excluded, 0);
        assert_eq!(cleared.coverage.outcome, outcome);
        assert_eq!(cleared.coverage.gaps.len(), gaps, "{outcome:?}");
        assert_eq!(cleared.coverage.is_complete(), is_complete, "{outcome:?}");
    }
}

/// Partial coverage never weakens a match, and the refusal carries the
/// coverage so it can be described.
#[test]
fn a_match_in_a_partial_report_refuses_with_its_coverage() {
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

    let refusal = in_use(assess(found, &own()));

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
/// certain lifetime, is dropped; every near miss refuses.
#[test]
fn only_this_operations_registered_handles_are_excluded() {
    let watch = |descriptor| evidence(EvidenceKind::WatchRegistration, &[TARGET], Some(descriptor));
    // Ok: evidence records excluded; Err: the PIDs refused.
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
        let result = assess(with(report("usable"), processes), &own());
        match expected {
            Ok(excluded) => {
                let cleared = result.unwrap_or_else(|refusal| panic!("{case}: {refusal:?}"));
                assert_eq!(cleared.own_handles_excluded, excluded, "{case}");
            }
            Err(pids) => {
                let refusal = in_use(result);
                assert_eq!(refusal.holders.iter().map(|h| h.pid).collect::<Vec<_>>(), pids, "{case}");
            }
        }
    }
}

/// A registered handle is dropped, while the same process's other evidence,
/// such as `wt` running from inside the checkout, still refuses.
#[test]
fn this_processes_other_evidence_still_refuses() {
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

    let refusal = in_use(assess(found, &own()));

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

    let refusal = in_use(assess(found, &own()));

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
    fn query(&self, _path: &Path, _index: Option<&Arc<TreeIndex>>) -> Result<PathUsageReport, PathUsageError> {
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

        let result = check_usage(&source, &path, &own(), None);

        assert!(
            matches!(&result, Err(UsageRefusal::Unverified(error)) if error.kind() == expected),
            "{expected}: {result:?}"
        );
        assert_eq!(source.calls.load(Ordering::SeqCst), 1, "{expected}");
    }

    let source = Scripted { answer: || Ok(report("unsupported")), calls: AtomicUsize::new(0) };
    assert!(check_usage(&source, &path, &own(), None).is_ok());
    assert_eq!(source.calls.load(Ordering::SeqCst), 1);
}

/// Through the real Sniff query: a process standing in a subdirectory of the
/// checkout refuses and is named by PID; the empty tree beside it clears.
/// The checkout's own inventory as the tree index finds the same holder
/// without a walk, and an index of another tree is never used for it.
#[test]
fn a_real_query_names_a_process_working_in_the_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let held = dir.path().join("held");
    let inside = held.join("sub");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(inside.join("file.rs"), "fn main() {}\n").unwrap();
    let free = dir.path().join("free");
    std::fs::create_dir(&free).unwrap();
    let inventory = crate::remove::observation::inventory::DeletionInventory::capture(&held, &[]).unwrap();
    let index = Arc::new(inventory.tree_index());
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

    let refused = check_usage(&SniffUsage, &held, &OwnedHandles::default(), None);
    let supplied = check_usage(&SniffUsage, &held, &OwnedHandles::default(), Some(&index));
    let cleared = check_usage(&SniffUsage, &free, &OwnedHandles::default(), None);
    let elsewhere = check_usage(&SniffUsage, &free, &OwnedHandles::default(), Some(&index));

    let _ = holder.kill();
    let _ = holder.wait();
    for (result, provenance) in [(refused, IndexProvenance::Walked), (supplied, IndexProvenance::Supplied)] {
        let refusal = in_use(result);
        assert_eq!(refusal.report.index_provenance, provenance);
        let found = refusal.holders.iter().find(|h| h.pid == holder.id()).unwrap_or_else(|| panic!("{refusal:#?}"));
        // Windows sees the directory handle a working directory holds; Unix
        // sees the working directory itself.
        #[cfg(unix)]
        assert!(found.evidence.iter().any(|e| e.kind == EvidenceKind::WorkingDirectory), "{found:#?}");
        #[cfg(windows)]
        assert!(!found.evidence.is_empty(), "{found:#?}");
        let enumeration = refusal.report.coverage.iter().find(|c| c.mechanism == Mechanism::ProcessEnumeration).expect("listed");
        assert_ne!(enumeration.status, CoverageStatus::NotAttempted, "{provenance:?}: processes were reached");
    }
    let cleared = cleared.unwrap_or_else(|refusal| panic!("{refusal:?}"));
    assert_eq!(cleared.report.target.kind, sniff::filesystem::query::TargetKind::Directory);
    assert!(
        matches!(&elsewhere, Err(UsageRefusal::Unverified(error)) if error.kind() == "invalid_option"),
        "an index of another root: {elsewhere:?}"
    );
}

/// Through the real Sniff query: a checkout that is gone by the time of the
/// query refuses as unverified rather than clearing.
#[test]
fn a_real_query_of_a_vanished_checkout_refuses() {
    let dir = tempfile::tempdir().unwrap();

    let result = check_usage(&SniffUsage, &dir.path().join("gone"), &OwnedHandles::default(), None);

    assert!(matches!(result, Err(UsageRefusal::Unverified(PathUsageError::MissingTarget { .. }))), "{result:?}");
}

// ---- Windows Search's one recheck ------------------------------------------

const SYSTEM32: &str = r"C:\Windows\System32";

/// A holder with an open file in the checkout, running `image`.
fn holder_running(pid: u32, image: Option<&str>) -> Holder {
    Holder {
        pid,
        creation_token: Some(91),
        identity_uncertain: false,
        name: image.and_then(|image| image.rsplit('\\').next()).map(NativeString::from),
        executable: image.map(NativeString::from),
        evidence: vec![HolderEvidence {
            kind: EvidenceKind::OpenHandle,
            location: EvidenceLocation::Path(NativeString::from(format!("{TARGET}/notes.txt").as_str())),
        }],
    }
}

/// Only an image named on the list and running from the system directory
/// itself is Windows Search, by its full path, whatever the case.
#[test]
fn windows_search_is_identified_by_its_full_image_path() {
    let rows: [(Option<&str>, bool); 13] = [
        (Some(r"C:\Windows\System32\SearchProtocolHost.exe"), true),
        (Some(r"C:\Windows\System32\SearchFilterHost.exe"), true),
        (Some(r"C:\WINDOWS\system32\SearchIndexer.exe"), true),
        (Some(r"C:\Windows\System32\searchprotocolhost.EXE"), true),
        (Some(r"\\?\C:\Windows\System32\SearchProtocolHost.exe"), true),
        // The same name anywhere else is an ordinary holder.
        (Some(r"C:\Users\ken\Downloads\SearchProtocolHost.exe"), false),
        (Some(r"C:\Windows\System32\sub\SearchIndexer.exe"), false),
        (Some(r"C:\Windows\SysWOW64\SearchProtocolHost.exe"), false),
        (Some(r"D:\Windows\System32\SearchProtocolHost.exe"), false),
        (Some("SearchProtocolHost.exe"), false),
        // Another system process, or no image path at all.
        (Some(r"C:\Windows\System32\svchost.exe"), false),
        (Some(r"C:\Windows\System32\SearchProtocolHost.exe.bak"), false),
        (None, false),
    ];
    for (image, expected) in rows {
        let holder = holder_running(6808, image);
        assert_eq!(is_windows_search(&holder, &NativeString::from(SYSTEM32)), expected, "{image:?}");
        assert_eq!(is_windows_search(&holder, &NativeString::from(r"\\?\C:\Windows\System32\")), expected, "{image:?}, verbatim directory");
    }
    let uncertain = Holder { identity_uncertain: true, ..holder_running(6808, Some(r"C:\Windows\System32\SearchIndexer.exe")) };
    assert!(!is_windows_search(&uncertain, &NativeString::from(SYSTEM32)), "a record not tied to one process lifetime");
    let found = holder_running(6808, Some(r"C:\Windows\System32\SearchIndexer.exe"));
    assert!(!is_windows_search(&found, &NativeString::from("")), "an empty system directory matches nothing");
}

/// A Sniff process record for Windows Search's protocol host.
fn search_process(pid: u32) -> ProcessRecord {
    let path = format!("{TARGET}/notes.txt");
    ProcessRecord {
        executable: Some(NativeString::from(r"C:\Windows\System32\SearchProtocolHost.exe")),
        ..process(pid, Some(91), Some("SearchProtocolHost.exe"), vec![evidence(EvidenceKind::OpenHandle, &[&path], None)])
    }
}

/// Another process with an open file in the checkout.
fn node() -> ProcessRecord {
    let path = format!("{TARGET}/.gitnexus/index.db");
    process(8124, Some(77), Some("node"), vec![evidence(EvidenceKind::OpenHandle, &[&path], None)])
}

/// Answers in order (the last one repeats); `None` is a failed query.
struct Sequence {
    answers: Vec<Option<Vec<ProcessRecord>>>,
    system: Option<&'static str>,
    calls: AtomicUsize,
    pauses: std::sync::Mutex<Vec<Duration>>,
}

impl Sequence {
    fn new(system: Option<&'static str>, answers: Vec<Option<Vec<ProcessRecord>>>) -> Self {
        Self { answers, system, calls: AtomicUsize::new(0), pauses: std::sync::Mutex::new(Vec::new()) }
    }

    fn counts(&self) -> (usize, usize) {
        (self.calls.load(Ordering::SeqCst), self.pauses.lock().unwrap().len())
    }
}

impl UsageSource for Sequence {
    fn query(&self, path: &Path, _index: Option<&Arc<TreeIndex>>) -> Result<PathUsageReport, PathUsageError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        match &self.answers[call.min(self.answers.len() - 1)] {
            Some(processes) => Ok(with(report("usable"), processes.clone())),
            None => Err(PathUsageError::MissingTarget { path: path.to_path_buf() }),
        }
    }

    fn system_directory(&self) -> Option<NativeString> {
        self.system.map(NativeString::from)
    }

    fn pause(&self, wait: Duration) {
        self.pauses.lock().unwrap().push(wait);
    }
}

/// The recheck applies only where the source names a system directory (on
/// a Windows host), and only to Windows Search by its full image path.
#[test]
fn only_windows_search_in_the_system_directory_is_rechecked() {
    let path = PathBuf::from(TARGET);
    let elsewhere = ProcessRecord {
        executable: Some(NativeString::from(r"C:\Temp\SearchProtocolHost.exe")),
        ..search_process(6808)
    };
    let rows: [(&str, Option<&'static str>, Vec<ProcessRecord>); 3] = [
        ("no system directory (every OS but Windows)", None, vec![search_process(6808)]),
        ("a same-named image elsewhere", Some(SYSTEM32), vec![elsewhere]),
        ("Search beside another holder", Some(SYSTEM32), vec![search_process(6808), node()]),
    ];
    for (row, system, first) in rows {
        let source = Sequence::new(system, vec![Some(first), Some(Vec::new())]);

        let refusal = in_use(check_usage(&source, &path, &own(), None));

        assert!(refusal.search_recheck.is_none(), "{row}");
        assert_eq!(source.counts(), (1, 0), "{row}: no pause and no second query");
    }
}

/// After the one recheck, its query decides: clear goes ahead with the
/// first report kept, another holder or a failed query refuses, and there is
/// never a third query.
#[test]
fn the_one_recheck_decides() {
    let path = PathBuf::from(TARGET);
    let search_only = || Some(vec![search_process(6808)]);

    let cleared = Sequence::new(Some(SYSTEM32), vec![search_only(), Some(Vec::new())]);
    let clearance = check_usage(&cleared, &path, &own(), None).unwrap_or_else(|refusal| panic!("{refusal:?}"));
    let recheck = clearance.search_recheck.expect("the recheck is recorded");
    assert_eq!((recheck.held, recheck.waited), (1, SEARCH_RECHECK_WAIT));
    assert_eq!(recheck.first.processes[0].pid, 6808);
    assert_eq!(recheck.summary(true), "Windows Search held 1 file(s); rechecked after 500 ms; released");
    assert_eq!(*cleared.pauses.lock().unwrap(), [SEARCH_RECHECK_WAIT]);
    assert_eq!(cleared.counts(), (2, 1));

    let still = Sequence::new(Some(SYSTEM32), vec![search_only(), search_only()]);
    let refusal = in_use(check_usage(&still, &path, &own(), None));
    assert!(refusal.search_recheck.is_some());
    assert_eq!(still.counts(), (2, 1), "Search again at the recheck is never rechecked again");

    let other = Sequence::new(Some(SYSTEM32), vec![search_only(), Some(vec![node()])]);
    let refusal = in_use(check_usage(&other, &path, &own(), None));
    assert_eq!(refusal.holders[0].pid, 8124);
    assert_eq!(other.counts(), (2, 1));

    let failed = Sequence::new(Some(SYSTEM32), vec![search_only(), None]);
    assert!(matches!(check_usage(&failed, &path, &own(), None), Err(UsageRefusal::Unverified(_))));
    assert_eq!(failed.counts(), (2, 1));
}

/// On Windows the host's system directory is where its Windows Search
/// processes run from.
#[cfg(windows)]
#[test]
fn the_host_system_directory_is_system32() {
    let system = SniffUsage.system_directory().expect("GetSystemDirectoryW");
    let expected = std::path::Path::new(&std::env::var_os("SystemRoot").expect("SystemRoot")).join("System32");
    assert!(system.display().eq_ignore_ascii_case(&expected.to_string_lossy()), "{system:?} vs {expected:?}");
    let search = holder_running(1, Some(&format!(r"{}\SearchIndexer.exe", system.display())));
    assert!(is_windows_search(&search, &system));
}

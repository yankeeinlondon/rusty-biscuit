use serde_json::json;
use sniff::filesystem::query::{Coverage, CoverageScope, MatchBasis, ProcessRecord, Truncation, WatchInfo};

use super::*;

const TARGET: &str = "/work/fix-path-spelling";

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
        parent_pid: None,
        app_name: None,
        ancestors: Vec::new(),
        ancestry_end: Default::default(),
        evidence,
    }
}

fn with(mut report: PathUsageReport, processes: Vec<ProcessRecord>) -> PathUsageReport {
    report.processes = processes;
    report
}

/// Each evidence kind makes a holder on its own, named by PID, its name and
/// executable, the kind, and the exact matched path.
#[test]
fn every_evidence_kind_makes_a_holder() {
    let path = format!("{TARGET}/.gitnexus/index.db");
    for kind in [
        EvidenceKind::WatchRegistration,
        EvidenceKind::OpenHandle,
        EvidenceKind::WorkingDirectory,
        EvidenceKind::LoadedModule,
    ] {
        let found = with(report("usable"), vec![process(8124, Some(77), Some("node"), vec![evidence(kind, &[&path], Some(5))])]);

        let holders = holders(&found);

        assert_eq!(holders.len(), 1, "{kind:?}");
        let holder = &holders[0];
        assert_eq!((holder.pid, holder.creation_token), (8124, Some(77)));
        assert_eq!(holder.name, Some(NativeString::from("node")));
        assert_eq!(holder.executable, Some(NativeString::from("/usr/bin/node")));
        assert_eq!(
            holder.evidence,
            [HolderEvidence { kind, location: EvidenceLocation::Path(NativeString::from(path.as_str())), writing: false }]
        );
    }
}

/// No match is no holder whatever the coverage; the summary says how much
/// of the host was seen.
#[test]
fn no_match_has_no_holders_whatever_the_coverage() {
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
        assert!(holders(&found).is_empty(), "{outcome:?}");
        let coverage = CoverageSummary::of(&found);
        assert_eq!(coverage.outcome, outcome);
        assert_eq!(coverage.gaps.len(), gaps, "{outcome:?}");
        assert_eq!(coverage.is_complete(), is_complete, "{outcome:?}");
    }
}

/// Partial coverage never weakens a match, and its gaps are summarized so
/// they can be described.
#[test]
fn a_match_in_a_partial_report_is_a_holder_and_its_gaps_are_summarized() {
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

    assert_eq!(holders(&found).iter().map(|holder| holder.pid).collect::<Vec<_>>(), [8124]);
    let coverage = CoverageSummary::of(&found);
    assert!(coverage.budget_exhausted);
    assert_eq!(coverage.evidence_omitted, 12);
    assert_eq!(
        coverage.gaps,
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

    let holders = holders(&found);

    let holder = &holders[0];
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

/// Through the real Sniff query: a process standing in a subdirectory of the
/// checkout is a holder named by PID; the empty tree beside it has none.
#[test]
fn a_real_query_names_a_process_working_in_the_checkout() {
    let dir = tempfile::tempdir().unwrap();
    let held = dir.path().join("held");
    let inside = held.join("sub");
    std::fs::create_dir_all(&inside).unwrap();
    std::fs::write(inside.join("file.rs"), "fn main() {}\n").unwrap();
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

    let found = SniffUsage.query(&held);
    let clear = SniffUsage.query(&free);

    let _ = holder.kill();
    let _ = holder.wait();
    let found = found.unwrap();
    let holders_found = holders(&found);
    let named = holders_found.iter().find(|h| h.pid == holder.id()).unwrap_or_else(|| panic!("{found:#?}"));
    // Windows sees the directory handle a working directory holds; Unix sees
    // the working directory itself.
    #[cfg(unix)]
    assert!(named.evidence.iter().any(|e| e.kind == EvidenceKind::WorkingDirectory), "{named:#?}");
    #[cfg(windows)]
    assert!(!named.evidence.is_empty(), "{named:#?}");
    let enumeration = found.coverage.iter().find(|c| c.mechanism == Mechanism::ProcessEnumeration).expect("listed");
    assert_ne!(enumeration.status, CoverageStatus::NotAttempted, "processes were reached");
    let clear = clear.unwrap();
    assert_eq!(clear.target.kind, sniff::filesystem::query::TargetKind::Directory);
    assert!(holders(&clear).iter().all(|h| h.pid != holder.id()), "{clear:#?}");
}

/// Through the real Sniff query: a checkout that is gone by the time of the
/// query is an error, never an empty report.
#[test]
fn a_real_query_of_a_vanished_checkout_is_an_error() {
    let dir = tempfile::tempdir().unwrap();

    let result = SniffUsage.query(&dir.path().join("gone"));

    assert!(matches!(result, Err(PathUsageError::MissingTarget { .. })), "{result:?}");
}

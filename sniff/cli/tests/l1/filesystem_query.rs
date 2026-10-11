//! `sniff filesystem query`: output projections, outcome exit codes, the
//! runtime error shape, argument errors, and literal rendering of hostile
//! names.
//!
//! Most tests replay a report built here through the hidden
//! `SNIFF_FILESYSTEM_QUERY_REPLAY` seam, so text and JSON are compared as
//! projections of one retained observation and every outcome can be produced
//! on any host. The live tests query a directory the fixture owns.

use std::path::{Path, PathBuf};
use std::process::Output;

use chrono::{TimeZone, Utc};
use serde_json::Value;
use sniff::filesystem::query::{
    Access, Coverage, CoverageScope, CoverageStatus, Evidence, EvidenceKind, FileIdentity,
    Limitation, LimitationExample, LimitationKind, MatchBasis, Mechanism, NativeString, Outcome,
    PathUsageReport, ProcessRecord, QueryTarget, SCHEMA_VERSION, TargetKind, WatchInfo,
};

use crate::common::SniffCliFixture;

const REPLAY_ENV: &str = "SNIFF_FILESYSTEM_QUERY_REPLAY";

fn identity(inode: u64) -> FileIdentity {
    FileIdentity::Unix { device: 7, inode }
}

fn coverage(mechanism: Mechanism, status: CoverageStatus) -> Coverage {
    let attempted = match status {
        CoverageStatus::Unsupported | CoverageStatus::NotAttempted => None,
        _ => Some(4),
    };
    let succeeded = match status {
        CoverageStatus::Complete => Some(4),
        CoverageStatus::Partial => Some(3),
        CoverageStatus::Failed => Some(0),
        _ => None,
    };
    let reason = match status {
        CoverageStatus::Unsupported => Some(format!("{mechanism:?} cannot be inventoried here")),
        CoverageStatus::NotAttempted => Some("the shared budget expired first".to_string()),
        _ => None,
    };
    Coverage {
        mechanism,
        scope: if mechanism == Mechanism::TreeIdentity {
            CoverageScope::TargetTree
        } else {
            CoverageScope::VisibleProcesses
        },
        status,
        attempted,
        succeeded,
        reason,
        truncation: None,
        limitations: Vec::new(),
    }
}

fn limitation(kind: LimitationKind, message: &str) -> Limitation {
    Limitation {
        kind,
        message: message.to_string(),
        count: 2,
        examples: vec![LimitationExample {
            pid: Some(77),
            path: None,
            detail: Some("fd 9".to_string()),
        }],
    }
}

fn evidence(kind: EvidenceKind, mechanism: Mechanism, path: &str, inode: u64) -> Evidence {
    Evidence {
        kind,
        mechanism,
        matched_paths: vec![NativeString::from(path)],
        observed_path: Some(NativeString::from(path)),
        match_basis: MatchBasis::Identity,
        identity: Some(identity(inode)),
        descriptor: None,
        watch: None,
        access: None,
        event_only: None,
    }
}

fn process(pid: u32, name: &str, evidence: Vec<Evidence>) -> ProcessRecord {
    ProcessRecord {
        pid,
        creation_token: Some(u64::from(pid) * 1_000),
        start_time: Some(Utc.with_ymd_and_hms(2026, 10, 6, 9, 0, 0).unwrap()),
        identity_uncertain: false,
        name: Some(NativeString::from(name)),
        executable: Some(NativeString::from(format!("/usr/bin/{name}").as_str())),
        user: Some("501".to_string()),
        evidence,
    }
}

/// A usable report with evidence of every kind, a partial mechanism with
/// limitations, an unsupported mechanism, and a report-wide limitation.
fn usable_report() -> PathUsageReport {
    let mut handle = evidence(
        EvidenceKind::OpenHandle,
        Mechanism::OpenHandles,
        "/work/app/index.db",
        11,
    );
    handle.descriptor = Some(12);
    handle.access = Some(Access {
        read: true,
        write: true,
    });
    handle.matched_paths.push(NativeString::from("/work/app/hard-link.db"));
    handle.observed_path = Some(NativeString::from("/elsewhere/alias.db"));
    let mut watch = evidence(
        EvidenceKind::WatchRegistration,
        Mechanism::Inotify,
        "/work/app/src",
        12,
    );
    watch.descriptor = Some(5);
    watch.watch = Some(WatchInfo {
        watch_id: Some(3),
        mask: Some(0xfc6),
        recursive: Some(false),
    });

    let mut open_handles = coverage(Mechanism::OpenHandles, CoverageStatus::Partial);
    open_handles.limitations.push(limitation(
        LimitationKind::PermissionDenied,
        "permission was denied for some processes",
    ));
    PathUsageReport {
        schema_version: SCHEMA_VERSION,
        target: QueryTarget {
            requested: NativeString::from("./app"),
            resolved: NativeString::from("/work/app"),
            kind: TargetKind::Directory,
            identity: identity(10),
            recursive: true,
        },
        outcome: Outcome::Usable,
        started_at: Utc.with_ymd_and_hms(2026, 10, 6, 9, 30, 0).unwrap(),
        finished_at: Utc.with_ymd_and_hms(2026, 10, 6, 9, 30, 1).unwrap(),
        budget_ms: 2_000,
        elapsed_us: 41_000,
        budget_exhausted: false,
        processes: vec![
            process(
                812,
                "node",
                vec![
                    evidence(
                        EvidenceKind::WorkingDirectory,
                        Mechanism::WorkingDirectories,
                        "/work/app",
                        10,
                    ),
                    handle,
                ],
            ),
            process(
                9001,
                "watcher",
                vec![
                    watch,
                    evidence(
                        EvidenceKind::LoadedModule,
                        Mechanism::LoadedModules,
                        "/work/app/lib.so",
                        13,
                    ),
                ],
            ),
        ],
        coverage: vec![
            coverage(Mechanism::ProcessEnumeration, CoverageStatus::Complete),
            coverage(Mechanism::TreeIdentity, CoverageStatus::Complete),
            open_handles,
            coverage(Mechanism::WorkingDirectories, CoverageStatus::Complete),
            coverage(Mechanism::Inotify, CoverageStatus::Complete),
            coverage(Mechanism::Fanotify, CoverageStatus::Unsupported),
        ],
        limitations: vec![limitation(
            LimitationKind::IdentityUnavailable,
            "the name of some processes could not be read",
        )],
    }
}

fn with_statuses(mut report: PathUsageReport, status: CoverageStatus) -> PathUsageReport {
    for record in &mut report.coverage {
        if !record.mechanism.is_prerequisite() {
            *record = coverage(record.mechanism, status);
        }
    }
    report
}

/// Every non-usable shape the library can report, with its outcome.
fn non_usable_reports() -> Vec<(&'static str, PathUsageReport)> {
    let mut skipped = with_statuses(usable_report(), CoverageStatus::NotAttempted);
    skipped.processes.clear();
    skipped.budget_exhausted = true;
    skipped.outcome = Outcome::Unavailable;

    let mut failed = with_statuses(usable_report(), CoverageStatus::Failed);
    failed.processes.clear();
    failed.outcome = Outcome::Unavailable;

    let mut unsupported = with_statuses(usable_report(), CoverageStatus::Unsupported);
    unsupported.processes.clear();
    unsupported.outcome = Outcome::Unsupported;

    vec![
        ("skipped", skipped),
        ("total failure", failed),
        ("unsupported", unsupported),
    ]
}

struct Replay {
    fixture: SniffCliFixture,
    file: PathBuf,
}

impl Replay {
    fn new(report: &PathUsageReport) -> Self {
        Self::from_json(&serde_json::to_string(report).unwrap())
    }

    fn from_json(json: &str) -> Self {
        let fixture = SniffCliFixture::named("sniff-filesystem-query");
        let file = fixture.tmp_dir().join("report.json");
        std::fs::write(&file, json).unwrap();
        Self { fixture, file }
    }

    fn run(&self, args: &[&str]) -> Output {
        self.fixture
            .command_std()
            .env(REPLAY_ENV, &self.file)
            .args(["filesystem", "query", "ignored-under-replay"])
            .args(args)
            .output()
            .unwrap()
    }
}

fn stdout(output: &Output) -> String {
    String::from_utf8(output.stdout.clone()).unwrap()
}

fn stderr(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).unwrap()
}

fn json(output: &Output) -> Value {
    serde_json::from_slice(&output.stdout)
        .unwrap_or_else(|error| panic!("stdout is not one JSON value ({error}):\n{}", stdout(output)))
}

fn mechanism_label(mechanism: Mechanism) -> &'static str {
    match mechanism {
        Mechanism::ProcessEnumeration => "Process enumeration",
        Mechanism::TreeIdentity => "Tree identity",
        Mechanism::OpenHandles => "Open handles",
        Mechanism::WorkingDirectories => "Working directories",
        Mechanism::Inotify => "inotify",
        Mechanism::Fanotify => "fanotify",
        Mechanism::Fsevents => "FSEvents",
        Mechanism::LoadedModules => "Loaded modules",
        Mechanism::DirectoryChangeSubscriptions => "Directory change subscriptions",
        Mechanism::Polling => "Polling watchers",
    }
}

fn status_word(status: CoverageStatus) -> &'static str {
    match status {
        CoverageStatus::Complete => "complete",
        CoverageStatus::Partial => "partial",
        CoverageStatus::Unsupported => "unsupported",
        CoverageStatus::Failed => "failed",
        CoverageStatus::NotAttempted => "not attempted",
    }
}

/// Asserts that every fact the text view must carry appears in `text`.
fn assert_text_projects(report: &PathUsageReport, text: &str) {
    assert!(text.contains(&*report.target.resolved.display()), "{text}");
    for process in &report.processes {
        assert!(text.contains(&format!("PID {}", process.pid)), "{text}");
        if let Some(name) = &process.name {
            assert!(text.contains(&*name.display()), "{text}");
        }
        for evidence in &process.evidence {
            for path in &evidence.matched_paths {
                assert!(text.contains(&*path.display()), "{text}");
            }
        }
    }
    for record in &report.coverage {
        let line = format!(
            "{}: {}",
            mechanism_label(record.mechanism),
            status_word(record.status)
        );
        assert!(text.contains(&line), "missing `{line}` in:\n{text}");
        if let Some(reason) = &record.reason {
            assert!(text.contains(reason.as_str()), "{text}");
        }
        for limitation in &record.limitations {
            assert!(text.contains(&limitation.message), "{text}");
        }
    }
    for limitation in &report.limitations {
        assert!(text.contains(&limitation.message), "{text}");
    }
}

#[test]
fn text_and_json_are_projections_of_one_retained_report() {
    let report = usable_report();
    let replay = Replay::new(&report);

    let as_json = replay.run(&["--json"]);
    assert_eq!(as_json.status.code(), Some(0), "{}", stderr(&as_json));
    assert_eq!(stderr(&as_json), "");
    let read_back: PathUsageReport = serde_json::from_slice(&as_json.stdout).unwrap();
    assert_eq!(read_back, report, "JSON is the report, unfiltered");

    for args in [&["--plain"][..], &[]] {
        let as_text = replay.run(args);
        assert_eq!(as_text.status.code(), Some(0), "{args:?}");
        assert_eq!(stderr(&as_text), "", "{args:?}: coverage belongs on stdout");
        let text = stdout(&as_text);
        assert_text_projects(&report, &text);
        assert!(text.contains("Discovery is partial."), "{text}");
        assert!(text.contains("(observed as /elsewhere/alias.db)"), "{text}");
        assert!(text.contains("read/write, descriptor 12"), "{text}");
        assert!(text.contains("descriptor 5, watch 3, mask 0xfc6, not recursive"), "{text}");
        assert!(
            text.contains("does not prove that deleting the target will fail"),
            "{text}"
        );
        assert!(!text.contains("blocks"), "never overstates contention:\n{text}");
    }
}

#[test]
fn plain_text_carries_no_escape_codes() {
    let output = Replay::new(&usable_report()).run(&["--plain"]);
    assert!(!stdout(&output).contains('\u{1b}'), "{}", stdout(&output));
}

#[test]
fn json_wins_over_plain() {
    let report = usable_report();
    let output = Replay::new(&report).run(&["--plain", "--json"]);
    let read_back: PathUsageReport = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(read_back, report);
}

#[test]
fn a_partial_report_is_usable_and_exits_zero() {
    let report = usable_report();
    assert_eq!(report.coverage[2].status, CoverageStatus::Partial);
    let output = Replay::new(&report).run(&["--plain"]);
    assert_eq!(output.status.code(), Some(0));
    assert!(stdout(&output).contains("Open handles: partial (3 of 4 succeeded)"));
}

#[test]
fn an_empty_usable_report_exits_zero_and_does_not_claim_the_target_is_unused() {
    let mut report = usable_report();
    report.processes.clear();
    report.coverage[2] = coverage(Mechanism::OpenHandles, CoverageStatus::Complete);
    let replay = Replay::new(&report);

    let text = replay.run(&["--plain"]);
    assert_eq!(text.status.code(), Some(0));
    let text = stdout(&text);
    assert!(text.contains("No processes were found using this path."), "{text}");
    assert!(
        text.contains("No matches does not establish that the target is unused."),
        "{text}"
    );
    assert!(!text.contains("Discovery is partial."), "{text}");

    let as_json = replay.run(&["--json"]);
    assert_eq!(as_json.status.code(), Some(0));
    assert_eq!(json(&as_json)["processes"], Value::Array(Vec::new()));
}

#[test]
fn non_usable_reports_exit_one_with_the_full_report_on_stdout() {
    for (label, report) in non_usable_reports() {
        let replay = Replay::new(&report);

        let as_json = replay.run(&["--json"]);
        assert_eq!(as_json.status.code(), Some(1), "{label}");
        assert_eq!(stderr(&as_json), "", "{label}");
        let read_back: PathUsageReport = serde_json::from_slice(&as_json.stdout).unwrap();
        assert_eq!(read_back, report, "{label}");

        let as_text = replay.run(&["--plain"]);
        assert_eq!(as_text.status.code(), Some(1), "{label}");
        assert_eq!(stderr(&as_text), "", "{label}: the report is not an error");
        let text = stdout(&as_text);
        assert_text_projects(&report, &text);
        let headline = match report.outcome {
            Outcome::Unsupported => "Discovery is unsupported here",
            _ => "Discovery was unavailable",
        };
        assert!(text.contains(headline), "{label}:\n{text}");
    }
}

#[test]
fn a_skipped_report_says_the_budget_ran_out() {
    let (_, skipped) = non_usable_reports().remove(0);
    let text = stdout(&Replay::new(&skipped).run(&["--plain"]));
    assert!(
        text.contains("Open handles: not attempted. the shared budget expired first"),
        "{text}"
    );
    assert!(
        text.contains("The 2000 ms budget ran out before every mechanism finished."),
        "{text}"
    );
}

#[test]
fn match_presence_does_not_change_the_exit_code() {
    let mut unavailable = usable_report();
    unavailable.outcome = Outcome::Unavailable;
    assert!(!unavailable.processes.is_empty());
    assert_eq!(Replay::new(&unavailable).run(&["--json"]).status.code(), Some(1));
}

#[test]
fn long_lists_are_shortened_in_text_only_and_say_how_much_was_left_out() {
    let mut report = usable_report();
    let template = report.processes[0].clone();
    report.processes = (1..=55)
        .map(|pid| ProcessRecord {
            pid,
            ..template.clone()
        })
        .collect();
    let path_evidence = report.processes[0].evidence[0].clone();
    report.processes[0].evidence = (0..25)
        .map(|index| Evidence {
            descriptor: Some(index),
            ..path_evidence.clone()
        })
        .collect();
    let replay = Replay::new(&report);

    let text = stdout(&replay.run(&["--plain"]));
    assert!(text.contains("PID 50 "), "{text}");
    assert!(!text.contains("PID 51 "), "{text}");
    assert!(
        text.contains("5 more processes omitted from this view; --json shows every process."),
        "{text}"
    );
    assert!(
        text.contains("5 more evidence records omitted from this view"),
        "{text}"
    );

    let as_json = json(&replay.run(&["--json"]));
    assert_eq!(as_json["processes"].as_array().unwrap().len(), 55);
    assert_eq!(
        as_json["processes"][0]["evidence"].as_array().unwrap().len(),
        25
    );
}

#[test]
fn hostile_names_and_paths_are_shown_literally_and_kept_exact_in_json() {
    let hostile_name = "evil\u{1b}[31mred\u{7}<b>bold</b>\nsecond line\u{202e}txt";
    let hostile_path = "/work/app/[link](http://x) *star* `tick` \u{1b}]8;;http://x\u{7}";
    let mut report = usable_report();
    report.processes[0].name = Some(NativeString::from(hostile_name));
    report.processes[0].evidence[1].matched_paths = vec![NativeString::from(hostile_path)];
    report.coverage[2].limitations[0].message = "denied <i>here</i>\r\nforged line".into();
    let replay = Replay::new(&report);

    for args in [&["--plain"][..], &[]] {
        let text = stdout(&replay.run(args));
        assert!(!text.contains("\u{1b}[31m"), "{args:?}: ANSI from a name ran:\n{text}");
        assert!(!text.contains("\u{1b}]8"), "{args:?}: OSC from a path ran:\n{text}");
        assert!(!text.contains('\u{7}'), "{args:?}");
        assert!(!text.contains('\u{202e}'), "{args:?}");
        assert!(!text.contains("\nsecond line"), "{args:?}: a newline forged a line");
        assert!(!text.contains("\nforged line"), "{args:?}: a newline forged a line");
        assert!(
            text.contains(r"evil\u{1b}[31mred\u{7}<b>bold</b>\nsecond line\u{202e}txt"),
            "{args:?}: the name is shown literally:\n{text}"
        );
        assert!(
            text.contains(r"/work/app/[link](http://x) *star* `tick` \u{1b}]8;;http://x\u{7}"),
            "{args:?}: the path is shown literally:\n{text}"
        );
        assert!(
            text.contains(r"denied <i>here</i>\r\nforged line"),
            "{args:?}:\n{text}"
        );
    }

    let read_back: PathUsageReport =
        serde_json::from_slice(&replay.run(&["--json"]).stdout).unwrap();
    assert_eq!(read_back, report, "JSON keeps the original values");
}

#[test]
fn a_malformed_replay_is_an_error_not_an_empty_report() {
    let output = Replay::from_json(r#"{"schema_version": 1}"#).run(&["--json"]);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(stdout(&output), "");
    assert!(stderr(&output).contains(REPLAY_ENV), "{}", stderr(&output));
}

// ----------------------------------------------------------------------------
// Live queries against a directory the fixture owns
// ----------------------------------------------------------------------------

fn live(fixture: &SniffCliFixture, args: &[&std::ffi::OsStr]) -> Output {
    fixture
        .command_std()
        .args(["filesystem", "query"])
        .args(args)
        .output()
        .unwrap()
}

fn target_dir(fixture: &SniffCliFixture) -> PathBuf {
    let dir = fixture.cwd().join("checkout");
    std::fs::create_dir_all(dir.join(".hidden")).unwrap();
    std::fs::write(dir.join(".hidden").join("file.txt"), "x").unwrap();
    dir
}

#[test]
fn a_live_json_perf_query_is_one_object_and_does_no_inventory() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-live");
    let dir = target_dir(&fixture);
    let output = live(
        &fixture,
        &[dir.as_os_str(), "--json".as_ref(), "--perf".as_ref()],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let value = json(&output);
    assert_eq!(value["outcome"], "usable");
    assert_eq!(value["target"]["kind"], "directory");
    let counters = value["performance"]["counters"]
        .as_object()
        .expect("structured performance counters");
    assert_eq!(counters["filesystem.query.tree_walks"], 1, "{counters:?}");
    assert_eq!(counters["filesystem.query.process_enumerations"], 1, "{counters:?}");
    for name in counters.keys() {
        assert!(
            name.starts_with("filesystem.query.") || name.starts_with("filesystem.io."),
            "the focused query did unrelated work: {name}"
        );
    }
    // The rendered summary is CLI diagnostics, so it is on stderr.
    assert!(stderr(&output).contains("Total"), "{}", stderr(&output));

    let mut report = value;
    report.as_object_mut().unwrap().remove("performance");
    let read_back: PathUsageReport = serde_json::from_value(report).unwrap();
    assert_eq!(read_back.outcome, Outcome::Usable);
}

#[test]
fn without_perf_there_is_no_performance_field() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-live");
    let dir = target_dir(&fixture);
    let output = live(&fixture, &[dir.as_os_str(), "--json".as_ref()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    assert_eq!(stderr(&output), "");
    let read_back: PathUsageReport = serde_json::from_slice(&output.stdout).unwrap();
    assert!(read_back.target.recursive);
}

#[test]
fn target_only_and_timeout_reach_the_library() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-live");
    let dir = target_dir(&fixture);
    let output = live(
        &fixture,
        &[
            dir.as_os_str(),
            "--json".as_ref(),
            "--target-only".as_ref(),
            "--timeout".as_ref(),
            "1500".as_ref(),
        ],
    );
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report: PathUsageReport = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!report.target.recursive);
    assert_eq!(report.budget_ms, 1500);
}

#[test]
fn a_relative_path_resolves_against_the_invocation_directory_not_base() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-live");
    target_dir(&fixture);
    let elsewhere = fixture.tmp_dir().join("elsewhere");
    std::fs::create_dir_all(elsewhere.join("checkout")).unwrap();
    let output = fixture
        .command_std()
        .arg("--base")
        .arg(&elsewhere)
        .args(["filesystem", "query", "checkout", "--json"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let report: PathUsageReport = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report.target.requested.as_path(), Path::new("checkout"));
    let expected = fixture.cwd().join("checkout").canonicalize().unwrap();
    assert_eq!(report.target.resolved.as_path(), expected);
}

#[test]
fn runtime_errors_have_the_stated_json_shape() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-errors");
    let missing = fixture.cwd().join("missing");
    for (argument, kind) in [
        (missing.as_os_str(), "missing_target"),
        ("@no-such-reference-for-sniff.txt".as_ref(), "reference_not_found"),
        ("https://example.com/x".as_ref(), "nonlocal_reference"),
    ] {
        let as_json = live(&fixture, &[argument, "--json".as_ref()]);
        assert_eq!(as_json.status.code(), Some(1), "{argument:?}");
        let value = json(&as_json);
        let object = value.as_object().unwrap();
        assert_eq!(object.len(), 1, "{value}");
        let error = value["error"].as_object().unwrap();
        assert_eq!(error.len(), 2, "{value}");
        assert_eq!(error["kind"], kind, "{value}");
        assert!(error["message"].as_str().is_some_and(|m| !m.is_empty()));
        assert!(stderr(&as_json).starts_with("Error: "), "{}", stderr(&as_json));

        let as_text = live(&fixture, &[argument]);
        assert_eq!(as_text.status.code(), Some(1), "{argument:?}");
        assert_eq!(stdout(&as_text), "", "{argument:?}: no report for an error");
        assert!(stderr(&as_text).starts_with("Error: "), "{argument:?}");
    }
}

#[test]
fn invalid_arguments_exit_two_without_a_report() {
    let fixture = SniffCliFixture::named("sniff-filesystem-query-args");
    for argv in [
        &["filesystem", "query"][..],
        &["filesystem", "query", ".", "--timeout", "0"],
        &["filesystem", "query", ".", "--timeout", "-1"],
        &["filesystem", "query", ".", "--timeout", "soon"],
        &["filesystem", "query", ".", "--timeout", "18446744073709551616"],
        &["filesystem", "--refresh-remotes", "query", "."],
        &["filesystem", "--latest-versions", "query", "."],
        &["filesystem", "query", ".", "--refresh-remotes"],
        &["filesystem", "query", ".", "--latest-versions", "--json"],
    ] {
        let output = fixture.command_std().args(argv).output().unwrap();
        assert_eq!(output.status.code(), Some(2), "{argv:?}");
        assert_eq!(stdout(&output), "", "{argv:?}");
        assert!(!stderr(&output).is_empty(), "{argv:?}");
    }
}

#[cfg(unix)]
#[test]
fn a_non_utf8_argument_reaches_the_query_as_native_bytes() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let fixture = SniffCliFixture::named("sniff-filesystem-query-bytes");
    let name = OsStr::from_bytes(b"missing-\xff");
    let output = live(&fixture, &[name, "--json".as_ref()]);
    assert_eq!(output.status.code(), Some(1), "{}", stderr(&output));
    assert_eq!(json(&output)["error"]["kind"], "missing_target");
}

/// Linux filesystems accept any bytes in a name; macOS APFS rejects
/// invalid UTF-8, so only the missing-target half runs there.
#[cfg(target_os = "linux")]
#[test]
fn a_non_utf8_directory_is_queried_and_serialized_losslessly() {
    use std::ffi::OsStr;
    use std::os::unix::ffi::OsStrExt;

    let fixture = SniffCliFixture::named("sniff-filesystem-query-bytes");
    let name = OsStr::from_bytes(b"dir-\xff");
    std::fs::create_dir(fixture.cwd().join(name)).unwrap();
    let output = live(&fixture, &[name, "--json".as_ref()]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr(&output));
    let value = json(&output);
    let native = &value["target"]["requested"]["native"];
    assert_eq!(native["encoding"], "unix_bytes");
    assert_eq!(native["units"], serde_json::json!([100, 105, 114, 45, 255]));
    let report: PathUsageReport = serde_json::from_value(value).unwrap();
    assert_eq!(report.target.requested.as_os_str(), name);
}

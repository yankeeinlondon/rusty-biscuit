use super::*;

const REPORT: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<testsuites name="nextest-run" tests="4" failures="1" errors="0" uuid="5c1d2c4e-0000-4000-8000-000000000001" timestamp="2026-11-01T10:00:00.000+00:00" time="30.0">
    <testsuite name="claudine-cli::l1" tests="3" disabled="0" errors="0" failures="1">
        <testcase name="sequence_budget::waits" classname="claudine-cli::l1" timestamp="2026-11-01T10:00:00.000+00:00" time="12.5">
        </testcase>
        <testcase name="fast::one" classname="claudine-cli::l1" timestamp="2026-11-01T10:00:00.000+00:00" time="0.2"/>
        <testcase name="slow::fails" classname="claudine-cli::l1" timestamp="2026-11-01T10:00:00.000+00:00" time="6.0">
            <failure type="test failure">boom</failure>
            <system-out>&lt;failure&gt; in output</system-out>
        </testcase>
    </testsuite>
    <testsuite name="@setup-script:build-ci-build" tests="1" disabled="0" errors="0" failures="0">
        <testcase name="build-ci-build" classname="@setup-script:build-ci-build" time="63.0">
        </testcase>
    </testsuite>
    <testsuite name="sniff" tests="1" disabled="0" errors="0" failures="0">
        <testcase name="tests::flaky" classname="sniff" timestamp="2026-11-01T10:00:00.000+00:00" time="5.0">
            <flakyFailure type="test failure">once</flakyFailure>
        </testcase>
    </testsuite>
</testsuites>
"#;

fn origin(recorded_at: u64, environment: &str) -> Origin {
    Origin {
        recorded_at,
        source: "local".into(),
        run: format!("run-{recorded_at}"),
        environment: environment.into(),
        host: "host".into(),
        commit: "abc".into(),
        tier: "L1".into(),
    }
}

#[test]
fn junit_cases_carry_binary_id_duration_and_outcome() {
    let junit = parse_junit(REPORT).expect("the report parses");
    assert_eq!(junit.run_uuid.as_deref(), Some("5c1d2c4e-0000-4000-8000-000000000001"));
    let summary: Vec<(&str, &str, f64, &str)> = junit
        .cases
        .iter()
        .map(|case| (case.binary_id.as_str(), case.test.as_str(), case.duration_s, case.outcome))
        .collect();
    assert_eq!(
        summary,
        vec![
            ("claudine-cli::l1", "sequence_budget::waits", 12.5, "pass"),
            ("claudine-cli::l1", "fast::one", 0.2, "pass"),
            ("claudine-cli::l1", "slow::fails", 6.0, "fail"),
            ("sniff", "tests::flaky", 5.0, "flaky"),
        ]
    );
}

#[test]
fn only_tests_at_or_above_the_threshold_are_recorded_slowest_first() {
    let junit = parse_junit(REPORT).expect("the report parses");
    let records = slow_records(&junit, &origin(1, "windows"), 5.0);
    let tests: Vec<(&str, &str)> = records.iter().map(|r| (r.package.as_str(), r.test.as_str())).collect();
    assert_eq!(
        tests,
        vec![
            ("claudine-cli", "sequence_budget::waits"),
            ("claudine-cli", "slow::fails"),
            ("sniff", "tests::flaky"),
        ]
    );
    assert!(records.iter().all(|record| record.threshold_s == 5.0));
}

/// One recorded invocation of `tier` in `directory`, as `record` writes it.
fn run_of(state: &str, group: &str, recorded_at: u64, tier: &str, directory: &str, report: &str) -> Run {
    let junit = parse_junit(report).expect("the report parses");
    let mut origin = origin(recorded_at, "windows");
    origin.tier = tier.into();
    Run {
        state: state.into(),
        recorded_at,
        group: group.into(),
        tier: tier.into(),
        directory: directory.into(),
        packages: junit.cases.iter().map(|case| package_of(&case.binary_id).to_owned()).collect(),
        tests: junit.cases.len(),
        threshold_s: 5.0,
        slow: slow_records(&junit, &origin, 5.0),
    }
}

const PLAYA_SLOW: &str = r#"<testsuites name="nextest-run" uuid="u2"><testsuite name="playa::l2"><testcase name="plays" classname="playa::l2" time="20.0"/><testcase name="quick" classname="playa::l2" time="0.1"/></testsuite></testsuites>"#;
const PLAYA_FAST: &str = r#"<testsuites name="nextest-run" uuid="u3"><testsuite name="playa::l2"><testcase name="plays" classname="playa::l2" time="0.4"/><testcase name="quick" classname="playa::l2" time="0.1"/></testsuite></testsuites>"#;
const SNIFF_FAST: &str = r#"<testsuites name="nextest-run" uuid="u4"><testsuite name="sniff"><testcase name="quick" classname="sniff" time="0.1"/></testsuite></testsuites>"#;

/// `just test` then `just sanity` in one state both stay in the summary; one
/// recipe's per-package invocations merge; a re-run of a scope replaces it.
#[test]
fn every_scope_of_a_state_is_kept_and_the_latest_group_of_each_wins() {
    let runs = vec![
        run_of("s1", "g1", 100, "L1", "claudine", REPORT),
        run_of("s1", "g2", 200, "sanity", "playa", PLAYA_SLOW),
        run_of("s1", "g2", 210, "sanity", "playa", SNIFF_FAST),
        run_of("s0", "g0", 50, "L1", "claudine", PLAYA_FAST),
    ];
    let scopes_now = scopes(&runs, "s1");
    let summary: Vec<(&str, &str, usize, usize)> = scopes_now
        .iter()
        .map(|scope| (scope.tier.as_str(), scope.directory.as_str(), scope.tests, scope.slow.len()))
        .collect();
    assert_eq!(summary, vec![("L1", "claudine", 4, 3), ("sanity", "playa", 3, 1)]);
    assert_eq!(
        scopes_now[1].packages.iter().map(String::as_str).collect::<Vec<_>>(),
        vec!["playa", "sniff"]
    );

    let mut rerun = runs.clone();
    rerun.push(run_of("s1", "g3", 300, "L1", "claudine", PLAYA_FAST));
    let replaced = scopes(&rerun, "s1");
    assert_eq!((replaced[0].group.as_str(), replaced[0].slow.len()), ("g3", 0));
}

#[test]
fn the_current_summary_lists_each_scope_with_its_rerun_command() {
    let runs = vec![
        run_of("s1", "g1", 1_793_527_200, "L1", "claudine", REPORT),
        run_of("s1", "g2", 1_793_527_260, "sanity", "playa", PLAYA_FAST),
        run_of("s1", "g4", 1_793_527_100, "L1", ".", SNIFF_FAST),
    ];
    let document = current_document("s1", &scopes(&runs, "s1"));
    for line in [
        "state: \"s1\"",
        "recorded: \"2026-11-01T10:01:00Z\"",
        "scopes: 3",
        "tiers: \"L1, sanity\"",
        "tests: 7",
        "slow: 3",
        "## L1 in `claudine`",
        "- Re-run: `cd claudine && just test`",
        "- Re-run: `cd playa && just sanity`",
        "- Re-run: `just test sniff`",
        "| `sequence_budget::waits` | `claudine-cli::l1` | 12s | pass |",
        "No test in this scope was slow.",
    ] {
        assert!(document.contains(line), "missing `{line}` in:\n{document}");
    }
    assert!(!document.contains("build-ci-build"), "a setup script is not a test");

    let dir = std::env::temp_dir().join(format!("slow-tests-current-{}", std::process::id()));
    fs::create_dir_all(dir.join("target/slow-tests")).unwrap();
    fs::write(dir.join(CURRENT), &document).unwrap();
    let line = current_line(&dir);
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(
        line.as_deref(),
        Some("Runs of working-tree state s1 in this checkout: L1, sanity (3 scope(s)), 7 test(s), 3 at or above 5s.")
    );
}

fn snapshot(id: &str, head: &str, changed: &[(&str, &str)]) -> StateSnapshot {
    StateSnapshot {
        id: id.into(),
        head: Some(head.into()),
        changed: changed.iter().map(|(path, token)| ((*path).into(), (*token).into())).collect(),
    }
}

#[test]
fn drift_lists_paths_whose_content_differs_plus_committed_paths() {
    let baseline = snapshot("s1", "h1", &[("a.rs", "file:1"), ("b.rs", "file:2"), ("gone.rs", "file:3")]);
    let live = snapshot("s2", "h1", &[("a.rs", "file:1"), ("b.rs", "file:9"), ("new.rs", "file:4")]);
    let drift = drift(&baseline, &live, &["c.rs".into()], HeadChange::Same);
    assert_eq!(drift.files, ["b.rs", "c.rs", "gone.rs", "new.rs"]);
}

#[test]
fn a_stale_message_names_the_changes_and_whether_they_touch_the_covered_scopes() {
    let scopes = vec![run_of("s1", "g1", 100, "L1", "claudine", PLAYA_SLOW)];
    let outside = Drift { files: vec!["sniff/lib/src/a.rs".into()], head: HeadChange::Ahead(2) };
    let message = stale_message(Some("2026-11-02T00:08:53Z"), &scopes, Some(&outside));
    assert!(message.contains("L1 in `claudine`, recorded 2026-11-02T00:08:53Z"), "{message}");
    assert!(message.contains("1 changed file(s): `sniff/lib/src/a.rs`"), "{message}");
    assert!(message.contains("2 new commit(s)"), "{message}");
    assert!(message.contains("may still hold"), "{message}");
    assert!(message.contains("`cd claudine && just test`"), "{message}");

    let inside = Drift { files: (0..7).map(|n| format!("claudine/{n}.rs")).collect(), head: HeadChange::Same };
    let message = stale_message(None, &scopes, Some(&inside));
    assert!(message.contains("7 changed file(s)") && message.contains("and 2 more"), "{message}");
    assert!(message.contains("7 of the changed files are inside"), "{message}");
}

#[test]
fn the_proof_needs_every_baseline_scope_rerun_clean_in_a_new_state() {
    let baseline = vec![
        run_of("s1", "g1", 100, "L2", "playa", PLAYA_SLOW),
        run_of("s1", "g1", 101, "L2", "playa", SNIFF_FAST),
        run_of("s1", "g2", 110, "sanity", "playa", PLAYA_FAST),
    ];
    let problems = |runs: &[Run], live: &str| proof_problems(runs, "s1", live);

    assert_eq!(problems(&baseline, "s1").len(), 1, "an unchanged tree fixed nothing");
    assert!(problems(&baseline, "missing")[0].contains("has not run"));
    assert!(proof_problems(&baseline, "nope", "s2")[0].contains("no run of the baseline state"));

    // Only L2 re-ran, and only over playa: sanity is missing, sniff is missing.
    let mut partial = baseline.clone();
    partial.push(run_of("s2", "g3", 200, "L2", "playa", PLAYA_FAST));
    let found = problems(&partial, "s2");
    assert_eq!(found.len(), 2, "{found:?}");
    assert!(found[0].contains("L2 in `playa` re-ran without sniff"), "{found:?}");
    assert!(found[1].contains("sanity in `playa` has not run"), "{found:?}");

    // Every scope re-ran, but a slow test remains.
    let mut still_slow = baseline.clone();
    still_slow.push(run_of("s2", "g3", 200, "L2", "playa", PLAYA_SLOW));
    still_slow.push(run_of("s2", "g3", 201, "L2", "playa", SNIFF_FAST));
    still_slow.push(run_of("s2", "g4", 210, "sanity", "playa", PLAYA_FAST));
    let found = problems(&still_slow, "s2");
    assert_eq!(found.len(), 1, "{found:?}");
    assert!(found[0].contains("still has 1 slow test(s): playa::l2 plays (20s)"), "{found:?}");

    let mut proven = baseline;
    proven.push(run_of("s2", "g3", 200, "L2", "playa", PLAYA_FAST));
    proven.push(run_of("s2", "g3", 201, "L2", "playa", SNIFF_FAST));
    proven.push(run_of("s2", "g4", 210, "sanity", "playa", PLAYA_FAST));
    assert_eq!(problems(&proven, "s2"), Vec::<String>::new());
}

#[test]
fn run_directories_are_root_relative_with_forward_slashes() {
    let root = std::env::temp_dir().join(format!("slow-tests-dirs-{}", std::process::id()));
    fs::create_dir_all(root.join("claudine").join("rendezvous")).unwrap();
    let nested = relative_directory(&root, &root.join("claudine").join("rendezvous"));
    let top = relative_directory(&root, &root);
    let outside = relative_directory(&root.join("claudine"), &root);
    let _ = fs::remove_dir_all(&root);
    assert_eq!((nested.as_str(), top.as_str(), outside.as_str()), ("claudine/rendezvous", ".", "."));
}


#[test]
fn a_document_without_testsuites_is_rejected() {
    assert!(parse_junit("<testsuite name=\"x\"/>").is_err());
}

#[test]
fn the_default_profile_period_is_read_in_both_spellings() {
    let table = "[profile.default]\nslow-timeout = { period = \"5s\", terminate-after = 6 }\n";
    assert_eq!(default_slow_period(table).unwrap(), 5.0);
    let string = "[profile.default]\nslow-timeout = \"1m\"\n[profile.ci]\nslow-timeout = \"30s\"\n";
    assert_eq!(default_slow_period(string).unwrap(), 60.0);
    assert!(default_slow_period("[profile.ci]\nslow-timeout = \"30s\"\n").is_err());
}

#[test]
fn the_repository_config_declares_a_readable_threshold() {
    let config = Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("scripts/ has a parent").join(".config/nextest.toml");
    let text = fs::read_to_string(config).expect("the workspace nextest config reads");
    assert!(default_slow_period(&text).expect("the default profile declares a slow-timeout") > 0.0);
}

#[test]
fn durations_accept_nextest_units() {
    assert_eq!(parse_duration("500ms").unwrap(), 0.5);
    assert_eq!(parse_duration("2m").unwrap(), 120.0);
    assert!(parse_duration("5 fortnights").is_err());
}

#[test]
fn remotes_reduce_to_one_identity_in_every_spelling() {
    for remote in [
        "https://github.com/yankeeinlondon/rusty-biscuit.git",
        "git@github.com:yankeeinlondon/rusty-biscuit.git",
        "ssh://git@github.com/yankeeinlondon/rusty-biscuit",
    ] {
        assert_eq!(repository_identity(remote), "github.com/yankeeinlondon/rusty-biscuit", "{remote}");
    }
}

#[test]
fn history_compares_the_latest_run_with_the_one_before() {
    let junit = parse_junit(REPORT).expect("the report parses");
    let mut records = slow_records(&junit, &origin(100, "windows"), 5.0);
    let mut later = slow_records(&junit, &origin(200, "windows"), 5.0);
    later[0].duration_s = 25.0;
    records.extend(later);
    records.extend(slow_records(&junit, &origin(150, "ubuntu-latest"), 5.0));

    let windows = histories(&records, &[], Some("windows"));
    assert_eq!(windows.len(), 3);
    let top = &windows[0];
    assert_eq!(top.latest.test, "sequence_budget::waits");
    assert_eq!(top.latest.recorded_at, 200);
    assert_eq!(top.previous.map(|previous| previous.duration_s), Some(12.5));
    assert_eq!(top.count, 2);
    assert_eq!(trend(top), "+100%");

    let sniff_only = histories(&records, &["sniff".to_owned()], None);
    assert_eq!(sniff_only.len(), 2, "one history per environment");
}

#[test]
fn appended_records_read_back_and_a_bad_line_is_skipped() {
    let dir = std::env::temp_dir().join(format!("slow-tests-log-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let log = dir.join("nested/slow-tests.jsonl");
    let junit = parse_junit(REPORT).expect("the report parses");
    let records = slow_records(&junit, &origin(1, "linux"), 5.0);
    append(&log, &records).expect("first append");
    fs::OpenOptions::new().append(true).open(&log).unwrap().write_all(b"not json\n").unwrap();
    append(&log, &records[..1]).expect("second append");
    let read: Vec<SlowTest> = read_jsonl(&log).expect("the log reads");
    assert_eq!(read.len(), records.len() + 1);
    assert_eq!(read[0], records[0]);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn ci_artifacts_are_read_through_their_manifests() {
    let dir = std::env::temp_dir().join(format!("slow-tests-artifacts-{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    let artifact = dir.join("junit-claudine-cli-L1-macos-latest");
    fs::create_dir_all(artifact.join("L1")).unwrap();
    fs::write(artifact.join("L1/claudine-cli.xml"), REPORT).unwrap();
    fs::write(
        artifact.join("manifest.jsonl"),
        concat!(
            r#"{"tier":"L1","package":"claudine-cli","xml":"L1/claudine-cli.xml","exit_code":0,"environment":"macos-latest","duration_s":40,"report_present":true}"#,
            "\n",
            r#"{"tier":"L2","package":"claudine-cli","xml":"L2/claudine-cli.xml","exit_code":101,"environment":"macos-latest","duration_s":1,"report_present":false}"#,
            "\n"
        ),
    )
    .unwrap();
    let view = RunView {
        head_sha: "deadbeef".into(),
        attempt: 2,
        created_at: "2026-11-01T10:00:00Z".into(),
    };
    let records = records_from_artifacts(&dir, "gh:42/2", &view, 5.0).expect("the artifacts read");
    let _ = fs::remove_dir_all(&dir);
    assert_eq!(records.len(), 3);
    assert!(records.iter().all(|record| record.source == "ci"
        && record.environment == "macos-latest"
        && record.commit == "deadbeef"
        && record.run == "gh:42/2"
        && record.recorded_at == 1_793_527_200));
}

#[test]
fn dates_round_trip_through_civil_days() {
    assert_eq!(parse_rfc3339("1970-01-01T00:00:00Z"), Some(0));
    assert_eq!(parse_rfc3339("2026-11-01T10:00:00Z"), Some(1_793_527_200));
    assert_eq!(format_date(1_793_527_200), "2026-11-01");
    assert_eq!(format_date(951_782_400), "2000-02-29");
}

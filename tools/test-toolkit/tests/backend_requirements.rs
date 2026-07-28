//! Contract tests for per-backend L2 enforcement and its execution evidence.
//!
//! The failure these guard against is a green CI cell that ran nothing: a tier
//! that skipped every backend-gated test, or a required backend whose name was
//! misspelled and therefore silently never enforced again.

use std::collections::BTreeSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::thread;

use test_toolkit::{
    BACKEND_EXECUTIONS_FILE, BISCUIT_JUNIT_STAGE_DIR, BISCUIT_TEST_LEVEL,
    BISCUIT_TEST_LEVEL_REQUIRED, BISCUIT_TEST_REQUIRED_BACKENDS, Backend, EnvGuard,
    ExecutionDecision, ExecutionRecord, HarnessSpec, Level, LevelDecision, RUN_LEVEL3,
    append_backend_execution, backend_executions_path, decision_counts, evaluate_harness,
    parse_required_backends, read_backend_executions, record_backend_execution, required_backends,
    stage_dir, unproven_backends, workspace_root,
};

/// Clears every gate-relevant variable so a test observes only what it sets.
///
/// Held for the test body; dropping restores the caller's environment.
#[must_use]
fn clean_env() -> Vec<EnvGuard> {
    vec![
        EnvGuard::remove_safe(BISCUIT_TEST_LEVEL),
        EnvGuard::remove_safe(BISCUIT_TEST_LEVEL_REQUIRED),
        EnvGuard::remove_safe(BISCUIT_TEST_REQUIRED_BACKENDS),
        EnvGuard::remove_safe(BISCUIT_JUNIT_STAGE_DIR),
        EnvGuard::remove_safe(RUN_LEVEL3),
    ]
}

fn record(backend: &str, test: &str, decision: &str) -> ExecutionRecord {
    ExecutionRecord {
        backend: backend.to_string(),
        test: test.to_string(),
        decision: decision.to_string(),
    }
}

// ---------------------------------------------------------------------------
// Parsing
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn required_backends_reads_and_normalizes_the_env_value() {
    let _clean = clean_env();
    let _set = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, " TMUX , Apple-Terminal ");

    let parsed = required_backends().expect("normalizes");
    assert_eq!(
        parsed.into_iter().collect::<Vec<_>>(),
        vec![Backend::Tmux, Backend::AppleTerminal]
    );
}

#[test]
#[serial_test::serial]
fn unset_and_blank_both_mean_no_requirement() {
    let _clean = clean_env();
    assert!(required_backends().expect("unset is fine").is_empty());

    let _blank = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "   ");
    assert!(required_backends().expect("blank is fine").is_empty());
}

#[test]
fn near_miss_names_never_match_a_backend() {
    // A prefix/suffix match here would make `BISCUIT_TEST_REQUIRED_BACKENDS=wez`
    // silently enforce WezTerm, and `tmux2` silently enforce tmux.
    for candidate in ["wez", "tmux2", "tmuxx", "kitt", "apple", "apple-terminal-x"] {
        assert!(
            parse_required_backends(candidate).is_err(),
            "`{candidate}` must not resolve to a backend",
        );
    }
}

#[test]
fn unknown_and_empty_entries_are_rejected_not_dropped() {
    for value in ["notaterminal", "tmux,notaterminal", "tmux,,kitty", "tmux,", ",tmux"] {
        let err = parse_required_backends(value)
            .expect_err("`{value}` must be rejected")
            .to_string();
        assert!(
            err.contains(BISCUIT_TEST_REQUIRED_BACKENDS),
            "diagnostic must name the variable: {err}",
        );
    }
}

#[test]
#[serial_test::serial]
fn a_malformed_list_panics_even_when_the_harness_is_available() {
    // Otherwise a typo is inert on every host where the backend happens to be
    // present, and the requirement is never enforced again.
    let _clean = clean_env();
    let _bad = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux,wezterm2");

    match evaluate_harness(Level::L2, true, Backend::Tmux) {
        LevelDecision::Panic(msg) => assert!(msg.contains("wezterm2"), "{msg}"),
        other => panic!("expected Panic for a malformed list, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Panic vs skip, per backend
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn a_required_backend_that_is_missing_panics() {
    let _clean = clean_env();
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    match evaluate_harness(Level::L2, false, Backend::Tmux) {
        LevelDecision::Panic(msg) => {
            assert!(msg.contains(BISCUIT_TEST_REQUIRED_BACKENDS), "{msg}");
            assert!(msg.contains("tmux"), "{msg}");
        }
        other => panic!("expected Panic, got {other:?}"),
    }
}

#[test]
#[serial_test::serial]
fn an_unrequired_backend_that_is_missing_still_skips_cleanly() {
    // This is the whole point of per-backend granularity: a headless runner
    // provisions tmux and must not be forced to host a GUI emulator.
    let _clean = clean_env();
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    for backend in [Backend::WezTerm, Backend::Kitty, Backend::AppleTerminal] {
        match evaluate_harness(Level::L2, false, backend) {
            LevelDecision::Skip(msg) => assert!(msg.contains(backend.label()), "{msg}"),
            other => panic!("expected Skip for {backend}, got {other:?}"),
        }
    }
}

#[test]
#[serial_test::serial]
fn a_label_only_requirement_can_never_be_demanded_per_backend() {
    let _clean = clean_env();
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    // "tmux harness" is a legacy diagnostic label, not the identifier.
    match evaluate_harness(Level::L2, false, "tmux harness") {
        LevelDecision::Skip(_) => {}
        other => panic!("expected Skip for a label-only requirement, got {other:?}"),
    }
}

#[test]
#[serial_test::serial]
fn a_required_backend_that_is_present_runs() {
    let _clean = clean_env();
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux,kitty");

    assert_eq!(
        evaluate_harness(Level::L2, true, Backend::Tmux),
        LevelDecision::Run
    );
}

// ---------------------------------------------------------------------------
// Composition with BISCUIT_TEST_LEVEL_REQUIRED
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn backend_requirement_composes_with_level_required() {
    let _clean = clean_env();
    let _level = EnvGuard::set_safe(BISCUIT_TEST_LEVEL_REQUIRED, "2");
    let _backends = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    // The backend-scoped diagnostic wins for a listed backend, because it names
    // the actionable fact.
    match evaluate_harness(Level::L2, false, Backend::Tmux) {
        LevelDecision::Panic(msg) => assert!(msg.contains(BISCUIT_TEST_REQUIRED_BACKENDS), "{msg}"),
        other => panic!("expected Panic, got {other:?}"),
    }

    // An unlisted backend still falls through to the all-or-nothing level gate.
    match evaluate_harness(Level::L2, false, Backend::WezTerm) {
        LevelDecision::Panic(msg) => assert!(msg.contains(BISCUIT_TEST_LEVEL_REQUIRED), "{msg}"),
        other => panic!("expected Panic, got {other:?}"),
    }
}

#[test]
#[serial_test::serial]
fn backend_requirement_does_not_replace_level_required() {
    let _clean = clean_env();
    let _backends = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    // Without BISCUIT_TEST_LEVEL_REQUIRED, an unlisted backend skips.
    match evaluate_harness(Level::L2, false, Backend::WezTerm) {
        LevelDecision::Skip(_) => {}
        other => panic!("expected Skip, got {other:?}"),
    }
}

#[test]
#[serial_test::serial]
fn an_explicit_level_ceiling_still_wins() {
    let _clean = clean_env();
    let _max = EnvGuard::set_safe(BISCUIT_TEST_LEVEL, "1");
    let _backends = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    match evaluate_harness(Level::L2, false, Backend::Tmux) {
        LevelDecision::Skip(msg) => assert!(msg.contains(BISCUIT_TEST_LEVEL), "{msg}"),
        other => panic!("expected Skip, got {other:?}"),
    }
}

// ---------------------------------------------------------------------------
// Evidence: location, recording, concurrency
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn stage_dir_prefers_the_env_override() {
    let _clean = clean_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, dir.path());

    assert_eq!(stage_dir(), dir.path());
    assert_eq!(
        backend_executions_path(),
        dir.path().join(BACKEND_EXECUTIONS_FILE)
    );
}

#[test]
#[serial_test::serial]
fn a_relative_stage_dir_anchors_on_the_workspace_root() {
    // nextest gives each test process its own package directory as the CWD, so
    // anchoring on the CWD would scatter the evidence across packages and
    // disagree with `just/devops.just`.
    let _clean = clean_env();
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, "target/nextest/elsewhere");

    assert_eq!(stage_dir(), workspace_root().join("target/nextest/elsewhere"));
}

#[test]
#[serial_test::serial]
fn stage_dir_defaults_under_the_workspace_root() {
    let _clean = clean_env();
    let path = backend_executions_path();

    assert!(
        path.ends_with(format!("target/nextest/ci-reports/{BACKEND_EXECUTIONS_FILE}")),
        "unexpected default evidence path: {}",
        path.display()
    );
    assert!(path.is_absolute(), "{}", path.display());
}

#[test]
#[serial_test::serial]
fn recording_is_a_no_op_without_a_required_backend_set() {
    let _clean = clean_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, dir.path());

    record_backend_execution(Backend::Tmux, "some::test", ExecutionDecision::Run);

    assert!(
        !dir.path().join(BACKEND_EXECUTIONS_FILE).exists(),
        "local dev must not pay for evidence it never checks"
    );
}

#[test]
#[serial_test::serial]
fn recording_writes_one_record_per_decision() {
    let _clean = clean_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, dir.path());
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    record_backend_execution(Backend::Tmux, "area::ran", ExecutionDecision::Run);
    record_backend_execution(Backend::WezTerm, "area::skipped", ExecutionDecision::Skip);

    let records =
        read_backend_executions(&dir.path().join(BACKEND_EXECUTIONS_FILE)).expect("readable");
    assert_eq!(
        records,
        vec![
            record("tmux", "area::ran", "run"),
            record("wezterm", "area::skipped", "skip"),
        ]
    );
}

#[test]
fn concurrent_appends_never_interleave() {
    // nextest runs each test in its own process, so the real writers are
    // concurrent processes sharing one file. Separate O_APPEND handles from
    // threads exercise the same kernel path.
    const WRITERS: usize = 16;
    const PER_WRITER: usize = 40;

    let dir = tempfile::tempdir().expect("tempdir");
    let path = Arc::new(dir.path().join(BACKEND_EXECUTIONS_FILE));
    let failures = Arc::new(AtomicUsize::new(0));

    let handles: Vec<_> = (0..WRITERS)
        .map(|writer| {
            let path = Arc::clone(&path);
            let failures = Arc::clone(&failures);
            thread::spawn(move || {
                for index in 0..PER_WRITER {
                    let name = format!("area::writer_{writer}::case_{index}");
                    if append_backend_execution(
                        &path,
                        Backend::Tmux,
                        &name,
                        ExecutionDecision::Run,
                    )
                    .is_err()
                    {
                        failures.fetch_add(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();

    for handle in handles {
        handle.join().expect("writer thread");
    }

    assert_eq!(failures.load(Ordering::Relaxed), 0, "append errors");

    // A torn line would fail to parse, so a clean read of the exact expected
    // count is the integrity assertion.
    let records = read_backend_executions(&path).expect("every line is a whole record");
    assert_eq!(records.len(), WRITERS * PER_WRITER);

    let names: BTreeSet<String> = records.into_iter().map(|record| record.test).collect();
    assert_eq!(names.len(), WRITERS * PER_WRITER, "records were lost");
}

#[test]
fn record_survives_a_test_name_containing_json_metacharacters() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(BACKEND_EXECUTIONS_FILE);
    let awkward = "area::has\"quote\\and\ttab";

    append_backend_execution(&path, Backend::Kitty, awkward, ExecutionDecision::Panic)
        .expect("append");

    let records = read_backend_executions(&path).expect("readable");
    assert_eq!(records, vec![record("kitty", awkward, "panic")]);
}

#[test]
fn a_corrupt_line_is_an_error_not_a_silent_pass() {
    let dir = tempfile::tempdir().expect("tempdir");
    let path = dir.path().join(BACKEND_EXECUTIONS_FILE);
    std::fs::write(&path, "{\"backend\":\"tmux\",\"test\":\"a\",\"decision\":\"run\"}\nnope\n")
        .expect("write");

    let err = read_backend_executions(&path).expect_err("must not accept garbage");
    assert!(err.to_string().contains("line 2"), "{err}");
}

// ---------------------------------------------------------------------------
// Execution proof
// ---------------------------------------------------------------------------

#[test]
fn an_installed_backend_with_zero_executed_tests_fails() {
    // The exact hazard: `tmux -V` succeeds, the tier selects no tmux test, and
    // the cell is green. Availability is not execution.
    let required: BTreeSet<Backend> = [Backend::Tmux].into_iter().collect();

    assert_eq!(
        unproven_backends(&required, &[]).into_iter().collect::<Vec<_>>(),
        vec![Backend::Tmux],
        "no records at all must fail",
    );

    let only_other_backends = vec![record("wezterm", "area::ran", "run")];
    assert_eq!(
        unproven_backends(&required, &only_other_backends)
            .into_iter()
            .collect::<Vec<_>>(),
        vec![Backend::Tmux],
        "another backend's evidence must not count",
    );

    let all_skipped = vec![
        record("tmux", "area::a", "skip"),
        record("tmux", "area::b", "skip"),
        record("tmux", "area::c", "panic"),
    ];
    assert_eq!(
        unproven_backends(&required, &all_skipped)
            .into_iter()
            .collect::<Vec<_>>(),
        vec![Backend::Tmux],
        "skips and panics are not executions",
    );
}

#[test]
fn one_executed_test_proves_the_backend() {
    let required: BTreeSet<Backend> = [Backend::Tmux].into_iter().collect();
    let records = vec![
        record("tmux", "area::skipped", "skip"),
        record("tmux", "area::ran", "run"),
    ];

    assert!(unproven_backends(&required, &records).is_empty());
    assert_eq!(decision_counts(Backend::Tmux, &records), (1, 1, 0));
}

#[test]
fn zero_required_backends_is_a_no_op() {
    let required = BTreeSet::new();
    assert!(unproven_backends(&required, &[]).is_empty());
}

// ---------------------------------------------------------------------------
// Macro wiring
// ---------------------------------------------------------------------------

#[test]
#[serial_test::serial]
fn require_level_records_the_run_decision_under_the_test_name() {
    let _clean = clean_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, dir.path());
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    test_toolkit::require_level!(Level::L2, true, Backend::Tmux);

    let records =
        read_backend_executions(&dir.path().join(BACKEND_EXECUTIONS_FILE)).expect("readable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].backend, "tmux");
    assert_eq!(records[0].decision, "run");
    assert!(
        records[0]
            .test
            .ends_with("require_level_records_the_run_decision_under_the_test_name"),
        "unexpected test identity: {}",
        records[0].test
    );
}

#[test]
#[serial_test::serial]
fn require_level_records_a_skip_and_returns_early() {
    let _clean = clean_env();
    let dir = tempfile::tempdir().expect("tempdir");
    let _override = EnvGuard::set_safe(BISCUIT_JUNIT_STAGE_DIR, dir.path());
    let _required = EnvGuard::set_safe(BISCUIT_TEST_REQUIRED_BACKENDS, "tmux");

    let evidence = dir.path().join(BACKEND_EXECUTIONS_FILE);
    skipping_body(&evidence);

    let records = read_backend_executions(&evidence).expect("readable");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].decision, "skip");
    assert_eq!(records[0].backend, "wezterm");
}

fn skipping_body(evidence: &std::path::Path) {
    test_toolkit::require_level!(Level::L2, false, Backend::WezTerm);
    panic!(
        "require_level! must have returned before this line ({})",
        evidence.display()
    );
}

#[test]
#[serial_test::serial]
fn require_level_still_accepts_a_bare_label() {
    let _clean = clean_env();
    let spec: HarnessSpec<'_> = "PTY (/dev/ptmx)".into();
    assert_eq!(spec.backend(), None);

    test_toolkit::require_level!(Level::L1, true, "PTY (/dev/ptmx)");
}

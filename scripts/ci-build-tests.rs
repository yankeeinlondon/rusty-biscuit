//! Tests for `ci-build`.
//!
//! Included from `ci-build.rs` via `#[path]` so they can exercise private
//! items; a `tests/` integration crate cannot reach into a `[[bin]]`.
//!
//! The end-to-end fixtures drive the real wrapper entry point with the host's
//! real `rustc`, because the claim this tool exists to support — "the consumer
//! ran no compiler" — is worthless if it is only ever proven against a stub.
//! They read process environment, which is safe here because the repository
//! runs these through Nextest, and Nextest gives every test its own process.

use super::*;

use std::io::Write as _;

// ---------------------------------------------------------------------------
// Fixtures
// ---------------------------------------------------------------------------

/// Nanosecond resolution so two scratch directories created inside one
/// millisecond by the same process still differ.
fn now_nanos() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| elapsed.as_nanos())
        .unwrap_or_default()
}

/// A private scratch directory that removes itself.
struct TempDir(PathBuf);

impl TempDir {
    fn new(tag: &str) -> Self {
        let base = env::temp_dir().join(format!(
            "ci-build-{}-{}-{}",
            tag,
            std::process::id(),
            now_nanos()
        ));
        fs::create_dir_all(&base).expect("creating the scratch directory");
        Self(base)
    }

    fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn strings(args: &[&str]) -> Vec<String> {
    args.iter().map(|arg| (*arg).to_owned()).collect()
}

fn event(package: &str, configuration: &str, crate_name: &str, start: u128, end: u128) -> Event {
    Event {
        schema_version: EVENT_SCHEMA_VERSION,
        package: package.to_owned(),
        configuration: configuration.to_owned(),
        crate_name: Some(crate_name.to_owned()),
        cargo_package: Some(crate_name.to_owned()),
        primary: false,
        crate_types: vec!["lib".to_owned()],
        target: None,
        probe: false,
        argv_digest: argv_digest("rustc", &strings(&[crate_name]), package, configuration),
        started_ms: start,
        finished_ms: end,
        duration_ms: u64::try_from(end - start).unwrap(),
        exit_code: Some(0),
        pid: 1,
    }
}

fn probe_event(package: &str, configuration: &str) -> Event {
    Event {
        probe: true,
        crate_name: None,
        ..event(package, configuration, "probe", 0, 1)
    }
}

/// A one-line crate the host compiler can build in milliseconds.
fn fixture_crate(dir: &Path) -> PathBuf {
    let path = dir.join("fixture_crate.rs");
    let mut file = fs::File::create(&path).expect("writing the fixture crate");
    file.write_all(b"pub fn answer() -> u8 { 42 }\n")
        .expect("writing the fixture crate body");
    path
}

/// Bind one measured command's environment, exactly as an instrumented CI step
/// does. Wrapper mode itself is selected by the caller.
fn bind_measurement(dir: &Path, package: &str, configuration: &str) {
    env::set_var(COUNTER_DIR_ENV, dir);
    env::set_var(PACKAGE_ENV, package);
    env::set_var(CONFIGURATION_ENV, configuration);
}

// ---------------------------------------------------------------------------
// Argument parsing on the rustc command line
// ---------------------------------------------------------------------------

#[test]
fn a_flag_value_is_read_in_both_spellings() {
    // rustc accepts `--crate-name x` and `--crate-name=x` interchangeably and
    // Cargo emits both over the life of one build; reading only one spelling
    // would silently drop half the crate names from the report.
    let separated = strings(&["--crate-name", "alpha", "--edition", "2021"]);
    let joined = strings(&["--crate-name=alpha", "--edition=2021"]);

    assert_eq!(arg_value(&separated, "--crate-name").as_deref(), Some("alpha"));
    assert_eq!(arg_value(&joined, "--crate-name").as_deref(), Some("alpha"));
    assert_eq!(arg_value(&separated, "--target"), None);
    assert_eq!(arg_value(&joined, "--target"), None);
}

#[test]
fn a_flag_whose_value_is_missing_reads_as_absent_rather_than_panicking() {
    let truncated = strings(&["--crate-name"]);
    assert_eq!(arg_value(&truncated, "--crate-name"), None);
}

#[test]
fn every_crate_type_is_collected_from_repeats_and_comma_lists() {
    assert_eq!(crate_types(&strings(&["--crate-type", "lib"])), vec!["lib"]);
    assert_eq!(
        crate_types(&strings(&["--crate-type", "lib", "--crate-type", "cdylib"])),
        vec!["lib", "cdylib"]
    );
    assert_eq!(
        crate_types(&strings(&["--crate-type=lib,cdylib"])),
        vec!["lib", "cdylib"]
    );
    assert!(crate_types(&strings(&["--crate-name", "alpha"])).is_empty());
}

#[test]
fn a_version_probe_is_recognized_in_every_spelling_cargo_uses() {
    assert!(is_probe(&strings(&["-vV"])));
    assert!(is_probe(&strings(&["--version"])));
    assert!(is_probe(&strings(&["-V"])));
    assert!(!is_probe(&strings(&["--crate-name", "alpha"])));
}

// ---------------------------------------------------------------------------
// The digest boundary
// ---------------------------------------------------------------------------

#[test]
fn the_digest_is_deterministic_and_separates_configurations() {
    let args = strings(&["--crate-name", "alpha"]);
    let a = argv_digest("rustc", &args, "alpha", "L1 ubuntu-latest");
    let b = argv_digest("rustc", &args, "alpha", "L1 ubuntu-latest");
    let other_configuration = argv_digest("rustc", &args, "alpha", "L2 ubuntu-latest");
    let other_args = argv_digest("rustc", &strings(&["--crate-name", "beta"]), "alpha", "L1 ubuntu-latest");

    assert_eq!(a, b, "the same inputs must hash the same way");
    assert_ne!(a, other_configuration);
    assert_ne!(a, other_args);
    assert_eq!(a.len(), 16, "a 64-bit xxHash renders as 16 hex digits");
}

#[test]
fn the_digest_cannot_be_forged_by_moving_a_separator_between_fields() {
    // Concatenating the fields without a separator would make
    // {"alpha", "beta"} and {"alphabet", "a"} hash identically.
    let args = strings(&[]);
    assert_ne!(
        argv_digest("rustc", &args, "alpha", "beta"),
        argv_digest("rustc", &args, "alphabet", "a")
    );
}

// ---------------------------------------------------------------------------
// Event files
// ---------------------------------------------------------------------------

#[test]
fn two_identical_events_both_survive_rather_than_overwriting_each_other() {
    // Cargo runs rustc many times concurrently. An event file name derived only
    // from the command line would make a parallel build under-count itself,
    // which is the exact failure this tool exists to detect.
    let dir = TempDir::new("collision");
    let same = event("alpha", "L1 ubuntu-latest", "serde", 10, 20);

    let first = write_event(dir.path(), &same).expect("first event");
    let second = write_event(dir.path(), &same).expect("second event");

    assert_ne!(first, second);
    let read = read_events(dir.path()).expect("reading both events back");
    assert_eq!(read.len(), 2);
    assert_eq!(summarize(&read).compiler_invocations, 2);
}

#[test]
fn a_repeated_write_read_round_trip_accumulates_rather_than_replacing() {
    let dir = TempDir::new("roundtrip");

    write_event(dir.path(), &event("alpha", "L1", "serde", 0, 10)).expect("first write");
    let after_one = summarize(&read_events(dir.path()).expect("first read"));
    assert_eq!(after_one.compiler_invocations, 1);

    write_event(dir.path(), &event("alpha", "L1", "anyhow", 10, 30)).expect("second write");
    let after_two = summarize(&read_events(dir.path()).expect("second read"));
    assert_eq!(after_two.compiler_invocations, 2);
    assert_eq!(after_two.slices[0].distinct_crates, 2);
    assert_eq!(after_two.compiler_ms, 30);
    assert_eq!(after_two.window_ms, 30);
}

#[test]
fn an_event_written_by_a_future_generation_is_refused_by_name() {
    let dir = TempDir::new("schema");
    let mut future = event("alpha", "L1", "serde", 0, 1);
    future.schema_version = EVENT_SCHEMA_VERSION + 1;
    let path = dir.path().join("00000-1-deadbeefdeadbeef.json");
    fs::write(&path, serde_json::to_string(&future).unwrap()).expect("writing the future event");

    let error = read_events(dir.path()).expect_err("a newer generation must be refused");
    let message = format!("{error:#}");
    assert!(message.contains("event schema version"), "{message}");
    assert!(
        message.contains(&(EVENT_SCHEMA_VERSION + 1).to_string()),
        "the refusal must name the generation it found: {message}"
    );
}

#[test]
fn a_malformed_event_file_names_itself_in_the_failure() {
    let dir = TempDir::new("malformed");
    fs::write(dir.path().join("broken.json"), "{not json").expect("writing the malformed event");

    let error = read_events(dir.path()).expect_err("malformed input must be refused");
    assert!(format!("{error:#}").contains("broken.json"), "{error:#}");
}

#[test]
fn a_counter_directory_that_was_never_created_reads_as_empty() {
    // A measured command that compiled nothing is a legitimate outcome; it must
    // not look like a tool failure.
    let dir = TempDir::new("absent");
    let missing = dir.path().join("never-used");
    assert!(read_events(&missing).expect("an absent directory is empty").is_empty());
}

#[test]
fn a_non_event_file_beside_the_events_is_ignored() {
    let dir = TempDir::new("mixed");
    write_event(dir.path(), &event("alpha", "L1", "serde", 0, 5)).expect("one event");
    fs::write(dir.path().join("notes.txt"), "not an event").expect("writing a stray file");

    assert_eq!(read_events(dir.path()).expect("reading").len(), 1);
}

// ---------------------------------------------------------------------------
// Aggregation
// ---------------------------------------------------------------------------

#[test]
fn work_is_aggregated_by_package_and_configuration() {
    let events = vec![
        event("alpha", "L1 ubuntu-latest", "serde", 0, 10),
        event("alpha", "L1 ubuntu-latest", "anyhow", 5, 20),
        event("alpha", "L2 ubuntu-latest", "serde", 100, 130),
        event("beta", "L1 ubuntu-latest", "serde", 200, 210),
    ];

    let report = summarize(&events);

    assert_eq!(report.slices.len(), 3);
    assert_eq!(report.compiler_invocations, 4);
    let l1 = &report.slices[0];
    assert_eq!((l1.package.as_str(), l1.configuration.as_str()), ("alpha", "L1 ubuntu-latest"));
    assert_eq!(l1.compiler_invocations, 2);
    assert_eq!(l1.distinct_crates, 2);
    // Two overlapping invocations: 25ms of compiler time inside a 20ms window.
    // Reporting only the window would hide half the work a parallel build did.
    assert_eq!(l1.compiler_ms, 25);
    assert_eq!(l1.window_ms, 20);
}

#[test]
fn the_same_dependency_compiled_under_two_tiers_is_counted_twice() {
    // This is the defect the specification describes: L1 and L2 compile the
    // same crate for the same package and environment. The counter must report
    // two invocations, not one deduplicated crate.
    let events = vec![
        event("claudine", "L1 ubuntu-latest", "claudine", 0, 100),
        event("claudine", "L2 ubuntu-latest", "claudine", 200, 300),
    ];

    let report = summarize(&events);

    assert_eq!(report.compiler_invocations, 2);
    assert_eq!(report.slices.len(), 2);
    assert!(report.slices.iter().all(|slice| slice.distinct_crates == 1));
}

#[test]
fn version_probes_are_counted_separately_and_never_as_compiler_work() {
    let events = vec![
        probe_event("alpha", "L1"),
        probe_event("alpha", "L1"),
        event("alpha", "L1", "serde", 10, 20),
    ];

    let report = summarize(&events);

    assert_eq!(report.compiler_invocations, 1);
    assert_eq!(report.probes, 2);
    assert_eq!(report.slices[0].probes, 2);
    // A probe's timestamps must not widen the measured build window.
    assert_eq!(report.slices[0].window_ms, 10);
}

#[test]
fn a_primary_package_invocation_is_distinguished_from_its_dependency_closure() {
    let mut own = event("alpha", "L1", "alpha", 0, 10);
    own.primary = true;
    let events = vec![own, event("alpha", "L1", "serde", 0, 10)];

    let report = summarize(&events);

    assert_eq!(report.compiler_invocations, 2);
    assert_eq!(report.primary_invocations, 1);
    assert_eq!(report.slices[0].primary_invocations, 1);
}

#[test]
fn a_failed_compiler_invocation_is_recorded_as_failed() {
    let mut failed = event("alpha", "L1", "alpha", 0, 10);
    failed.exit_code = Some(1);
    let mut lost = event("alpha", "L1", "beta", 0, 10);
    lost.exit_code = None;

    let report = summarize(&[failed, lost]);

    assert_eq!(report.slices[0].failed_invocations, 2);
}

#[test]
fn an_empty_counter_summarizes_to_explicit_zeroes() {
    let report = summarize(&[]);
    assert_eq!(report.events_read, 0);
    assert_eq!(report.compiler_invocations, 0);
    assert_eq!(report.window_ms, 0);
    assert!(report.slices.is_empty());
    assert_eq!(report.schema_version, REPORT_SCHEMA_VERSION);
    assert_eq!(report.event_schema_version, EVENT_SCHEMA_VERSION);
}

// ---------------------------------------------------------------------------
// Rendering
// ---------------------------------------------------------------------------

#[test]
fn an_empty_report_says_so_rather_than_reading_as_proof_of_reuse() {
    let rendered = render(&summarize(&[]), &Terminal::new());
    assert!(rendered.contains("No compiler work recorded"), "{rendered}");
    assert!(
        rendered.contains("never proof of reuse"),
        "a zero must not be presentable as a compile-once result: {rendered}"
    );
}

#[test]
fn a_populated_report_shows_every_label_and_its_counts() {
    let report = summarize(&[
        event("alpha", "L1 ubuntu-latest", "serde", 0, 10),
        event("beta", "L2 macos-latest", "anyhow", 0, 10),
    ]);

    let rendered = render(&report, &Terminal::new());

    for needle in ["alpha", "beta", "L1 ubuntu-latest", "L2 macos-latest"] {
        assert!(rendered.contains(needle), "{needle} missing from:\n{rendered}");
    }
    assert_eq!(rows(&report).len(), 2);
    assert_eq!(rows(&report)[0][2], "1", "the rustc column is the invocation count");
}

#[test]
fn a_failed_invocation_is_reported_beside_the_table() {
    let mut failed = event("alpha", "L1", "alpha", 0, 10);
    failed.exit_code = Some(1);

    let rendered = render(&summarize(&[failed]), &Terminal::new());

    assert!(rendered.contains("exited non-zero"), "{rendered}");
    // Measured against a real `just _test biscuit-hash`: a fully green run
    // reports one, from a build-script feature probe. The report must say so,
    // or the first reader treats a healthy build as broken.
    assert!(
        rendered.contains("autocfg"),
        "the non-zero count must disclose that probe builds fail deliberately: {rendered}"
    );
}

#[test]
fn excluded_probes_are_disclosed_rather_than_dropped_silently() {
    let rendered = render(
        &summarize(&[probe_event("alpha", "L1"), event("alpha", "L1", "serde", 0, 5)]),
        &Terminal::new(),
    );
    assert!(rendered.contains("version/target probe"), "{rendered}");
}

// ---------------------------------------------------------------------------
// Command line
// ---------------------------------------------------------------------------

#[test]
fn report_takes_its_directory_from_the_flag_in_both_spellings_or_the_environment() {
    let separated = parse(&strings(&["report", "--counter-dir", "/tmp/a"]), None).unwrap();
    let joined = parse(&strings(&["report", "--counter-dir=/tmp/a"]), None).unwrap();
    let inherited = parse(&strings(&["report"]), Some(PathBuf::from("/tmp/a"))).unwrap();

    let expected = Invocation::Report {
        dir: PathBuf::from("/tmp/a"),
        json: false,
    };
    assert_eq!(separated, expected);
    assert_eq!(joined, expected);
    assert_eq!(inherited, expected);
}

#[test]
fn an_explicit_directory_overrides_the_inherited_one() {
    let parsed = parse(
        &strings(&["report", "--counter-dir", "/tmp/explicit"]),
        Some(PathBuf::from("/tmp/inherited")),
    )
    .unwrap();
    assert_eq!(
        parsed,
        Invocation::Report {
            dir: PathBuf::from("/tmp/explicit"),
            json: false
        }
    );
}

#[test]
fn a_report_with_no_directory_anywhere_names_the_environment_variable() {
    let error = parse(&strings(&["report"]), None).expect_err("a directory is required");
    assert!(format!("{error:#}").contains(COUNTER_DIR_ENV), "{error:#}");
}

#[test]
fn json_selects_the_machine_interface() {
    assert_eq!(
        parse(&strings(&["report", "--json", "--counter-dir=/tmp/a"]), None).unwrap(),
        Invocation::Report {
            dir: PathBuf::from("/tmp/a"),
            json: true
        }
    );
}

#[test]
fn an_unknown_subcommand_or_flag_is_refused_rather_than_ignored() {
    let subcommand = parse(&strings(&["summarise"]), None).expect_err("unknown subcommand");
    assert!(format!("{subcommand:#}").contains("summarise"), "{subcommand:#}");

    let flag = parse(&strings(&["report", "--counter-dir=/tmp/a", "--verbose"]), None)
        .expect_err("unknown flag");
    assert!(format!("{flag:#}").contains("--verbose"), "{flag:#}");
}

#[test]
fn no_arguments_and_help_both_print_usage() {
    assert_eq!(parse(&[], None).unwrap(), Invocation::Usage);
    assert_eq!(parse(&strings(&["--help"]), None).unwrap(), Invocation::Usage);
    assert_eq!(parse(&strings(&["help"]), None).unwrap(), Invocation::Usage);
    assert!(USAGE.contains(COUNTER_DIR_ENV));
    assert!(USAGE.contains(WRAP_ENV));
}

#[test]
fn reset_clears_only_event_files_and_reports_how_many() {
    let dir = TempDir::new("reset");
    write_event(dir.path(), &event("alpha", "L1", "serde", 0, 5)).expect("one event");
    write_event(dir.path(), &event("alpha", "L1", "anyhow", 0, 5)).expect("another event");
    fs::write(dir.path().join("keep.txt"), "kept").expect("writing a stray file");

    assert_eq!(reset(dir.path()).expect("resetting"), 2);
    assert!(read_events(dir.path()).expect("reading").is_empty());
    assert!(dir.path().join("keep.txt").exists());
}

#[test]
fn reset_creates_a_directory_that_does_not_exist_yet() {
    let dir = TempDir::new("reset-new");
    let fresh = dir.path().join("fresh");
    assert_eq!(reset(&fresh).expect("resetting a fresh directory"), 0);
    assert!(fresh.is_dir());
}

// ---------------------------------------------------------------------------
// Planned build keys
// ---------------------------------------------------------------------------

#[test]
fn key_reads_a_file_or_stdin_and_rejects_an_unknown_flag() {
    assert_eq!(
        parse(&strings(&["key", "--input", "/tmp/request.json"]), None).unwrap(),
        Invocation::Key {
            input: Some(PathBuf::from("/tmp/request.json"))
        }
    );
    assert_eq!(
        parse(&strings(&["key", "--input=-"]), None).unwrap(),
        Invocation::Key { input: None }
    );
    assert_eq!(
        parse(&strings(&["key"]), None).unwrap(),
        Invocation::Key { input: None }
    );
    // An unknown flag is named without its value: `--salt=1` and `--salt 1`
    // are the same mistake and should read the same way.
    let error = parse(&strings(&["key", "--salt=1"]), None).expect_err("unknown flag");
    assert!(format!("{error:#}").contains("--salt"), "{error:#}");
}

#[test]
fn a_key_is_sixteen_hex_digits_of_xxhash_and_is_stable_across_runs() {
    let request = KeyRequest {
        schema_version: KEY_SCHEMA_VERSION,
        material: vec![r#"{"package":"claudine","target":"x86_64-unknown-linux-gnu"}"#.to_owned()],
    };
    let first = keys(&request).expect("digesting");
    let second = keys(&request).expect("digesting again");

    assert_eq!(first, second);
    assert_eq!(first.schema_version, KEY_SCHEMA_VERSION);
    assert_eq!(first.keys.len(), 1);
    assert_eq!(first.keys[0].len(), 16);
    assert!(
        first.keys[0].chars().all(|c| c.is_ascii_hexdigit() && !c.is_ascii_uppercase()),
        "{}",
        first.keys[0]
    );
    // The digest is `biscuit-hash`'s, not a local reimplementation: the value
    // must be reproducible by any other caller of `xx_hash`.
    assert_eq!(first.keys[0], format!("{:016x}", xx_hash(&request.material[0])));
}

#[test]
fn one_differing_byte_of_material_produces_another_key() {
    let base = keys(&KeyRequest {
        schema_version: KEY_SCHEMA_VERSION,
        material: vec![r#"{"features":""}"#.to_owned()],
    })
    .expect("digesting");
    let changed = keys(&KeyRequest {
        schema_version: KEY_SCHEMA_VERSION,
        material: vec![r#"{"features":"--all-features"}"#.to_owned()],
    })
    .expect("digesting");
    assert_ne!(base.keys, changed.keys);
}

#[test]
fn a_batch_answers_in_request_order_and_repeats_an_identical_input() {
    let response = keys(&KeyRequest {
        schema_version: KEY_SCHEMA_VERSION,
        material: vec!["alpha".to_owned(), "beta".to_owned(), "alpha".to_owned()],
    })
    .expect("digesting");
    assert_eq!(response.keys.len(), 3);
    assert_eq!(response.keys[0], response.keys[2]);
    assert_ne!(response.keys[0], response.keys[1]);
}

#[test]
fn an_empty_batch_answers_with_an_empty_list_rather_than_failing() {
    let response = keys(&KeyRequest {
        schema_version: KEY_SCHEMA_VERSION,
        material: Vec::new(),
    })
    .expect("digesting");
    assert!(response.keys.is_empty());
}

#[test]
fn a_key_request_from_another_generation_is_refused() {
    let error = keys(&KeyRequest {
        schema_version: KEY_SCHEMA_VERSION + 1,
        material: vec!["alpha".to_owned()],
    })
    .expect_err("another generation");
    assert!(format!("{error:#}").contains("schema version"), "{error:#}");
}

// ---------------------------------------------------------------------------
// End to end, through the real wrapper entry point and the real compiler
// ---------------------------------------------------------------------------

#[test]
fn the_wrapper_compiles_the_real_crate_and_records_exactly_one_event() {
    let dir = TempDir::new("e2e-compile");
    let counters = dir.path().join("events");
    let source = fixture_crate(dir.path());
    let out = dir.path().join("out");
    fs::create_dir_all(&out).expect("creating the output directory");
    bind_measurement(&counters, "fixture", "L1 host");

    let code = run_wrapper(vec![
        "rustc".to_owned(),
        "--crate-name".to_owned(),
        "fixture_crate".to_owned(),
        "--crate-type".to_owned(),
        "lib".to_owned(),
        "--emit=metadata".to_owned(),
        "--out-dir".to_owned(),
        out.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("the wrapper must run the compiler");

    assert_eq!(code, 0, "the fixture crate compiles");
    assert!(
        fs::read_dir(&out).unwrap().next().is_some(),
        "the wrapper must exec the real compiler, not swallow the invocation"
    );

    let events = read_events(&counters).expect("reading the recorded event");
    assert_eq!(events.len(), 1);
    let recorded = &events[0];
    assert_eq!(recorded.package, "fixture");
    assert_eq!(recorded.configuration, "L1 host");
    assert_eq!(recorded.crate_name.as_deref(), Some("fixture_crate"));
    assert_eq!(recorded.crate_types, vec!["lib"]);
    assert!(!recorded.probe);
    assert_eq!(recorded.exit_code, Some(0));
    assert_eq!(recorded.schema_version, EVENT_SCHEMA_VERSION);

    let report = summarize(&events);
    assert_eq!(report.compiler_invocations, 1);
    assert_eq!(report.slices[0].distinct_crates, 1);
}

#[test]
fn the_wrapper_records_a_version_probe_without_counting_it_as_compiler_work() {
    // This is Cargo's very first call to any wrapper. Counting it would make
    // every consumer look like it compiled something.
    let dir = TempDir::new("e2e-probe");
    let counters = dir.path().join("events");
    bind_measurement(&counters, "fixture", "L1 host");

    let code = run_wrapper(vec!["rustc".to_owned(), "-vV".to_owned()])
        .expect("the wrapper must run the probe");

    assert_eq!(code, 0);
    let report = summarize(&read_events(&counters).expect("reading"));
    assert_eq!(report.compiler_invocations, 0);
    assert_eq!(report.probes, 1);
}

#[test]
fn the_wrapper_propagates_a_compiler_failure_and_records_it() {
    let dir = TempDir::new("e2e-failure");
    let counters = dir.path().join("events");
    bind_measurement(&counters, "fixture", "L1 host");

    let code = run_wrapper(vec![
        "rustc".to_owned(),
        "--crate-name".to_owned(),
        "fixture_crate".to_owned(),
        "--this-flag-does-not-exist".to_owned(),
    ])
    .expect("the wrapper must still return the compiler's status");

    assert_ne!(code, 0, "a failed compile must stay a failed compile");
    let report = summarize(&read_events(&counters).expect("reading"));
    assert_eq!(report.compiler_invocations, 1);
    assert_eq!(report.slices[0].failed_invocations, 1);
}

#[test]
fn the_wrapper_records_nothing_when_no_counter_directory_is_bound() {
    // The wrapper must be harmless when the measurement environment is absent:
    // a stray `RUSTC_WRAPPER` without a counter directory compiles normally.
    let dir = TempDir::new("e2e-unbound");
    let source = fixture_crate(dir.path());
    let out = dir.path().join("out");
    fs::create_dir_all(&out).expect("creating the output directory");
    env::remove_var(COUNTER_DIR_ENV);

    let code = run_wrapper(vec![
        "rustc".to_owned(),
        "--crate-name".to_owned(),
        "fixture_crate".to_owned(),
        "--crate-type".to_owned(),
        "lib".to_owned(),
        "--emit=metadata".to_owned(),
        "--out-dir".to_owned(),
        out.to_string_lossy().into_owned(),
        source.to_string_lossy().into_owned(),
    ])
    .expect("the wrapper must still compile");

    assert_eq!(code, 0);
    assert!(fs::read_dir(&out).unwrap().next().is_some());
}

#[test]
fn a_missing_compiler_is_an_error_rather_than_a_silent_success() {
    let dir = TempDir::new("e2e-missing");
    bind_measurement(&dir.path().join("events"), "fixture", "L1 host");

    let error = run_wrapper(vec!["definitely-not-a-real-compiler-9ce1".to_owned()])
        .expect_err("a missing compiler must fail");

    assert!(
        format!("{error:#}").contains("definitely-not-a-real-compiler-9ce1"),
        "{error:#}"
    );
}

#[test]
fn the_wrapper_needs_a_command() {
    let error = run_wrapper(Vec::new()).expect_err("an empty command line must fail");
    assert!(format!("{error:#}").contains("no command"), "{error:#}");
}

#[test]
fn wrapper_mode_is_selected_only_by_a_non_empty_flag() {
    env::remove_var(WRAP_ENV);
    assert!(!in_wrapper_mode());
    env::set_var(WRAP_ENV, "");
    assert!(
        !in_wrapper_mode(),
        "an empty value must not arm the wrapper: CI spells a disabled flag that way"
    );
    env::set_var(WRAP_ENV, "1");
    assert!(in_wrapper_mode());
    env::remove_var(WRAP_ENV);
}

#[test]
fn an_empty_counter_directory_variable_reads_as_unbound() {
    env::set_var(COUNTER_DIR_ENV, "");
    assert_eq!(counter_dir_from_env(), None);
    env::set_var(COUNTER_DIR_ENV, "/tmp/bound");
    assert_eq!(counter_dir_from_env(), Some(PathBuf::from("/tmp/bound")));
    env::remove_var(COUNTER_DIR_ENV);
}

#[test]
fn the_json_report_is_a_versioned_document_a_status_artifact_can_embed() {
    let dir = TempDir::new("json");
    write_event(dir.path(), &event("alpha", "L1 ubuntu-latest", "serde", 0, 10))
        .expect("one event");

    let report = summarize(&read_events(dir.path()).expect("reading"));
    let json: serde_json::Value =
        serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();

    assert_eq!(json["schema_version"], REPORT_SCHEMA_VERSION);
    assert_eq!(json["event_schema_version"], EVENT_SCHEMA_VERSION);
    assert_eq!(json["compiler_invocations"], 1);
    assert_eq!(json["slices"][0]["package"], "alpha");
    assert_eq!(json["slices"][0]["configuration"], "L1 ubuntu-latest");
    assert_eq!(json["slices"][0]["compiler_ms"], 10);
}

/// The report document this tool serializes is what `ci-build produce` stores
/// as a manifest's `compiler_work`, and what the JavaScript publisher
/// (`scripts/ci/artifacts/publish.cjs`) aggregates field by field into each
/// owner's measurement artifact. Node cannot see this struct, so its test
/// asserts against a fixture generated here rather than a hand-copied shape: a
/// renamed or dropped field fails this test before it can silently zero the
/// owner totals AC8 is read from.
///
/// It sits outside `scripts/ci/` because the retired baseline instrument
/// carried this file's tests onto a pre-cutover revision and refused any
/// construction that added a `scripts/ci/` path. Nothing enforces that now;
/// the location is kept because moving a fixture buys nothing.
const PUBLISHER_FIXTURE: &str = "fixtures/compiler-work/publisher-documents.json";

const BLESS_PUBLISHER_FIXTURE: &str = "BLESS_COMPILER_WORK_FIXTURE";

/// The documents the publisher's fixture is built from: one ordinary measured
/// build, one that spans two configurations, and one whose only rustc
/// invocations were version probes — a real zero the publisher must not be able
/// to present as compile-once evidence.
fn publisher_documents() -> serde_json::Value {
    let mut own = event("alpha", "L1 ubuntu-latest", "alpha", 0, 120);
    own.primary = true;
    let alpha = vec![
        own,
        event("alpha", "L1 ubuntu-latest", "serde", 0, 90),
        event("alpha", "L1 ubuntu-latest", "anyhow", 10, 60),
        probe_event("alpha", "L1 ubuntu-latest"),
    ];
    let beta = vec![
        event("beta", "L1 ubuntu-latest", "serde", 0, 40),
        event("beta", "L2 ubuntu-latest", "serde", 100, 150),
    ];
    let probes_only = vec![
        probe_event("gamma", "L1 ubuntu-latest"),
        probe_event("gamma", "L1 ubuntu-latest"),
    ];
    serde_json::json!({
        "alpha": summarize(&alpha),
        "beta": summarize(&beta),
        "probes_only": summarize(&probes_only),
    })
}

#[test]
fn the_javascript_publisher_reads_a_fixture_generated_from_this_report() {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join(PUBLISHER_FIXTURE);
    let mut produced = serde_json::to_string_pretty(&publisher_documents())
        .expect("a report always serializes");
    produced.push('\n');

    if env::var_os(BLESS_PUBLISHER_FIXTURE).is_some_and(|value| !value.is_empty()) {
        fs::create_dir_all(path.parent().expect("the fixture path has a parent"))
            .expect("creating the fixture directory");
        fs::write(&path, &produced).expect("writing the fixture");
        return;
    }

    let committed = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!(
            "missing {} ({error}); regenerate with {BLESS_PUBLISHER_FIXTURE}=1",
            path.display()
        )
    });
    assert_eq!(
        produced,
        committed,
        "{} no longer matches this tool's own serialization, so the publisher's \
         test is asserting against a document the producer never writes. \
         Regenerate with {BLESS_PUBLISHER_FIXTURE}=1 once the diff is intentional.",
        path.display()
    );
}

// ---------------------------------------------------------------------------
// End to end, through Cargo's real `RUSTC_WRAPPER` contract
// ---------------------------------------------------------------------------
//
// The fixtures above call `run_wrapper` directly, which proves the recording
// but not the integration: the whole measurement rests on Cargo actually
// invoking `$RUSTC_WRAPPER <rustc> <args…>`, once per compile, around a build
// script as well as a library. These two drive the shipped binary through that
// path instead.
//
// They are deliberately coarse. Nextest runs each test in its own process, and
// each process would otherwise have to resolve the binary again; a `[[bin]]`'s
// own test harness does not build its binary and Cargo publishes no
// `CARGO_BIN_EXE_…` for it, so resolution means a (cached) `cargo build`.

/// Absolute path to the built `ci-build`, rebuilt before it is driven.
///
/// `current_exe()` is `…/target/<profile>/deps/ci_build-<hash>` under both
/// Nextest and `cargo test`, so the profile directory is two levels up. The
/// build is what makes this suite self-sufficient rather than dependent on a
/// preceding recipe step.
///
/// **Unconditional, deliberately.** `cargo nextest run --bin ci-build` builds
/// the test harness, not the binary beside it, so a stale `ci-build` from an
/// earlier build served every end-to-end fixture here and made the suite pass
/// against code that was no longer there. It cost a confusing failure twice.
/// On a warm tree Cargo answers in well under a second; the alternative is a
/// green run that proves nothing.
///
/// `pub(crate)` because the archive fixtures drive the same shipped binary; a
/// second resolver would be a second chance to test something other than what
/// ships.
///
/// Cargo is resolved at run time, never through `env!("CARGO")`: that value
/// is the PRODUCER's toolchain path, and this suite runs from an archive on a
/// consumer that provisions its own (run 35326800778 failed 14 fixtures here
/// with `NotFound` for the baked path).
pub(crate) fn shipped_wrapper() -> PathBuf {
    let exe = env::current_exe().expect("the test binary has a path");
    let profile_dir = exe
        .parent()
        .and_then(Path::parent)
        .expect("…/target/<profile>/deps/<test binary>")
        .to_path_buf();
    let binary = profile_dir.join(format!("ci-build{}", env::consts::EXE_SUFFIX));
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let status = Command::new(cargo_bin())
        .arg("build")
        .arg("--quiet")
        .args(["--no-default-features", "--features", "build-tools"])
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--bin")
        .arg("ci-build")
        .status()
        .expect("building the ci-build binary");
    assert!(status.success(), "ci-build must build before it can be driven");
    assert!(binary.exists(), "expected {} to exist", binary.display());
    private_handle(&profile_dir, &binary)
}

/// A private path to `binary` that a concurrent rebuild cannot invalidate.
///
/// Nextest runs one process per test and every fixture here rebuilds the shipped
/// binary, so a stale tree has several Cargos replacing `ci-build` at once. A
/// test that had already handed that path to Cargo as its `RUSTC_WRAPPER` then
/// execs into the replacement window and fails with a bare `No such file or
/// directory` — a flake whose message names neither the wrapper nor the race.
/// A hard link survives it: Cargo writes the new build to a new inode, and this
/// name keeps resolving to the complete bytes the caller verified.
/// The `cargo` this process should drive: the run-time `CARGO` when the runner
/// set one, else the `cargo` on PATH, which the rustup proxy resolves through
/// the repository's pinned toolchain.
pub(crate) fn cargo_bin() -> std::ffi::OsString {
    env::var_os("CARGO").unwrap_or_else(|| "cargo".into())
}

fn private_handle(profile_dir: &Path, binary: &Path) -> PathBuf {
    let links = profile_dir.join(".ci-build-wrappers");
    if fs::create_dir_all(&links).is_err() {
        return binary.to_path_buf();
    }
    let private = links.join(format!(
        "ci-build-{}{}",
        std::process::id(),
        env::consts::EXE_SUFFIX
    ));
    let _ = fs::remove_file(&private);
    // Copy is the fallback rather than the failure: a filesystem without hard
    // links still gets an exec target of its own, just at the cost of the copy.
    if fs::hard_link(binary, &private).is_err() && fs::copy(binary, &private).is_err() {
        return binary.to_path_buf();
    }
    private
}

/// A dependency-free package with a build script, so the fixture exercises the
/// `build_script_build` compile as well as the library compile that follows it.
/// No registry access is needed.
fn fixture_package(root: &Path) -> PathBuf {
    let dir = root.join("fixture");
    fs::create_dir_all(dir.join("src")).expect("creating the fixture package");
    fs::write(
        dir.join("Cargo.toml"),
        "[workspace]\n\
         [package]\n\
         name = \"ci_build_fixture\"\n\
         version = \"0.1.0\"\n\
         edition = \"2021\"\n\
         publish = false\n",
    )
    .expect("writing the fixture manifest");
    fs::write(dir.join("build.rs"), "fn main() {}\n").expect("writing the fixture build script");
    fs::write(dir.join("src/lib.rs"), "pub fn answer() -> u8 {\n    42\n}\n")
        .expect("writing the fixture library");
    dir
}

/// One `cargo build` of the fixture, measured exactly as an instrumented CI
/// step measures one: `RUSTC_WRAPPER` set on this command and nothing else.
fn measured_build(
    wrapper: &Path,
    manifest: &Path,
    target_dir: &Path,
    counters: &Path,
    configuration: &str,
) {
    let status = Command::new(cargo_bin())
        .arg("build")
        .arg("--quiet")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir)
        .env("RUSTC_WRAPPER", wrapper)
        .env(WRAP_ENV, "1")
        .env(COUNTER_DIR_ENV, counters)
        .env(PACKAGE_ENV, "ci_build_fixture")
        .env(CONFIGURATION_ENV, configuration)
        .status()
        .expect("running cargo under the wrapper");
    assert!(
        status.success(),
        "the fixture must build: a wrapper that breaks the build is worse than no measurement"
    );
}

/// The shipped binary's own `--json` machine interface, parsed.
fn shipped_report(wrapper: &Path, counters: &Path) -> serde_json::Value {
    let output = Command::new(wrapper)
        .arg("report")
        .arg("--counter-dir")
        .arg(counters)
        .arg("--json")
        .output()
        .expect("running ci-build report");
    assert!(
        output.status.success(),
        "report failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("the report is JSON")
}

#[test]
fn cargo_drives_the_shipped_wrapper_and_a_warm_rebuild_records_no_work() {
    // The distinction this whole tool exists for: wall-clock time cannot tell a
    // restored `Swatinem/rust-cache` from genuine reuse, but the compiler-work
    // count can. A cold build must record work; an identical warm rebuild must
    // record none.
    let wrapper = shipped_wrapper();
    let scratch = TempDir::new("e2e-cargo");
    let package = fixture_package(scratch.path());
    let manifest = package.join("Cargo.toml");
    let target_dir = scratch.path().join("target");

    let cold = scratch.path().join("cold-events");
    measured_build(&wrapper, &manifest, &target_dir, &cold, "L1 host");
    let cold_report = shipped_report(&wrapper, &cold);

    assert_eq!(cold_report["schema_version"], REPORT_SCHEMA_VERSION);
    assert!(
        cold_report["compiler_invocations"].as_u64().unwrap() >= 2,
        "a build script plus a library is at least two rustc invocations: {cold_report}"
    );
    assert!(
        cold_report["primary_invocations"].as_u64().unwrap() >= 1,
        "the fixture is the selected package: {cold_report}"
    );
    assert!(
        cold_report["probes"].as_u64().unwrap() >= 1,
        "Cargo always asks a wrapper for `rustc -vV` first: {cold_report}"
    );
    let slices = cold_report["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 1, "one measured label, one slice: {cold_report}");
    assert_eq!(slices[0]["package"], "ci_build_fixture");
    assert_eq!(slices[0]["configuration"], "L1 host");
    assert!(slices[0]["distinct_crates"].as_u64().unwrap() >= 2, "{cold_report}");

    // Same target directory, same inputs: Cargo has nothing to do.
    let warm = scratch.path().join("warm-events");
    measured_build(&wrapper, &manifest, &target_dir, &warm, "L1 host");
    let warm_report = shipped_report(&wrapper, &warm);

    assert_eq!(
        warm_report["compiler_invocations"], 0,
        "a fully cached rebuild must record zero compiler work: {warm_report}"
    );
    // Cargo caches the wrapper's `rustc -vV` answer in the build fingerprint,
    // so a genuine no-op build does not invoke the wrapper even once. The
    // probe count is therefore free to be zero here; what must never happen is
    // a probe being counted as compiler work, which the unit fixtures pin.
    assert_eq!(
        warm_report["compiler_invocations"].as_u64().unwrap()
            + warm_report["probes"].as_u64().unwrap(),
        warm_report["events_read"].as_u64().unwrap(),
        "every recorded event is either compiler work or a probe: {warm_report}"
    );

    // And a reset restores the counter to a state that records again, rather
    // than staying empty and looking like proof of reuse forever.
    let status = Command::new(&wrapper)
        .arg("reset")
        .arg("--counter-dir")
        .arg(&cold)
        .status()
        .expect("running ci-build reset");
    assert!(status.success());
    assert_eq!(shipped_report(&wrapper, &cold)["events_read"], 0);
    measured_build(
        &wrapper,
        &manifest,
        &scratch.path().join("target-after-reset"),
        &cold,
        "L1 host",
    );
    assert!(shipped_report(&wrapper, &cold)["compiler_invocations"].as_u64().unwrap() > 0);
}

#[test]
fn two_tier_configurations_compile_the_same_package_twice_and_are_reported_apart() {
    // This is the defect under measurement: L1 and L2 compile the same package
    // in separate jobs. Two target directories stand in for two runners.
    //
    // It also proves the negative the instrumentation must never get wrong: a
    // build that sets no wrapper contributes nothing, so a stale counter
    // directory cannot be read as evidence about an unmeasured command.
    let wrapper = shipped_wrapper();
    let scratch = TempDir::new("e2e-tiers");
    let package = fixture_package(scratch.path());
    let manifest = package.join("Cargo.toml");
    let counters = scratch.path().join("events");

    measured_build(&wrapper, &manifest, &scratch.path().join("t1"), &counters, "L1 host");
    measured_build(&wrapper, &manifest, &scratch.path().join("t2"), &counters, "L2 host");

    let report = shipped_report(&wrapper, &counters);
    let slices = report["slices"].as_array().expect("slices");
    assert_eq!(slices.len(), 2, "{report}");
    assert_eq!(slices[0]["configuration"], "L1 host");
    assert_eq!(slices[1]["configuration"], "L2 host");
    assert_eq!(
        slices[0]["compiler_invocations"], slices[1]["compiler_invocations"],
        "the same work done twice is what the specification calls the defect: {report}"
    );

    let before = report["events_read"].as_u64().unwrap();
    let status = Command::new(cargo_bin())
        .arg("build")
        .arg("--quiet")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(&manifest)
        .arg("--target-dir")
        .arg(scratch.path().join("t3"))
        .env_remove("RUSTC_WRAPPER")
        .env_remove(WRAP_ENV)
        .env(COUNTER_DIR_ENV, &counters)
        .status()
        .expect("running cargo without the wrapper");
    assert!(status.success());
    assert_eq!(
        shipped_report(&wrapper, &counters)["events_read"].as_u64().unwrap(),
        before,
        "an unmeasured build must leave the counter untouched"
    );
}

#[test]
fn cargo_target_information_is_not_compilation_but_emitted_artifacts_are() {
    let query = strings(&["-", "--crate-name", "___", "--print=file-names", "--print=cfg"]);
    assert!(is_probe(&query));
    let mut compilation = query;
    compilation.push("--emit=link".into());
    assert!(!is_probe(&compilation));
    assert!(!is_probe(&strings(&["-", "--crate-name", "build_script_probe"])));
}

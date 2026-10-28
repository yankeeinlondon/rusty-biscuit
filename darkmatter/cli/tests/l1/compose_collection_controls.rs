//! `md compose` controls for `::map` and `::filter`: the two limit flags, the
//! `untyped-map-variables` / `untyped-filter-variables` recoveries, and the
//! `collection-blocks` feature, from the command line to the library options.
//!
//! The library owns the defaults and the semantics; these tests prove the
//! flags reach the options before reference validation, preflight, or
//! composition start, and that a refused value or recovery runs nothing.

use crate::collection_blocks::{Recorder, stderr_of, stdout_of, template_rule};

const MAP: &str = "::map data \"p => p\"\n- {{p}}\n::end-map\n";
const FILTER: &str = "::filter data \"p => p != 'b'\"\n- {{p}}\n::end-filter\n";

/// Frontmatter with no `$schema`: its sources are untyped.
fn untyped(body: &str) -> String {
    format!("---\ndata: [a, b, c]\n---\n{body}")
}

fn typed(body: &str) -> String {
    format!("---\n$schema:\n  data: string[](required)\ndata: [a, b, c]\n---\n{body}")
}

#[test]
fn omitted_limits_leave_the_library_defaults_in_force() {
    let recorder = Recorder::new("collection_controls_defaults");
    recorder.document(&typed(MAP));

    let output = recorder.compose(&[]);
    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output).trim_end(), "- a\n- b\n- c");

    let help = recorder.fixture.command().args(["compose", "--help"]).output().unwrap();
    let help = stdout_of(&help);
    for flag in ["--max-iteration-visits <N>", "--max-expansion-bytes <N>"] {
        let line = help.lines().find(|line| line.contains(flag)).unwrap_or_else(|| panic!("{flag} in help"));
        assert!(!line.contains("default:"), "the CLI states no default of its own: {line}");
    }
}

#[test]
fn custom_limits_that_fit_compose_the_document() {
    let recorder = Recorder::new("collection_controls_limits_fit");
    recorder.document(&typed(MAP));

    let output = recorder.compose(&["--max-iteration-visits", "3", "--max-expansion-bytes", "1000000"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert_eq!(stdout_of(&output).trim_end(), "- a\n- b\n- c");
}

#[test]
fn a_custom_visit_limit_is_applied() {
    let recorder = Recorder::new("collection_controls_visit_limit");
    recorder.document(&typed(FILTER));

    let output = recorder.compose(&["--max-iteration-visits=2"]);
    let stderr = stderr_of(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty());
    assert!(stderr.contains("dm.iteration.visit_limit") && stderr.contains("limit of 2"), "{stderr}");
}

/// Zero, negative, overflowing, and non-integer values are usage errors
/// (status 2) that run nothing; there is no unlimited spelling.
macro_rules! rejected_limit {
    ($name:ident, $flag:literal, $value:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&typed(MAP));

            let output = recorder.compose(&[$flag, $value]);
            let stderr = stderr_of(&output);

            assert_eq!(output.status.code(), Some(2), "{stderr}");
            assert!(output.stdout.is_empty(), "{}", stdout_of(&output));
            assert!(stderr.contains("invalid value") && stderr.contains($flag), "{stderr}");
        }
    };
}

rejected_limit!(visits_reject_zero, "--max-iteration-visits", "0");
rejected_limit!(visits_reject_negative, "--max-iteration-visits", "-5");
rejected_limit!(visits_reject_overflow, "--max-iteration-visits", "18446744073709551616");
rejected_limit!(visits_reject_unlimited, "--max-iteration-visits", "unlimited");
rejected_limit!(bytes_reject_zero, "--max-expansion-bytes", "0");
rejected_limit!(bytes_reject_negative, "--max-expansion-bytes", "-1");
rejected_limit!(bytes_reject_overflow, "--max-expansion-bytes", "99999999999999999999999");
rejected_limit!(bytes_reject_fraction, "--max-expansion-bytes", "1.5");

#[test]
fn the_largest_value_is_accepted() {
    let recorder = Recorder::new("collection_controls_max_value");
    recorder.document(&typed(MAP));

    let output = recorder.compose(&["--max-iteration-visits", "18446744073709551615"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
}

/// An untyped source fails by default, composes with its recovery as a
/// warning (its code is in the report), and is not recovered by the other directive's
/// recovery.
macro_rules! recovery {
    ($name:ident, $body:expr, $id:literal, $other:literal, $code:literal, $output:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&untyped($body));

            let denied = recorder.compose(&[]);
            assert_eq!(denied.status.code(), Some(1), "{}", stderr_of(&denied));
            assert!(denied.stdout.is_empty());
            assert!(stderr_of(&denied).contains($code), "{}", stderr_of(&denied));

            let wrong_kind = recorder.compose(&["--allow", $other]);
            assert_eq!(wrong_kind.status.code(), Some(1), "{}", stderr_of(&wrong_kind));
            assert!(wrong_kind.stdout.is_empty());

            for allow in [$id, "all"] {
                let recovered = recorder.compose(&["--allow", allow]);
                let stderr = stderr_of(&recovered);
                assert_eq!(recovered.status.code(), Some(0), "--allow {allow}: {stderr}");
                assert_eq!(stdout_of(&recovered).trim_end(), $output, "--allow {allow}");
                assert!(stderr.contains("has no declared iterable type"), "the warning is reported: {stderr}");
            }

            let report = recorder.compose(&["--allow", $id, "--output", "report-json"]);
            let report: serde_json::Value = serde_json::from_str(&stdout_of(&report)).expect("one JSON object");
            assert_eq!(report["outcome"], "success");
            let diagnostics = report["diagnostics"].as_array().expect("an array");
            assert!(
                diagnostics.iter().any(|diagnostic| diagnostic["code"] == $code && diagnostic["severity"] == "warning"),
                "the warning carries its code: {report:#}"
            );
        }
    };
}

recovery!(
    untyped_map_variables_recovers_a_map_source,
    MAP,
    "untyped-map-variables",
    "untyped-filter-variables",
    "dm.collection.untyped_map_source",
    "- a\n- b\n- c"
);
recovery!(
    untyped_filter_variables_recovers_a_filter_source,
    FILTER,
    "untyped-filter-variables",
    "untyped-map-variables",
    "dm.collection.untyped_filter_source",
    "- a\n- c"
);

/// A recovered source authored before a later static rejection keeps its
/// warning in the failure report, ahead of the rejection.
#[test]
fn a_recovered_source_warning_survives_a_later_static_rejection_in_the_report() {
    let recorder = Recorder::new("a_recovered_source_warning_survives_a_later_static_rejection_in_the_report");
    for (id, earlier, later, warning) in [
        ("untyped-map-variables", "::map unknown\n- {{i}}\n::end-map\n", "::filter scalar\n- x\n::end-filter\n", "dm.collection.untyped_map_source"),
        ("untyped-filter-variables", "::filter unknown\n- {{i}}\n::end-filter\n", "::map scalar\n- x\n::end-map\n", "dm.collection.untyped_filter_source"),
    ] {
        recorder.document(&format!("---\n$schema:\n  scalar: number(required)\nunknown: [1]\nscalar: 2\n---\n{earlier}{later}"));
        let output = recorder.compose(&["--allow", id, "--output", "report-json"]);
        assert_eq!(output.status.code(), Some(1), "--allow {id}: {}", stderr_of(&output));
        let report: serde_json::Value = serde_json::from_str(&stdout_of(&output)).expect("one JSON object");
        assert_eq!(report["outcome"], "failure", "{report:#}");
        let rows: Vec<(&str, &str)> = report["diagnostics"]
            .as_array()
            .expect("an array")
            .iter()
            .map(|diagnostic| (diagnostic["code"].as_str().unwrap(), diagnostic["severity"].as_str().unwrap()))
            .collect();
        assert_eq!(rows, [(warning, "warning"), ("dm.collection.invalid_source", "error")], "--allow {id}: {report:#}");
    }
}

/// `--deny` of the recovery, by id or by code, beats `--allow all`; the
/// opposite spellings of one selection conflict; `--deny all` spares a
/// recovery named by `--allow`.
macro_rules! recovery_policy {
    ($name:ident, $body:expr, $id:literal, $code:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&untyped($body));

            for deny in [$id, $code] {
                let output = recorder.compose(&["--allow", "all", "--deny", deny]);
                assert_eq!(output.status.code(), Some(1), "--deny {deny}: {}", stderr_of(&output));
                assert!(output.stdout.is_empty(), "--deny {deny}");
            }
            for deny in [$id, $code] {
                let output = recorder.compose(&["--allow", $id, "--deny", deny]);
                assert_eq!(output.status.code(), Some(2), "--deny {deny}: {}", stderr_of(&output));
                assert!(output.stdout.is_empty());
            }
            let output = recorder.compose(&["--allow", $id, "--deny", "all"]);
            assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
        }
    };
}

recovery_policy!(
    map_recovery_precedence_and_conflicts,
    MAP,
    "untyped-map-variables",
    "dm.collection.untyped_map_source"
);
recovery_policy!(
    filter_recovery_precedence_and_conflicts,
    FILTER,
    "untyped-filter-variables",
    "dm.collection.untyped_filter_source"
);

/// A recovery that is not granted fails before preflight finishes: a shell
/// command authored ahead of the untyped source never runs. With the
/// recovery the same document runs it, so the option reached preflight.
#[test]
fn a_blocked_recovery_fails_before_any_command_runs() {
    let recorder = Recorder::new("collection_controls_blocked_before_effects");
    recorder.document(&untyped(&format!("::shell argv-recorder first\n\n{MAP}")));
    recorder.whitelist("prefix argv-recorder\n");

    let blocked = recorder.compose(&[]);
    assert_eq!(blocked.status.code(), Some(1), "{}", stderr_of(&blocked));
    assert!(blocked.stdout.is_empty());
    assert!(recorder.runs().is_empty(), "no effect before the refusal: {:?}", recorder.runs());

    let denied = recorder.compose(&["--allow", "all", "--deny", "untyped-map-variables"]);
    assert_eq!(denied.status.code(), Some(1), "{}", stderr_of(&denied));
    assert!(recorder.runs().is_empty());

    let recovered = recorder.compose(&["--allow", "untyped-map-variables"]);
    assert_eq!(recovered.status.code(), Some(0), "{}", stderr_of(&recovered));
    assert_eq!(recorder.runs(), vec![vec!["first".to_string()]]);
}

/// A recovered untyped source still passes through template approval.
#[test]
fn a_recovered_source_still_needs_shell_approval() {
    let recorder = Recorder::new("collection_controls_recovery_needs_approval");
    recorder.document(&untyped("::map data \"p => p\"\n::shell argv-recorder {{ p }}\n::end-map\n"));

    let unapproved = recorder.compose(&["--allow", "untyped-map-variables"]);
    assert_ne!(unapproved.status.code(), Some(0));
    assert!(stderr_of(&unapproved).contains("Approval required"), "{}", stderr_of(&unapproved));
    assert!(recorder.runs().is_empty());

    recorder.whitelist(&template_rule("argv-recorder {{ p }}"));
    let approved = recorder.compose(&["--allow", "untyped-map-variables"]);
    assert_eq!(approved.status.code(), Some(0), "{}", stderr_of(&approved));
    assert_eq!(recorder.runs().len(), 3);
}

#[test]
fn disabling_collection_blocks_leaves_the_directives_unexpanded() {
    let recorder = Recorder::new("collection_controls_disabled");
    recorder.document(&typed(MAP));

    let output = recorder.compose(&["--disable", "collection-blocks"]);
    let stdout = stdout_of(&output);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
    assert!(stdout.contains("::map data") && stdout.contains("::end-map"), "{stdout}");
    assert!(!stdout.contains("- a\n- b"), "{stdout}");

    let output = recorder.compose(&[]);
    assert!(!stdout_of(&output).contains("::map"), "enabled by default");
}

/// Disabling the stage also keeps an untyped source from being analyzed.
#[test]
fn a_disabled_stage_does_not_fail_on_an_untyped_source() {
    let recorder = Recorder::new("collection_controls_disabled_untyped");
    recorder.document(&untyped(MAP));

    let output = recorder.compose(&["--disable", "collection-blocks"]);

    assert_eq!(output.status.code(), Some(0), "{}", stderr_of(&output));
}

macro_rules! wrong_verb {
    ($name:ident, $verb:literal, $id:literal) => {
        #[test]
        fn $name() {
            let recorder = Recorder::new(stringify!($name));
            recorder.document(&typed(MAP));

            let output = recorder.compose(&[$verb, $id]);

            assert_eq!(output.status.code(), Some(2), "{}", stderr_of(&output));
            assert!(output.stdout.is_empty());
        }
    };
}

wrong_verb!(collection_blocks_cannot_be_allowed, "--allow", "collection-blocks");
wrong_verb!(collection_blocks_cannot_be_denied, "--deny", "collection-blocks");
wrong_verb!(a_recovery_cannot_be_disabled, "--disable", "untyped-map-variables");

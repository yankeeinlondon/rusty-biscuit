use super::*;

/// The check's own codes are declared in Claudine's catalog, never beside it.
#[test]
fn check_codes_are_cataloged_unique_and_disjoint_from_darkmatter_findings() {
    const CHECK_CODES: &[&str] = &[
        SELECTION_KEY_AS_DATA,
        NO_EXECUTABLE,
        MULTIPLE_EXECUTABLES,
        SEQUENCE_SHAPE,
        PROVIDER_UNRESOLVED,
        PROVIDER_UNKNOWN,
        PROVIDER_NOT_INSTALLED,
        MODEL_REQUIRED,
        INVALID_ARGUMENTS,
        INTERNAL_BUG,
    ];
    for code in CHECK_CODES {
        assert!(claudine::diagnostics::code_spec(code).is_some(), "`{code}` is not in Claudine's catalog");
    }
    let mut codes = CHECK_CODES.to_vec();
    codes.sort_unstable();
    codes.dedup();
    assert_eq!(codes.len(), CHECK_CODES.len());
    assert!(CHECK_CODES.iter().all(|code| darkmatter::findings::FINDING_CODES.iter().all(|(held, _)| held != code)));
}

#[test]
fn the_summary_line_counts_findings_checks_and_previews() {
    let mut sink = FindingSink::new();
    let subject = sink.register_subject(Subject::document("doc.md", None));
    sink.push(subject, Diagnostic::actionable("x.error", Severity::Error, "bad", "fix it"));
    sink.not_checked(subject, "provider", "no executable", None);
    let outcome = Outcome { file: None, report: sink.finish(), previews: vec!["text".into()], tasks: 2 };
    assert_eq!(
        outcome.summary_line(),
        "Dry run found 1 error and 0 warnings; 1 check not run. Previewed 1 of 2 tasks."
    );
    assert_eq!(outcome.envelope()["outcome"], "failure");
    assert_eq!(outcome.envelope()["file"], Value::Null);
}

/// Rule 10's cohort rule: a missing input warns only when the run could ask
/// for it and every missing shape can be asked for.
#[test]
fn a_missing_input_warns_only_when_the_whole_cohort_can_be_collected() {
    use super::tasks::missing_input_severity;
    assert_eq!(missing_input_severity(true, true), Severity::Warning);
    assert_eq!(missing_input_severity(true, false), Severity::Error);
    assert_eq!(missing_input_severity(false, true), Severity::Error);
    assert_eq!(missing_input_severity(false, false), Severity::Error);
}

#[test]
fn property_segments_follow_keys_and_indexes() {
    use super::tasks::property_segments;
    use biscuit_file::yaml::YamlPathSegment::{Index, Key};
    assert_eq!(
        property_segments("sequence[0].group.tasks[12]"),
        [Key("sequence".into()), Index(0), Key("group".into()), Key("tasks".into()), Index(12)]
    );
    assert_eq!(property_segments("groups[1].tasks[0]"), [Key("groups".into()), Index(1), Key("tasks".into()), Index(0)]);
    assert_eq!(property_segments("[3]"), [Index(3)], "a bare list's item has no key");
    assert_eq!(property_segments("tasks[0][1]"), [Key("tasks".into()), Index(0), Index(1)]);
    assert!(property_segments("").is_empty());
}

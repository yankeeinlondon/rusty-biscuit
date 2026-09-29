//! Evaluation through the public API: evidence values, time boundaries,
//! declaration validation, defaults, the reader, and the lifecycle example.

use chrono::{DateTime, NaiveDate, Utc};
use content_policy::reader::{MalformedCause, ReadError};
use content_policy::{
    Action, DateSource, DiagnosticCode, DocumentError, EntryOutcome, EvaluationContext,
    EvidenceRecord, Invalid, Location, Policy, PolicyOptions, PolicySource, Report, ResultKind,
    Status, UnknownReason, WarningCode, evaluate_document, evaluate_policy, evaluate_record,
};
use serde_json::{Value, json};

fn at(text: &str) -> DateTime<Utc> {
    if text.len() == 10 {
        let date = NaiveDate::parse_from_str(text, "%Y-%m-%d").unwrap();
        return date.and_hms_opt(0, 0, 0).unwrap().and_utc();
    }
    DateTime::parse_from_rfc3339(text).unwrap().to_utc()
}

fn context(when: &str) -> EvaluationContext {
    EvaluationContext::new(at(when))
}

fn record(value: Value) -> EvidenceRecord {
    match value {
        Value::Object(map) => EvidenceRecord::from(map),
        other => panic!("not an object: {other}"),
    }
}

fn document(frontmatter: &str) -> Vec<u8> {
    format!("---\n{frontmatter}---\n\n# Body\n").into_bytes()
}

fn report(bytes: &[u8], when: &str) -> Report {
    evaluate_document(bytes, &context(when)).unwrap_or_else(|error| panic!("{error}"))
}

fn invalid(bytes: &[u8], when: &str) -> Invalid {
    match evaluate_document(bytes, &context(when)) {
        Err(DocumentError::Invalid(invalid)) => invalid,
        other => panic!("expected invalid, got {other:?}"),
    }
}

fn outcome(report: &Report, index: usize) -> EntryOutcome {
    report.results[index].outcome
}

// --- Evidence values (AC 2) -------------------------------------------------

/// One row per line of the spec's Evidence values table, in each date
/// position: a referenced baseline, the shorthand's default property, and a
/// referenced `ValidUntil` deadline.
#[test]
fn evidence_values_table_in_every_date_position() {
    let positions = [
        ("ValidFor(3mo, @reviewed)", "reviewed"),
        ("ValidFor(3mo)", "last_updated"),
        ("ValidUntil(@reviewed)", "reviewed"),
    ];
    for (rule, property) in positions {
        let evaluate = |value: Option<Value>| {
            let mut evidence = record(json!({ "content_policy": [rule] }));
            if let Some(value) = value {
                evidence.insert(property, value);
            }
            evaluate_record(&evidence, &context("2026-10-01"))
        };
        let missing = |result: Result<Report, Invalid>| {
            let report = result.unwrap();
            assert_eq!(report.status, Status::Unknown, "{rule}");
            assert_eq!(outcome(&report, 0), EntryOutcome::Unknown(UnknownReason::MissingBaseline));
        };
        missing(evaluate(None));
        missing(evaluate(Some(Value::Null)));

        let date = evaluate(Some(json!("2026-09-28"))).unwrap();
        assert_ne!(date.status, Status::Unknown, "{rule}: a date resolves");
        let evidence = date.results[0].baseline.as_ref().or(date.results[0].deadline.as_ref()).unwrap();
        assert_eq!(evidence.property.as_deref(), Some(property));
        assert_eq!(evidence.value.map(|value| value.0.to_string()).as_deref(), Some("2026-09-28"));

        for (value, code) in [
            (json!("2026-02-30"), DiagnosticCode::InvalidDate),
            (json!("Sept 28"), DiagnosticCode::InvalidDate),
            (json!(""), DiagnosticCode::InvalidDate),
            (json!("2026-09-28T10:00:00Z"), DiagnosticCode::DatesOnly),
            (json!(20260928), DiagnosticCode::WrongType),
            (json!(true), DiagnosticCode::WrongType),
            (json!(["2026-09-28"]), DiagnosticCode::WrongType),
            (json!({"date": "2026-09-28"}), DiagnosticCode::WrongType),
        ] {
            let invalid = evaluate(Some(value.clone())).unwrap_err();
            assert_eq!(invalid.diagnostics.len(), 1, "{rule} {value}");
            assert_eq!(invalid.diagnostics[0].code, code, "{rule} {value}");
            assert_eq!(
                invalid.diagnostics[0].location,
                Location::Property { name: property.to_string(), entry: Some(0) },
                "{rule} {value}"
            );
        }
    }
}

/// How YAML 1.1-looking spellings arrive through the reader, and what they
/// mean in a date position.
#[test]
fn yaml_1_1_spellings_in_a_date_position() {
    for (authored, expected) in [
        ("yes", Err(DiagnosticCode::InvalidDate)),
        ("on", Err(DiagnosticCode::InvalidDate)),
        ("y", Err(DiagnosticCode::InvalidDate)),
        ("True", Err(DiagnosticCode::WrongType)),
        ("010", Err(DiagnosticCode::InvalidDate)),
        (".inf", Ok(())),
        (".nan", Ok(())),
        ("~", Ok(())),
        ("null", Ok(())),
    ] {
        let bytes = document(&format!("last_updated: {authored}\ncontent_policy:\n  - ValidFor(3mo)\n"));
        match expected {
            Ok(()) => {
                let report = report(&bytes, "2026-10-01");
                assert_eq!(
                    outcome(&report, 0),
                    EntryOutcome::Unknown(UnknownReason::MissingBaseline),
                    "{authored}"
                );
            }
            Err(code) => assert!(invalid(&bytes, "2026-10-01").has(code), "{authored}"),
        }
    }
}

#[test]
fn quoted_and_unquoted_dates_are_equivalent() {
    let plain = report(&document("last_updated: 2026-09-28\ncontent_policy:\n  - ValidFor(3mo)\n"), "2026-12-28");
    for quoted in ["\"2026-09-28\"", "'2026-09-28'"] {
        let other = report(
            &document(&format!("last_updated: {quoted}\ncontent_policy:\n  - ValidFor(3mo)\n")),
            "2026-12-28",
        );
        assert_eq!(other, plain, "{quoted}");
    }
    assert_eq!(plain.status, Status::Stale);
}

// --- Inline and referenced baselines (AC 1, AC 2) ---------------------------

#[test]
fn inline_referenced_and_defaulted_baselines_evaluate_alike() {
    let evidence = |rule: &str| {
        record(json!({ "last_updated": "2026-09-28", "content_policy": [rule] }))
    };
    let rules = [
        ("ValidFor(3mo, 2026-09-28)", DateSource::Inline, None),
        ("ValidFor(3mo, @last_updated)", DateSource::Property, Some("last_updated")),
        ("ValidFor(3mo)", DateSource::DefaultProperty, Some("last_updated")),
    ];
    for when in ["2026-12-27", "2026-12-28", "2027-06-01"] {
        let reports: Vec<Report> = rules
            .iter()
            .map(|(rule, _, _)| evaluate_record(&evidence(rule), &context(when)).unwrap())
            .collect();
        for (report, (rule, source, property)) in reports.iter().zip(&rules) {
            assert_eq!(report.status, reports[0].status, "{rule} at {when}");
            let result = &report.results[0];
            assert_eq!(result.outcome, reports[0].results[0].outcome, "{rule} at {when}");
            assert_eq!(result.due.map(|due| due.0.to_string()).as_deref(), Some("2026-12-28"));
            let baseline = result.baseline.as_ref().unwrap();
            assert_eq!(baseline.source, *source, "{rule}");
            assert_eq!(baseline.property.as_deref(), *property, "{rule}");
            assert_eq!(baseline.value.unwrap().0.to_string(), "2026-09-28");
        }
    }
}

#[test]
fn compact_and_long_forms_normalize_consistently() {
    let compact = record(json!({ "last_updated": "2026-09-28", "content_policy": ["ValidFor(3mo)"] }));
    let long = record(json!({
        "last_updated": "2026-09-28",
        "content_policy": [{ "rule": "ValidFor(3mo)", "action": "refresh" }],
    }));
    let compact = evaluate_record(&compact, &context("2026-12-28")).unwrap();
    let long = evaluate_record(&long, &context("2026-12-28")).unwrap();
    assert_eq!(compact, long);
    assert_eq!(compact.policy.source, PolicySource::Declared);
    assert_eq!(compact.results[0].rule, "ValidFor(3mo)");
    assert_eq!(compact.results[0].action, Action::Refresh);
}

// --- Time semantics (AC 3) --------------------------------------------------

fn status_of(rule: &str, evidence: Value, when: &str) -> Report {
    let mut evidence = record(evidence);
    evidence.insert("content_policy", json!([rule]));
    evaluate_record(&evidence, &context(when)).unwrap()
}

#[test]
fn valid_until_takes_effect_at_utc_midnight() {
    let before = status_of("ValidUntil(2027-01-01)", json!({}), "2026-12-31T23:59:59Z");
    assert_eq!(outcome(&before, 0), EntryOutcome::NotTriggered);
    let on = status_of("ValidUntil(2027-01-01)", json!({}), "2027-01-01T00:00:00Z");
    assert_eq!(outcome(&on, 0), EntryOutcome::Triggered);
    // An evaluation instant written with an offset is the same UTC instant.
    let tokyo = status_of("ValidUntil(2027-01-01)", json!({}), "2027-01-01T08:59:59+09:00");
    assert_eq!(outcome(&tokyo, 0), EntryOutcome::NotTriggered);
    // A future deadline is normal, not unknown.
    let early = status_of("ValidUntil(2027-01-01)", json!({}), "2020-01-01");
    assert_eq!((early.status, early.evaluation_complete), (Status::Fresh, true));
}

#[test]
fn valid_for_is_due_on_its_computed_date() {
    let evidence = json!({ "last_updated": "2026-09-28" });
    let day_before = status_of("ValidFor(3mo)", evidence.clone(), "2026-12-27T23:59:59Z");
    assert_eq!(outcome(&day_before, 0), EntryOutcome::NotTriggered);
    let due = status_of("ValidFor(3mo)", evidence, "2026-12-28T00:00:00Z");
    assert_eq!(outcome(&due, 0), EntryOutcome::Triggered);
}

#[test]
fn month_ends_and_leap_years_clamp() {
    for (rule, baseline, due) in [
        ("ValidFor(1mo)", "2026-01-31", "2026-02-28"),
        ("ValidFor(1mo)", "2024-01-31", "2024-02-29"),
        ("ValidFor(1yr)", "2024-02-29", "2025-02-28"),
        ("ValidFor(10d)", "2026-12-25", "2027-01-04"),
        ("ValidFor(2wk)", "2026-02-20", "2026-03-06"),
    ] {
        let report = status_of(rule, json!({ "last_updated": baseline }), baseline);
        assert_eq!(report.results[0].due.unwrap().0.to_string(), due, "{rule} from {baseline}");
        let day_before = NaiveDate::parse_from_str(due, "%Y-%m-%d").unwrap().pred_opt().unwrap();
        let before = status_of(rule, json!({ "last_updated": baseline }), &day_before.to_string());
        assert_eq!(outcome(&before, 0), EntryOutcome::NotTriggered, "{rule} from {baseline}");
        let on = status_of(rule, json!({ "last_updated": baseline }), due);
        assert_eq!(outcome(&on, 0), EntryOutcome::Triggered, "{rule} from {baseline}");
    }
}

#[test]
fn a_future_baseline_is_unknown_not_fresh() {
    let report = status_of("ValidFor(3mo)", json!({ "last_updated": "2026-09-29" }), "2026-09-28T23:59:59Z");
    assert_eq!(report.status, Status::Unknown);
    assert_eq!(outcome(&report, 0), EntryOutcome::Unknown(UnknownReason::InconsistentBaseline));
    // Same-day baseline is consistent.
    let same_day = status_of("ValidFor(3mo)", json!({ "last_updated": "2026-09-29" }), "2026-09-29T00:00:00Z");
    assert_eq!(same_day.status, Status::Fresh);
}

#[test]
fn a_referenced_baseline_out_of_calendar_range_is_a_validation_error() {
    let mut evidence = record(json!({ "last_updated": "2026-01-01" }));
    evidence.insert("content_policy", json!(["ValidFor(4000000000yr)"]));
    let invalid = evaluate_record(&evidence, &context("2026-01-02")).unwrap_err();
    assert!(invalid.has(DiagnosticCode::DateOutOfRange), "{invalid}");
    // Inline, the same overflow is caught while parsing the declaration.
    let inline = Policy::from_declaration(&json!(["ValidFor(4000000000yr, 2026-01-01)"])).unwrap_err();
    assert!(inline.has(DiagnosticCode::DateOutOfRange));
}

// --- Declarations, defaults, and fail-closed cases (AC 1, 10, 16, 17, 26) ---

#[test]
fn every_invalid_entry_is_listed_and_there_is_no_verdict() {
    let bytes = document(
        "content_policy:\n  - Duration(3mo)\n  - ValidFor(3mo)\n  - rule: ValidUntil(2027-01-01)\n  - ValidFor(3mo, @review.last_checked)\n",
    );
    let invalid = invalid(&bytes, "2026-10-01");
    let found: Vec<(DiagnosticCode, Location)> = invalid
        .diagnostics
        .iter()
        .map(|diagnostic| (diagnostic.code, diagnostic.location.clone()))
        .collect();
    assert_eq!(
        found,
        [
            (DiagnosticCode::UnknownRule, Location::Entry { index: 0 }),
            (DiagnosticCode::MissingAction, Location::Entry { index: 2 }),
            (DiagnosticCode::NestedReference, Location::Entry { index: 3 }),
        ]
    );
}

#[test]
fn fail_closed_declarations_never_yield_fresh() {
    let cases: [(&str, DiagnosticCode, &str); 4] = [
        ("content_policy: []\n", DiagnosticCode::EmptyPolicy, "Evergreen"),
        ("content_policy:\n  - Evergreen\n  - ValidFor(3mo)\n", DiagnosticCode::EvergreenCombined, "only entry"),
        ("content_policy: ValidFor(3mo)\n", DiagnosticCode::NotAList, "- ValidFor(3mo)"),
        ("content_policy:\n", DiagnosticCode::NullPolicy, "Evergreen"),
    ];
    for (frontmatter, code, hint) in cases {
        let bytes = document(&format!("last_updated: 2026-09-28\n{frontmatter}"));
        let invalid = invalid(&bytes, "2026-09-29");
        assert!(invalid.has(code), "{frontmatter}: {invalid}");
        assert!(
            invalid.diagnostics.iter().any(|diagnostic| diagnostic.message.contains(hint)),
            "{frontmatter}: {invalid}"
        );
    }
    let newer = Policy::from_json(r#"{"grammar_version":2,"entries":[{"rule":"Evergreen","action":"refresh"}]}"#);
    assert!(newer.unwrap_err().has(DiagnosticCode::UnsupportedGrammarVersion));
}

#[test]
fn an_absent_policy_uses_the_callers_default() {
    let bytes = document("last_updated: 2026-09-28\ntitle: notes\n");
    let builtin = report(&bytes, "2026-09-29");
    assert_eq!(builtin.policy.source, PolicySource::Defaulted);
    assert_eq!(builtin.results[0].rule, "ValidFor(6mo)");
    assert_eq!(builtin.status, Status::Fresh);
    assert_eq!(report(&bytes, "2027-03-28").status, Status::Stale);

    let fail_closed = PolicyOptions::default().with_default_policy(Policy::from_text("TimeSensitive").unwrap());
    let context = context("2026-09-29").with_options(fail_closed);
    let stale = evaluate_document(&bytes, &context).unwrap();
    assert_eq!((stale.status, stale.action), (Status::Stale, Some(Action::Refresh)));
    assert_eq!(stale.policy.source, PolicySource::Defaulted);

    // No frontmatter at all is evaluated the same way.
    let bare = evaluate_document(b"# Just a body\n", &context).unwrap();
    assert_eq!((bare.status, bare.policy.source), (Status::Stale, PolicySource::Defaulted));
}

#[test]
fn a_declared_policy_matching_the_default_shares_its_identity() {
    let declared = report(&document("last_updated: 2026-09-28\ncontent_policy:\n  - ValidFor(6mo)\n"), "2026-09-29");
    let defaulted = report(&document("last_updated: 2026-09-28\n"), "2026-09-29");
    assert_eq!(declared.policy.identity, defaulted.policy.identity);
    assert_ne!(declared.policy.source, defaulted.policy.source);
}

#[test]
fn custom_key_and_date_property() {
    let bytes = document("reviewed: 2026-09-28\nfreshness:\n  - ValidFor(1mo)\ncontent_policy:\n  - TimeSensitive\n");
    let options = PolicyOptions::default().with_key("freshness").with_date_property("reviewed");
    let report = evaluate_document(&bytes, &context("2026-10-27").with_options(options)).unwrap();
    assert_eq!(report.status, Status::Fresh);
    assert_eq!(report.results[0].baseline.as_ref().unwrap().property.as_deref(), Some("reviewed"));
    // No kebab-case alias of the default key is read.
    let kebab = document("last_updated: 2026-09-28\ncontent-policy:\n  - TimeSensitive\n");
    assert_eq!(report_policy_source(&kebab), PolicySource::Defaulted);
}

fn report_policy_source(bytes: &[u8]) -> PolicySource {
    report(bytes, "2026-09-29").policy.source
}

#[test]
fn legacy_duration_and_update_policy() {
    let duration = document("last_updated: 2026-09-28\ncontent_policy:\n  - Duration(3mo)\n");
    let invalid = invalid(&duration, "2026-09-29");
    assert_eq!(invalid.diagnostics[0].code, DiagnosticCode::UnknownRule);
    assert!(invalid.diagnostics[0].message.contains("ValidFor"));

    let legacy = document("last_updated: 2026-09-28\nupdate_policy:\n  - Duration(6mo)\n");
    let report = report(&legacy, "2026-09-29");
    assert_eq!(report.policy.source, PolicySource::Defaulted);
    assert_eq!(report.results[0].rule, "ValidFor(6mo)");
}

#[test]
fn rule_and_action_names_are_case_sensitive() {
    let rule = invalid(&document("content_policy:\n  - validfor(3mo)\n"), "2026-09-29");
    assert!(rule.has(DiagnosticCode::UnknownRule));
    let action = invalid(
        &document("content_policy:\n  - rule: ValidFor(3mo)\n    action: Archive\n"),
        "2026-09-29",
    );
    assert!(action.has(DiagnosticCode::UnknownAction));
}

#[test]
fn a_dotted_reference_is_rejected_even_when_the_literal_key_exists() {
    let evidence = record(json!({
        "review.last_checked": "2026-09-28",
        "content_policy": ["ValidFor(3mo, @review.last_checked)"],
    }));
    let invalid = evaluate_record(&evidence, &context("2026-09-29")).unwrap_err();
    assert_eq!(invalid.diagnostics.len(), 1);
    assert_eq!(invalid.diagnostics[0].code, DiagnosticCode::NestedReference);
    assert_eq!(invalid.diagnostics[0].location, Location::Entry { index: 0 });
}

#[test]
fn the_flow_list_comma_trap() {
    let split = invalid(&document("content_policy: [ValidFor(3mo, 2026-09-28)]\n"), "2026-09-29");
    assert_eq!(split.diagnostics.len(), 2, "{split}");
    for diagnostic in &split.diagnostics {
        assert_eq!(diagnostic.code, DiagnosticCode::UnbalancedParentheses);
        assert!(diagnostic.message.contains("comma"), "{}", diagnostic.message);
        assert!(diagnostic.message.contains("block list"), "{}", diagnostic.message);
        assert!(diagnostic.message.contains("\n  - ValidFor(3mo, 2026-09-28)"), "{}", diagnostic.message);
    }

    let unquoted_reference = document("last_updated: 2026-09-28\ncontent_policy: [ValidFor(3mo, @last_updated)]\n");
    assert!(matches!(
        evaluate_document(&unquoted_reference, &context("2026-09-29")),
        Err(DocumentError::Read { error: ReadError::Malformed { cause: MalformedCause::Yaml, .. }, .. })
    ));

    let quoted = document("last_updated: 2026-09-28\ncontent_policy: [\"ValidFor(3mo, @last_updated)\"]\n");
    assert_eq!(report(&quoted, "2026-12-28").status, Status::Stale);
    let quoted_inline = document("content_policy: [\"ValidFor(3mo, 2026-09-28)\"]\n");
    assert_eq!(report(&quoted_inline, "2026-12-27").status, Status::Fresh);
}

// --- Reader through evaluation (AC 18, 27, 28) ------------------------------

#[test]
fn duplicate_top_level_keys_are_validation_errors() {
    for (frontmatter, key) in [
        ("content_policy:\n  - ValidFor(3mo)\ncontent_policy:\n  - Evergreen\n", "content_policy"),
        ("last_updated: 2026-09-28\nlast_updated: 2026-09-29\ncontent_policy:\n  - ValidFor(3mo)\n", "last_updated"),
    ] {
        let invalid = invalid(&document(frontmatter), "2026-09-29");
        assert_eq!(invalid.diagnostics[0].code, DiagnosticCode::DuplicateKey);
        assert_eq!(
            invalid.diagnostics[0].location,
            Location::Property { name: key.to_string(), entry: None }
        );
    }
}

#[test]
fn tab_indented_frontmatter_is_evaluated_with_a_warning() {
    let bytes = b"---\nprompt: |-\n\tLine one\n\t\tLine two\nlast_updated: 2026-09-28\ncontent_policy:\n\t- ValidFor(3mo)\n---\nbody\n";
    let report = report(bytes, "2026-12-28");
    assert_eq!(report.status, Status::Stale);
    assert_eq!(report.warnings.len(), 1);
    assert_eq!(report.warnings[0].code, WarningCode::TabIndentationRepaired);
    assert!(report.warnings[0].message.contains("tab"));
    assert!(report.to_json().contains("\"tab_indentation_repaired\""));
}

#[test]
fn unquoted_templates_are_rejected_with_the_reason() {
    let bytes = document("prompt: {{ topic }} notes\nlast_updated: 2026-09-28\n");
    let error = evaluate_document(&bytes, &context("2026-09-29").with_document("prompts/a.md")).unwrap_err();
    let DocumentError::Read { document, error } = &error else { panic!("{error:?}") };
    assert_eq!(document.as_deref(), Some("prompts/a.md"));
    assert!(matches!(error, ReadError::Malformed { cause: MalformedCause::ExpressionTemplate, .. }));
    assert!(error.to_string().contains("expression protection"), "{error}");
}

#[test]
fn a_clip_chomped_block_scalar_as_the_last_key_reads_like_darkmatter() {
    let bytes = document("last_updated: 2026-09-28\ncontent_policy:\n  - ValidFor(3mo, @reviewed)\nreviewed: >\n  2026-09-28\n");
    // With the final terminator kept, the folded scalar would be
    // "2026-09-28\n" and read as an invalid date.
    assert_eq!(report(&bytes, "2026-12-28").status, Status::Stale);
}

// --- Evaluation writes nothing (AC 5) and needs no Markdown (AC 11) --------

#[test]
fn evaluation_never_captures_a_baseline() {
    let evidence = record(json!({ "content_policy": ["ValidFor(3mo)"] }));
    let before = evidence.clone();
    for _ in 0..2 {
        let report = evaluate_record(&evidence, &context("2026-09-29")).unwrap();
        assert_eq!(report.status, Status::Unknown);
        assert_eq!(outcome(&report, 0), EntryOutcome::Unknown(UnknownReason::MissingBaseline));
    }
    assert_eq!(evidence, before);

    let bytes = document("content_policy:\n  - ValidFor(3mo)\n");
    let copy = bytes.clone();
    for _ in 0..2 {
        assert_eq!(report(&bytes, "2026-09-29").status, Status::Unknown);
    }
    assert_eq!(bytes, copy);
}

#[test]
fn a_plain_evidence_map_needs_no_document() {
    // A cache manifest: no frontmatter, no Markdown, its own date property.
    let manifest = record(json!({ "generated_on": "2026-09-01", "model": "x" }));
    let policy = Policy::from_json(
        r#"{"grammar_version":1,"entries":[{"rule":"ValidFor(30d, @generated_on)","action":"refresh"}]}"#,
    )
    .unwrap();
    let context = context("2026-10-01").with_document("cache/manifest.json");
    let report = evaluate_policy(&policy, &manifest, &context).unwrap();
    assert_eq!(report.status, Status::Stale);
    assert_eq!(report.document.as_deref(), Some("cache/manifest.json"));
    assert_eq!(report.policy.source, PolicySource::Declared);
}

// --- Report shape -----------------------------------------------------------

#[test]
fn every_entry_is_evaluated_without_short_circuit() {
    let evidence = record(json!({
        "content_policy": [
            "TimeSensitive",
            { "rule": "ValidUntil(2027-01-01)", "action": "remove" },
            { "rule": "ValidFor(1yr, @missing)", "action": "archive" },
        ],
    }));
    let report = evaluate_record(&evidence, &context("2027-01-01")).unwrap();
    let kinds: Vec<ResultKind> = report.results.iter().map(|result| result.outcome.kind()).collect();
    assert_eq!(kinds, [ResultKind::Triggered, ResultKind::Triggered, ResultKind::Unknown]);
    assert_eq!((report.status, report.action), (Status::Expired, Some(Action::Remove)));
    assert!(!report.evaluation_complete);
    assert!(report.action_resolution_complete);
}

/// The JSON field names are public contract once the CLI prints them.
#[test]
fn report_json_field_names_are_frozen() {
    let bytes = document(
        "last_updated: 2026-09-28\ncontent_policy:\n  - rule: ValidFor(3mo, @last_updated)\n    action: refresh\n  - ValidUntil(2027-01-01)\n",
    );
    let context = context("2026-12-28").with_document("notes.md");
    let report = evaluate_document(&bytes, &context).unwrap();
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    let identity = report.policy.identity.clone();
    assert_eq!(
        json,
        json!({
            "document": "notes.md",
            "evaluated_at": "2026-12-28T00:00:00Z",
            "policy": { "source": "declared", "grammar_version": 1, "identity": identity },
            "status": "stale",
            "action": "refresh",
            "evaluation_complete": true,
            "action_resolution_complete": true,
            "results": [
                {
                    "index": 0,
                    "rule": "ValidFor(3mo, @last_updated)",
                    "action": "refresh",
                    "renewal": "renewable",
                    "result": "triggered",
                    "unknown_reason": null,
                    "baseline": { "source": "property", "property": "last_updated", "value": "2026-09-28" },
                    "deadline": null,
                    "due": "2026-12-28",
                    "reason": "Validity interval elapsed",
                },
                {
                    "index": 1,
                    "rule": "ValidUntil(2027-01-01)",
                    "action": "refresh",
                    "renewal": "nonrenewable",
                    "result": "not_triggered",
                    "unknown_reason": null,
                    "baseline": null,
                    "deadline": { "source": "inline", "property": null, "value": "2027-01-01" },
                    "due": "2027-01-01",
                    "reason": "Before its fixed deadline",
                },
            ],
            "warnings": [],
        })
    );
    let unknown = evaluate_record(&record(json!({ "content_policy": ["ValidFor(3mo)"] })), &context).unwrap();
    let json: Value = serde_json::from_str(&unknown.to_json()).unwrap();
    assert_eq!(json["results"][0]["result"], "unknown");
    assert_eq!(json["results"][0]["unknown_reason"], "missing_baseline");
    assert_eq!(json["results"][0]["baseline"]["source"], "default_property");
    assert_eq!(json["status"], "unknown");
    assert_eq!(json["action"], Value::Null);
}

// --- Lifecycle example (AC 7, library half; steps without renewal) ---------

const LIFECYCLE: &str = "last_updated: 2026-09-28\ncontent_policy:\n  - rule: ValidFor(3mo, @last_updated)\n    action: refresh\n  - rule: ValidUntil(2027-01-01)\n    action: archive\n";

#[test]
fn lifecycle_steps_through_the_library() {
    let original = document(LIFECYCLE);
    let step = |bytes: &[u8], when: &str| {
        let report = report(bytes, when);
        (report.status, report.action, report.results.iter().map(|r| r.outcome.kind()).collect::<Vec<_>>())
    };
    use ResultKind::{NotTriggered, Triggered, Unknown};

    // 1: fresh.
    assert_eq!(step(&original, "2026-12-27"), (Status::Fresh, None, vec![NotTriggered, NotTriggered]));
    // 2: stale, ValidFor triggered.
    assert_eq!(
        step(&original, "2026-12-28"),
        (Status::Stale, Some(Action::Refresh), vec![Triggered, NotTriggered])
    );
    // 5: after renewal to 2026-12-29 (hand-edited here), the old date is in
    // the baseline's past.
    let renewed = document(&LIFECYCLE.replace("2026-09-28", "2026-12-29"));
    assert_eq!(step(&renewed, "2026-12-29"), (Status::Fresh, None, vec![NotTriggered, NotTriggered]));
    assert_eq!(step(&renewed, "2026-12-28"), (Status::Unknown, None, vec![Unknown, NotTriggered]));
    // 6: expired by the deadline; ValidFor is not triggered.
    assert_eq!(
        step(&renewed, "2027-01-01"),
        (Status::Expired, Some(Action::Archive), vec![NotTriggered, Triggered])
    );
    // 8: the second entry's action becomes remove; both entries still reported.
    let removal = document(&LIFECYCLE.replace("2026-09-28", "2026-12-29").replace("action: archive", "action: remove"));
    let report = report(&removal, "2027-01-01");
    assert_eq!((report.status, report.action), (Status::Expired, Some(Action::Remove)));
    assert_eq!(report.results.len(), 2);
    assert_eq!(report.results[1].reason, "Fixed deadline reached");
}

// --- Migrated repository documents (AC 16) ----------------------------------

/// The 23 documents migrated from `Duration(...)` and `update_policy` all
/// declare `ValidFor` rules under `content_policy` and evaluate without
/// diagnostics. Each read is an `include_bytes!`, so an edit to one of them
/// schedules this test.
#[test]
fn migrated_documents_evaluate_without_diagnostics() {
    let documents: [(&str, &[u8]); 23] = [
        ("tm/about", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/about.md")),
        ("tm/cmux", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/cmux.md")),
        ("tm/ghostty", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/ghostty.md")),
        ("tm/tmux", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/tmux.md")),
        ("tm/wezterm", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/wezterm.md")),
        ("tm/zellij", include_bytes!("../../../biscuit-terminal/docs/research/terminal-multiplexing/zellij.md")),
        ("acp/gemini-cli", include_bytes!("../../../claudine/docs/research/acp/gemini-cli.md")),
        ("acp/json-rpc", include_bytes!("../../../claudine/docs/research/acp/json-rpc.md")),
        ("acp/kimi-code-cli", include_bytes!("../../../claudine/docs/research/acp/kimi-code-cli.md")),
        ("playa/Android", include_bytes!("../../../.claude/skills/playa/audio-programming/Android.md")),
        ("playa/crates", include_bytes!("../../../.claude/skills/playa/audio-programming/crates.md")),
        ("playa/IOS", include_bytes!("../../../.claude/skills/playa/audio-programming/IOS.md")),
        ("playa/linux", include_bytes!("../../../.claude/skills/playa/audio-programming/linux.md")),
        ("playa/macOS", include_bytes!("../../../.claude/skills/playa/audio-programming/macOS.md")),
        ("playa/typescript-libraries", include_bytes!("../../../.claude/skills/playa/audio-programming/typescript-libraries.md")),
        ("playa/windows", include_bytes!("../../../.claude/skills/playa/audio-programming/windows.md")),
        ("sniff/Android", include_bytes!("../../../sniff/docs/research/audio-programming/Android.md")),
        ("sniff/crates", include_bytes!("../../../sniff/docs/research/audio-programming/crates.md")),
        ("sniff/IOS", include_bytes!("../../../sniff/docs/research/audio-programming/IOS.md")),
        ("sniff/linux", include_bytes!("../../../sniff/docs/research/audio-programming/linux.md")),
        ("sniff/macOS", include_bytes!("../../../sniff/docs/research/audio-programming/macOS.md")),
        ("sniff/typescript-libraries", include_bytes!("../../../sniff/docs/research/audio-programming/typescript-libraries.md")),
        ("sniff/windows", include_bytes!("../../../sniff/docs/research/audio-programming/windows.md")),
    ];
    for (name, bytes) in documents {
        let text = std::str::from_utf8(bytes).unwrap();
        assert!(!text.contains("update_policy:"), "{name} still has update_policy");
        let report = evaluate_document(bytes, &context("2026-09-29").with_document(name))
            .unwrap_or_else(|error| panic!("{error}"));
        assert_eq!(report.policy.source, PolicySource::Declared, "{name}");
        assert!(!report.results.is_empty(), "{name}");
        for result in &report.results {
            assert!(result.rule.starts_with("ValidFor("), "{name}: {}", result.rule);
        }
        if name == "acp/json-rpc" {
            assert_eq!(report.results.len(), 1);
            assert_eq!(report.results[0].rule, "ValidFor(1yr)");
            assert!(text.contains("- ValidFor(1yr) # pending: MajorVersion(latest_version)"));
        }
    }
}

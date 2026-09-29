//! The Input Robustness Matrix: one walk per format, each cell a single edit
//! to a real fixture, asserted through the public result.
//!
//! The `FileChanged` fingerprint column walks the same fixture with a
//! `FileChanged` policy and a scripted file provider.

use chrono::{DateTime, NaiveDate, Utc};
use content_policy::reader::{MalformedCause, ReadError};
use content_policy::{
    DiagnosticCode, DocumentError, EntryField, EntryOutcome, EvaluationContext, EvidenceRecord,
    FingerprintScheme, Location, Policy, PolicySource, Status, UnknownReason, evaluate_document,
    evaluate_policy,
};
use serde_json::json;

mod fake;

/// A migrated `biscuit-terminal` terminal-multiplexing note after
/// `md hash --save` stamped it (body trimmed after stamping). Its policy is
/// `- ValidFor(3mo)` over `last_updated: 2026-03-18`.
const FIXTURE: &str = include_str!("fixtures/stamped-note.md");

fn at(date: &str) -> DateTime<Utc> {
    NaiveDate::parse_from_str(date, "%Y-%m-%d")
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

/// The day after the fixture's baseline: fresh under both the declared
/// `ValidFor(3mo)` and the built-in default `ValidFor(6mo)`.
const WHEN: &str = "2026-03-19";

#[derive(Debug)]
enum Expect {
    /// A report with this status and policy source.
    Report(Status, PolicySource),
    /// A report whose first entry is unknown for this reason.
    Unknown(UnknownReason),
    /// No verdict: a diagnostic with this code (and location, when given).
    Invalid(DiagnosticCode, Option<Location>),
    /// The frontmatter cannot be read.
    Malformed(MalformedCause),
}

use Expect::{Invalid, Malformed, Report, Unknown};

fn entry(index: usize) -> Option<Location> {
    Some(Location::Entry { index })
}

fn field(index: usize, field: EntryField) -> Option<Location> {
    Some(Location::EntryField { index, field })
}

fn property(name: &str) -> Option<Location> {
    Some(Location::Property { name: name.to_string(), entry: Some(0) })
}

const POLICY: &str = "content_policy:\n  - ValidFor(3mo)\n";
const ENTRY: &str = "  - ValidFor(3mo)\n";
const DATE: &str = "last_updated: 2026-03-18\n";

fn long_form(rule: &str, action: &str) -> String {
    format!("  - {rule}\n    {action}\n")
}

fn check(name: &str, bytes: &[u8], expect: &Expect) {
    check_in(&EvaluationContext::new(at(WHEN)), name, bytes, expect);
}

fn check_in(context: &EvaluationContext, name: &str, bytes: &[u8], expect: &Expect) {
    let result = evaluate_document(bytes, context);
    match (expect, &result) {
        (Report(status, source), Ok(report)) => {
            assert_eq!((report.status, report.policy.source), (*status, *source), "{name}");
        }
        (Unknown(reason), Ok(report)) => {
            assert_eq!(report.status, Status::Unknown, "{name}");
            assert_eq!(report.results[0].outcome, EntryOutcome::Unknown(*reason), "{name}");
        }
        (Invalid(code, location), Err(DocumentError::Invalid(invalid))) => {
            let diagnostic = invalid
                .diagnostics
                .iter()
                .find(|diagnostic| diagnostic.code == *code)
                .unwrap_or_else(|| panic!("{name}: expected {code:?}, got {invalid}"));
            if let Some(location) = location {
                assert_eq!(&diagnostic.location, location, "{name}");
            }
        }
        (Malformed(cause), Err(DocumentError::Read { error: ReadError::Malformed { cause: found, .. }, .. })) => {
            assert_eq!(found, cause, "{name}");
        }
        _ => panic!("{name}: expected {expect:?}, got {result:?}"),
    }
}

#[test]
fn yaml_frontmatter_matrix() {
    // Control row: the unedited fixture gives a valid, fresh report, so each
    // edit below is what changes the result.
    check("control", FIXTURE.as_bytes(), &Report(Status::Fresh, PolicySource::Declared));
    assert!(FIXTURE.contains(POLICY) && FIXTURE.contains(DATE) && FIXTURE.contains("\nhash: "));

    let cells: Vec<(&str, &str, String, Expect)> = vec![
        // --- Policy value (`content_policy`) ---
        ("policy absent", POLICY, String::new(), Report(Status::Fresh, PolicySource::Defaulted)),
        ("policy null (no value)", POLICY, "content_policy:\n".into(), Invalid(DiagnosticCode::NullPolicy, Some(Location::Policy))),
        ("policy null (~)", POLICY, "content_policy: ~\n".into(), Invalid(DiagnosticCode::NullPolicy, Some(Location::Policy))),
        ("policy string", POLICY, "content_policy: ValidFor(3mo)\n".into(), Invalid(DiagnosticCode::NotAList, Some(Location::Policy))),
        ("policy mapping", POLICY, "content_policy: {rule: ValidFor(3mo)}\n".into(), Invalid(DiagnosticCode::WrongType, Some(Location::Policy))),
        ("policy number", POLICY, "content_policy: 3\n".into(), Invalid(DiagnosticCode::WrongType, Some(Location::Policy))),
        ("policy one element wrong", ENTRY, format!("{ENTRY}  - 123\n"), Invalid(DiagnosticCode::WrongType, entry(1))),
        ("policy every element wrong", ENTRY, "  - 123\n".into(), Invalid(DiagnosticCode::WrongType, entry(0))),
        ("policy empty list", POLICY, "content_policy: []\n".into(), Invalid(DiagnosticCode::EmptyPolicy, Some(Location::Policy))),
        ("policy empty mapping", POLICY, "content_policy: {}\n".into(), Invalid(DiagnosticCode::WrongType, Some(Location::Policy))),
        ("policy duplicate key", POLICY, format!("{POLICY}content_policy:\n  - Evergreen\n"), Invalid(DiagnosticCode::DuplicateKey, Some(Location::Property { name: "content_policy".into(), entry: None }))),
        ("policy malformed yaml", POLICY, "content_policy: [ValidFor(3mo)\n".into(), Malformed(MalformedCause::Yaml)),
        ("policy unquoted template", POLICY, "content_policy: {{ policy }}\n".into(), Malformed(MalformedCause::ExpressionTemplate)),
        ("policy trailing text", ENTRY, "  - ValidFor(3mo) x\n".into(), Invalid(DiagnosticCode::InvalidRuleSyntax, entry(0))),
        // --- One entry ---
        ("entry null", ENTRY, "  -\n".into(), Invalid(DiagnosticCode::NullEntry, entry(0))),
        ("entry boolean", ENTRY, "  - true\n".into(), Invalid(DiagnosticCode::WrongType, entry(0))),
        ("entry list", ENTRY, "  - [ValidFor(3mo)]\n".into(), Invalid(DiagnosticCode::WrongType, entry(0))),
        ("entry empty mapping", ENTRY, "  - {}\n".into(), Invalid(DiagnosticCode::MissingRule, entry(0))),
        ("entry duplicate key", ENTRY, "  - rule: ValidFor(3mo)\n    rule: Evergreen\n    action: refresh\n".into(), Invalid(DiagnosticCode::DuplicateKey, None)),
        ("entry comma split", POLICY, "content_policy: [ValidFor(3mo, 2026-03-18)]\n".into(), Invalid(DiagnosticCode::UnbalancedParentheses, entry(0))),
        ("entry unknown key", ENTRY, "  - rule: ValidFor(3mo)\n    action: refresh\n    note: x\n".into(), Invalid(DiagnosticCode::UnknownField, entry(0))),
        // --- `rule` (long form) ---
        ("long form control", ENTRY, long_form("rule: ValidFor(3mo)", "action: refresh"), Report(Status::Fresh, PolicySource::Declared)),
        ("rule absent", ENTRY, "  - action: refresh\n".into(), Invalid(DiagnosticCode::MissingRule, entry(0))),
        ("rule null", ENTRY, long_form("rule:", "action: refresh"), Invalid(DiagnosticCode::WrongType, field(0, EntryField::Rule))),
        ("rule number", ENTRY, long_form("rule: 5", "action: refresh"), Invalid(DiagnosticCode::WrongType, field(0, EntryField::Rule))),
        ("rule mapping", ENTRY, long_form("rule: {a: 1}", "action: refresh"), Invalid(DiagnosticCode::WrongType, field(0, EntryField::Rule))),
        ("rule empty", ENTRY, long_form("rule: \"\"", "action: refresh"), Invalid(DiagnosticCode::InvalidRuleSyntax, field(0, EntryField::Rule))),
        ("rule trailing", ENTRY, long_form("rule: ValidFor(3mo))", "action: refresh"), Invalid(DiagnosticCode::UnbalancedParentheses, field(0, EntryField::Rule))),
        ("rule unknown", ENTRY, long_form("rule: Duration(3mo)", "action: refresh"), Invalid(DiagnosticCode::UnknownRule, field(0, EntryField::Rule))),
        // --- `action` (long form) ---
        ("action absent", ENTRY, "  - rule: ValidFor(3mo)\n".into(), Invalid(DiagnosticCode::MissingAction, entry(0))),
        ("action null", ENTRY, long_form("rule: ValidFor(3mo)", "action:"), Invalid(DiagnosticCode::WrongType, field(0, EntryField::Action))),
        ("action number", ENTRY, long_form("rule: ValidFor(3mo)", "action: 5"), Invalid(DiagnosticCode::WrongType, field(0, EntryField::Action))),
        ("action empty", ENTRY, long_form("rule: ValidFor(3mo)", "action: \"\""), Invalid(DiagnosticCode::UnknownAction, field(0, EntryField::Action))),
        ("action trailing", ENTRY, long_form("rule: ValidFor(3mo)", "action: archive x"), Invalid(DiagnosticCode::UnknownAction, field(0, EntryField::Action))),
        ("action unknown", ENTRY, long_form("rule: ValidFor(3mo)", "action: delete"), Invalid(DiagnosticCode::UnknownAction, field(0, EntryField::Action))),
        // --- Date property (`last_updated`) ---
        ("date absent", DATE, String::new(), Unknown(UnknownReason::MissingBaseline)),
        ("date null (no value)", DATE, "last_updated:\n".into(), Unknown(UnknownReason::MissingBaseline)),
        ("date null (~)", DATE, "last_updated: ~\n".into(), Unknown(UnknownReason::MissingBaseline)),
        ("date quoted", DATE, "last_updated: \"2026-03-18\"\n".into(), Report(Status::Fresh, PolicySource::Declared)),
        ("date number", DATE, "last_updated: 20260318\n".into(), Invalid(DiagnosticCode::WrongType, property("last_updated"))),
        ("date boolean", DATE, "last_updated: true\n".into(), Invalid(DiagnosticCode::WrongType, property("last_updated"))),
        ("date list", DATE, "last_updated: [2026-03-18]\n".into(), Invalid(DiagnosticCode::WrongType, property("last_updated"))),
        ("date mapping", DATE, "last_updated: {on: 2026-03-18}\n".into(), Invalid(DiagnosticCode::WrongType, property("last_updated"))),
        ("date empty", DATE, "last_updated: \"\"\n".into(), Invalid(DiagnosticCode::InvalidDate, property("last_updated"))),
        ("date duplicate key", DATE, format!("{DATE}{DATE}"), Invalid(DiagnosticCode::DuplicateKey, Some(Location::Property { name: "last_updated".into(), entry: None }))),
        ("date trailing", DATE, "last_updated: 2026-03-18x\n".into(), Invalid(DiagnosticCode::InvalidDate, property("last_updated"))),
        ("date not a calendar date", DATE, "last_updated: 2026-02-30\n".into(), Invalid(DiagnosticCode::InvalidDate, property("last_updated"))),
        ("date timestamp", DATE, "last_updated: 2026-03-18T10:00:00Z\n".into(), Invalid(DiagnosticCode::DatesOnly, property("last_updated"))),
    ];
    for (name, from, to, expect) in &cells {
        assert_eq!(FIXTURE.matches(from).count(), 1, "{name}: edit target must be unique");
        let edited = FIXTURE.replacen(from, to, 1);
        assert_ne!(edited, FIXTURE, "{name}: the edit changed nothing");
        check(name, edited.as_bytes(), expect);
    }
}

/// The fingerprint property column: the fixture's policy becomes one
/// `FileChanged` rule over `fp`, whose stored value matches the scripted
/// watched file; each cell is then one edit to the `fp` line.
#[test]
fn fingerprint_property_matrix() {
    const WATCHED: &[u8] = b"pub fn config() {}\n";
    let stored = FingerprintScheme::Blake3Lf.fingerprint(WATCHED);
    let hex = &stored["blake3-lf:".len()..];
    let fp = format!("fp: {stored}\n");
    let base = FIXTURE
        .replacen(POLICY, "content_policy:\n  - FileChanged(src/config.rs, @fp)\n", 1)
        .replacen(DATE, &format!("{DATE}{fp}"), 1);
    assert_ne!(base, FIXTURE);
    let provider = fake::FakeFiles::new().present("src/config.rs", WATCHED);
    let context = EvaluationContext::new(at(WHEN)).with_files(provider, "docs");

    // Control row: the stored fingerprint matches, so the report is fresh.
    check_in(&context, "control", base.as_bytes(), &Report(Status::Fresh, PolicySource::Declared));

    let fingerprint = || property("fp");
    let cells: Vec<(&str, String, Expect)> = vec![
        ("absent", String::new(), Unknown(UnknownReason::MissingBaseline)),
        ("null (no value)", "fp:\n".into(), Unknown(UnknownReason::MissingBaseline)),
        ("null (~)", "fp: ~\n".into(), Unknown(UnknownReason::MissingBaseline)),
        ("quoted", format!("fp: \"{stored}\"\n"), Report(Status::Fresh, PolicySource::Declared)),
        ("number", "fp: 12\n".into(), Invalid(DiagnosticCode::WrongType, fingerprint())),
        ("boolean", "fp: true\n".into(), Invalid(DiagnosticCode::WrongType, fingerprint())),
        ("list", format!("fp: [{stored}]\n"), Invalid(DiagnosticCode::WrongType, fingerprint())),
        ("mapping", format!("fp: {{v: {stored}}}\n"), Invalid(DiagnosticCode::WrongType, fingerprint())),
        ("empty", "fp: \"\"\n".into(), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("duplicate key", format!("{fp}{fp}"), Invalid(DiagnosticCode::DuplicateKey, Some(Location::Property { name: "fp".into(), entry: None }))),
        ("non-hex digest", "fp: blake3-lf:zz\n".into(), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("no digest", "fp: blake3-lf\n".into(), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("no scheme", "fp: ':abc'\n".into(), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("uppercase digest", format!("fp: blake3-lf:{}\n", hex.to_uppercase()), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("short digest", format!("fp: blake3-lf:{}\n", &hex[..63]), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("trailing text", format!("fp: {stored} x\n"), Invalid(DiagnosticCode::InvalidFingerprint, fingerprint())),
        ("unrecognized scheme", "fp: sha256:ab\n".into(), Unknown(UnknownReason::IncompatibleFingerprint)),
        ("other recognized scheme", format!("fp: {}\n", FingerprintScheme::Blake3.fingerprint(b"other")), Report(Status::Stale, PolicySource::Declared)),
    ];
    for (name, to, expect) in &cells {
        assert_eq!(base.matches(fp.as_str()).count(), 1, "{name}: edit target must be unique");
        let edited = base.replacen(fp.as_str(), to, 1);
        assert_ne!(edited, base, "{name}: the edit changed nothing");
        check_in(&context, name, edited.as_bytes(), expect);
    }
}

/// A policy serialized by the library itself, over which each cell is one
/// edit to the JSON text.
#[test]
fn serialized_policy_matrix() {
    let source = Policy::from_declaration(&json!([
        "ValidFor(3mo)",
        { "rule": "ValidUntil(2027-01-01)", "action": "archive" },
    ]))
    .unwrap()
    .to_json();
    let evidence: EvidenceRecord = [("last_updated".to_string(), json!("2026-03-18"))].into_iter().collect();

    // Control row.
    let control = Policy::from_json(&source).unwrap();
    let report = evaluate_policy(&control, &evidence, &EvaluationContext::new(at(WHEN))).unwrap();
    assert_eq!(report.status, Status::Fresh);

    const VERSION: &str = r#""grammar_version":1,"#;
    const ENTRIES_START: &str = r#""entries":["#;
    let entries = &source[source.find(ENTRIES_START).unwrap()..source.len() - 1];
    let entries_after_comma = format!(",{entries}");
    let first = r#"{"rule":"ValidFor(3mo)","action":"refresh"}"#;
    assert!(source.contains(VERSION) && source.contains(first));

    let cells: Vec<(&str, &str, String, DiagnosticCode, Option<&str>)> = vec![
        // --- `grammar_version` ---
        ("version absent", VERSION, String::new(), DiagnosticCode::MalformedSerialization, Some("grammar_version")),
        ("version null", VERSION, r#""grammar_version":null,"#.into(), DiagnosticCode::MalformedSerialization, None),
        ("version string", VERSION, r#""grammar_version":"1","#.into(), DiagnosticCode::MalformedSerialization, None),
        ("version duplicate", VERSION, format!("{VERSION}{VERSION}"), DiagnosticCode::MalformedSerialization, Some("duplicate field")),
        ("version newer", VERSION, r#""grammar_version":2,"#.into(), DiagnosticCode::UnsupportedGrammarVersion, None),
        ("version zero", VERSION, r#""grammar_version":0,"#.into(), DiagnosticCode::MalformedSerialization, None),
        // --- `entries` ---
        ("entries absent", &entries_after_comma, String::new(), DiagnosticCode::MalformedSerialization, Some("entries")),
        ("entries null", entries, r#""entries":null"#.into(), DiagnosticCode::MalformedSerialization, None),
        ("entries object", entries, r#""entries":{}"#.into(), DiagnosticCode::MalformedSerialization, None),
        ("entries one element wrong", first, format!("{first},5"), DiagnosticCode::MalformedSerialization, Some("entry 2")),
        ("entries every element wrong", entries, r#""entries":[5]"#.into(), DiagnosticCode::MalformedSerialization, Some("entry 1")),
        ("entries empty", entries, r#""entries":[]"#.into(), DiagnosticCode::EmptyPolicy, None),
        ("entries duplicate", entries, format!("{entries},{entries}"), DiagnosticCode::MalformedSerialization, Some("duplicate field")),
        ("entry duplicate field", first, r#"{"rule":"ValidFor(3mo)","rule":"Evergreen","action":"refresh"}"#.into(), DiagnosticCode::MalformedSerialization, Some("duplicate field")),
        ("entry unknown field", first, r#"{"rule":"ValidFor(3mo)","action":"refresh","note":"x"}"#.into(), DiagnosticCode::MalformedSerialization, Some("unknown field")),
        ("entry invalid rule", first, r#"{"rule":"Duration(3mo)","action":"refresh"}"#.into(), DiagnosticCode::UnknownRule, None),
        ("entry invalid action", first, r#"{"rule":"ValidFor(3mo)","action":"delete"}"#.into(), DiagnosticCode::UnknownAction, None),
        // --- Trailing or invalid content ---
        ("unknown top-level field", VERSION, format!(r#"{VERSION}"extra":1,"#), DiagnosticCode::MalformedSerialization, Some("unknown field")),
        ("trailing bytes", "]}", "]} x".into(), DiagnosticCode::MalformedSerialization, Some("trailing")),
    ];
    for (name, from, to, code, message) in &cells {
        assert!(source.contains(from), "{name}: edit target missing");
        let edited = source.replacen(from, to, 1);
        assert_ne!(edited, source, "{name}: the edit changed nothing");
        let invalid = Policy::from_json(&edited).expect_err(name);
        let diagnostic = invalid
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code == *code)
            .unwrap_or_else(|| panic!("{name}: expected {code:?}, got {invalid}"));
        if let Some(message) = message {
            assert!(diagnostic.message.contains(message), "{name}: {}", diagnostic.message);
        }
    }
}

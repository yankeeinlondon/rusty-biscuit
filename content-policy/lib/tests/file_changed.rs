//! `FileChanged` evaluation through the public API with a scripted provider:
//! the outcome table, fingerprint schemes, path rules, observation sharing,
//! identity, and the evidence-map base directory.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::{DateTime, NaiveDate, Utc};
use content_policy::reader::{MalformedCause, ReadError};
use content_policy::{
    DiagnosticCode, DocumentError, EntryOutcome, EvaluationContext, EvidenceRecord,
    FileObservation, FileProvider, FingerprintScheme, Invalid, Location, Policy, Report, Status,
    UnknownReason, evaluate_document, evaluate_record,
};
use serde_json::{Value, json};

mod fake;

use fake::FakeFiles;

const CONTENT: &[u8] = b"fn main() {}\n";

fn at() -> DateTime<Utc> {
    NaiveDate::from_ymd_opt(2026, 9, 29)
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc()
}

fn base() -> PathBuf {
    PathBuf::from("docs")
}

fn context(provider: Arc<dyn FileProvider>) -> EvaluationContext {
    EvaluationContext::new(at()).with_files(provider, base())
}

fn lf(bytes: &[u8]) -> String {
    FingerprintScheme::Blake3Lf.fingerprint(bytes)
}

fn record(value: Value) -> EvidenceRecord {
    match value {
        Value::Object(map) => EvidenceRecord::from(map),
        other => panic!("not an object: {other}"),
    }
}

/// A record watching `src/config.rs` into `fp`, holding `stored` (omitted
/// when `None`).
fn watching(stored: Option<Value>) -> EvidenceRecord {
    let mut value = json!({ "content_policy": ["FileChanged(src/config.rs, @fp)"] });
    if let Some(stored) = stored {
        value["fp"] = stored;
    }
    record(value)
}

fn evaluate(record: &EvidenceRecord, provider: Arc<dyn FileProvider>) -> Result<Report, Invalid> {
    evaluate_record(record, &context(provider))
}

fn invalid(result: Result<Report, Invalid>) -> Invalid {
    result.expect_err("expected no verdict")
}

// --- Outcome table (AC 8) ---------------------------------------------------

/// One row per line of the spec's `FileChanged` outcome table, driven by the
/// scripted provider.
#[test]
fn every_row_of_the_outcome_table() {
    let stored = || Some(json!(lf(CONTENT)));
    type Row = (&'static str, Option<Value>, FileObservation, EntryOutcome, Status, &'static str);
    let rows: Vec<Row> = vec![
        ("fingerprints match", stored(), FileObservation::Present(CONTENT.to_vec()), EntryOutcome::NotTriggered, Status::Fresh, "Watched file unchanged"),
        ("fingerprints differ", stored(), FileObservation::Present(b"fn main() { edit() }\n".to_vec()), EntryOutcome::Triggered, Status::Stale, "Watched file changed"),
        ("file missing", stored(), FileObservation::Missing, EntryOutcome::Triggered, Status::Stale, "Source removed"),
        ("file unreadable", stored(), FileObservation::Unreadable("permission denied".into()), EntryOutcome::Unknown(UnknownReason::UnreadableFile), Status::Unknown, "permission denied"),
        ("path is a directory", stored(), FileObservation::NotAFile, EntryOutcome::Unknown(UnknownReason::NotAFile), Status::Unknown, "not a file"),
        ("property absent", None, FileObservation::Present(CONTENT.to_vec()), EntryOutcome::Unknown(UnknownReason::MissingBaseline), Status::Unknown, "No content fingerprint"),
        ("property null", Some(Value::Null), FileObservation::Present(CONTENT.to_vec()), EntryOutcome::Unknown(UnknownReason::MissingBaseline), Status::Unknown, "No content fingerprint"),
        ("unrecognized scheme", Some(json!("sha256:ab")), FileObservation::Present(CONTENT.to_vec()), EntryOutcome::Unknown(UnknownReason::IncompatibleFingerprint), Status::Unknown, "`sha256`"),
        // A missing file never proves change without a baseline to compare.
        ("property absent, file missing", None, FileObservation::Missing, EntryOutcome::Unknown(UnknownReason::MissingBaseline), Status::Unknown, "No content fingerprint"),
        ("unrecognized scheme, file missing", Some(json!("sha256:ab")), FileObservation::Missing, EntryOutcome::Unknown(UnknownReason::IncompatibleFingerprint), Status::Unknown, "`sha256`"),
    ];
    for (name, stored_value, observation, outcome, status, reason) in rows {
        let provider = FakeFiles::new().with("src/config.rs", observation);
        let report = evaluate(&watching(stored_value.clone()), provider)
            .unwrap_or_else(|error| panic!("{name}: {error}"));
        let result = &report.results[0];
        assert_eq!(result.outcome, outcome, "{name}");
        assert_eq!(report.status, status, "{name}");
        assert!(result.reason.contains(reason), "{name}: {}", result.reason);
        let file = result.file.as_ref().expect("file evidence");
        assert_eq!(file.path, "src/config.rs", "{name}");
        assert_eq!(file.property, "fp", "{name}");
        assert_eq!(file.stored, stored_value.and_then(|v| v.as_str().map(String::from)), "{name}");
        assert!(result.baseline.is_none() && result.deadline.is_none() && result.due.is_none(), "{name}");
    }
}

#[test]
fn the_report_carries_the_current_fingerprint_when_it_can_be_computed() {
    let edited = b"fn main() { edit() }\n";
    let provider = FakeFiles::new().present("src/config.rs", edited);
    let report = evaluate(&watching(Some(json!(lf(CONTENT)))), provider).unwrap();
    assert_eq!(report.results[0].file.as_ref().unwrap().current, Some(lf(edited)));

    // Nothing stored: the current value is what a first capture would write.
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let report = evaluate(&watching(None), provider).unwrap();
    assert_eq!(report.results[0].file.as_ref().unwrap().current, Some(lf(CONTENT)));

    // An unrecognized scheme cannot be recomputed; a missing file has no bytes.
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let report = evaluate(&watching(Some(json!("sha256:ab"))), provider).unwrap();
    assert_eq!(report.results[0].file.as_ref().unwrap().current, None);
    let report = evaluate(&watching(Some(json!(lf(CONTENT)))), FakeFiles::new()).unwrap();
    assert_eq!(report.results[0].file.as_ref().unwrap().current, None);
}

#[test]
fn a_path_outside_the_boundary_is_a_validation_error_with_no_verdict() {
    let provider = FakeFiles::new().with(
        "../../elsewhere.rs",
        FileObservation::OutsideBoundary("`/work/elsewhere.rs` is outside the repository root `/work/repo`".into()),
    );
    let evidence = record(json!({
        "content_policy": ["FileChanged(../../elsewhere.rs, @fp)", "TimeSensitive"],
    }));
    let invalid = invalid(evaluate(&evidence, provider));
    assert_eq!(invalid.diagnostics.len(), 1, "{invalid}");
    let diagnostic = &invalid.diagnostics[0];
    assert_eq!(diagnostic.code, DiagnosticCode::OutsideBoundary);
    assert_eq!(diagnostic.location, Location::Entry { index: 0 });
    assert!(diagnostic.message.contains("/work/repo"), "{}", diagnostic.message);
}

#[test]
fn without_a_provider_the_entry_is_unknown() {
    let report = evaluate_record(&watching(Some(json!(lf(CONTENT)))), &EvaluationContext::new(at())).unwrap();
    assert_eq!(report.results[0].outcome, EntryOutcome::Unknown(UnknownReason::MissingProvider));
    assert_eq!(report.status, Status::Unknown);
    assert!(!report.evaluation_complete);
}

#[test]
fn a_file_result_serializes_its_evidence() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let report = evaluate(&watching(Some(json!(lf(CONTENT)))), provider).unwrap();
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(
        json["results"][0],
        json!({
            "index": 0,
            "rule": "FileChanged(src/config.rs, @fp)",
            "action": "refresh",
            "renewal": "renewable",
            "result": "not_triggered",
            "unknown_reason": null,
            "baseline": null,
            "deadline": null,
            "file": { "path": "src/config.rs", "property": "fp", "stored": lf(CONTENT), "current": lf(CONTENT) },
            "due": null,
            "reason": "Watched file unchanged",
        })
    );
    let report = evaluate(&watching(Some(json!("sha256:ab"))), FakeFiles::new()).unwrap();
    let json: Value = serde_json::from_str(&report.to_json()).unwrap();
    assert_eq!(json["results"][0]["unknown_reason"], "incompatible_fingerprint");
}

// --- Fingerprint schemes (AC 8) ---------------------------------------------

#[test]
fn blake3_lf_is_stable_across_line_endings_and_blake3_is_not() {
    let lf_bytes = b"line one\nline two\n";
    let crlf_bytes = b"line one\r\nline two\r\n";
    for (scheme, expected) in [
        (FingerprintScheme::Blake3Lf, EntryOutcome::NotTriggered),
        (FingerprintScheme::Blake3, EntryOutcome::Triggered),
    ] {
        let stored = scheme.fingerprint(lf_bytes);
        // The same content checked out with CRLF line endings.
        let provider = FakeFiles::new().present("src/config.rs", crlf_bytes);
        let report = evaluate(&watching(Some(json!(stored))), provider).unwrap();
        assert_eq!(report.results[0].outcome, expected, "{scheme}");
        let current = report.results[0].file.as_ref().unwrap().current.clone().unwrap();
        assert!(current.starts_with(&format!("{scheme}:")), "{current}");
    }
}

// --- Declaration and path rules (AC 8, 32 lexical rows, 33) -----------------

fn declaration_error(rule: &str) -> (DiagnosticCode, String) {
    let evidence = record(json!({ "content_policy": [rule] }));
    let provider = FakeFiles::new();
    let invalid = invalid(evaluate(&evidence, Arc::clone(&provider) as Arc<dyn FileProvider>));
    assert!(provider.requests().is_empty(), "{rule}: an invalid rule is never observed");
    // The same diagnostic without any provider.
    let without = invalid_without_provider(&evidence);
    assert_eq!(without.diagnostics, invalid.diagnostics, "{rule}");
    let diagnostic = &invalid.diagnostics[0];
    assert_eq!(diagnostic.location, Location::Entry { index: 0 }, "{rule}");
    (diagnostic.code, diagnostic.message.clone())
}

fn invalid_without_provider(evidence: &EvidenceRecord) -> Invalid {
    evaluate_record(evidence, &EvaluationContext::new(at())).expect_err("expected no verdict")
}

#[test]
fn the_one_argument_form_is_a_validation_error() {
    let (code, message) = declaration_error("FileChanged(src/config.rs)");
    assert_eq!(code, DiagnosticCode::InvalidArguments);
    assert!(message.contains("FileChanged(src/config.rs, @config_fingerprint)"), "{message}");
    for rule in ["FileChanged()", "FileChanged(src/config.rs, fp)", "FileChanged(src/config.rs, @)"] {
        let (code, _) = declaration_error(rule);
        assert!(matches!(code, DiagnosticCode::InvalidArguments | DiagnosticCode::InvalidReference), "{rule}: {code:?}");
    }
    assert_eq!(declaration_error("FileChanged(src/config.rs, @a.b)").0, DiagnosticCode::NestedReference);
    assert_eq!(declaration_error("filechanged(src/config.rs, @fp)").0, DiagnosticCode::UnknownRule);
}

/// Every rejected lexical row of the spec's path table, through the public
/// API, with and without a provider.
#[test]
fn every_rejected_lexical_path_form_is_a_validation_error() {
    for path in [
        "/etc/hosts",
        "C:\\x",
        "C:/x",
        "C:x",
        "src\\config.rs",
        "~/x",
        "@x",
        "%x",
        "vault:x",
        "{{HOME}}/x",
        "https://example.com/x.md",
        " src/config.rs",
        "src/config.rs ",
    ] {
        let (code, message) = declaration_error(&format!("FileChanged({path}, @fp)"));
        assert_eq!(code, DiagnosticCode::InvalidPath, "{path:?}: {message}");
    }
}

/// AC 33: `,` and `)` are the grammar's delimiters.
#[test]
fn a_path_containing_a_comma_or_a_closing_parenthesis_is_a_validation_error() {
    for rule in [
        "FileChanged(notes/a,b.md, @fp)",
        "FileChanged(notes/a)b.md, @fp)",
        "FileChanged(notes/(draft).md, @fp)",
    ] {
        let (code, message) = declaration_error(rule);
        assert_eq!(code, DiagnosticCode::InvalidPath, "{rule}: {message}");
        assert!(message.contains("`,` or `)`"), "{rule}: {message}");
    }
    // In a one-line list the unquoted `@` is a YAML indicator, so the block
    // is malformed, as for `ValidFor(3mo, @last_updated)`; quoted, it
    // evaluates.
    let bytes = b"---\ncontent_policy: [FileChanged(src/config.rs, @fp)]\n---\n";
    let Err(DocumentError::Read { error: ReadError::Malformed { cause, .. }, .. }) =
        evaluate_document(bytes, &context(FakeFiles::new()))
    else {
        panic!("expected malformed frontmatter");
    };
    assert_eq!(cause, MalformedCause::Yaml);
    let quoted = format!(
        "---\nfp: {}\ncontent_policy: [\"FileChanged(src/config.rs, @fp)\"]\n---\n",
        lf(CONTENT)
    );
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let report = evaluate_document(quoted.as_bytes(), &context(provider)).unwrap();
    assert_eq!(report.status, Status::Fresh);
}

/// AC 33: ` #` and `: ` are YAML traps that quoting the whole rule avoids.
#[test]
fn a_quoted_rule_whose_path_holds_a_yaml_trap_evaluates() {
    let hash = "notes/a #1.md";
    let colon = "logs/run: 2.txt";
    let document = format!(
        "---\nnotes_fingerprint: {}\nlog_fingerprint: {}\ncontent_policy:\n  - \"FileChanged({hash}, @notes_fingerprint)\"\n  - \"FileChanged({colon}, @log_fingerprint)\"\n---\n",
        lf(b"note"),
        lf(b"log")
    );
    let provider = FakeFiles::new().present(hash, b"note").present(colon, b"log");
    let report = evaluate_document(document.as_bytes(), &context(Arc::clone(&provider) as Arc<dyn FileProvider>)).unwrap();
    assert_eq!(report.status, Status::Fresh);
    assert_eq!(report.results[0].file.as_ref().unwrap().path, hash);
    assert_eq!(report.results[1].file.as_ref().unwrap().path, colon);
    assert_eq!(provider.requests_for(hash), 1);

    // Unquoted, ` #` starts a comment and truncates the rule.
    let truncated = format!("---\ncontent_policy:\n  - FileChanged({hash}, @notes_fingerprint)\n---\n");
    let Err(DocumentError::Invalid(invalid)) = evaluate_document(truncated.as_bytes(), &context(FakeFiles::new())) else {
        panic!("expected no verdict");
    };
    assert_eq!(invalid.diagnostics[0].code, DiagnosticCode::UnbalancedParentheses, "{invalid}");
}

// --- Observation sharing, base directory, identity ---------------------------

#[test]
fn identical_requests_share_one_observation_per_run() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT).present("src/other.rs", CONTENT);
    let evidence = record(json!({
        "a": lf(CONTENT),
        "b": lf(CONTENT),
        "c": lf(CONTENT),
        "content_policy": [
            "FileChanged(src/config.rs, @a)",
            {"rule": "FileChanged(src/config.rs, @b)", "action": "archive"},
            "FileChanged(src/other.rs, @c)",
        ],
    }));
    let context = context(Arc::clone(&provider) as Arc<dyn FileProvider>);
    let report = evaluate_record(&evidence, &context).unwrap();
    assert_eq!(report.status, Status::Fresh);
    assert_eq!(provider.requests_for("src/config.rs"), 1);
    assert_eq!(provider.requests_for("src/other.rs"), 1);
    // A second evaluation is a second run.
    evaluate_record(&evidence, &context).unwrap();
    assert_eq!(provider.requests_for("src/config.rs"), 2);
}

/// AC 8 and 11: a plain evidence map, with no document behind it, resolves
/// paths against the directory the caller supplies.
#[test]
fn the_evidence_map_api_uses_the_callers_base_directory() {
    let provider = FakeFiles::new().present("inputs/prompt.md", CONTENT);
    let manifest = record(json!({
        "prompt_fingerprint": lf(CONTENT),
        "content_policy": ["FileChanged(inputs/prompt.md, @prompt_fingerprint)"],
    }));
    let cache_dir = Path::new("/var/cache/artifacts/42");
    let context = EvaluationContext::new(at()).with_files(Arc::clone(&provider) as Arc<dyn FileProvider>, cache_dir);
    assert_eq!(context.base_dir(), Some(cache_dir));
    let report = evaluate_record(&manifest, &context).unwrap();
    assert_eq!(report.status, Status::Fresh);
    assert_eq!(report.document, None);
    assert_eq!(provider.requests(), vec![("inputs/prompt.md".to_string(), cache_dir.to_path_buf())]);
}

/// AC 23: the path and the property are policy; the stored fingerprint is
/// evidence.
#[test]
fn identity_includes_the_path_and_property_but_not_the_fingerprint() {
    let identity = |rule: &str| Policy::from_declaration(&json!([rule])).unwrap().identity();
    let base = identity("FileChanged(src/config.rs, @fp)");
    assert_ne!(identity("FileChanged(src/other.rs, @fp)"), base);
    assert_ne!(identity("FileChanged(./src/config.rs, @fp)"), base);
    assert_ne!(identity("FileChanged(src/config.rs, @other)"), base);
    assert_ne!(identity("ValidFor(3mo, @fp)"), base);
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let before = evaluate(&watching(Some(json!(lf(b"old")))), Arc::clone(&provider) as Arc<dyn FileProvider>).unwrap();
    let after = evaluate(&watching(Some(json!(lf(CONTENT)))), provider).unwrap();
    assert_eq!(before.policy.identity, base);
    assert_eq!(after.policy.identity, base);
}

#[test]
fn file_rules_round_trip_through_the_normalized_policy() {
    let policy = Policy::from_declaration(&json!([
        "FileChanged(&Cargo.toml, @manifest_fingerprint)",
        {"rule": "FileChanged(notes/a #1.md, @notes)", "action": "remove"},
    ]))
    .unwrap();
    let reread = Policy::from_json(&policy.to_json()).unwrap();
    assert_eq!(reread, policy);
    assert!(policy.to_json().contains(r#""rule":"FileChanged(&Cargo.toml, @manifest_fingerprint)""#));
}

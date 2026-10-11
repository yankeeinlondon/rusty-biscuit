//! `FileChanged` renewal through the public API with a scripted provider:
//! first capture, scheme preservation, unavailable evidence, flow-style
//! policies, consolidation, and the lifecycle.

use std::sync::Arc;

use chrono::NaiveDate;
use content_policy::{
    BaselineTarget, ChangeKind, ConflictKind, EntryOutcome, EvaluationContext, EvidenceIssueKind,
    FileObservation, FileProvider, FingerprintScheme, RenewalContext, RenewalError, RenewalPlan,
    Status, evaluate_document, plan_renewal,
};
use serde_json::Value;

mod fake;

use fake::FakeFiles;

const TODAY: &str = "2026-09-29";
const CONTENT: &[u8] = b"pub fn config() {}\n";

fn today() -> NaiveDate {
    NaiveDate::parse_from_str(TODAY, "%Y-%m-%d").unwrap()
}

fn renewal(provider: &Arc<FakeFiles>) -> RenewalContext {
    RenewalContext::new(today()).with_files(Arc::clone(provider) as Arc<dyn FileProvider>, "docs")
}

fn plan(source: &str, provider: &Arc<FakeFiles>) -> RenewalPlan {
    plan_renewal(source.as_bytes(), &renewal(provider)).unwrap_or_else(|error| panic!("{error}\n{source}"))
}

fn renewed(source: &str, provider: &Arc<FakeFiles>) -> String {
    let plan = plan(source, provider);
    String::from_utf8(plan.apply_to(source.as_bytes()).unwrap()).unwrap()
}

fn status(source: &str, provider: &Arc<FakeFiles>) -> Status {
    let context = EvaluationContext::new(today().and_hms_opt(0, 0, 0).unwrap().and_utc())
        .with_files(Arc::clone(provider) as Arc<dyn FileProvider>, "docs");
    evaluate_document(source.as_bytes(), &context).unwrap_or_else(|error| panic!("{error}")).status
}

fn lf(bytes: &[u8]) -> String {
    FingerprintScheme::Blake3Lf.fingerprint(bytes)
}

fn raw(bytes: &[u8]) -> String {
    FingerprintScheme::Blake3.fingerprint(bytes)
}

fn issues(source: &str, context: &RenewalContext) -> Vec<(EvidenceIssueKind, usize, String)> {
    match plan_renewal(source.as_bytes(), context) {
        Err(RenewalError::MissingEvidence { issues, .. }) => issues
            .into_iter()
            .map(|issue| (issue.kind, issue.entry, issue.property))
            .collect(),
        other => panic!("expected missing evidence, got {other:?}"),
    }
}

#[test]
fn a_first_capture_writes_a_blake3_lf_fingerprint() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let source = "---\ntitle: Guide\ncontent_policy:\n  - FileChanged(src/config.rs, @config_fingerprint)\n---\n\nBody\n";
    let plan = plan(source, &provider);
    assert_eq!(plan.changes.len(), 1);
    let change = &plan.changes[0];
    assert_eq!(change.target, BaselineTarget::Property { name: "config_fingerprint".into() });
    assert_eq!((change.kind, change.previous.as_deref()), (ChangeKind::NewBaseline, None));
    assert_eq!(change.value, lf(CONTENT));
    assert!(change.value.starts_with("blake3-lf:"));
    let captured = String::from_utf8(plan.apply_to(source.as_bytes()).unwrap()).unwrap();
    assert_eq!(
        captured,
        source.replace("---\n\nBody", &format!("config_fingerprint: {}\n---\n\nBody", lf(CONTENT)))
    );
    assert_eq!(status(&captured, &provider), Status::Fresh);

    // A `null` placeholder is filled in place, before its comment.
    let placeholder = "---\nconfig_fingerprint:   # captured by renew\ncontent_policy:\n  - FileChanged(src/config.rs, @config_fingerprint)\n---\n";
    assert_eq!(
        renewed(placeholder, &provider),
        placeholder.replace("config_fingerprint:   #", &format!("config_fingerprint: {}   #", lf(CONTENT)))
    );
}

#[test]
fn renewal_keeps_an_existing_scheme() {
    let edited = b"pub fn config() { changed() }\n";
    let provider = FakeFiles::new().present("assets/logo.png", edited);
    let old = raw(CONTENT);
    let source = format!("---\nlogo: {old}\ncontent_policy:\n  - FileChanged(assets/logo.png, @logo)\n---\n");
    assert_eq!(status(&source, &provider), Status::Stale);
    let plan = plan(&source, &provider);
    let change = &plan.changes[0];
    assert_eq!(change.kind, ChangeKind::Renewed);
    assert_eq!(change.previous.as_deref(), Some(old.as_str()));
    assert_eq!(change.value, raw(edited));
    let renewed = String::from_utf8(plan.apply_to(source.as_bytes()).unwrap()).unwrap();
    assert_eq!(renewed, source.replace(&old, &raw(edited)));
    assert_eq!(status(&renewed, &provider), Status::Fresh);

    // An unchanged file writes nothing.
    let plan = plan_renewal(renewed.as_bytes(), &renewal(&provider)).unwrap();
    assert_eq!(plan.changes[0].kind, ChangeKind::Unchanged);
    assert!(plan.edits.is_empty());
}

/// Each way the evidence can be unavailable, listed together, and the time
/// baseline beside them is not advanced either.
#[test]
fn unavailable_evidence_writes_nothing_and_lists_every_issue() {
    let provider = FakeFiles::new()
        .with("gone.rs", FileObservation::Missing)
        .with("locked.rs", FileObservation::Unreadable("permission denied".into()))
        .with("dir", FileObservation::NotAFile)
        .present("fine.rs", CONTENT);
    let source = "---\nlast_updated: 2026-01-01\nd: sha256:ab\ncontent_policy:\n  - ValidFor(3mo)\n  - FileChanged(gone.rs, @a)\n  - FileChanged(locked.rs, @b)\n  - FileChanged(dir, @c)\n  - FileChanged(fine.rs, @d)\n---\n";
    assert_eq!(
        issues(source, &renewal(&provider)),
        vec![
            (EvidenceIssueKind::SourceRemoved, 1, "a".to_string()),
            (EvidenceIssueKind::Unreadable, 2, "b".to_string()),
            (EvidenceIssueKind::NotAFile, 3, "c".to_string()),
            (EvidenceIssueKind::IncompatibleFingerprint, 4, "d".to_string()),
        ]
    );
    let error = plan_renewal(source.as_bytes(), &renewal(&provider)).unwrap_err().to_string();
    assert!(error.contains("nothing was written"), "{error}");
    assert!(error.contains("entry 5: `d` holds a `sha256` fingerprint"), "{error}");

    // No provider at all.
    let source = "---\ncontent_policy:\n  - FileChanged(src/config.rs, @fp)\n---\n";
    assert_eq!(
        issues(source, &RenewalContext::new(today())),
        vec![(EvidenceIssueKind::MissingProvider, 0, "fp".to_string())]
    );
}

/// The policy list is flow-style, but the only edit target is the
/// fingerprint property outside it.
#[test]
fn a_fingerprint_property_renews_when_the_policy_list_is_flow_style() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let old = lf(b"old");
    let source = format!(
        "---\nfp: {old}\ncontent_policy: [\"FileChanged(src/config.rs, @fp)\", TimeSensitive]\n---\n"
    );
    assert_eq!(renewed(&source, &provider), source.replace(&old, &lf(CONTENT)));
}

#[test]
fn time_and_file_baselines_renew_together_and_identical_requests_share_one_read() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let source = format!(
        "---\nlast_updated: 2026-01-01\na: {}\ncontent_policy:\n  - ValidFor(3mo)\n  - FileChanged(src/config.rs, @a)\n  - rule: FileChanged(src/config.rs, @a)\n    action: archive\n  - FileChanged(src/config.rs, @b)\n---\n",
        lf(b"old")
    );
    let plan = plan(&source, &provider);
    // Evaluation and planning shared one observation of the file.
    assert_eq!(provider.requests_for("src/config.rs"), 1);
    let summary: Vec<_> = plan
        .changes
        .iter()
        .map(|change| (change.target.clone(), change.entries.clone(), change.kind, change.value.clone()))
        .collect();
    assert_eq!(
        summary,
        vec![
            (BaselineTarget::Property { name: "last_updated".into() }, vec![0], ChangeKind::Renewed, TODAY.to_string()),
            (BaselineTarget::Property { name: "a".into() }, vec![1, 2], ChangeKind::Renewed, lf(CONTENT)),
            (BaselineTarget::Property { name: "b".into() }, vec![3], ChangeKind::NewBaseline, lf(CONTENT)),
        ]
    );
    let json: Value = serde_json::from_str(&plan.to_json()).unwrap();
    assert_eq!(json["changes"][1]["previous"], Value::from(lf(b"old")));
    assert_eq!(json["changes"][1]["value"], Value::from(lf(CONTENT)));
    let renewed = String::from_utf8(plan.apply_to(source.as_bytes()).unwrap()).unwrap();
    assert_eq!(status(&renewed, &provider), Status::Fresh);
}

#[test]
fn two_files_written_to_one_property_conflict() {
    let provider = FakeFiles::new().present("a.rs", b"a").present("b.rs", b"b");
    let source = "---\ncontent_policy:\n  - FileChanged(a.rs, @fp)\n  - FileChanged(b.rs, @fp)\n---\n";
    match plan_renewal(source.as_bytes(), &renewal(&provider)) {
        Err(RenewalError::Conflict { conflicts, .. }) => {
            assert_eq!(conflicts.len(), 1);
            assert_eq!(conflicts[0].kind, ConflictKind::DifferentWrites);
            assert_eq!((conflicts[0].property.as_str(), conflicts[0].entries.clone()), ("fp", vec![0, 1]));
        }
        other => panic!("expected a conflict, got {other:?}"),
    }
}

/// AC 8: the lifecycle with a scripted provider. Capture, match, edit the
/// watched file, triggered, renew, fresh; renewal never changes the identity.
#[test]
fn the_file_changed_lifecycle_with_a_fake_provider() {
    let provider = FakeFiles::new().present("src/config.rs", CONTENT);
    let original = "---\ncontent_policy:\n  - rule: FileChanged(src/config.rs, @config_fingerprint)\n    action: archive\n---\n";
    let context = || {
        EvaluationContext::new(today().and_hms_opt(0, 0, 0).unwrap().and_utc())
            .with_files(Arc::clone(&provider) as Arc<dyn FileProvider>, "docs")
    };
    let first = evaluate_document(original.as_bytes(), &context()).unwrap();
    assert_eq!(first.status, Status::Unknown);

    let captured = renewed(original, &provider);
    let report = evaluate_document(captured.as_bytes(), &context()).unwrap();
    assert_eq!(report.status, Status::Fresh);
    assert_eq!(report.policy.identity, first.policy.identity);

    let provider = provider.present("src/config.rs", b"pub fn config() { edited() }\n");
    let report = evaluate_document(captured.as_bytes(), &context()).unwrap();
    assert_eq!((report.status, report.action.map(|a| a.as_str())), (Status::Expired, Some("archive")));
    assert_eq!(report.results[0].outcome, EntryOutcome::Triggered);

    let recaptured = renewed(&captured, &provider);
    assert_eq!(evaluate_document(recaptured.as_bytes(), &context()).unwrap().status, Status::Fresh);
    assert_eq!(recaptured.len(), captured.len(), "a renewal replaces the value in place");
}

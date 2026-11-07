//! A reference issue as a shared diagnostic for documents with no authored
//! positions, which `md` itself never checks, and the serialized record.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::expression::{Diagnostic, DiagnosticSource, Severity};
use darkmatter::markdown::compose::report_diagnostics::reference_issue_diagnostic;
use darkmatter::markdown::compose::{ComposeContext, ComposeOptions, ComposeRequest, ComposeSource, RequestSnapshot};
use darkmatter::markdown::reference::ReferenceGraphOptions;
use darkmatter::markdown::reference::validate::ReferenceValidationOptions;
use serde_json::{Value, json};

fn the_record(markdown: Markdown, dir: &Path) -> Diagnostic {
    let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""));
    let request = ComposeRequest::prepare(options, &RequestSnapshot::new(dir)).unwrap();
    let validation = ReferenceValidationOptions::with_graph(ReferenceGraphOptions::with_compose(&request));
    let report = markdown.validate_references(validation).expect("the check runs");
    let [issue] = report.issues.as_slice() else { panic!("one issue: {:?}", report.issues) };
    reference_issue_diagnostic(issue)
}

#[test]
fn an_in_memory_document_has_no_file_and_null_positions() {
    let dir = tempfile::tempdir().unwrap();

    let record = the_record(Markdown::from("# Top\n\n[bad](http://[)\n".to_string()), dir.path());

    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["source"]["file"], Value::Null, "{value:#}");
    assert_eq!(value["source"]["line"], Value::Null, "{value:#}");
    assert_eq!(value["source"]["column"], Value::Null, "{value:#}");
    assert_eq!(value["source"]["inserted"], false, "{value:#}");
    assert_eq!(value["span"], Value::Null, "{value:#}");
    assert_eq!(value["reason"], "malformed URL", "{value:#}");
}

/// A name gives the document a frame of its own, which its text indexes.
#[test]
fn a_named_in_memory_document_is_located_in_its_own_text() {
    let dir = tempfile::tempdir().unwrap();
    let name = dir.path().join("named.md");
    let markdown = Markdown::from("# Top\n\n[bad](http://[)\n".to_string()).with_source(ComposeSource::File(name.clone()));

    let record = the_record(markdown, dir.path());

    let source = record.source.expect("the document is named");
    assert_eq!(source.file.as_deref(), Some(name.as_path()));
    assert_eq!((source.line, source.column), (Some(3), Some(1)));
    assert_eq!(source.section.as_deref(), Some("top"));
}

#[test]
fn a_remote_documents_file_is_its_url() {
    let dir = tempfile::tempdir().unwrap();
    let url = "https://host.example/a.md";
    let markdown = Markdown::from("# Top\n\n[bad](http://[)\n".to_string()).with_source(ComposeSource::Url(url.parse().unwrap()));

    let record = the_record(markdown, dir.path());

    let value = serde_json::to_value(&record).unwrap();
    assert_eq!(value["source"]["file"], url, "{value:#}");
    assert_eq!(value["source"]["line"], Value::Null, "{value:#}");
    assert_eq!(value["source"]["column"], Value::Null, "{value:#}");
}

/// Null, omitted, and `false` follow the data contract: positions are always
/// written, a section and the reference fields only when there is one.
#[test]
fn a_record_with_no_reference_details_omits_those_fields() {
    let record = Diagnostic {
        code: "dm.test.code".to_string(),
        severity: Severity::Warning,
        message: "message".to_string(),
        source: Some(DiagnosticSource::file_only(None)),
        expected_type: None,
        actual_type: None,
        span: None,
        iteration_path: Default::default(),
        reference_details: None,
        data_location: None,
    };

    assert_eq!(
        serde_json::to_value(&record).unwrap(),
        json!({
            "code": "dm.test.code",
            "severity": "warning",
            "message": "message",
            "source": { "file": null, "line": null, "column": null, "inserted": false },
            "expected_type": null,
            "actual_type": null,
            "span": null,
            "iteration_path": [],
        })
    );
}

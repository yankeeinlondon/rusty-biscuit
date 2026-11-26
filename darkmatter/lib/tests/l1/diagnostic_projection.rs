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
        reference_details: None,
        data_location: None,
        grants_needed: None,
        schema_default: None,
        related: Vec::new(),
        hint: None,
        notes: Vec::new(),
        trail: darkmatter::findings::Trail::default(),
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

/// The trail is the one stored copy of iteration and inclusion context;
/// `iteration_path` and `included_via` are serialized from it.
mod trail_projections {
    use darkmatter::findings::{ReferenceDetails, Trail, TrailFrame};
    use darkmatter::markdown::compose::ComposeSource;
    use darkmatter::markdown::compose::expression::iteration::{IterationFrame, IterationPath, IterationPosition};
    use darkmatter::markdown::compose::expression::{Diagnostic, Severity};
    use darkmatter::markdown::reference::ReferenceTrailStep;
    use serde_json::json;

    fn record() -> Diagnostic {
        Diagnostic::projected("dm.test.code", Severity::Error, "message", None)
    }

    fn teams_then_lead() -> IterationPath {
        IterationPath::empty()
            .pushed(IterationFrame { collection: "teams".into(), position: IterationPosition::Index(2) })
            .pushed(IterationFrame { collection: "members".into(), position: IterationPosition::Key("lead".into()) })
    }

    fn two_steps() -> Vec<ReferenceTrailStep> {
        vec![
            ReferenceTrailStep { file: ComposeSource::File("/w/root.md".into()), line: Some(3) },
            ReferenceTrailStep { file: ComposeSource::File("/w/child.md".into()), line: None },
        ]
    }

    fn reference(reports_inclusion: bool) -> ReferenceDetails {
        ReferenceDetails { reference: Some("./gone.md".into()), reports_inclusion, ..ReferenceDetails::default() }
    }

    const PREFIX: &str = r#"{"code":"dm.test.code","severity":"error","message":"message","source":null,"expected_type":null,"actual_type":null,"span":null,"#;

    #[test]
    fn iteration_frames_serialize_as_the_iteration_path_and_publish_no_trail() {
        let found = record().with_iteration_path(&teams_then_lead());

        assert_eq!(
            serde_json::to_string(&found).unwrap(),
            format!(
                r#"{PREFIX}"iteration_path":[{{"collection":"teams","position":2}},{{"collection":"members","position":"lead"}}]}}"#
            )
        );
    }

    #[test]
    fn a_reference_record_with_no_included_frames_writes_an_empty_inclusion_path() {
        let found = record().with_reference_details(reference(true));

        assert_eq!(
            serde_json::to_string(&found).unwrap(),
            format!(r#"{PREFIX}"iteration_path":[],"reference":"./gone.md","included_via":[]}}"#)
        );
    }

    #[test]
    fn a_record_that_reports_no_inclusion_path_omits_it() {
        let found = record().with_reference_details(reference(false));

        assert_eq!(
            serde_json::to_string(&found).unwrap(),
            format!(r#"{PREFIX}"iteration_path":[],"reference":"./gone.md"}}"#)
        );
    }

    #[test]
    fn included_frames_serialize_in_order_as_file_and_line_steps() {
        let found = record().with_reference_details(reference(true)).with_trail(Trail::included(&two_steps()));

        assert_eq!(
            serde_json::to_string(&found).unwrap(),
            format!(
                r#"{PREFIX}"iteration_path":[],"reference":"./gone.md","included_via":[{{"file":"/w/root.md","line":3}},{{"file":"/w/child.md","line":null}}]}}"#
            )
        );
    }

    #[test]
    fn a_trail_with_a_host_frame_is_published_whole_beside_the_projections() {
        let found = record()
            .with_reference_details(reference(true))
            .with_trail(Trail::included(&two_steps()[..1]))
            .with_iteration_path(&teams_then_lead())
            .with_trail(Trail::host("task", "metadata", "sequence[1].prompt"));

        let value = serde_json::to_value(&found).unwrap();

        assert_eq!(
            value["trail"],
            json!([
                { "kind": "host", "host": "task", "label": "metadata", "property": "sequence[1].prompt" },
                { "kind": "included", "file": "/w/root.md", "line": 3 },
                { "kind": "iteration", "collection": "teams", "position": 2 },
                { "kind": "iteration", "collection": "members", "position": "lead" },
            ]),
            "{value:#}"
        );
        assert_eq!(value["included_via"], json!([{ "file": "/w/root.md", "line": 3 }]), "{value:#}");
        assert_eq!(
            value["iteration_path"],
            json!([{ "collection": "teams", "position": 2 }, { "collection": "members", "position": "lead" }]),
            "{value:#}"
        );
    }

    #[test]
    fn the_accessors_project_the_trail_and_respect_inclusion_applicability() {
        let found = record()
            .with_trail(Trail::included(&two_steps()))
            .with_iteration_path(&teams_then_lead())
            .with_trail(Trail::host("task", "find", "sequence[0]"));

        assert_eq!(found.iteration_path(), teams_then_lead());
        assert_eq!(found.included_via(), None, "not a reference record");
        let found = found.with_reference_details(reference(true));
        assert_eq!(found.included_via(), Some(two_steps()));
        assert!(matches!(found.trail.frames()[0], TrailFrame::Host { .. }));
        // Replacing the iteration path keeps every other frame.
        let found = found.with_iteration_path(&IterationPath::empty());
        assert!(found.iteration_path().is_empty());
        assert_eq!(found.included_via(), Some(two_steps()));
        assert_eq!(found.trail.frames().len(), 3);
    }
}

//! Content Policy's shipped editor schema imported by a document's own
//! `$schema`, the way an author or DMLS types `content_policy` without the
//! runtime baseline.
//!
//! The shared `content_policy_fixtures` declarations run here against a
//! document-declared `policy[](min(1))@./content-policy.yaml`; the embedded
//! and disk baselines hold the same table in `content_policy_embedded_baseline`
//! and `content_policy_disk_baseline`.

use std::collections::BTreeSet;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, SchemaOriginKind, ValidationProblemKind, ValidationReport,
};

use crate::content_policy_fixtures::{DECLARATIONS, Verdict, render};
use crate::request_support::context_at;

/// Appended after the declaration so every rendered position stays where
/// `render` put it.
const DOCUMENT_SCHEMA: &str = "$schema:\n  content_policy: \"policy[](min(1))@./content-policy.yaml\"\n";

fn validate(api: &DarkmatterSchemas, root: &Path, frontmatter: &str) -> (ValidationReport, Markdown) {
    let path = root.join("doc.md");
    std::fs::write(&path, format!("---\n{frontmatter}{DOCUMENT_SCHEMA}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("the document loads")
        .with_source(ComposeSource::File(path));
    (api.validate(&markdown).expect("the schema prepares"), markdown)
}

fn kind_name(kind: &ValidationProblemKind) -> &'static str {
    match kind {
        ValidationProblemKind::Missing => "missing",
        ValidationProblemKind::Type => "type",
        ValidationProblemKind::Invalid => "invalid",
    }
}

/// Every shared declaration, in each presentation, reports exactly its
/// expected problems at the authored node when the document itself imports
/// the shipped schema.
#[test]
fn declarations_meet_their_expectations_through_a_document_import() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("content-policy.yaml"), shipped).expect("write schema");
    let api = DarkmatterSchemas::new(context_at(&root));

    let mut failures = Vec::new();
    for case in DECLARATIONS {
        for rendered in render(case.declaration) {
            let label = format!("{} ({:?})", case.name, rendered.presentation);
            let (report, markdown) = validate(&api, &root, &rendered.frontmatter);
            let actual: BTreeSet<_> = report
                .problems
                .iter()
                .map(|p| (p.path.clone(), kind_name(&p.kind), p.line, p.column))
                .collect();
            let expected: BTreeSet<_> = case
                .problems
                .iter()
                .map(|p| {
                    let (line, column) = rendered.start(p.at);
                    (p.path.to_string(), p.kind, Some(line), Some(column))
                })
                .collect();
            let verdict = if report.valid { Verdict::Accept } else { Verdict::Reject };
            if verdict != case.schema || actual != expected {
                failures.push(format!(
                    "{label}: expected {:?} {expected:?}, got {verdict:?} {actual:?}",
                    case.schema
                ));
            }
            let origin = api
                .effective_for(&markdown)
                .expect("prepares")
                .expect("the document schema applies")
                .origins
                .get("content_policy")
                .map(|origin| origin.kind);
            if origin != Some(SchemaOriginKind::Document) {
                failures.push(format!("{label}: content_policy typed by {origin:?}"));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// A long-form entry with no `rule`, or `rule: null`, is rejected by the
/// shipped schema and by Content Policy's parser, at the entry for a missing
/// field and at the field for a null one.
#[test]
fn long_form_without_rule_is_rejected_by_the_schema_and_the_parser() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("content-policy.yaml"), shipped).expect("write schema");
    let api = DarkmatterSchemas::new(context_at(&root));
    let frontmatter = "content_policy:\n  - {action: refresh}\n  - {rule: null, action: refresh}\n";

    let (report, _) = validate(&api, &root, frontmatter);

    assert!(!report.valid);
    let paths: Vec<_> = report.problems.iter().map(|problem| problem.path.as_str()).collect();
    assert_eq!(paths, ["/content_policy/0", "/content_policy/1/rule"], "{:?}", report.problems);
    let declaration: serde_json::Value =
        serde_yaml_ng::from_str::<serde_json::Value>(frontmatter).expect("parses")["content_policy"].clone();
    assert!(content_policy::Policy::from_declaration(&declaration).is_err());
    for entry in declaration.as_array().expect("a list") {
        let alone = serde_json::Value::Array(vec![entry.clone()]);
        assert!(content_policy::Policy::from_declaration(&alone).is_err(), "parser accepted {alone}");
    }
}

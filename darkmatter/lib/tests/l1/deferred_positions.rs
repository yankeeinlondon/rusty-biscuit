//! Source position maps are built only when a diagnostic needs a
//! coordinate: a valid whole-YAML document or schema trigger builds none,
//! and an invalid one builds one map for the reported source and locates its
//! problem in the original text, a byte-order mark and CRLF lines included.
//! The counts need `work-counters`, which `just test` enables.

#![cfg(feature = "work-counters")]

use darkmatter::markdown::schemas::{InvalidTriggerFinding, YamlOutcome, triggers};
use darkmatter::work_counters::{Work, WorkSnapshot, snapshot};
use simplified_schema::SchemaError;

use crate::yaml_kind_registry::{Fixture, write};

/// A document that validates against its inline `$schema`.
const VALID_YAML: &str = "$schema:\n    name: string(required)\n    port: number\nname: api\nport: 8080\n";

/// A valid trigger selecting documents with a `title`.
const VALID_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string(required)\n$schema:\n  level: number\n";

/// A trigger whose match accepts every document, located on the `match` value.
const VACUOUS_TRIGGER: &str = "kind: schema-trigger\nmatch:\n  title: string\n";

fn maps_built_since(before: &WorkSnapshot) -> u64 {
    snapshot().since(before).get(Work::SourcePositionMapConstruction)
}

/// The three encodings of `text` every located cell is checked in.
fn encodings(text: &str) -> [(&'static str, String); 3] {
    [
        ("LF", text.to_string()),
        ("BOM + CRLF", format!("\u{feff}{}", text.replace('\n', "\r\n"))),
        ("Unicode", text.replacen('\n', " # «Ωμέγα»\n", 1)),
    ]
}

/// The findings of scanning a package whose `schemas/trigger.yaml` is `text`.
fn scan_trigger(fixture: &Fixture, text: &str) -> Vec<InvalidTriggerFinding> {
    write(&fixture.package().join("schemas/trigger.yaml"), text);
    let context = fixture.context(&fixture.package().join("doc.md"), &[]);
    triggers::scan(&context).expect("an invalid trigger never fails discovery").findings().to_vec()
}

#[test]
fn a_valid_yaml_document_builds_no_position_map() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");

    let before = snapshot();
    let outcome = fixture.validate(&document, VALID_YAML).expect("loads");

    let YamlOutcome::Validated(validation) = outcome else { panic!("validated") };
    assert!(validation.report.valid, "{:?}", validation.report.problems);
    assert_eq!(maps_built_since(&before), 0);
}

#[test]
fn an_invalid_yaml_document_builds_one_map_and_locates_the_original_text() {
    let fixture = Fixture::new();
    let document = fixture.package().join("doc.yaml");
    for (label, text) in encodings(&VALID_YAML.replace("port: 8080", "port: high")) {
        let before = snapshot();
        let outcome = fixture.validate(&document, &text).expect("loads");

        let YamlOutcome::Validated(validation) = outcome else { panic!("{label}: validated") };
        let [problem] = validation.report.problems.as_slice() else {
            panic!("{label}: one problem: {:?}", validation.report.problems)
        };
        assert_eq!((problem.line, problem.column), (Some(5), Some(7)), "{label}");
        let range = validation.byte_range(problem).expect("located");
        assert_eq!(&text[range], "high", "{label}");
        assert_eq!(maps_built_since(&before), 1, "{label}");
    }
}

#[test]
fn a_valid_trigger_builds_no_position_map() {
    let fixture = Fixture::new();

    let before = snapshot();
    let findings = scan_trigger(&fixture, VALID_TRIGGER);

    assert!(findings.is_empty(), "{findings:?}");
    assert_eq!(maps_built_since(&before), 0);
}

#[test]
fn an_invalid_trigger_builds_one_map_and_locates_the_original_text() {
    let fixture = Fixture::new();
    for (label, text) in encodings(VACUOUS_TRIGGER) {
        let before = snapshot();
        let findings = scan_trigger(&fixture, &text);

        let [finding] = findings.as_slice() else { panic!("{label}: one finding: {findings:?}") };
        assert!(matches!(finding.cause(), SchemaError::TriggerVacuousMatch), "{label}: {finding:?}");
        assert_eq!(finding.position(), Some((3, 3)), "{label}: {finding:?}");
        assert_eq!(maps_built_since(&before), 1, "{label}");
    }
}

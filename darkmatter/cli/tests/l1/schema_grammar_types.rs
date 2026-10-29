//! `md compose` and `md schema validate` end to end for tuples, `|` unions,
//! and description-only `function` types in a document's `$schema`.

use crate::common;

use common::CliProcessFixture;
use predicates::prelude::*;

const TUPLE_SCHEMA: &str = "$schema:\n  point: 'tuple([number, number]; required)'\n  id: 'string | number'\n";

#[test]
fn compose_accepts_a_document_matching_a_tuple_and_a_union() {
    let fixture = CliProcessFixture::named("compose_accepts_a_document_matching_a_tuple_and_a_union");
    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin(format!("---\n{TUPLE_SCHEMA}point: [1, 2]\nid: 7\n---\nAt {{{{ point[0] }}}}.\n"))
        .assert()
        .success()
        .stdout(predicate::str::contains("At 1."));
}

#[test]
fn compose_rejects_a_document_whose_tuple_has_too_many_positions() {
    let fixture = CliProcessFixture::named("compose_rejects_a_document_whose_tuple_has_too_many_positions");
    fixture
        .command()
        .args(["compose", "-"])
        .write_stdin(format!("---\n{TUPLE_SCHEMA}point: [1, 2, 3]\nid: 7\n---\nBody\n"))
        .assert()
        .failure()
        .stderr(predicate::str::contains("point"));
}

#[test]
fn schema_validate_rejects_a_function_typed_property_as_a_schema_error() {
    let fixture = CliProcessFixture::named("schema_validate_rejects_a_function_typed_property_as_a_schema_error");
    std::fs::write(
        fixture.cwd().join("doc.md"),
        "---\n$schema:\n  run: 'function(parameters(void); returns(string))'\n---\nBody\n",
    )
    .expect("write fixture");
    fixture
        .command()
        .args(["schema", "validate", "doc.md"])
        .assert()
        .code(2)
        .stdout(predicate::str::contains("description-only").and(predicate::str::contains("run")));
}

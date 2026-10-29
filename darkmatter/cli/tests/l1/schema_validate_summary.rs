//! `md schema validate`'s schema-presence wording comes from the final
//! validation's own preparation: after assignments it reflects the schema the
//! assignments selected, and a pending shell value is reported as before.
//! Preparation counts are asserted at the library (`schemas_validation_summary`
//! in the darkmatter L1 suite); the binary is never built with counters.

use std::path::PathBuf;
use std::process::Output;

use crate::common;

use common::CliProcessFixture;

const NO_SCHEMA: &str = "doesn't have a schema definition so is valid by default.";
const HAS_SCHEMA: &str = "has a schema definition and is valid.";

fn write(fixture: &CliProcessFixture, relative: &str, content: &str) -> PathBuf {
    let path = fixture.cwd().join(relative);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
    std::fs::write(&path, content).expect("write fixture");
    path
}

fn validate(fixture: &CliProcessFixture, args: &[&str], document: &PathBuf) -> Output {
    fixture
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema"])
        .args(args)
        .arg(document)
        .output()
        .expect("run md schema validate")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

#[test]
fn an_assignment_that_activates_a_trigger_is_worded_as_having_a_schema() {
    let fixture = CliProcessFixture::named("schema-validate-summary-trigger");
    assert!(fixture.initialize_repository());
    write(
        &fixture,
        "schemas/summary.trigger.yaml",
        "kind: schema-trigger\nmatch:\n  summary_mode: enum(on; required)\n$schema:\n  summary_field: string(required)\n",
    );
    let document = write(
        &fixture,
        "doc.md",
        "---\ntitle: plain\nsummary_field: x\n---\nbody\n",
    );

    let unassigned = validate(&fixture, &[], &document);
    let assigned = validate(&fixture, &["summary_mode=on"], &document);

    assert_eq!(unassigned.status.code(), Some(0));
    assert!(
        stdout(&unassigned).contains(NO_SCHEMA),
        "{}",
        stdout(&unassigned)
    );
    assert_eq!(assigned.status.code(), Some(0), "{}", stdout(&assigned));
    assert!(
        stdout(&assigned).contains(HAS_SCHEMA),
        "{}",
        stdout(&assigned)
    );
}

#[test]
fn a_pending_shell_value_is_valid_against_its_document_schema() {
    let fixture = CliProcessFixture::named("schema-validate-summary-pending");
    let document = write(
        &fixture,
        "doc.md",
        "---\n$schema:\n  summary_when: string(required)\nsummary_when: $(date)\n---\nbody\n",
    );

    let output = validate(&fixture, &[], &document);

    assert_eq!(output.status.code(), Some(0), "{}", stdout(&output));
    assert!(stdout(&output).contains(HAS_SCHEMA), "{}", stdout(&output));
}

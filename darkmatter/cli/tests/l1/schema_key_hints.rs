//! The schema hint of a removed or misspelled key through `md`'s published
//! surfaces: `md schema validate` (human `hint:` row and JSON `hint`) and
//! `md compose --output json` (the diagnostic's `hint`). A problem without a
//! proven hint keeps its existing record.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;

const DOCUMENT: &str = "---\n$schema:\n  name: string\n  setup: \"removed(use a preceding `side_effect:` task)\"\n  $constraints: { closed: true }\nnmae: x\nsetup: true\n---\nBody\n";

#[test]
fn schema_validate_json_carries_each_problems_hint() {
    let fixture = CliProcessFixture::named("schema_key_hints_json");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);
    let output = fixture
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema", "--output", "json"])
        .arg(&document)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("{error}: {}", String::from_utf8_lossy(&output.stdout))
    });
    let problems = report["files"][0]["problems"].as_array().unwrap_or_else(|| panic!("{report:#}"));
    let hints: Vec<(&str, &str)> = problems
        .iter()
        .map(|problem| (problem["message"].as_str().unwrap(), problem["hint"].as_str().unwrap_or("<none>")))
        .collect();
    assert!(hints.contains(&("key `setup` has been removed", "use a preceding `side_effect:` task")), "{hints:#?}");
    assert!(hints.iter().any(|(message, hint)| message.contains("nmae") && *hint == "did you mean `name`?"), "{hints:#?}");
}

#[test]
fn schema_validate_human_output_shows_the_hint_row() {
    let fixture = CliProcessFixture::named("schema_key_hints_human");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);
    let output = fixture
        .command()
        .args(["schema", "validate", "--disable", "baseline-schema"])
        .arg(&document)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
    assert!(text.contains("key `setup` has been removed"), "{text}");
    assert!(text.contains("hint: use a preceding `side_effect:` task"), "{text}");
    assert!(text.contains("hint: did you mean `name`?"), "{text}");
}

#[test]
fn compose_json_diagnostics_carry_the_removed_key_code_and_hint() {
    let fixture = CliProcessFixture::named("schema_key_hints_compose");
    let document = fixture.write_file("cwd/doc.md", DOCUMENT);
    let output = fixture
        .command()
        .args(["compose", "--disable", "baseline-schema", "--output", "json"])
        .arg(&document)
        .output()
        .unwrap();
    assert!(!output.status.success());
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("{error}: {}", String::from_utf8_lossy(&output.stdout))
    });
    let diagnostics = report["diagnostics"].as_array().unwrap_or_else(|| panic!("{report:#}"));
    let removed = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "dm.schema.removed_key")
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    assert_eq!(removed["hint"], "use a preceding `side_effect:` task");
    let unknown = diagnostics
        .iter()
        .find(|diagnostic| diagnostic["code"] == "dm.schema.unknown_key")
        .unwrap_or_else(|| panic!("{diagnostics:#?}"));
    assert_eq!(unknown["hint"], "did you mean `name`?");
}

#[test]
fn compose_reports_near_miss_advice_as_a_warning_and_still_succeeds() {
    let fixture = CliProcessFixture::named("schema_key_hints_advice");
    let document = fixture.write_file(
        "cwd/doc.md",
        "---\n$schema:\n  prompt: string\n  \"<string>\": unknown\n  $advise-unknown-keys: [agent]\npromt: x\nagent: y\ncolour: blue\n---\nBody\n",
    );
    let output = fixture
        .command()
        .args(["compose", "--disable", "baseline-schema", "--output", "json"])
        .arg(&document)
        .output()
        .unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let report: Value = serde_json::from_slice(&output.stdout).unwrap_or_else(|error| {
        panic!("{error}: {}", String::from_utf8_lossy(&output.stdout))
    });
    let diagnostics = report["diagnostics"].as_array().unwrap_or_else(|| panic!("{report:#}"));
    let advised: Vec<&Value> =
        diagnostics.iter().filter(|diagnostic| diagnostic["code"] == "dm.schema.near_miss_key").collect();
    let [advice] = advised.as_slice() else { panic!("one advice for `promt` only: {diagnostics:#?}") };
    assert_eq!(advice["severity"], "warning");
    assert_eq!(advice["hint"], "did you mean `prompt`?");
    assert_eq!(advice["message"], "key `promt` is not a declared field, so it is kept as data");
    assert_eq!(advice["source"]["line"], 6, "{advice:#}");
}

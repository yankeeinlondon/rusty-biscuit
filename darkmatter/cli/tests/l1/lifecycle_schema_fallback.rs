//! `md schema validate` against Darkmatter's baseline lifecycle action
//! types: the tolerant-parsing cases of `md compose`, judged by the schema.
//! An action `md` does not run (`say`) passes untyped, while a malformed
//! supported action and a forbidden one are schema errors at their event.

use crate::common::CliProcessFixture;
use serde_json::Value;

/// The NDJSON record `md schema validate --format json` writes for one
/// document whose frontmatter is `frontmatter`, and the exit status.
fn validate(name: &str, frontmatter: &str) -> (Option<i32>, Value) {
    let fixture = CliProcessFixture::named(name);
    fixture.write_file("cwd/doc.md", &format!("---\n{frontmatter}---\nBody\n"));
    let output = fixture
        .command()
        .args(["schema", "validate", "--format", "json", "doc.md"])
        .output()
        .expect("md runs");
    let stdout = String::from_utf8_lossy(&output.stdout);
    let line = stdout.lines().next().unwrap_or_else(|| {
        panic!("no record; stderr: {}", String::from_utf8_lossy(&output.stderr));
    });
    (output.status.code(), serde_json::from_str(line).expect("an NDJSON record"))
}

/// The JSON pointers of the record's problems.
fn problem_paths(record: &Value) -> Vec<String> {
    record["problems"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|problem| problem["path"].as_str().map(str::to_string))
        .collect()
}

fn assert_valid(name: &str, frontmatter: &str) {
    let (status, record) = validate(name, frontmatter);
    assert_eq!(record["valid"], Value::Bool(true), "{frontmatter}{record}");
    assert_eq!(status, Some(0), "{record}");
}

/// Invalid, with at least one problem under `event`'s pointer.
fn assert_error_at(name: &str, frontmatter: &str, event: &str) {
    let (status, record) = validate(name, frontmatter);
    assert_eq!(record["valid"], Value::Bool(false), "{frontmatter}{record}");
    assert_eq!(status, Some(1), "{record}");
    let pointer = format!("/{event}");
    let paths = problem_paths(&record);
    assert!(
        paths.iter().any(|path| path == &pointer || path.starts_with(&format!("{pointer}/"))),
        "a problem at `{event}`: {paths:?} {record}"
    );
}

#[test]
fn another_hosts_verb_in_composed_passes_untyped() {
    assert_valid("fallback-say", "composed:\n  - say: hi\n");
}

#[test]
fn a_malformed_info_long_form_in_composed_is_a_schema_error_at_composed() {
    assert_error_at("fallback-info-long", "composed:\n  - info: { text: hi, bogus: 1 }\n", "composed");
}

#[test]
fn a_shell_in_initialize_is_a_schema_error_at_initialize() {
    assert_error_at("fallback-initialize-shell", "initialize:\n  - shell: \"echo hi\"\n", "initialize");
}

#[test]
fn a_short_form_info_in_composed_is_valid() {
    assert_valid("fallback-info-short", "composed:\n  - info: hi\n");
}

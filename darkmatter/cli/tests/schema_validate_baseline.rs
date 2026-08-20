//! Byte-identical regression for `md schema validate` on the pre-`literal`/
//! `expression` legacy schema cases (spec AC #8: existing validation output is
//! a pure addition and must not drift).
//!
//! Each case under `tests/fixtures/schema_validate_baseline/<case>/` carries its
//! input document (`doc.md` plus any referenced schema file) and two checked-in
//! snapshots: `expected.json` (the `--format json` line) and `expected.pretty`
//! (the default pretty rendering with color disabled). The host-specific
//! `file://` document URL in the pretty snapshot is stored as the `{DOC_URL}`
//! placeholder and the rendered URL is normalized back to it, so the remaining
//! bytes — wording, ordering, indentation, arm prefixes, line annotations, and
//! the additive `description` sub-line — are compared exactly.

use std::fs;
use std::path::Path;
use tempfile::TempDir;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/schema_validate_baseline"
);

/// The pre-change legacy cases the Phase-1 baseline recorded.
const CASES: &[&str] = &[
    "property_union_invalid",
    "property_union_nested_object",
    "property_union_valid",
    "root_union",
    "root_union_none_match",
    "root_union_fileref",
    "missing_required",
    "enum_member_invalid",
];

/// Copy every input file for a case (everything but the `expected.*` snapshots)
/// into a fresh working directory so the CLI resolves a deterministic,
/// relative `doc.md` argument.
fn stage_inputs(case: &str) -> TempDir {
    let src = Path::new(FIXTURES).join(case);
    let work = TempDir::new().unwrap();
    for entry in fs::read_dir(&src).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_str().unwrap();
        if name == "expected.json" || name == "expected.pretty" {
            continue;
        }
        fs::copy(entry.path(), work.path().join(name)).unwrap();
    }
    work
}

fn read_snapshot(case: &str, name: &str) -> String {
    fs::read_to_string(Path::new(FIXTURES).join(case).join(name)).unwrap()
}

fn normalize_document_url(mut output: String) -> String {
    let href_start = output.find("(file://").expect("document link should exist") + 1;
    let href_end = href_start
        + output[href_start..]
            .find(')')
            .expect("document link should be closed");
    output.replace_range(href_start..href_end, "{DOC_URL}");
    output
}

#[test]
fn schema_validate_legacy_json_output_is_byte_identical() {
    for case in CASES {
        let work = stage_inputs(case);
        let expected = read_snapshot(case, "expected.json");
        let expects_success = expected.contains("\"valid\":true");

        let output = assert_cmd::Command::cargo_bin("md").unwrap()
            .current_dir(work.path())
            .args(["schema", "validate", "--format", "json", "doc.md"])
            .output()
            .unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            stdout, expected,
            "JSON output drifted for legacy case `{case}`"
        );
        assert_eq!(
            output.status.success(),
            expects_success,
            "exit status drifted for legacy JSON case `{case}`"
        );
    }
}

#[test]
fn schema_validate_legacy_pretty_output_is_byte_identical() {
    for case in CASES {
        let work = stage_inputs(case);
        let template = read_snapshot(case, "expected.pretty");
        let expects_success =
            read_snapshot(case, "expected.json").contains("\"valid\":true");

        let output = assert_cmd::Command::cargo_bin("md").unwrap()
            .current_dir(work.path())
            .env("NO_COLOR", "1")
            .args(["schema", "validate", "doc.md"])
            .output()
            .unwrap();

        let stdout = normalize_document_url(String::from_utf8(output.stdout).unwrap());
        assert_eq!(
            stdout, template,
            "pretty output drifted for legacy case `{case}`"
        );
        assert_eq!(
            output.status.success(),
            expects_success,
            "exit status drifted for legacy pretty case `{case}`"
        );
    }
}

//! Byte-identical regression for `md schema validate` on the pre-`literal`/
//! `expression` legacy schema cases (spec AC #8: existing validation output is
//! a pure addition and must not drift).
//!
//! Each case under `tests/fixtures/schema_validate_baseline/<case>/` carries its
//! input document (`doc.md` plus any referenced schema file) and two checked-in
//! snapshots: `expected.json` (the `--format json` line) and `expected.pretty`
//! (the default pretty rendering with color disabled). The host-specific
//! `file://` document URL in the pretty snapshot is stored as the `{DOC_URL}`
//! placeholder and substituted at run time, so the remaining bytes — wording,
//! ordering, indentation, arm prefixes, line annotations, and the additive
//! `description` sub-line — are compared exactly.

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

        // The rendered pretty output links to the document by its canonical
        // (symlink-resolved) absolute path, which is host-specific. Rebuild the
        // exact URL the CLI emits and substitute it into the snapshot.
        let canonical = fs::canonicalize(work.path().join("doc.md")).unwrap();
        let canonical = canonical.to_string_lossy();
        #[cfg(windows)]
        let canonical = canonical.strip_prefix(r"\\?\").unwrap_or(&canonical);
        let expected = template.replace("{DOC_URL}", &format!("file://{canonical}"));

        let output = assert_cmd::Command::cargo_bin("md").unwrap()
            .current_dir(work.path())
            .env("NO_COLOR", "1")
            .args(["schema", "validate", "doc.md"])
            .output()
            .unwrap();

        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(
            stdout, expected,
            "pretty output drifted for legacy case `{case}`"
        );
        assert_eq!(
            output.status.success(),
            expects_success,
            "exit status drifted for legacy pretty case `{case}`"
        );
    }
}

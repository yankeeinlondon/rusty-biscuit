//! CLI-boundary wiring for Darkmatter's named-type loader (`name@./file.yaml`)
//! in `claudine compose`.
//!
//! The loader's field and document shapes are proven in-process by
//! Darkmatter's `schemas_named_import_errors` and envelope matrix. Here one
//! control and two invalid imported files show the loader is wired into the
//! launch: a malformed or duplicate-key file stops the launch with an error
//! naming that file, and the provider never starts.
//!
//! The marker provider is the compiled `claudine-fake-goose`, so these run on
//! every OS.

use std::fs;
use std::path::{Path, PathBuf};

use crate::common;
use common::{CliProcessFixture, strip_ansi};

const DOC: &str = "---\n$schema:\n  title: name@./types.yaml\ntitle: a long enough title\n---\nWrite the summary.\n";

/// The unedited imported file: a tagged table whose `name` type is required
/// and at least ten characters long. Required, so a rejected value blocks the
/// launch instead of being dropped with a warning.
const TYPES: &str = "kind: schema\ntypes:\n  name: string(required; min(10))\n";

struct Run {
    success: bool,
    /// ANSI-free stderr with whitespace and block gutters collapsed.
    stderr: String,
    launches: usize,
}

fn compose_with(name: &str, doc: &str, types: &[u8]) -> Run {
    let fixture = CliProcessFixture::named(name);
    fs::copy(
        biscuit_test_harness::bin_exe!("claudine-fake-goose"),
        fixture.bin_dir().join(format!("goose{}", std::env::consts::EXE_SUFFIX)),
    )
    .expect("the fake provider is built with `test-fixtures`");
    let marker: PathBuf = fixture.cwd().join("provider-launched.txt");
    let prompt = fixture.cwd().join("prompt.md");
    fs::write(&prompt, doc).unwrap();
    fs::write(fixture.cwd().join("types.yaml"), types).unwrap();

    let output = fixture
        .command()
        .env("PROVIDER_MARKER", &marker)
        .args(["compose", "--goose", prompt.to_str().unwrap()])
        .output()
        .unwrap();
    let stderr = strip_ansi(&String::from_utf8_lossy(&output.stderr))
        .replace(['┃', '│'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    Run { success: output.status.success(), stderr, launches: launches(&marker) }
}

fn launches(marker: &Path) -> usize {
    fs::read_to_string(marker).map_or(0, |log| log.lines().count())
}

#[test]
fn a_valid_named_type_import_launches_the_provider_once() {
    let run = compose_with("named-import-control", DOC, TYPES.as_bytes());

    assert!(run.success, "stderr: {}", run.stderr);
    assert_eq!(run.launches, 1, "stderr: {}", run.stderr);
}

/// The imported constraint is load-bearing: a value it rejects stops the
/// launch, so the control above passed because the import was applied.
#[test]
fn a_value_the_imported_type_rejects_stops_the_launch() {
    let doc = DOC.replace("a long enough title", "short");
    let run = compose_with("named-import-rejects", &doc, TYPES.as_bytes());

    assert!(!run.success, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("title"), "the report names the property: {}", run.stderr);
    assert_eq!(run.launches, 0, "stderr: {}", run.stderr);
}

#[test]
fn a_malformed_imported_file_is_a_located_error_and_launches_nothing() {
    let run = compose_with("named-import-malformed", DOC, b"kind: schema\ntypes: [\n");

    assert!(!run.success, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("types.yaml"), "the error names the imported file: {}", run.stderr);
    assert!(run.stderr.contains("not valid YAML"), "stderr: {}", run.stderr);
    assert_eq!(run.launches, 0, "stderr: {}", run.stderr);
}

#[test]
fn a_duplicate_key_in_the_imported_file_is_a_located_error_and_launches_nothing() {
    let run = compose_with(
        "named-import-duplicate",
        DOC,
        b"kind: schema\ntypes:\n  name: string(required; min(10))\n  name: string\n",
    );

    assert!(!run.success, "stderr: {}", run.stderr);
    assert!(run.stderr.contains("types.yaml"), "the error names the imported file: {}", run.stderr);
    assert!(run.stderr.contains("not valid YAML"), "stderr: {}", run.stderr);
    assert_eq!(run.launches, 0, "stderr: {}", run.stderr);
}

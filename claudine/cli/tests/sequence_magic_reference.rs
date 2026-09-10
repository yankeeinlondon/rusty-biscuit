#![cfg(unix)]

//! Integration tests for `claudine sequence` external-file and
//! magic-reference resolution.
//!
//! Split out of `sequence_cli.rs`: covers relative-path and `@magic`
//! reference resolution, source-doc-relative resolution, and clear
//! not-found errors.

use std::fs;
mod common;
use common::{CliProcessFixture, init_git_repo, strip_ansi, write, write_executable};

// ============================================================================
// External file references
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_resolves_external_file_via_relative_path() {
    let fixture = CliProcessFixture::named("sequence-magic-reference");
    let count_path = fixture.cwd().join("call-count.txt");

    // External YAML in the same directory as the source markdown.
    let steps_yaml = fixture.cwd().join("steps.yaml");
    fs::write(&steps_yaml, "sequence:\n  - alpha\n  - beta\n").unwrap();

    let md_file = fixture.cwd().join("seq.md");
    fs::write(&md_file, "---\nsequence: steps.yaml\n---\nStep {{state}}\n").unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 0
"#,
    );

    fixture
        .command()
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(calls.trim(), "2", "both YAML-defined steps should run");
}

#[cfg(unix)]
#[test]
fn sequence_resolves_external_file_via_magic_reference() {
    // Initialize a real git repo so FileReference's @ magic (driven by
    // git2::Repository::discover) can find the repo root.
    let fixture = CliProcessFixture::named("sequence-magic-reference");
    let repo_root = fixture.cwd().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    if !init_git_repo(&repo_root) {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let count_path = fixture.cwd().join("call-count.txt");

    // Magic reference target: @fixtures/steps.yaml resolves from git root.
    let fixtures_dir = repo_root.join("fixtures");
    fs::create_dir_all(&fixtures_dir).unwrap();
    fs::write(
        fixtures_dir.join("steps.yaml"),
        "sequence:\n  - one\n  - two\n  - three\n",
    )
    .unwrap();

    let md_dir = repo_root.join("prompts");
    fs::create_dir_all(&md_dir).unwrap();
    let md_file = md_dir.join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: '@fixtures/steps.yaml'\n---\nStep {{state}}\n",
    )
    .unwrap();

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 0
"#,
    );

    fixture
        .command_builder()
        // `@` resolves from the git root the launch context discovers, so the
        // launch directory is the repository this test built.
        .ambient_context(&repo_root)
        .build()
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "3",
        "@ magic reference should resolve to fixtures/steps.yaml and all 3 steps should run"
    );
}

#[cfg(unix)]
#[test]
fn sequence_magic_reference_uses_source_doc_location_not_cwd() {
    // A `@` magic reference in the sequence frontmatter MUST resolve
    // relative to the markdown source document's location, not the
    // process CWD. Otherwise a user running
    // `claudine sequence /abs/path/to/seq.md` from some other directory
    // would get the wrong file or a spurious "not found" error.
    //
    // This test sets up TWO distinct git repos: `repo_root/` (where the
    // source doc lives) and `unrelated/` (the process CWD). Each repo
    // has its own `fixtures/steps.yaml` with a different step count.
    // If resolution was driven by CWD, the wrong file would be loaded.
    let fixture = CliProcessFixture::named("sequence-magic-reference");

    // --- Primary repo: source doc + correct fixtures (2 steps) ---
    let repo_root = fixture.cwd().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    if !init_git_repo(&repo_root) {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let correct_fixtures = repo_root.join("fixtures");
    fs::create_dir_all(&correct_fixtures).unwrap();
    fs::write(
        correct_fixtures.join("steps.yaml"),
        "sequence:\n  - alpha\n  - beta\n",
    )
    .unwrap();
    let md_dir = repo_root.join("prompts");
    fs::create_dir_all(&md_dir).unwrap();
    let md_file = md_dir.join("seq.md");
    fs::write(
        &md_file,
        "---\nsequence: '@fixtures/steps.yaml'\n---\nStep {{state}}\n",
    )
    .unwrap();

    // --- Unrelated repo: decoy fixtures (3 steps) used iff resolution is CWD-driven ---
    let unrelated = fixture.cwd().join("unrelated");
    fs::create_dir_all(&unrelated).unwrap();
    init_git_repo(&unrelated);
    let decoy_fixtures = unrelated.join("fixtures");
    fs::create_dir_all(&decoy_fixtures).unwrap();
    fs::write(
        decoy_fixtures.join("steps.yaml"),
        "sequence:\n  - wrong1\n  - wrong2\n  - wrong3\n",
    )
    .unwrap();

    let count_path = fixture.cwd().join("call-count.txt");

    write_executable(
        &fixture.bin_dir().join("goose"),
        r#"#!/bin/sh
count=0
if [ -f "$CLAUDINE_COUNT_FILE" ]; then
  IFS= read -r count < "$CLAUDINE_COUNT_FILE"
fi
count=$((count + 1))
printf '%s' "$count" > "$CLAUDINE_COUNT_FILE"
exit 0
"#,
    );

    // Run FROM the unrelated repo, but target the doc inside repo_root.
    fixture
        .command_builder()
        // The decoy repository *is* the subject: the launch CWD must lose to
        // the source document's own location.
        .ambient_context(&unrelated)
        .build()
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .args(["sequence", "--goose", md_file.to_str().unwrap()])
        .assert()
        .success();

    let calls = fs::read_to_string(&count_path).unwrap();
    assert_eq!(
        calls.trim(),
        "2",
        "@ magic reference must resolve from the source document's location \
         (repo_root/fixtures/steps.yaml — 2 steps), not the process CWD \
         (unrelated/fixtures/steps.yaml — 3 steps). Got {calls} invocations."
    );
}

#[cfg(unix)]
#[test]
fn sequence_external_file_not_found_fails_clearly() {
    let fixture = CliProcessFixture::named("sequence-magic-reference");
    let md_file = fixture.cwd().join("seq.md");
    write(&md_file, "---\nsequence: does-not-exist.yaml\n---\nBody\n");

    let assert = fixture
        .command()
        .args(["sequence", md_file.to_str().unwrap()])
        .assert()
        .failure();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let plain = strip_ansi(&stderr);
    assert!(
        plain.contains("external sequence") || plain.contains("does-not-exist.yaml"),
        "error should mention the missing external file; stderr: {plain}"
    );
}

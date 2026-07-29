//! Integration tests for `claudine sequence` external-file and
//! magic-reference resolution.
//!
//! Split out of `sequence_cli.rs`: covers relative-path and `@magic`
//! reference resolution, source-doc-relative resolution, and clear
//! not-found errors.

use std::fs;
use tempfile::tempdir;
mod common;
use common::{augmented_path, strip_ansi, write_executable};

// ============================================================================
// External file references
// ============================================================================

#[cfg(unix)]
#[test]
fn sequence_resolves_external_file_via_relative_path() {
    let workspace = tempdir().unwrap();
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    // External YAML in the same directory as the source markdown.
    let steps_yaml = workspace.path().join("steps.yaml");
    fs::write(&steps_yaml, "sequence:\n  - alpha\n  - beta\n").unwrap();

    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, "---\nsequence: steps.yaml\n---\nStep {{state}}\n").unwrap();

    write_executable(
        &path_dir.join("goose"),
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(workspace.path())
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
    let workspace = tempdir().unwrap();
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    let git_init_ok = std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !git_init_ok {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

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
        &path_dir.join("goose"),
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

    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(&repo_root)
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
    let workspace = tempdir().unwrap();

    // --- Primary repo: source doc + correct fixtures (2 steps) ---
    let repo_root = workspace.path().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    let git_init_ok = std::process::Command::new("git")
        .arg("init")
        .current_dir(&repo_root)
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if !git_init_ok {
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
    let unrelated = workspace.path().join("unrelated");
    fs::create_dir_all(&unrelated).unwrap();
    std::process::Command::new("git")
        .arg("init")
        .current_dir(&unrelated)
        .status()
        .ok();
    let decoy_fixtures = unrelated.join("fixtures");
    fs::create_dir_all(&decoy_fixtures).unwrap();
    fs::write(
        decoy_fixtures.join("steps.yaml"),
        "sequence:\n  - wrong1\n  - wrong2\n  - wrong3\n",
    )
    .unwrap();

    let path_dir = workspace.path().join("bin");
    fs::create_dir_all(&path_dir).unwrap();
    let count_path = workspace.path().join("call-count.txt");

    write_executable(
        &path_dir.join("goose"),
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
    assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
        .env("HOME", workspace.path())
        .env("PATH", augmented_path(&path_dir))
        .env("CLAUDINE_COUNT_FILE", &count_path)
        .current_dir(&unrelated)
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
    let workspace = tempdir().unwrap();
    let md_file = workspace.path().join("seq.md");
    fs::write(&md_file, "---\nsequence: does-not-exist.yaml\n---\nBody\n").unwrap();

    let assert = assert_cmd::Command::cargo_bin("claudine").unwrap()
        .env("NO_COLOR", "1")
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

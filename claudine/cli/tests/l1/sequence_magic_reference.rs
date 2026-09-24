//! Integration tests for `claudine sequence` external-file and
//! magic-reference resolution.
//!
//! Split out of `sequence_cli.rs`: covers relative-path and `@magic`
//! reference resolution, source-doc-relative resolution, and clear
//! not-found errors.

use std::fs;
use crate::common;
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
fn sequence_magic_reference_follows_launch_scope_and_relative_stays_source_anchored() {
    // Ruling 2 of 2026-09-23-local-before-home: a nested `@` reference keeps
    // the invocation's launch tree as its local root, while explicit-relative
    // references stay anchored on the source document. Launching from
    // `unrelated/` with the document in `repo_root/`, `@fixtures/steps.yaml`
    // resolves against the launch tree (`unrelated/fixtures`), and
    // `./fixtures/steps.yaml` resolves beside the document
    // (`repo_root/prompts/fixtures`) regardless of the process CWD.
    //
    // This test sets up TWO distinct git repos: `repo_root/` (where the
    // source doc lives) and `unrelated/` (the process CWD), each carrying
    // its own fixtures with a distinguishable step count.
    let fixture = CliProcessFixture::named("sequence-magic-reference");

    // --- Primary repo: source doc + source-relative fixtures (1 step) ---
    let repo_root = fixture.cwd().join("repo");
    fs::create_dir_all(&repo_root).unwrap();
    if !init_git_repo(&repo_root) {
        eprintln!("git init unavailable; skipping magic reference test");
        return;
    }
    let md_dir = repo_root.join("prompts");
    let relative_fixtures = md_dir.join("fixtures");
    fs::create_dir_all(&relative_fixtures).unwrap();
    fs::write(
        relative_fixtures.join("steps.yaml"),
        "sequence:\n  - beside-doc\n",
    )
    .unwrap();
    let magic_md = md_dir.join("seq-magic.md");
    fs::write(
        &magic_md,
        "---\nsequence: '@fixtures/steps.yaml'\n---\nStep {{state}}\n",
    )
    .unwrap();
    let relative_md = md_dir.join("seq-relative.md");
    fs::write(
        &relative_md,
        "---\nsequence: './fixtures/steps.yaml'\n---\nStep {{state}}\n",
    )
    .unwrap();

    // --- Unrelated repo: launch-tree fixtures (3 steps), searched by `@` ---
    let unrelated = fixture.cwd().join("unrelated");
    fs::create_dir_all(&unrelated).unwrap();
    init_git_repo(&unrelated);
    let launch_fixtures = unrelated.join("fixtures");
    fs::create_dir_all(&launch_fixtures).unwrap();
    fs::write(
        launch_fixtures.join("steps.yaml"),
        "sequence:\n  - launch1\n  - launch2\n  - launch3\n",
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

    // `@` resolves from the launch tree: the launch repository's fixtures
    // are the intended winner, not a decoy.
    let magic_count = fixture.cwd().join("magic-count.txt");
    fixture
        .command_builder()
        .ambient_context(&unrelated)
        .build()
        .env("CLAUDINE_COUNT_FILE", &magic_count)
        .args(["sequence", "--goose", magic_md.to_str().unwrap()])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(&magic_count).unwrap().trim(),
        "3",
        "nested `@` must resolve from the launch tree \
         (unrelated/fixtures/steps.yaml — 3 steps)."
    );

    // Explicit-relative resolves beside the source document, not the CWD.
    let relative_count = fixture.cwd().join("relative-count.txt");
    fixture
        .command_builder()
        .ambient_context(&unrelated)
        .build()
        .env("CLAUDINE_COUNT_FILE", &relative_count)
        .args(["sequence", "--goose", relative_md.to_str().unwrap()])
        .assert()
        .success();
    assert_eq!(
        fs::read_to_string(&relative_count).unwrap().trim(),
        "1",
        "explicit-relative must resolve from the source document's directory \
         (repo_root/prompts/fixtures/steps.yaml — 1 step), not the process CWD."
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

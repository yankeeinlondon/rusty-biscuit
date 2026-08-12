//! Integration tests for `claudine sequence <TAB>` positional completion.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. The
//! `sequence` mode is unique in accepting both markdown (`.md` /
//! `.markdown`) **and** YAML (`.yaml` / `.yml`) files; the root-level
//! document must carry a `sequence` key. Scope set mirrors `inline-compose`:
//! in addition to prompt directories, `<repo>/docs/` and each agent peer's
//! `skills/` tree are walked.

use std::fs;

mod common;
use common::TestWorkspace;
use common::completion::{
    fake_home, run_complete, run_complete_with_home, seed_cargo_workspace, write_file,
};

// ---------------------------------------------------------------------
// sequence accepts markdown with `sequence:` frontmatter
// ---------------------------------------------------------------------

#[test]
fn sequence_surfaces_markdown_with_sequence_key() {
    let ws = TestWorkspace::named("complete-sequence-md");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(
        &prompts.join("steps.md"),
        "---\nsequence:\n  - a\n  - b\n---\n",
    );
    write_file(&prompts.join("other.md"), "---\ntitle: X\n---\nBody\n");

    let got = run_complete(ws.path(), &["sequence", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/steps.md"),
        "sequence must surface steps.md with sequence key: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("other.md")),
        "sequence must not surface files without sequence key: {got:?}"
    );
}

// ---------------------------------------------------------------------
// sequence accepts YAML files with top-level `sequence:`
// ---------------------------------------------------------------------

#[test]
fn sequence_surfaces_yaml_files() {
    let ws = TestWorkspace::named("complete-sequence-yaml");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("steps.yaml"), "sequence:\n  - one\n  - two\n");
    write_file(&prompts.join("steps.yml"), "sequence:\n  - one\n");
    write_file(&prompts.join("other.yaml"), "other:\n  - x\n");

    let got = run_complete(ws.path(), &["sequence", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/steps.yaml"),
        "sequence must surface .yaml files: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/steps.yml"),
        "sequence must surface .yml files: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("other.yaml")),
        "sequence must not surface YAML without top-level sequence key: {got:?}"
    );
}

// ---------------------------------------------------------------------
// sequence — docs/ extras
// ---------------------------------------------------------------------

#[test]
fn sequence_surfaces_docs_directory() {
    let ws = TestWorkspace::named("complete-sequence-docs");
    seed_cargo_workspace(ws.path());
    write_file(
        &ws.path().join("docs").join("runbook.md"),
        "---\nsequence:\n  - a\n---\n",
    );

    let got = run_complete(ws.path(), &["sequence", ""]);
    assert!(
        got.iter().any(|c| c.ends_with("runbook.md")),
        "sequence must include docs/ extras: {got:?}"
    );
}

// ---------------------------------------------------------------------
// sequence — magic path resolves
// ---------------------------------------------------------------------

#[test]
fn sequence_magic_resolves_relative() {
    let ws = TestWorkspace::named("complete-sequence-magic");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("deploy.md"), "---\nsequence:\n  - a\n---\n");

    let got = run_complete(ws.path(), &["sequence", "@dep"]);
    assert!(
        got.iter().any(|c| c == "@deploy.md"),
        "@ magic must keep sigil and render filename: {got:?}"
    );
    assert!(
        got.iter().all(|c| c.starts_with('@') && !c.contains('/')),
        "every magic candidate must be `@<basename>`: {got:?}"
    );
}

// ---------------------------------------------------------------------
// sequence prefix progression
// ---------------------------------------------------------------------

#[test]
fn sequence_long_prefix_includes_directories() {
    let ws = TestWorkspace::named("complete-sequence-dirs");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("rollout.md"), "---\nsequence:\n  - a\n---\n");
    fs::create_dir_all(prompts.join("rollouts")).unwrap();

    let got = run_complete(ws.path(), &["sequence", "rol"]);
    assert!(
        got.iter().any(|c| c == "prompts/rollouts/"),
        "3+ char partial must surface directories: {got:?}"
    );
}

// ---------------------------------------------------------------------
// sequence magic priority (finding #5): repo extras (docs/) outrank
// user-global prompts in the magic-scope ordering.
// ---------------------------------------------------------------------

#[test]
fn sequence_magic_dedups_duplicate_basename() {
    // The same sequence filename in repo `docs/` and `~/.claudine/prompts/`
    // collapses to a single `@deploy.md`; runtime resolves the closest.
    let ws = TestWorkspace::named("complete-sequence-magic-priority");
    seed_cargo_workspace(ws.path());

    // Repo-local match under docs/.
    write_file(
        &ws.path().join("docs").join("deploy.md"),
        "---\nsequence:\n  - repo\n---\n",
    );

    // User-global match under ~/.claudine/prompts/.
    let home = fake_home(ws.path());
    let user_prompts = home.join(".claudine").join("prompts");
    write_file(
        &user_prompts.join("deploy.md"),
        "---\nsequence:\n  - user\n---\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["sequence", "@deploy"]);

    assert_eq!(
        got.iter().filter(|c| *c == "@deploy.md").count(),
        1,
        "duplicate basename across tiers must collapse to one `@deploy.md`: {got:?}"
    );
    assert!(
        got.iter().all(|c| c.starts_with('@') && !c.contains('/')),
        "every magic candidate must be `@<basename>`: {got:?}"
    );
}

// ---------------------------------------------------------------------
// repo-wide directory walk (review-1 findings #2 and #3)
// ---------------------------------------------------------------------

#[test]
fn sequence_one_char_prefix_surfaces_repo_dirs() {
    // Finding #2 and #3: 1-char prefix surfaces directories via the
    // repo-wide walk for `sequence` mode too. The repo-wide walk is
    // mode-agnostic.
    let ws = TestWorkspace::named("complete-sequence-one-char-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("claudine")).unwrap();

    let got = run_complete(ws.path(), &["sequence", "c"]);
    assert!(
        got.iter().any(|c| c == "claudine/"),
        "sequence 1-char prefix must surface `claudine/`: {got:?}"
    );
}

#[test]
fn sequence_magic_surfaces_no_dirs() {
    // Filename-magic contract: `sequence @<short><TAB>` never surfaces a
    // directory — magic mode is a filename search.
    let ws = TestWorkspace::named("complete-sequence-magic-short-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("claudine")).unwrap();

    let got = run_complete(ws.path(), &["sequence", "@cl"]);
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "sequence magic must not surface directories: {got:?}"
    );
}

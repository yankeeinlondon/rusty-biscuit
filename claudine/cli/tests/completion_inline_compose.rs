//! Integration tests for `claudine inline-compose <TAB>` positional
//! completion.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. The
//! `inline-compose` mode differs from `compose` in two ways:
//!
//! - Files must carry a **non-empty** `prompt:` frontmatter key. Plain
//!   markdown without the key is not a candidate.
//! - Scope adds two mode-specific extras: `<repo>/docs/` and each agent
//!   peer directory's `skills/` tree (`.claude/skills/`, `.codex/skills/`,
//!   etc.), with symlinks disabled for skills.

use std::fs;

mod common;
use common::TestWorkspace;
use common::completion::{
    fake_home, run_complete, run_complete_with_home, seed_cargo_workspace, write_file,
};

// ---------------------------------------------------------------------
// inline-compose requires non-empty prompt:
// ---------------------------------------------------------------------

#[test]
fn inline_compose_surfaces_files_with_prompt_key() {
    let ws = TestWorkspace::named("complete-inline-has-prompt");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(
        &prompts.join("inline.md"),
        "---\nprompt: Write a poem\n---\nBody\n",
    );
    write_file(&prompts.join("plain.md"), "---\ntitle: X\n---\nBody\n");

    let got = run_complete(ws.path(), &["inline-compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/inline.md"),
        "inline-compose must surface inline.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("plain.md")),
        "inline-compose must NOT surface files without prompt: {got:?}"
    );
}

#[test]
fn inline_compose_rejects_empty_and_whitespace_prompts() {
    let ws = TestWorkspace::named("complete-inline-empty-prompt");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("empty.md"), "---\nprompt: \"\"\n---\n");
    write_file(&prompts.join("ws.md"), "---\nprompt: \"   \"\n---\n");

    let got = run_complete(ws.path(), &["inline-compose", ""]);
    assert!(
        !got.iter().any(|c| c.ends_with("empty.md")),
        "empty prompt must not surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("ws.md")),
        "whitespace-only prompt must not surface: {got:?}"
    );
}

// ---------------------------------------------------------------------
// inline-compose adds docs/ extras
// ---------------------------------------------------------------------

#[test]
fn inline_compose_surfaces_docs_directory() {
    let ws = TestWorkspace::named("complete-inline-docs");
    seed_cargo_workspace(ws.path());
    write_file(
        &ws.path().join("docs").join("spec.md"),
        "---\nprompt: Describe\n---\nBody\n",
    );

    let got = run_complete(ws.path(), &["inline-compose", ""]);
    assert!(
        got.iter().any(|c| c.ends_with("spec.md")),
        "inline-compose must include docs/ extras: {got:?}"
    );
}

// ---------------------------------------------------------------------
// inline-compose magic paths
// ---------------------------------------------------------------------

#[test]
fn inline_compose_magic_resolves_relative() {
    let ws = TestWorkspace::named("complete-inline-magic");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(
        &prompts.join("writer.md"),
        "---\nprompt: Write stuff\n---\n",
    );

    let got = run_complete(ws.path(), &["inline-compose", "@writ"]);
    assert!(
        got.iter().any(|c| c == "@writer.md"),
        "@ magic must keep `@` and render the filename only: {got:?}"
    );
}

// ---------------------------------------------------------------------
// inline-compose prefix progression
// ---------------------------------------------------------------------

#[test]
fn inline_compose_long_prefix_surfaces_directories() {
    let ws = TestWorkspace::named("complete-inline-dirs");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("planner.md"), "---\nprompt: Plan\n---\n");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let got = run_complete(ws.path(), &["inline-compose", "pla"]);
    assert!(
        got.iter().any(|c| c == "prompts/planning/"),
        "inline-compose 3+ char partial must include dirs: {got:?}"
    );
}

// ---------------------------------------------------------------------
// inline-compose magic priority (finding #5): repo extras (docs/)
// outrank user-global prompts in the magic-scope ordering.
// ---------------------------------------------------------------------

#[test]
fn inline_compose_magic_dedups_duplicate_basename() {
    // The same filename in repo `docs/` and `~/.claudine/prompts/` collapses
    // to a single `@plan.md`; runtime resolves the closest.
    let ws = TestWorkspace::named("complete-inline-magic-priority");
    seed_cargo_workspace(ws.path());

    // Repo-local match under docs/.
    write_file(
        &ws.path().join("docs").join("plan.md"),
        "---\nprompt: Repo docs plan\n---\nBody\n",
    );

    // User-global match under ~/.claudine/prompts/.
    let home = fake_home(ws.path());
    let user_prompts = home.join(".claudine").join("prompts");
    write_file(
        &user_prompts.join("plan.md"),
        "---\nprompt: User-global plan\n---\nBody\n",
    );

    let got = run_complete_with_home(ws.path(), &home, &["inline-compose", "@plan"]);

    assert_eq!(
        got.iter().filter(|c| *c == "@plan.md").count(),
        1,
        "duplicate basename across tiers must collapse to one `@plan.md`: {got:?}"
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
fn inline_compose_one_char_prefix_surfaces_repo_dirs() {
    // Finding #2 and #3: 1-char prefix surfaces directories via the
    // repo-wide walk for `inline-compose` mode too. The repo-wide walk
    // is mode-agnostic.
    let ws = TestWorkspace::named("complete-inline-one-char-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("claudine")).unwrap();

    let got = run_complete(ws.path(), &["inline-compose", "c"]);
    assert!(
        got.iter().any(|c| c == "claudine/"),
        "inline-compose 1-char prefix must surface `claudine/`: {got:?}"
    );
}

#[test]
fn inline_compose_magic_surfaces_no_dirs() {
    // Filename-magic contract: `inline-compose @<short><TAB>` never surfaces
    // a directory — magic mode is a filename search.
    let ws = TestWorkspace::named("complete-inline-magic-short-dirs");
    seed_cargo_workspace(ws.path());
    fs::create_dir_all(ws.path().join("claudine")).unwrap();

    let got = run_complete(ws.path(), &["inline-compose", "@cl"]);
    assert!(
        !got.iter().any(|c| c.ends_with('/')),
        "inline-compose magic must not surface directories: {got:?}"
    );
}

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
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::TestWorkspace;

fn seed_cargo_workspace(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::write(
        root.join("Cargo.toml"),
        "[workspace]\nresolver = \"2\"\nmembers = [\"pkg\"]\n",
    )
    .unwrap();
    let pkg = root.join("pkg");
    fs::create_dir_all(pkg.join("src")).unwrap();
    fs::write(
        pkg.join("Cargo.toml"),
        "[package]\nname = \"pkg\"\nversion = \"0.0.0\"\nedition = \"2021\"\n",
    )
    .unwrap();
    fs::write(pkg.join("src").join("lib.rs"), "").unwrap();
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

fn fake_home(cwd: &Path) -> PathBuf {
    let parent = cwd.parent().unwrap_or(cwd);
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    let leaf = cwd
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("claudine-home");
    let home = parent.join(format!("{leaf}-home-{nonce}-{}", std::process::id()));
    fs::create_dir_all(&home).expect("create fake home");
    home
}

fn run_complete(cwd: &Path, argv_tail: &[&str]) -> Vec<String> {
    let home = fake_home(cwd);
    run_complete_with_home(cwd, &home, argv_tail)
}

fn run_complete_with_home(cwd: &Path, home: &Path, argv_tail: &[&str]) -> Vec<String> {
    let current = argv_tail.len();
    let reference = cargo_bin_cmd!("claudine");
    let program = reference.get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd)
        .env("HOME", home)
        .env("NO_COLOR", "1")
        .env_remove("COMPLETE")
        .env_remove("_CLAP_COMPLETE_INDEX")
        .env_remove("_CLAP_IFS")
        .arg("__complete")
        .arg("--current")
        .arg(current.to_string())
        .arg("--")
        .arg("claudine");
    for arg in argv_tail {
        cmd.arg(arg);
    }
    let output = cmd.output().expect("completion subprocess to run");
    assert!(
        output.status.success(),
        "completion subprocess failed: stderr={}",
        String::from_utf8_lossy(&output.stderr),
    );
    String::from_utf8(output.stdout)
        .expect("utf-8")
        .split('\n')
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}

// ---------------------------------------------------------------------
// inline-compose requires non-empty prompt:
// ---------------------------------------------------------------------

#[test]
fn inline_compose_surfaces_files_with_prompt_key() {
    let ws = TestWorkspace::named("complete-inline-has-prompt");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("inline.md"), "---\nprompt: Write a poem\n---\nBody\n");
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
    write_file(&prompts.join("writer.md"), "---\nprompt: Write stuff\n---\n");

    let got = run_complete(ws.path(), &["inline-compose", "@writ"]);
    assert!(
        got.iter().any(|c| c == "prompts/writer.md"),
        "@ magic must resolve to relative path: {got:?}"
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
fn inline_compose_magic_prefers_repo_docs_over_user_global() {
    // Finding #5: the magic-scope iterator orders repo-local extras
    // (`docs/`, skills) before `user_claudine`, so a match in repo
    // `docs/` sorts before a match in `~/.claudine/prompts/`.
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

    let repo_pos = got.iter().position(|c| c.ends_with("docs/plan.md"));
    let user_pos = got
        .iter()
        .position(|c| c.contains(".claudine/prompts/plan.md"));
    assert!(
        repo_pos.is_some(),
        "repo-local docs/plan.md must appear: {got:?}"
    );
    assert!(
        user_pos.is_some(),
        "user-global plan.md must appear: {got:?}"
    );
    assert!(
        repo_pos < user_pos,
        "repo-local docs/plan.md ({repo_pos:?}) must sort before user-global ({user_pos:?}): {got:?}"
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

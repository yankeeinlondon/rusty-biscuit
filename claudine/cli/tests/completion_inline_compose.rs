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
    let current = argv_tail.len();
    let reference = cargo_bin_cmd!("claudine");
    let program = reference.get_program().to_os_string();
    let mut cmd = Command::new(program);
    cmd.current_dir(cwd)
        .env("HOME", &home)
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

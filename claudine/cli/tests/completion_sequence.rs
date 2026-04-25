//! Integration tests for `claudine sequence <TAB>` positional completion.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. The
//! `sequence` mode is unique in accepting both markdown (`.md` /
//! `.markdown`) **and** YAML (`.yaml` / `.yml`) files; the root-level
//! document must carry a `sequence` key. Scope set mirrors `inline-compose`:
//! in addition to prompt directories, `<repo>/docs/` and each agent peer's
//! `skills/` tree are walked.

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
// sequence accepts markdown with `sequence:` frontmatter
// ---------------------------------------------------------------------

#[test]
fn sequence_surfaces_markdown_with_sequence_key() {
    let ws = TestWorkspace::named("complete-sequence-md");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("steps.md"), "---\nsequence:\n  - a\n  - b\n---\n");
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
        got.iter().any(|c| c == "prompts/deploy.md"),
        "@ magic must strip sigil on selection: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.starts_with('@')),
        "no @-prefixed candidate should be emitted: {got:?}"
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

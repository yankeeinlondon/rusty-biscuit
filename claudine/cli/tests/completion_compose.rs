//! Integration tests for `claudine compose <TAB>` positional completion.
//!
//! Phase 3 of the `2026-04-24-improved-shell-completions` feature. These
//! tests drive the compiled `claudine` binary's hidden `__complete`
//! subcommand against seeded temp-directory fixtures and assert that the
//! composition completer's contract holds end-to-end:
//!
//! - `.md` files whose frontmatter **does not** carry a `prompt` key
//!   surface in compose mode.
//! - `inline-compose` targets (files with a `prompt` frontmatter key) do
//!   **not** surface.
//! - `@`-prefixed partials resolve to relative paths (the `@` sigil is
//!   stripped on selection).
//! - Prefix-length progression: 0 chars → no directories; 3+ chars →
//!   directories are surfaced with a trailing `/`.
//! - `docs/` and `.claude/skills/` — inline-compose-only extras — do NOT
//!   surface in compose mode.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use assert_cmd::cargo::cargo_bin_cmd;

mod common;
use common::TestWorkspace;

/// Seed a minimal Cargo workspace so `sniff::detect_repo_structure`
/// recognizes the tempdir as a monorepo. A plain `.git` is not enough for
/// the composition completer because the scope resolver uses `sniff` for
/// monorepo shape; the git-root fallback only covers non-workspace scopes.
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
        "completion subprocess failed: status={:?}, stderr={}",
        output.status,
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
// compose empty partial — files only, no prompt-keyed files
// ---------------------------------------------------------------------

#[test]
fn compose_empty_partial_surfaces_plain_markdown() {
    let ws = TestWorkspace::named("complete-compose-empty");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "---\ntitle: Plan\n---\nBody\n");
    write_file(&prompts.join("notes.md"), "# plain body only\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "compose empty partial must surface plan.md: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/notes.md"),
        "compose empty partial must surface notes.md: {got:?}"
    );
}

#[test]
fn compose_empty_partial_skips_prompt_frontmatter_files() {
    let ws = TestWorkspace::named("complete-compose-skip-prompt");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plain.md"), "# plain\n");
    write_file(
        &prompts.join("inline.md"),
        "---\nprompt: Say hi\n---\nBody\n",
    );

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/plain.md"),
        "compose must surface plain.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("inline.md")),
        "compose must NOT surface files with `prompt:` frontmatter: {got:?}"
    );
}

#[test]
fn compose_empty_partial_does_not_include_directories() {
    let ws = TestWorkspace::named("complete-compose-no-dirs-empty");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    fs::create_dir_all(prompts.join("subdir")).unwrap();
    write_file(&prompts.join("subdir").join("inner.md"), "# inner\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c == "prompts/subdir/"),
        "empty partial must not surface directories: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose — docs/ and skills/ are NOT in scope for compose
// ---------------------------------------------------------------------

#[test]
fn compose_does_not_surface_docs_or_skills() {
    let ws = TestWorkspace::named("complete-compose-no-extras");
    seed_cargo_workspace(ws.path());
    write_file(&ws.path().join("docs").join("guide.md"), "# g\n");
    write_file(
        &ws.path().join(".claude").join("skills").join("expert.md"),
        "# ex\n",
    );

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c.contains("docs/guide.md")),
        "compose must NOT surface docs/: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("expert.md")),
        "compose must NOT surface .claude/skills/: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose prefix-length progression
// ---------------------------------------------------------------------

#[test]
fn compose_short_prefix_matches_filenames_no_dirs() {
    let ws = TestWorkspace::named("complete-compose-short");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let got = run_complete(ws.path(), &["compose", "pl"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "short prefix must match plan.md: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("planning/")),
        "short prefix must NOT include directories: {got:?}"
    );
}

#[test]
fn compose_long_prefix_includes_directories() {
    let ws = TestWorkspace::named("complete-compose-long");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");
    fs::create_dir_all(prompts.join("planning")).unwrap();

    let got = run_complete(ws.path(), &["compose", "pla"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "long prefix must still match files: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/planning/"),
        "long prefix must include directories with trailing `/`: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose committed-directory navigation
// ---------------------------------------------------------------------

#[test]
fn compose_committed_dir_walks_inside_only() {
    let ws = TestWorkspace::named("complete-compose-committed");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("outer.md"), "# o\n");
    write_file(&prompts.join("planning").join("deep.md"), "# d\n");

    let got = run_complete(ws.path(), &["compose", "prompts/planning/"]);
    assert!(
        got.iter().any(|c| c == "prompts/planning/deep.md"),
        "committed dir must surface inside: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.ends_with("outer.md")),
        "committed dir must not leak parent: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose @ magic-path resolution
// ---------------------------------------------------------------------

#[test]
fn compose_magic_path_strips_sigil_and_renders_relative() {
    let ws = TestWorkspace::named("complete-compose-magic");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("plan.md"), "# p\n");

    let got = run_complete(ws.path(), &["compose", "@plan"]);
    assert!(
        got.iter().any(|c| c == "prompts/plan.md"),
        "@ sigil must strip on selection; got: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.starts_with('@')),
        "no @-prefixed candidates should be emitted: {got:?}"
    );
}

// ---------------------------------------------------------------------
// compose underscore and gitignore filters
// ---------------------------------------------------------------------

#[test]
fn compose_hides_underscore_prefixed_files() {
    let ws = TestWorkspace::named("complete-compose-underscore");
    seed_cargo_workspace(ws.path());
    let prompts = ws.path().join("prompts");
    write_file(&prompts.join("_wip.md"), "# w\n");
    write_file(&prompts.join("ok.md"), "# o\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        !got.iter().any(|c| c.contains("_wip.md")),
        "underscore-prefixed file must be hidden: {got:?}"
    );
    assert!(
        got.iter().any(|c| c == "prompts/ok.md"),
        "non-underscore file must surface: {got:?}"
    );
}

#[test]
fn compose_honors_gitignore_at_nested_depth() {
    let ws = TestWorkspace::named("complete-compose-gitignore");
    seed_cargo_workspace(ws.path());
    write_file(&ws.path().join(".gitignore"), "prompts/hidden/\n");
    write_file(
        &ws.path().join("prompts").join("hidden").join("bad.md"),
        "# b\n",
    );
    write_file(&ws.path().join("prompts").join("good.md"), "# g\n");

    let got = run_complete(ws.path(), &["compose", ""]);
    assert!(
        got.iter().any(|c| c == "prompts/good.md"),
        "non-ignored file must surface: {got:?}"
    );
    assert!(
        !got.iter().any(|c| c.contains("bad.md")),
        "gitignored file must not surface: {got:?}"
    );
}

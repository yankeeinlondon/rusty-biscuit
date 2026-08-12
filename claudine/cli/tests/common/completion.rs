//! Shared helpers for the `completion_*` integration tests.
//!
//! The six `cli/tests/completion_*.rs` files all drive the compiled
//! `claudine __complete` subcommand against seeded temp-directory fixtures
//! and parse the candidate list out of stdout. Before this module existed,
//! every test file carried a copy of `seed_cargo_workspace`, `write_file`,
//! `fake_home`, and `run_complete*`, which silently drifted.
//!
//! These helpers are lifted **verbatim** from the canonical
//! `completion_compose.rs` source — same edition, same env-removal set,
//! same stdout parsing. New helpers belong in their own future submodules
//! when they are actually needed.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

/// Seed a minimal Cargo workspace with a single `pkg` member.
///
/// `sniff::detect_repo_structure` recognizes the tempdir as a monorepo
/// only when a workspace `Cargo.toml` is present alongside the `.git`
/// marker. A plain `.git` is not enough for the composition completer
/// because the scope resolver needs the workspace shape; the git-root
/// fallback only covers non-workspace scopes.
pub(crate) fn seed_cargo_workspace(root: &Path) {
    seed_cargo_workspace_members(root, &["pkg"]);
}

/// Seed a `.git` marker without a Cargo workspace.
///
/// Use this when a test wants the plain-git fallback path
/// (`effective_repo_root`) instead of the `sniff` workspace path.
pub(crate) fn seed_plain_git_repo(root: &Path) {
    fs::create_dir_all(root.join(".git")).unwrap();
}

/// Seed a Cargo workspace with explicit members.
///
/// Each `member` is created as a placeholder package with a minimal
/// `Cargo.toml` and an empty `src/lib.rs`. Slashes in member paths are
/// flattened into the package `name` so nested members like
/// `claudine/lib` produce a valid `claudine-lib` package.
pub(crate) fn seed_cargo_workspace_members(root: &Path, members: &[&str]) {
    fs::create_dir_all(root.join(".git")).unwrap();
    let members_list = members
        .iter()
        .map(|member| format!("\"{member}\""))
        .collect::<Vec<_>>()
        .join(", ");
    fs::write(
        root.join("Cargo.toml"),
        format!("[workspace]\nresolver = \"2\"\nmembers = [{members_list}]\n"),
    )
    .unwrap();
    for member in members {
        let pkg = root.join(member);
        fs::create_dir_all(pkg.join("src")).unwrap();
        let name = member.replace('/', "-");
        fs::write(
            pkg.join("Cargo.toml"),
            format!("[package]\nname = \"{name}\"\nversion = \"0.0.0\"\nedition = \"2021\"\n"),
        )
        .unwrap();
        fs::write(pkg.join("src").join("lib.rs"), "").unwrap();
    }
}

/// Create the parent directory tree (if needed) and write `content` to
/// `path`.
pub(crate) fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, content).unwrap();
}

/// Build a hermetic fake-home directory next to (not inside) `cwd`.
///
/// The completion engine walks `$HOME/prompts`, `$HOME/sequences`,
/// `$HOME/.claudine/prompts`, and `$HOME/.claudine/sequences` as part of
/// the curated scope. Without this sandbox, tests would walk the
/// developer's real `~/.claudine` tree and surface unrelated candidates.
pub(crate) fn fake_home(cwd: &Path) -> PathBuf {
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

/// Invoke `claudine __complete` as a subprocess and collect stdout lines.
///
/// `argv_tail` is the argv as the user would have typed it, excluding the
/// binary name. The cursor sits at the trailing element. A fresh fake home
/// is created next to `cwd` for the duration of the call.
pub(crate) fn run_complete(cwd: &Path, argv_tail: &[&str]) -> Vec<String> {
    let home = fake_home(cwd);
    run_complete_with_home(cwd, &home, argv_tail)
}

/// Invoke `claudine __complete` with an explicit `HOME`.
///
/// Use when a test needs to seed `$HOME/.claudine/...` fixtures and assert
/// the user-global scope is rendered home-relative.
pub(crate) fn run_complete_with_home(cwd: &Path, home: &Path, argv_tail: &[&str]) -> Vec<String> {
    let current = argv_tail.len();
    let reference = assert_cmd::Command::cargo_bin("claudine").unwrap();
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

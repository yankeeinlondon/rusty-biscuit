//! Pure-std + git2 fixture builder functions.
//!
//! These functions materialize deterministic workloads into a
//! caller-supplied directory. No `TempDir` involvement here so the same
//! builder logic can be reused across Criterion benches, example
//! profiling binaries, and integration tests without dragging
//! `tempfile` into example/test compilation units that do not need it.
//!
//! The module is intentionally `pub` at every item so consumers that
//! include it via `#[path = "..."]` can call the builders directly.

#![allow(dead_code)]

use git2::{IndexAddOption, Repository, Signature};
use std::fs;
use std::path::Path;

/// Number of Rust packages in the synthetic `large_monorepo` fixture.
pub const LARGE_MONOREPO_RUST_PKGS: usize = 60;

/// Number of JavaScript packages in the synthetic `large_monorepo` fixture.
pub const LARGE_MONOREPO_JS_PKGS: usize = 30;

/// Number of churn commits applied on top of the initial monorepo commit.
pub const LARGE_MONOREPO_CHURN_COMMITS: u32 = 20;

/// Total commits the `large_monorepo` fixture should contain: one
/// initial layout commit plus every churn commit.
pub const LARGE_MONOREPO_TOTAL_COMMITS: u32 = LARGE_MONOREPO_CHURN_COMMITS + 1;

/// Number of dirty working-tree files left behind by the `large_monorepo`
/// builder so git status always has work to do.
pub const LARGE_MONOREPO_DIRTY_FILES: usize = 2;

/// Commits in the `small_git_repo` fixture.
pub const SMALL_GIT_REPO_COMMITS: u32 = 5;

/// Dirty files in the `small_git_repo` fixture.
pub const SMALL_GIT_REPO_DIRTY_FILES: usize = 2;

/// Write a file, creating parent directories as needed.
pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, contents).expect("write fixture file");
}

/// Stage everything and create a commit using a deterministic signature.
pub fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().expect("load repo index");
    index
        .add_all(["."].iter(), IndexAddOption::DEFAULT, None)
        .expect("stage files");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("resolve tree");
    let sig = Signature::new("Bench Runner", "bench@sniff.test", &git2::Time::new(0, 0))
        .expect("build signature");
    let parent_commit = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    let parents: Vec<&git2::Commit> = parent_commit.as_ref().into_iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .expect("create commit");
}

/// Build a small ~10-file git repository rooted at `root` with a
/// handful of commits and a couple of dirty working-tree files.
///
/// Returns the opened `Repository` handle.
pub fn build_small_git_repo(root: &Path) -> Repository {
    let repo = Repository::init(root).expect("init small git repo");

    write_file(&root.join("README.md"), "# small repo\n");
    write_file(&root.join(".gitignore"), "target/\n");
    write_file(
        &root.join("Cargo.toml"),
        "[package]\nname = \"small\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write_file(&root.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n");
    write_file(&root.join("src/main.rs"), "fn main() {}\n");
    write_file(&root.join("tests/basic.rs"), "#[test] fn ok() {}\n");
    write_file(&root.join("docs/intro.md"), "# intro\n");
    write_file(&root.join("docs/usage.md"), "# usage\n");
    write_file(&root.join("LICENSE"), "All rights reserved.\n");
    write_file(&root.join("CHANGELOG.md"), "# changelog\n");
    commit_all(&repo, "c1: initial");

    write_file(&root.join("src/lib.rs"), "pub fn one() -> u32 { 10 }\n");
    commit_all(&repo, "c2: bump one");

    write_file(
        &root.join("src/mod_a.rs"),
        "pub fn a() -> &'static str { \"a\" }\n",
    );
    commit_all(&repo, "c3: add mod_a");

    write_file(&root.join("docs/usage.md"), "# usage\n\nuse it.\n");
    commit_all(&repo, "c4: usage body");

    write_file(&root.join("CHANGELOG.md"), "# changelog\n\n- added mod_a\n");
    commit_all(&repo, "c5: changelog entry");

    // Dirty files at the end — git status should surface these.
    write_file(&root.join("src/lib.rs"), "pub fn one() -> u32 { 11 }\n");
    write_file(&root.join("notes.txt"), "dirty scratch\n");

    repo
}

/// Build a large synthetic monorepo rooted at `root`:
/// 60 Rust packages, 30 JavaScript packages, 21 commits, and a couple
/// of dirty files so git status has meaningful work.
///
/// Returns the opened `Repository` handle.
pub fn build_large_monorepo(root: &Path) -> Repository {
    let repo = Repository::init(root).expect("init large monorepo");

    let rust_pkgs = LARGE_MONOREPO_RUST_PKGS;
    let js_pkgs = LARGE_MONOREPO_JS_PKGS;

    let cargo_members: Vec<String> = (0..rust_pkgs)
        .map(|i| format!("\"crates/pkg{i:02}\""))
        .collect();
    let workspace_manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n    {}\n]\n",
        cargo_members.join(",\n    ")
    );
    write_file(&root.join("Cargo.toml"), &workspace_manifest);
    write_file(&root.join("README.md"), "# large monorepo fixture\n");
    write_file(&root.join(".gitignore"), "target/\nnode_modules/\n");

    for i in 0..rust_pkgs {
        let pkg = root.join(format!("crates/pkg{i:02}"));
        write_file(
            &pkg.join("Cargo.toml"),
            &format!("[package]\nname = \"pkg{i:02}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        write_file(
            &pkg.join("src/lib.rs"),
            &format!("pub fn n() -> u32 {{ {i} }}\n"),
        );
        write_file(
            &pkg.join("src/util.rs"),
            "pub fn util() -> &'static str { \"util\" }\n",
        );
        write_file(&pkg.join("tests/smoke.rs"), "#[test] fn ok() {}\n");
        write_file(&pkg.join("README.md"), &format!("# pkg{i:02}\n"));
    }

    let js_members: Vec<String> = (0..js_pkgs)
        .map(|i| format!("  - 'apps/app{i:02}'"))
        .collect();
    write_file(
        &root.join("pnpm-workspace.yaml"),
        &format!("packages:\n{}\n", js_members.join("\n")),
    );
    write_file(&root.join("package.json"), "{\"private\": true}\n");

    for i in 0..js_pkgs {
        let pkg = root.join(format!("apps/app{i:02}"));
        write_file(
            &pkg.join("package.json"),
            &format!("{{\"name\":\"app{i:02}\",\"version\":\"0.1.0\"}}\n"),
        );
        write_file(
            &pkg.join("src/index.ts"),
            &format!("export const name = 'app{i:02}';\n"),
        );
        write_file(&pkg.join("README.md"), &format!("# app{i:02}\n"));
    }

    commit_all(&repo, "c1: initial monorepo layout");

    // Churn commits touching different packages to create a deep-ish history.
    for round in 0..LARGE_MONOREPO_CHURN_COMMITS {
        let pkg_idx = (round as usize) % rust_pkgs;
        let pkg = root.join(format!("crates/pkg{pkg_idx:02}"));
        write_file(
            &pkg.join("src/lib.rs"),
            &format!("pub fn n() -> u32 {{ {pkg_idx} }}\npub const ROUND: u32 = {round};\n"),
        );
        commit_all(&repo, &format!("c{}: churn round {round}", round + 2));
    }

    // Leave dirty paths so git status has work to do.
    write_file(
        &root.join("crates/pkg00/src/lib.rs"),
        "pub fn n() -> u32 { 999 }\n",
    );
    write_file(
        &root.join("apps/app00/src/index.ts"),
        "export const name = 'dirty';\n",
    );

    repo
}

/// Build a non-git directory containing a shallow-wide language mix
/// plus one deeply nested path. Useful for language/file-type scanning
/// benchmarks that should not pay git discovery costs.
pub fn build_language_mix_tree(root: &Path) {
    for i in 0..20u32 {
        write_file(
            &root.join(format!("shallow/file_{i:02}.rs")),
            "fn noop() {}\n",
        );
        write_file(
            &root.join(format!("shallow/file_{i:02}.ts")),
            "export {};\n",
        );
        write_file(&root.join(format!("shallow/file_{i:02}.py")), "pass\n");
        write_file(&root.join(format!("shallow/file_{i:02}.md")), "# note\n");
    }

    let mut deep = root.join("deep");
    for level in 0..10u32 {
        deep = deep.join(format!("lvl_{level}"));
        write_file(&deep.join("mod.rs"), "pub fn x() {}\n");
        write_file(&deep.join("index.ts"), "export {};\n");
        write_file(&deep.join("README.md"), "# deep\n");
    }
}

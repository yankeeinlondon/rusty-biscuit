//! Deterministic fixture builders for `sniff` benches.
//!
//! Benchmarks must be reproducible across runs, so each helper returns
//! a fully-populated, committed directory anchored in a `TempDir`. The
//! caller is responsible for holding the `TempDir` handle for the
//! lifetime of the bench group — dropping it deletes the fixture.

use git2::{IndexAddOption, Repository, Signature};
use std::fs;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

/// A fixture rooted in a `TempDir`.
///
/// `path` is the on-disk root; the `TempDir` owns cleanup when dropped.
pub struct Fixture {
    // The `TempDir` handle must live as long as the fixture so the
    // underlying directory stays on disk; the field is never read
    // directly.
    #[allow(dead_code)]
    pub dir: TempDir,
    pub path: PathBuf,
}

impl Fixture {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

fn write(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(path, contents).unwrap();
}

fn commit_all(repo: &Repository, message: &str) {
    let mut index = repo.index().unwrap();
    index
        .add_all(["."].iter(), IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = Signature::new("Bench Runner", "bench@sniff.test", &git2::Time::new(0, 0)).unwrap();
    let parent_commit = repo
        .head()
        .ok()
        .and_then(|h| h.target())
        .and_then(|oid| repo.find_commit(oid).ok());
    let parents: Vec<&git2::Commit> = parent_commit.as_ref().into_iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
        .unwrap();
}

/// Small git repo: ~10 files, 5 commits, a couple of dirty files at the end.
pub fn small_git_repo() -> Fixture {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    let repo = Repository::init(&root).unwrap();

    write(&root.join("README.md"), "# small repo\n");
    write(&root.join(".gitignore"), "target/\n");
    write(
        &root.join("Cargo.toml"),
        "[package]\nname = \"small\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );
    write(&root.join("src/lib.rs"), "pub fn one() -> u32 { 1 }\n");
    write(&root.join("src/main.rs"), "fn main() {}\n");
    write(&root.join("tests/basic.rs"), "#[test] fn ok() {}\n");
    write(&root.join("docs/intro.md"), "# intro\n");
    write(&root.join("docs/usage.md"), "# usage\n");
    write(&root.join("LICENSE"), "All rights reserved.\n");
    write(&root.join("CHANGELOG.md"), "# changelog\n");
    commit_all(&repo, "c1: initial");

    write(&root.join("src/lib.rs"), "pub fn one() -> u32 { 10 }\n");
    commit_all(&repo, "c2: bump one");

    write(
        &root.join("src/mod_a.rs"),
        "pub fn a() -> &'static str { \"a\" }\n",
    );
    commit_all(&repo, "c3: add mod_a");

    write(&root.join("docs/usage.md"), "# usage\n\nuse it.\n");
    commit_all(&repo, "c4: usage body");

    write(&root.join("CHANGELOG.md"), "# changelog\n\n- added mod_a\n");
    commit_all(&repo, "c5: changelog entry");

    write(&root.join("src/lib.rs"), "pub fn one() -> u32 { 11 }\n");
    write(&root.join("notes.txt"), "dirty scratch\n");

    Fixture { dir, path: root }
}

/// Large synthetic monorepo: 60 rust packages + 30 js packages + deep git history.
///
/// This is big enough to exercise shared-walk and rayon fan-out but small
/// enough to build in <1s on a modern laptop.
pub fn large_monorepo() -> Fixture {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();
    let repo = Repository::init(&root).unwrap();

    let rust_pkgs = 60usize;
    let js_pkgs = 30usize;

    let cargo_members: Vec<String> = (0..rust_pkgs).map(|i| format!("\"crates/pkg{i:02}\"")).collect();
    let workspace_manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n    {}\n]\n",
        cargo_members.join(",\n    ")
    );
    write(&root.join("Cargo.toml"), &workspace_manifest);
    write(&root.join("README.md"), "# large monorepo fixture\n");
    write(&root.join(".gitignore"), "target/\nnode_modules/\n");

    for i in 0..rust_pkgs {
        let pkg = root.join(format!("crates/pkg{i:02}"));
        write(
            &pkg.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"pkg{i:02}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
            ),
        );
        write(
            &pkg.join("src/lib.rs"),
            &format!("pub fn n() -> u32 {{ {i} }}\n"),
        );
        write(
            &pkg.join("src/util.rs"),
            "pub fn util() -> &'static str { \"util\" }\n",
        );
        write(&pkg.join("tests/smoke.rs"), "#[test] fn ok() {}\n");
        write(&pkg.join("README.md"), &format!("# pkg{i:02}\n"));
    }

    let js_members: Vec<String> = (0..js_pkgs).map(|i| format!("  - 'apps/app{i:02}'")).collect();
    write(
        &root.join("pnpm-workspace.yaml"),
        &format!("packages:\n{}\n", js_members.join("\n")),
    );
    write(&root.join("package.json"), "{\"private\": true}\n");

    for i in 0..js_pkgs {
        let pkg = root.join(format!("apps/app{i:02}"));
        write(
            &pkg.join("package.json"),
            &format!("{{\"name\":\"app{i:02}\",\"version\":\"0.1.0\"}}\n"),
        );
        write(
            &pkg.join("src/index.ts"),
            &format!("export const name = 'app{i:02}';\n"),
        );
        write(&pkg.join("README.md"), &format!("# app{i:02}\n"));
    }

    commit_all(&repo, "c1: initial monorepo layout");

    // Churn: 20 commits touching different packages to create a deep-ish history.
    for round in 0..20u32 {
        let pkg_idx = (round as usize) % rust_pkgs;
        let pkg = root.join(format!("crates/pkg{pkg_idx:02}"));
        write(
            &pkg.join("src/lib.rs"),
            &format!(
                "pub fn n() -> u32 {{ {pkg_idx} }}\npub const ROUND: u32 = {round};\n"
            ),
        );
        commit_all(&repo, &format!("c{}: churn round {round}", round + 2));
    }

    // Leave a couple of dirty paths so git status has work to do.
    write(
        &root.join("crates/pkg00/src/lib.rs"),
        "pub fn n() -> u32 { 999 }\n",
    );
    write(&root.join("apps/app00/src/index.ts"), "export const name = 'dirty';\n");

    Fixture { dir, path: root }
}

/// Language-mix directory with both shallow and deep trees.
///
/// Not a git repo. Designed for language/file-type scanning benchmarks
/// that should not pay git discovery costs.
pub fn language_mix_tree() -> Fixture {
    let dir = TempDir::new().unwrap();
    let root = dir.path().to_path_buf();

    // Shallow top-level mix
    for i in 0..20u32 {
        write(&root.join(format!("shallow/file_{i:02}.rs")), "fn noop() {}\n");
        write(&root.join(format!("shallow/file_{i:02}.ts")), "export {};\n");
        write(&root.join(format!("shallow/file_{i:02}.py")), "pass\n");
        write(&root.join(format!("shallow/file_{i:02}.md")), "# note\n");
    }

    // Deep nested path
    let mut deep = root.join("deep");
    for level in 0..10u32 {
        deep = deep.join(format!("lvl_{level}"));
        write(&deep.join("mod.rs"), "pub fn x() {}\n");
        write(&deep.join("index.ts"), "export {};\n");
        write(&deep.join("README.md"), "# deep\n");
    }

    Fixture { dir, path: root }
}

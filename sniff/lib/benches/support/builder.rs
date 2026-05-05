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

/// Number of Rust packages in the synthetic `huge_monorepo` fixture.
pub const HUGE_MONOREPO_RUST_PKGS: usize = 200;

/// Number of JavaScript packages in the synthetic `huge_monorepo` fixture.
pub const HUGE_MONOREPO_JS_PKGS: usize = 100;

/// Number of Python packages in the synthetic `huge_monorepo` fixture.
pub const HUGE_MONOREPO_PYTHON_PKGS: usize = 50;

/// Number of Go modules in the synthetic `huge_monorepo` fixture.
pub const HUGE_MONOREPO_GO_PKGS: usize = 25;

/// Files per package in the huge monorepo fixture.
pub const HUGE_MONOREPO_FILES_PER_PKG: usize = 10;

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

/// Build a huge synthetic monorepo rooted at `root`:
/// 200 Rust packages, 100 JavaScript packages, 50 Python packages,
/// 25 Go modules, and ~10 files per package.
///
/// This fixture is designed to stress-test manifest caching,
/// index normalization, and package boundary enrichment.
///
/// Returns the opened `Repository` handle.
pub fn build_huge_monorepo(root: &Path) -> Repository {
    let repo = Repository::init(root).expect("init huge monorepo");

    let rust_pkgs = HUGE_MONOREPO_RUST_PKGS;
    let js_pkgs = HUGE_MONOREPO_JS_PKGS;
    let py_pkgs = HUGE_MONOREPO_PYTHON_PKGS;
    let go_pkgs = HUGE_MONOREPO_GO_PKGS;
    let files_per_pkg = HUGE_MONOREPO_FILES_PER_PKG;

    // Root workspace manifests
    let cargo_members: Vec<String> = (0..rust_pkgs)
        .map(|i| format!("\"crates/pkg{i:03}\""))
        .collect();
    let workspace_manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n    {}\n]\n",
        cargo_members.join(",\n    ")
    );
    write_file(&root.join("Cargo.toml"), &workspace_manifest);
    write_file(&root.join("README.md"), "# huge monorepo fixture\n");
    write_file(
        &root.join(".gitignore"),
        "target/\nnode_modules/\n__pycache__/\n",
    );

    // Rust packages
    for i in 0..rust_pkgs {
        let pkg = root.join(format!("crates/pkg{i:03}"));
        write_file(
            &pkg.join("Cargo.toml"),
            &format!("[package]\nname = \"pkg{i:03}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
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
        // Add extra files to hit the file-count target
        for f in 0..files_per_pkg.saturating_sub(4) {
            write_file(
                &pkg.join(format!("src/extra_{f}.rs")),
                &format!("pub fn extra_{f}() {{}}\n"),
            );
        }
    }

    // JavaScript packages
    let js_members: Vec<String> = (0..js_pkgs)
        .map(|i| format!("  - 'apps/app{i:03}'"))
        .collect();
    write_file(
        &root.join("pnpm-workspace.yaml"),
        &format!("packages:\n{}\n", js_members.join("\n")),
    );
    write_file(&root.join("package.json"), "{\"private\": true}\n");

    for i in 0..js_pkgs {
        let pkg = root.join(format!("apps/app{i:03}"));
        write_file(
            &pkg.join("package.json"),
            &format!("{{\"name\":\"app{i:03}\",\"version\":\"0.1.0\"}}\n"),
        );
        write_file(
            &pkg.join("src/index.ts"),
            &format!("export const name = 'app{i:03}';\n"),
        );
        write_file(&pkg.join("src/types.ts"), "export interface Config {{}}\n");
        for f in 0..files_per_pkg.saturating_sub(3) {
            write_file(
                &pkg.join(format!("src/module_{f}.ts")),
                &format!("export const mod_{f} = {{}};\n"),
            );
        }
    }

    // Python packages
    for i in 0..py_pkgs {
        let pkg = root.join(format!("python/lib{i:03}"));
        write_file(
            &pkg.join("pyproject.toml"),
            &format!("[project]\nname = \"lib{i:03}\"\nversion = \"0.1.0\"\n"),
        );
        write_file(
            &pkg.join("src/__init__.py"),
            &format!("def hello_{i}():\n    return {i}\n"),
        );
        for f in 0..files_per_pkg.saturating_sub(2) {
            write_file(
                &pkg.join(format!("src/module_{f}.py")),
                &format!("def func_{f}():\n    pass\n"),
            );
        }
    }

    // Go modules
    for i in 0..go_pkgs {
        let pkg = root.join(format!("go/mod{i:03}"));
        write_file(
            &pkg.join("go.mod"),
            &format!("module example.com/mod{i:03}\n\ngo 1.21\n"),
        );
        write_file(
            &pkg.join("main.go"),
            &format!("package main\n\nfunc main() {{}}\n// module {i}\n"),
        );
        for f in 0..files_per_pkg.saturating_sub(2) {
            write_file(
                &pkg.join(format!("pkg_{f}.go")),
                &format!("package main\n\nfunc Pkg{f}() {{}}\n"),
            );
        }
    }

    commit_all(&repo, "c1: initial huge monorepo layout");

    // Leave a few dirty files
    write_file(
        &root.join("crates/pkg000/src/lib.rs"),
        "pub fn n() -> u32 { 999 }\n",
    );
    write_file(
        &root.join("apps/app000/src/index.ts"),
        "export const name = 'dirty';\n",
    );

    repo
}

/// Build a git repo with a configurable number of dirty (modified) files.
///
/// The repo contains `dirty_count` tracked source files, each committed once
/// and then rewritten in the working tree so `git status` produces exactly
/// `dirty_count` modified entries. This isolates the per-file diff cost from
/// noise like renames, deletions, and untracked files so the resulting
/// benchmarks scale cleanly with dirty-file count.
///
/// Returns the opened `Repository` handle.
pub fn build_git_repo_with_dirty_files(root: &Path, dirty_count: usize) -> Repository {
    let repo = Repository::init(root).expect("init dirty-files repo");

    write_file(&root.join(".gitignore"), "target/\n");
    write_file(
        &root.join("Cargo.toml"),
        "[package]\nname = \"dirty\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );

    for i in 0..dirty_count {
        write_file(
            &root.join(format!("src/m{i:04}.rs")),
            &format!("pub fn m{i:04}() -> u32 {{ {i} }}\n"),
        );
    }

    commit_all(&repo, "c1: initial dirty-files layout");

    // Rewrite every file so each tracked source path is modified.
    for i in 0..dirty_count {
        write_file(
            &root.join(format!("src/m{i:04}.rs")),
            &format!("pub fn m{i:04}() -> u32 {{ {} }}\n", i + 1),
        );
    }

    repo
}

/// Build a directory tree of markdown documents for docs-parser benchmarks.
///
/// Creates `total_docs` markdown files. The first `with_blast_radius` files
/// declare a non-trivial `blast_radius` frontmatter list pointing at synthetic
/// source paths; the rest carry typical frontmatter (title, prompt) but no
/// `blast_radius`. The root is also initialized as a git repo so
/// `detect_blast_radius_docs`/`detect_docs` accept it.
///
/// Document bodies are intentionally small and uniform so the parsed-vs-only
/// difference comes from frontmatter handling rather than content size.
pub fn build_docs_repo(root: &Path, total_docs: usize, with_blast_radius: usize) {
    let with_br = with_blast_radius.min(total_docs);
    let repo = Repository::init(root).expect("init docs repo");

    write_file(&root.join("README.md"), "# docs fixture\n");
    write_file(&root.join(".gitignore"), "target/\n");

    for i in 0..total_docs {
        let body = format!(
            "# Doc {i:04}\n\n\
             Section one.\n\n\
             ```rust\nfn noop() {{}}\n```\n\n\
             Section two with a [link](https://example.test/{i}).\n"
        );
        let frontmatter = if i < with_br {
            format!(
                "---\n\
                 title: Doc {i:04}\n\
                 prompt: example\n\
                 blast_radius:\n  - src/m{i:04}.rs\n  - src/shared.rs\n\
                 ---\n"
            )
        } else {
            format!(
                "---\n\
                 title: Doc {i:04}\n\
                 prompt: example\n\
                 ---\n"
            )
        };
        let dir = i / 50;
        write_file(
            &root.join(format!("docs/d{dir:03}/doc_{i:04}.md")),
            &format!("{frontmatter}\n{body}"),
        );
    }

    write_file(
        &root.join("src/shared.rs"),
        "pub fn shared() -> &'static str { \"shared\" }\n",
    );

    commit_all(&repo, "c1: initial docs fixture");
}

/// Build a git repo with `remote_count` fake remote-tracking branches
/// pointing at different commits in the history, for deep-git containment
/// benchmarks.
///
/// The repo contains `commit_count` commits on a linear history.  Fake
/// remotes `remote0`, `remote1`, … are created with one branch each
/// (`main`) pointing at successive commits so the containment walk has
/// multiple ancestry paths to follow.
///
/// Returns the opened `Repository` handle.
pub fn build_git_repo_with_fake_remotes(
    root: &Path,
    commit_count: usize,
    remote_count: usize,
) -> Repository {
    let repo = Repository::init(root).expect("init fake-remotes repo");

    write_file(&root.join(".gitignore"), "target/\n");

    // Create a linear chain of commits.
    for i in 0..commit_count {
        write_file(
            &root.join(format!("src/m{i:04}.rs")),
            &format!("pub fn m{i:04}() -> u32 {{ {i} }}\n"),
        );
        commit_all(&repo, &format!("c{i}: commit {i}"));
    }

    // Create fake remote-tracking refs pointing at different commits.
    let max_remote = remote_count.min(commit_count);
    for r in 0..max_remote {
        let oid = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .map(|c| c.id());
        if let Some(base_oid) = oid {
            // Walk back `r` commits from HEAD to get a unique tip per remote.
            let mut target = base_oid;
            for _ in 0..r {
                if let Ok(commit) = repo.find_commit(target) {
                    if let Ok(parent) = commit.parent(0) {
                        target = parent.id();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }
            let ref_name = format!("refs/remotes/remote{r}/main");
            let _ = repo.reference(&ref_name, target, true, "bench remote");
        }
    }

    repo
}

/// Build a git repo with `package_count` Cargo packages, each with a
/// `Cargo.toml`, `src/lib.rs`, and a few classified files.
///
/// This fixture is designed to isolate `refresh_package_boundaries` by
/// providing a controllable number of manifest boundaries and source
/// files without the overhead of a multi-ecosystem monorepo.
///
/// Returns the opened `Repository` handle.
pub fn build_cargo_monorepo(root: &Path, package_count: usize) -> Repository {
    let repo = Repository::init(root).expect("init cargo monorepo");

    let cargo_members: Vec<String> = (0..package_count)
        .map(|i| format!("\"crates/pkg{i:03}\""))
        .collect();
    let workspace_manifest = format!(
        "[workspace]\nresolver = \"2\"\nmembers = [\n    {}\n]\n",
        cargo_members.join(",\n    ")
    );
    write_file(&root.join("Cargo.toml"), &workspace_manifest);
    write_file(&root.join("README.md"), "# cargo monorepo fixture\n");
    write_file(&root.join(".gitignore"), "target/\n");

    for i in 0..package_count {
        let pkg = root.join(format!("crates/pkg{i:03}"));
        write_file(
            &pkg.join("Cargo.toml"),
            &format!(
                "[package]\nname = \"pkg{i:03}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"
            ),
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
        write_file(&pkg.join("README.md"), &format!("# pkg{i:03}\n"));
    }

    commit_all(&repo, "c1: initial cargo monorepo layout");

    // Leave a couple of dirty files so the inventory has classified
    // files to assign to packages during boundary refresh.
    write_file(
        &root.join("crates/pkg000/src/lib.rs"),
        "pub fn n() -> u32 { 999 }\n",
    );
    write_file(
        &root.join("crates/pkg001/src/util.rs"),
        "pub fn util() -> &'static str { \"dirty\" }\n",
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

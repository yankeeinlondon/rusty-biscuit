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
use std::process::{Command, Stdio};

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

/// Number of commits in the default `git_repo_with_worktrees` fixture base
/// history before any worktrees are added.
pub const WORKTREES_BASE_COMMITS: u32 = 3;

/// Initialize a git repository with cross-platform-stable settings.
///
/// All fixture repos disable `core.autocrlf` and pin `core.eol = lf` so the
/// same file contents produce identical blobs (and therefore identical dirty
/// counts and diff text) on Windows, Linux, and macOS. Without this, a
/// developer machine with `autocrlf = true` would rewrite line endings on
/// checkout and skew status/diff fixtures.
pub fn init_repo(root: &Path) -> Repository {
    let repo = Repository::init(root).expect("init git repo");
    if let Ok(mut config) = repo.config() {
        let _ = config.set_bool("core.autocrlf", false);
        let _ = config.set_str("core.eol", "lf");
    }
    repo
}

/// Write a file, creating parent directories as needed.
pub fn write_file(path: &Path, contents: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directory");
    }
    fs::write(path, contents).expect("write fixture file");
}

/// Stage everything and create a commit using a deterministic signature whose
/// author/committer time is `seconds` past the Unix epoch.
///
/// Monotonically increasing `seconds` across a history gives the revwalk a real
/// commit-time ordering to sort and gate on, which the recent-commit time
/// cutoff benchmarks depend on.
pub fn commit_all_at(repo: &Repository, message: &str, seconds: i64) {
    let mut index = repo.index().expect("load repo index");
    index
        .add_all(["."].iter(), IndexAddOption::DEFAULT, None)
        .expect("stage files");
    index.write().expect("write index");
    let tree_id = index.write_tree().expect("write tree");
    let tree = repo.find_tree(tree_id).expect("resolve tree");
    let sig = Signature::new(
        "Bench Runner",
        "bench@sniff.test",
        &git2::Time::new(seconds, 0),
    )
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
    let repo = init_repo(root);

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
    let repo = init_repo(root);

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
    let repo = init_repo(root);

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
    build_git_repo_with_dirty_files_of_size(root, dirty_count, 64)
}

/// Build a git repo with `dirty_count` modified files of exactly `bytes_per_file`.
///
/// The payload changes at both ends after the initial commit so full and deep
/// status requests cannot short-circuit on a shared prefix or suffix. Fixture
/// construction is intentionally separate from every timed benchmark loop.
pub fn build_git_repo_with_dirty_files_of_size(
    root: &Path,
    dirty_count: usize,
    bytes_per_file: usize,
) -> Repository {
    let repo = init_repo(root);

    write_file(&root.join(".gitignore"), "target/\n");
    write_file(
        &root.join("Cargo.toml"),
        "[package]\nname = \"dirty\"\nversion = \"0.1.0\"\nedition = \"2021\"\n",
    );

    for i in 0..dirty_count {
        let mut payload = vec![b'a'; bytes_per_file.max(1)];
        payload[0] = b'0' + (i % 10) as u8;
        let path = root.join(format!("src/m{i:04}.rs"));
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent).expect("create dirty fixture directory");
        }
        fs::write(path, payload).expect("write tracked dirty fixture file");
    }

    commit_all(&repo, "c1: initial dirty-files layout");

    // Rewrite every file so each tracked source path is modified.
    for i in 0..dirty_count {
        let mut payload = vec![b'b'; bytes_per_file.max(1)];
        payload[0] = b'0' + ((i + 1) % 10) as u8;
        fs::write(root.join(format!("src/m{i:04}.rs")), payload)
            .expect("rewrite dirty fixture file");
    }

    repo
}

/// Build a deep, wide tree whose only formatting evidence is at its root.
pub fn build_deep_wide_formatting_tree(root: &Path, depth: usize, width: usize) {
    write_file(&root.join(".editorconfig"), "root = true\n[*]\nindent_size = 4\n");
    for branch in 0..width {
        let mut dir = root.join(format!("branch-{branch:03}"));
        for level in 0..depth {
            dir = dir.join(format!("level-{level:03}"));
            write_file(&dir.join("source.rs"), "pub fn fixture() {}\n");
        }
    }
}

/// Build a mixed Cargo, JavaScript, Python, and Go monorepo of `package_count`.
pub fn build_mixed_monorepo(root: &Path, package_count: usize) -> Repository {
    let repo = init_repo(root);
    let rust_count = package_count * 2 / 5;
    let js_count = package_count * 3 / 10;
    let python_count = package_count / 5;
    // The uv workspace root is represented as one virtual package in the
    // canonical catalog, so reserve one slot for it.
    let go_count = package_count
        .saturating_sub(rust_count + js_count + python_count)
        .saturating_sub(1);

    let cargo_members = (0..rust_count)
        .map(|i| format!("\"crates/rust-{i:04}\""))
        .collect::<Vec<_>>()
        .join(",\n    ");
    write_file(
        &root.join("Cargo.toml"),
        &format!("[workspace]\nresolver = \"2\"\nmembers = [\n    {cargo_members}\n]\n"),
    );
    let js_members = (0..js_count)
        .map(|i| format!("  - 'apps/js-{i:04}'"))
        .collect::<Vec<_>>()
        .join("\n");
    write_file(
        &root.join("pnpm-workspace.yaml"),
        &format!("packages:\n{js_members}\n"),
    );
    write_file(&root.join("package.json"), "{\"private\":true}\n");
    write_file(
        &root.join("pyproject.toml"),
        "[tool.uv.workspace]\nmembers = [\"python/*\"]\n",
    );
    let go_members = (0..go_count)
        .map(|i| format!("\t./go/mod-{i:04}"))
        .collect::<Vec<_>>()
        .join("\n");
    write_file(
        &root.join("go.work"),
        &format!("go 1.21\n\nuse (\n{go_members}\n)\n"),
    );
    write_file(&root.join("README.md"), "# mixed monorepo\n");

    for i in 0..rust_count {
        let package = root.join(format!("crates/rust-{i:04}"));
        write_file(
            &package.join("Cargo.toml"),
            &format!("[package]\nname = \"rust-{i:04}\"\nversion = \"0.1.0\"\n"),
        );
        write_file(&package.join("src/lib.rs"), "pub fn fixture() {}\n");
    }
    for i in 0..js_count {
        let package = root.join(format!("apps/js-{i:04}"));
        write_file(
            &package.join("package.json"),
            &format!("{{\"name\":\"js-{i:04}\",\"version\":\"0.1.0\"}}\n"),
        );
        write_file(&package.join("src/index.ts"), "export const fixture = true;\n");
    }
    for i in 0..python_count {
        let package = root.join(format!("python/py-{i:04}"));
        write_file(
            &package.join("pyproject.toml"),
            &format!("[project]\nname = \"py-{i:04}\"\nversion = \"0.1.0\"\n"),
        );
        write_file(&package.join("src/__init__.py"), "FIXTURE = True\n");
    }
    for i in 0..go_count {
        let package = root.join(format!("go/mod-{i:04}"));
        write_file(
            &package.join("go.mod"),
            &format!("module example.test/mod-{i:04}\n\ngo 1.21\n"),
        );
        write_file(&package.join("main.go"), "package main\nfunc main() {}\n");
    }
    commit_all(&repo, "mixed monorepo fixture");
    repo
}

/// Build an over-cap inventory tree with Markdown files distributed through it.
pub fn build_inventory_docs_tree(root: &Path, file_count: usize, doc_count: usize) {
    for i in 0..file_count {
        let directory = root.join(format!("bucket-{:03}", i / 100));
        if i < doc_count {
            write_file(
                &directory.join(format!("doc-{i:05}.md")),
                &format!("---\ntitle: Doc {i}\n---\n# Doc {i}\n"),
            );
        } else {
            write_file(&directory.join(format!("file-{i:05}.txt")), "fixture\n");
        }
    }
}

/// Add package-owned Markdown documents to an existing mixed monorepo.
pub fn add_package_documents(root: &Path, package_count: usize, document_count: usize) {
    let rust_count = (package_count * 2 / 5).max(1);
    for i in 0..document_count {
        let owner = i % rust_count;
        write_file(
            &root.join(format!("crates/rust-{owner:04}/docs/doc-{i:05}.md")),
            &format!("---\ntitle: Package Doc {i}\n---\n# Package Doc {i}\n"),
        );
    }
}

/// Add a nested pnpm membership authority below an existing repository root.
pub fn add_nested_workspace(root: &Path) {
    write_file(
        &root.join("nested/pnpm-workspace.yaml"),
        "packages:\n  - 'packages/*'\n",
    );
    write_file(
        &root.join("nested/package.json"),
        "{\"private\":true}\n",
    );
    write_file(
        &root.join("nested/packages/member/package.json"),
        "{\"name\":\"nested-member\",\"version\":\"0.1.0\"}\n",
    );
    write_file(
        &root.join("nested/packages/member/src/index.ts"),
        "export const nested = true;\n",
    );
}

/// Build a long history where only every `match_every`th commit touches `wanted/`.
pub fn build_sparse_path_history_repo(
    root: &Path,
    commits: usize,
    match_every: usize,
) -> Repository {
    let repo = init_repo(root);
    let cadence = match_every.max(1);
    for i in 0..commits {
        let relative = if i % cadence == 0 {
            "wanted/history.txt"
        } else {
            "other/history.txt"
        };
        write_file(&root.join(relative), &format!("commit {i}\n"));
        commit_all_at(&repo, &format!("history {i}"), 1_000 + i as i64);
    }
    repo
}

/// Build paths which differ only by component case.
pub fn build_case_variant_tree(root: &Path, files_per_variant: usize) {
    for i in 0..files_per_variant {
        write_file(
            &root.join(format!("Case/Tree/file-{i:04}.rs")),
            "pub fn upper() {}\n",
        );
        write_file(
            &root.join(format!("case/tree/file-{i:04}.rs")),
            "pub fn lower() {}\n",
        );
    }
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
    let repo = init_repo(root);

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
    let repo = init_repo(root);

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
    let repo = init_repo(root);

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

/// Build a git repo with `count` linked worktrees for worktree fan-out
/// benchmarks.
///
/// The base repository carries [`WORKTREES_BASE_COMMITS`] commits. Each linked
/// worktree lives under `_wt/` (gitignored so the base working tree stays
/// clean) on its own branch with one divergent commit, so every worktree is
/// exactly one commit ahead of its base branch — giving the ahead/behind and
/// merge calculations in `get_worktrees` real work to do.
///
/// Returns the opened base `Repository` handle.
pub fn build_git_repo_with_worktrees(root: &Path, count: usize) -> Repository {
    let repo = init_repo(root);

    write_file(&root.join(".gitignore"), "target/\n_wt/\n");
    write_file(&root.join("README.md"), "# worktrees fixture\n");
    write_file(&root.join("src/lib.rs"), "pub fn base() -> u32 { 0 }\n");

    let mut seconds = 1_000i64;
    commit_all_at(&repo, "c1: initial worktrees layout", seconds);
    for i in 1..WORKTREES_BASE_COMMITS {
        seconds += 60;
        write_file(
            &root.join("src/lib.rs"),
            &format!("pub fn base() -> u32 {{ {i} }}\n"),
        );
        commit_all_at(&repo, &format!("c{}: base churn {i}", i + 1), seconds);
    }

    let wt_root = root.join("_wt");
    fs::create_dir_all(&wt_root).expect("create worktree root");

    for i in 0..count {
        let name = format!("wt{i:03}");
        let wt_path = wt_root.join(&name);
        repo.worktree(&name, &wt_path, None)
            .expect("create linked worktree");

        // One divergent commit per worktree so ahead/behind has work.
        let wt_repo = Repository::open(&wt_path).expect("open linked worktree");
        write_file(
            &wt_path.join(format!("feature_{i:03}.rs")),
            &format!("pub fn feature_{i:03}() -> u32 {{ {i} }}\n"),
        );
        commit_all_at(
            &wt_repo,
            &format!("wt{i:03}: divergent commit"),
            seconds + 1_000 + i as i64,
        );
    }

    repo
}

/// Build a git repo with `commits` commits on a single linear history for
/// deep-history revwalk and commit-graph benchmarks.
///
/// Each commit rewrites a rolling source file and appends to `history.txt`, so
/// every commit carries a small non-empty diff. Commit times increase
/// monotonically (60s apart) so the revwalk has a meaningful commit-time order
/// to sort and gate on.
///
/// Returns the opened `Repository` handle.
pub fn build_deep_history_repo(root: &Path, commits: usize) -> Repository {
    let repo = init_repo(root);

    write_file(&root.join(".gitignore"), "target/\n");

    let mut seconds = 1_000i64;
    for i in 0..commits {
        write_file(&root.join("history.txt"), &format!("line {i}\n"));
        write_file(
            &root.join(format!("src/f{:03}.rs", i % 16)),
            &format!("pub const N: u32 = {i};\n"),
        );
        commit_all_at(&repo, &format!("c{i}: commit {i}"), seconds);
        seconds += 60;
    }

    repo
}

/// Whether a usable `git` executable is on `PATH`.
///
/// Commit-graph generation shells out to `git` (libgit2 exposes no
/// commit-graph writer), so graph-dependent benchmarks must skip cleanly when
/// no `git` binary is available.
pub fn git_available() -> bool {
    Command::new("git")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

/// Write a commit-graph file for `root` using the external `git` executable.
///
/// ## Returns
///
/// `true` when the commit-graph was written; `false` when `git` is unavailable
/// or the command failed, so callers can skip graph-dependent benchmarks with a
/// clear message rather than failing outright.
pub fn write_commit_graph(root: &Path) -> bool {
    Command::new("git")
        .args(["commit-graph", "write", "--reachable"])
        .current_dir(root)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
}

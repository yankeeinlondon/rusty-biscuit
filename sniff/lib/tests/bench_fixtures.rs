//! Invariant tests for the deterministic bench fixture builders.
//!
//! The bench support modules are not executed during `cargo test` by
//! default, so regressions in the fixture shape (commit counts,
//! workspace manifest, dirty files, package counts) previously went
//! unnoticed until a slow end-to-end bench caught them — if it caught
//! them at all. These tests materialize each fixture into a tempdir
//! and assert the invariants the bench cases rely on.

#[path = "../benches/support/builder.rs"]
mod builder;

use std::fs;
use std::path::Path;

use git2::{Repository, StatusOptions};
use tempfile::TempDir;

use builder::{
    LARGE_MONOREPO_CHURN_COMMITS, LARGE_MONOREPO_DIRTY_FILES, LARGE_MONOREPO_JS_PKGS,
    LARGE_MONOREPO_RUST_PKGS, LARGE_MONOREPO_TOTAL_COMMITS, SMALL_GIT_REPO_COMMITS,
    SMALL_GIT_REPO_DIRTY_FILES, build_language_mix_tree, build_large_monorepo, build_small_git_repo,
};

fn count_commits(repo: &Repository) -> u32 {
    let mut revwalk = repo.revwalk().expect("revwalk");
    revwalk.push_head().expect("push HEAD");
    revwalk.count() as u32
}

fn count_dirty_files(repo: &Repository) -> usize {
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    opts.recurse_untracked_dirs(true);
    repo.statuses(Some(&mut opts))
        .expect("repo statuses")
        .iter()
        .filter(|entry| {
            let status = entry.status();
            !status.is_ignored() && !status.is_empty()
        })
        .count()
}

#[test]
fn small_git_repo_has_expected_shape() {
    let dir = TempDir::new().expect("tempdir");
    let repo = build_small_git_repo(dir.path());

    assert_eq!(
        count_commits(&repo),
        SMALL_GIT_REPO_COMMITS,
        "small_git_repo should contain {SMALL_GIT_REPO_COMMITS} commits"
    );

    let dirty = count_dirty_files(&repo);
    assert_eq!(
        dirty, SMALL_GIT_REPO_DIRTY_FILES,
        "small_git_repo should leave {SMALL_GIT_REPO_DIRTY_FILES} dirty files, got {dirty}"
    );

    // Sanity-check expected file surface.
    for expected in [
        "Cargo.toml",
        "README.md",
        "src/lib.rs",
        "src/main.rs",
        "src/mod_a.rs",
        "tests/basic.rs",
        "docs/intro.md",
        "docs/usage.md",
        "CHANGELOG.md",
        "LICENSE",
    ] {
        assert!(
            dir.path().join(expected).is_file(),
            "small_git_repo missing {expected}"
        );
    }
}

#[test]
fn large_monorepo_has_expected_shape() {
    let dir = TempDir::new().expect("tempdir");
    let repo = build_large_monorepo(dir.path());

    assert_eq!(
        count_commits(&repo),
        LARGE_MONOREPO_TOTAL_COMMITS,
        "large_monorepo should contain {LARGE_MONOREPO_TOTAL_COMMITS} commits \
         (1 initial + {LARGE_MONOREPO_CHURN_COMMITS} churn)"
    );

    let dirty = count_dirty_files(&repo);
    assert_eq!(
        dirty, LARGE_MONOREPO_DIRTY_FILES,
        "large_monorepo should leave {LARGE_MONOREPO_DIRTY_FILES} dirty files, got {dirty}"
    );

    // Workspace + docs + ignore.
    assert!(dir.path().join("Cargo.toml").is_file());
    assert!(dir.path().join("pnpm-workspace.yaml").is_file());
    assert!(dir.path().join("package.json").is_file());
    assert!(dir.path().join(".gitignore").is_file());
    assert!(dir.path().join("README.md").is_file());

    // Workspace manifest should mention every rust package.
    let cargo = fs::read_to_string(dir.path().join("Cargo.toml")).expect("read Cargo.toml");
    for i in 0..LARGE_MONOREPO_RUST_PKGS {
        let entry = format!("\"crates/pkg{i:02}\"");
        assert!(
            cargo.contains(&entry),
            "workspace manifest missing {entry}"
        );
    }

    // Every rust package has its manifest and lib.rs.
    for i in 0..LARGE_MONOREPO_RUST_PKGS {
        let pkg = dir.path().join(format!("crates/pkg{i:02}"));
        assert!(
            pkg.join("Cargo.toml").is_file(),
            "missing Cargo.toml for pkg{i:02}"
        );
        assert!(
            pkg.join("src/lib.rs").is_file(),
            "missing src/lib.rs for pkg{i:02}"
        );
    }

    // pnpm-workspace manifest should mention every js package.
    let pnpm = fs::read_to_string(dir.path().join("pnpm-workspace.yaml"))
        .expect("read pnpm-workspace.yaml");
    for i in 0..LARGE_MONOREPO_JS_PKGS {
        let entry = format!("apps/app{i:02}");
        assert!(pnpm.contains(&entry), "pnpm-workspace missing {entry}");
    }

    // Every js package has its manifest and entry point.
    for i in 0..LARGE_MONOREPO_JS_PKGS {
        let pkg = dir.path().join(format!("apps/app{i:02}"));
        assert!(
            pkg.join("package.json").is_file(),
            "missing package.json for app{i:02}"
        );
        assert!(
            pkg.join("src/index.ts").is_file(),
            "missing src/index.ts for app{i:02}"
        );
    }
}

#[test]
fn large_monorepo_dirty_files_are_rust_and_js() {
    let dir = TempDir::new().expect("tempdir");
    let _repo = build_large_monorepo(dir.path());

    let rust_dirty = fs::read_to_string(dir.path().join("crates/pkg00/src/lib.rs"))
        .expect("read dirty rust file");
    assert!(
        rust_dirty.contains("999"),
        "crates/pkg00/src/lib.rs should carry the dirty marker 999"
    );

    let js_dirty = fs::read_to_string(dir.path().join("apps/app00/src/index.ts"))
        .expect("read dirty js file");
    assert!(
        js_dirty.contains("dirty"),
        "apps/app00/src/index.ts should carry the dirty marker 'dirty'"
    );
}

#[test]
fn language_mix_tree_is_not_a_git_repo() {
    let dir = TempDir::new().expect("tempdir");
    build_language_mix_tree(dir.path());

    assert!(
        !dir.path().join(".git").exists(),
        "language_mix_tree should never create a .git directory"
    );

    // Shallow layer: 20 files in each of four languages.
    for i in 0..20u32 {
        let base = dir.path().join("shallow").join(format!("file_{i:02}"));
        for ext in ["rs", "ts", "py", "md"] {
            let path = base.with_extension(ext);
            assert!(path.is_file(), "missing {}", path.display());
        }
    }

    // Deep layer: walk 10 nested levels and verify each has files.
    let mut deep = dir.path().join("deep");
    for level in 0..10u32 {
        deep = deep.join(format!("lvl_{level}"));
        assert!(
            deep.join("mod.rs").is_file(),
            "missing mod.rs at {}",
            deep.display()
        );
        assert!(
            deep.join("index.ts").is_file(),
            "missing index.ts at {}",
            deep.display()
        );
        assert!(
            deep.join("README.md").is_file(),
            "missing README.md at {}",
            deep.display()
        );
    }
}

#[test]
fn fixture_builders_are_idempotent_over_fresh_dirs() {
    // Two distinct temp dirs should produce the same commit count and
    // the same package fan-out so successive bench runs compare like
    // for like.
    let a = TempDir::new().expect("tempdir a");
    let b = TempDir::new().expect("tempdir b");

    let repo_a = build_large_monorepo(a.path());
    let repo_b = build_large_monorepo(b.path());

    assert_eq!(count_commits(&repo_a), count_commits(&repo_b));
    assert_eq!(count_dirty_files(&repo_a), count_dirty_files(&repo_b));

    let pkgs_a = count_rust_packages(a.path());
    let pkgs_b = count_rust_packages(b.path());
    assert_eq!(pkgs_a, pkgs_b);
    assert_eq!(pkgs_a, LARGE_MONOREPO_RUST_PKGS);
}

fn count_rust_packages(root: &Path) -> usize {
    let crates = root.join("crates");
    if !crates.is_dir() {
        return 0;
    }
    fs::read_dir(&crates)
        .expect("read crates/")
        .filter_map(|entry| entry.ok())
        .filter(|entry| {
            entry
                .file_type()
                .map(|ft| ft.is_dir())
                .unwrap_or(false)
                && entry.path().join("Cargo.toml").is_file()
        })
        .count()
}

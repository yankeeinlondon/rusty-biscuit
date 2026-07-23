//! Deterministic fixture wrappers for `sniff` benches.
//!
//! The actual file-generation logic lives in [`super::builder`] so it
//! can be shared with profiling examples and integration tests that do
//! not depend on `tempfile`. This file layers a `TempDir`-owning
//! [`Fixture`] wrapper on top so bench groups can drop a fixture to
//! clean up their scratch tree.

// Consumers include this module via `#[path = "..."]` and each uses only the
// fixtures its own cases need, so an unused wrapper here is expected rather
// than dead.
#![allow(dead_code)]

use std::path::{Path, PathBuf};
use tempfile::TempDir;

use super::builder;

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

/// Small git repo fixture backed by a `TempDir`.
pub fn small_git_repo() -> Fixture {
    let dir = TempDir::new().expect("tempdir for small_git_repo");
    let root = dir.path().to_path_buf();
    builder::build_small_git_repo(&root);
    Fixture { dir, path: root }
}

/// Large synthetic monorepo fixture backed by a `TempDir`.
pub fn large_monorepo() -> Fixture {
    let dir = TempDir::new().expect("tempdir for large_monorepo");
    let root = dir.path().to_path_buf();
    builder::build_large_monorepo(&root);
    Fixture { dir, path: root }
}

/// Huge synthetic monorepo fixture backed by a `TempDir`.
///
/// 200 Rust + 100 JS + 50 Python + 25 Go packages with ~10 files each.
/// Designed to stress-test manifest caching and package enrichment.
pub fn huge_monorepo() -> Fixture {
    let dir = TempDir::new().expect("tempdir for huge_monorepo");
    let root = dir.path().to_path_buf();
    builder::build_huge_monorepo(&root);
    Fixture { dir, path: root }
}

/// Language-mix directory fixture backed by a `TempDir`.
pub fn language_mix_tree() -> Fixture {
    let dir = TempDir::new().expect("tempdir for language_mix_tree");
    let root = dir.path().to_path_buf();
    builder::build_language_mix_tree(&root);
    Fixture { dir, path: root }
}

/// Git repo fixture with a configurable number of dirty (modified) files.
///
/// See [`builder::build_git_repo_with_dirty_files`] for the precise layout.
pub fn git_repo_with_dirty_files(dirty_count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for git_repo_with_dirty_files");
    let root = dir.path().to_path_buf();
    builder::build_git_repo_with_dirty_files(&root, dirty_count);
    Fixture { dir, path: root }
}

/// Dirty Git fixture with an exact per-file payload size.
pub fn git_repo_with_dirty_files_of_size(dirty_count: usize, bytes_per_file: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for sized dirty git fixture");
    let root = dir.path().to_path_buf();
    builder::build_git_repo_with_dirty_files_of_size(&root, dirty_count, bytes_per_file);
    Fixture { dir, path: root }
}

/// Deep/wide formatting-only fixture.
pub fn deep_wide_formatting_tree(depth: usize, width: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for formatting fixture");
    let root = dir.path().to_path_buf();
    builder::build_deep_wide_formatting_tree(&root, depth, width);
    Fixture { dir, path: root }
}

/// Parameterized mixed-ecosystem monorepo fixture.
pub fn mixed_monorepo(package_count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for mixed monorepo fixture");
    let root = dir.path().to_path_buf();
    builder::build_mixed_monorepo(&root, package_count);
    Fixture { dir, path: root }
}

/// Parameterized over-cap inventory and documentation fixture.
pub fn inventory_docs_tree(file_count: usize, doc_count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for inventory/docs fixture");
    let root = dir.path().to_path_buf();
    builder::build_inventory_docs_tree(&root, file_count, doc_count);
    Fixture { dir, path: root }
}

/// Mixed monorepo carrying package-owned Markdown documents.
pub fn documented_mixed_monorepo(package_count: usize, document_count: usize) -> Fixture {
    let fixture = mixed_monorepo(package_count);
    builder::add_package_documents(fixture.path(), package_count, document_count);
    fixture
}

/// Large mixed monorepo with a distinct nested workspace authority.
pub fn nested_mixed_monorepo(package_count: usize) -> Fixture {
    let fixture = mixed_monorepo(package_count);
    builder::add_nested_workspace(fixture.path());
    fixture
}

/// Sparse-prefix path-history fixture.
pub fn sparse_path_history_repo(commits: usize, match_every: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for sparse path history fixture");
    let root = dir.path().to_path_buf();
    builder::build_sparse_path_history_repo(&root, commits, match_every);
    Fixture { dir, path: root }
}

/// Case-variant filesystem fixture.
pub fn case_variant_tree(files_per_variant: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for filesystem case fixture");
    let root = dir.path().to_path_buf();
    builder::build_case_variant_tree(&root, files_per_variant);
    Fixture { dir, path: root }
}

/// Docs-parser fixture: a git repo with a configurable number of markdown
/// documents, only some of which declare a `blast_radius` frontmatter list.
///
/// See [`builder::build_docs_repo`] for the precise layout.
pub fn docs_repo(total_docs: usize, with_blast_radius: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for docs_repo");
    let root = dir.path().to_path_buf();
    builder::build_docs_repo(&root, total_docs, with_blast_radius);
    Fixture { dir, path: root }
}

/// Git repo with fake remote-tracking branches for deep-git containment
/// benchmarks.
///
/// See [`builder::build_git_repo_with_fake_remotes`] for the precise layout.
pub fn git_repo_with_fake_remotes(commit_count: usize, remote_count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for git_repo_with_fake_remotes");
    let root = dir.path().to_path_buf();
    builder::build_git_repo_with_fake_remotes(&root, commit_count, remote_count);
    Fixture { dir, path: root }
}

/// Cargo monorepo fixture with a configurable number of Rust packages.
///
/// See [`builder::build_cargo_monorepo`] for the precise layout.
pub fn cargo_monorepo(package_count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for cargo_monorepo");
    let root = dir.path().to_path_buf();
    builder::build_cargo_monorepo(&root, package_count);
    Fixture { dir, path: root }
}

/// Git repo fixture with `count` linked worktrees for fan-out benchmarks.
///
/// See [`builder::build_git_repo_with_worktrees`] for the precise layout.
pub fn git_repo_with_worktrees(count: usize) -> Fixture {
    let dir = TempDir::new().expect("tempdir for git_repo_with_worktrees");
    let root = dir.path().to_path_buf();
    builder::build_git_repo_with_worktrees(&root, count);
    Fixture { dir, path: root }
}

/// Deep linear-history git repo fixture, optionally with a commit-graph.
///
/// When `with_commit_graph` is true the fixture shells out to `git` to write
/// `.git/objects/info/commit-graph`. Returns `None` when a commit-graph was
/// requested but no `git` executable is available, so graph benchmarks can skip
/// with a clear message instead of measuring a graph-absent repo as if present.
///
/// See [`builder::build_deep_history_repo`] for the precise layout.
pub fn deep_history_repo(commits: usize, with_commit_graph: bool) -> Option<Fixture> {
    if with_commit_graph && !builder::git_available() {
        return None;
    }
    let dir = TempDir::new().expect("tempdir for deep_history_repo");
    let root = dir.path().to_path_buf();
    builder::build_deep_history_repo(&root, commits);
    if with_commit_graph && !builder::write_commit_graph(&root) {
        return None;
    }
    Some(Fixture { dir, path: root })
}

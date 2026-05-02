//! Deterministic fixture wrappers for `sniff` benches.
//!
//! The actual file-generation logic lives in [`super::builder`] so it
//! can be shared with profiling examples and integration tests that do
//! not depend on `tempfile`. This file layers a `TempDir`-owning
//! [`Fixture`] wrapper on top so bench groups can drop a fixture to
//! clean up their scratch tree.

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

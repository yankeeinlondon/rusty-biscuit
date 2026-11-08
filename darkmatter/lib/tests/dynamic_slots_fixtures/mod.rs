//! Miniature repositories for dynamic schema slot tests.
//!
//! [`SlotRepo::build`] copies a fixture tree (normally
//! `tests/fixtures/dynamic_slots/claudine`, which the calling `#[test]` names
//! so CI maps the fixture to that test) into a fresh Git repository under
//! `claudine/`, and leaves the slot's data file, `claudine/docs/os.yaml`, in
//! the state a [`RepoState`] names. Every other file is tracked and
//! committed, so each approval-matrix cell differs from the clean control in
//! exactly one way.

use std::path::{Path, PathBuf};
use std::process::Command;

use tempfile::TempDir;

/// The slot data file, relative to the repository root.
pub const DATA_FILE: &str = "claudine/docs/os.yaml";
/// The dynamic schema, relative to the repository root.
pub const SCHEMA_FILE: &str = "claudine/docs/research/sandboxing/providers/sandboxing.yaml";
/// A document validated by [`SCHEMA_FILE`], relative to the repository root.
pub const DOCUMENT_FILE: &str = "claudine/docs/research/sandboxing/providers/provider.md";

/// What Git knows about [`DATA_FILE`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RepoState {
    /// Tracked and identical to the committed content.
    Clean,
    /// Tracked, with an uncommitted change (one more list entry).
    DirtyData,
    /// Present but never added.
    UntrackedData,
    /// Present, untracked, and matched by a committed `.gitignore`.
    IgnoredData,
    /// The same files with no repository at all.
    NoRepository,
}

/// The line [`RepoState::DirtyData`] appends to the data file.
pub const DIRTY_ENTRY: &str = "- FreeBSD\n";

/// A fixture repository; the temporary directory lives as long as this value.
pub struct SlotRepo {
    _temp: TempDir,
    root: PathBuf,
}

impl SlotRepo {
    /// Copies `source` to `<root>/claudine` and prepares `state`.
    pub fn build(source: &Path, state: RepoState) -> Self {
        let temp = TempDir::new().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(temp.path())
            .expect("canonical temp dir")
            .join("repo");
        copy_tree(source, &root.join("claudine"));
        let repo = Self { _temp: temp, root };
        if state == RepoState::NoRepository {
            return repo;
        }

        git(&repo.root, &["init", "-q", "-b", "main"]);
        match state {
            RepoState::UntrackedData => {
                git(&repo.root, &["add", "-A", "--", ".", &format!(":(exclude){DATA_FILE}")]);
            }
            RepoState::IgnoredData => {
                std::fs::write(repo.root.join(".gitignore"), format!("{DATA_FILE}\n")).expect("gitignore");
                git(&repo.root, &["add", "-A"]);
            }
            _ => git(&repo.root, &["add", "-A"]),
        }
        git(&repo.root, &["commit", "-q", "-m", "fixture"]);
        if state == RepoState::DirtyData {
            let data = repo.path(DATA_FILE);
            let mut text = std::fs::read_to_string(&data).expect("data file");
            text.push_str(DIRTY_ENTRY);
            std::fs::write(&data, text).expect("dirty data file");
        }
        repo
    }

    /// The repository root (canonical, simplified).
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// `relative` (a `/`-separated fixture path) under the root.
    pub fn path(&self, relative: &str) -> PathBuf {
        relative.split('/').fold(self.root.clone(), |path, segment| path.join(segment))
    }
}

/// Runs Git in `root` with a fixture identity and no host configuration, so
/// a developer's signing or hooks setup cannot change the fixture.
pub fn git(root: &Path, args: &[&str]) {
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=fixture",
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .current_dir(root)
        .env_remove("GIT_DIR")
        .env_remove("GIT_WORK_TREE")
        .env_remove("GIT_INDEX_FILE")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", if cfg!(windows) { "NUL" } else { "/dev/null" })
        .status()
        .expect("git must be runnable");
    assert!(status.success(), "git {args:?} failed in {}", root.display());
}

fn copy_tree(source: &Path, target: &Path) {
    std::fs::create_dir_all(target).expect("fixture directory");
    for entry in std::fs::read_dir(source).expect("fixture source") {
        let entry = entry.expect("fixture entry");
        let destination = target.join(entry.file_name());
        if entry.file_type().expect("fixture entry type").is_dir() {
            copy_tree(&entry.path(), &destination);
        } else {
            std::fs::copy(entry.path(), &destination).expect("copy fixture file");
        }
    }
}

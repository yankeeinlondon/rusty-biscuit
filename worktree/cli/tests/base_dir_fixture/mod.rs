//! A temporary repository and home for `wt create` base-directory tests.
//!
//! The child sees only the fixture: `HOME` and `USERPROFILE` point at the
//! fixture home (the config lookup is environment-first, and Windows reads
//! `USERPROFILE`), `WT` is set only when a test asks, and stdin is closed so
//! setup can never prompt.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

/// A base-directory value in the other operating system's syntax: the
/// incident's `b:\wt` on POSIX, a POSIX path on Windows.
pub const FOREIGN: &str = if cfg!(windows) { "/home/me/wt" } else { r"b:\wt" };

pub fn git(repo: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(repo).args(args).env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    String::from_utf8(output.stdout).unwrap()
}

pub struct Fixture {
    pub root: tempfile::TempDir,
    pub repo: PathBuf,
    pub home: PathBuf,
}

/// What `wt create` could have changed in the repository.
#[derive(Debug, PartialEq, Eq)]
pub struct RepoState {
    entries: Vec<String>,
    branches: String,
    worktrees: String,
}

impl Fixture {
    pub fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let home = root.path().join("home");
        fs::create_dir(&repo).unwrap();
        fs::create_dir(&home).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        let config = repo.join(".git").join("config");
        let mut text = fs::read_to_string(&config).unwrap();
        text.push_str("[user]\n\tname = Test User\n\temail = test@example.com\n[commit]\n\tgpgsign = false\n");
        fs::write(&config, text).unwrap();
        fs::write(repo.join("README.md"), b"initial\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-q", "-m", "initial"]);
        Self { root, repo, home }
    }

    pub fn config_path(&self) -> PathBuf {
        self.home.join(".worktree.json")
    }

    /// Writes `~/.worktree.json` with `base_dir` as given, JSON-escaped.
    pub fn write_config(&self, base_dir: &str) -> Vec<u8> {
        let bytes = serde_json::to_vec_pretty(&serde_json::json!({ "base_dir": base_dir })).unwrap();
        fs::write(self.config_path(), &bytes).unwrap();
        bytes
    }

    /// A directory under the fixture root, created.
    pub fn dir(&self, name: &str) -> PathBuf {
        let dir = self.root.path().join(name);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    /// Runs `wt create <branch>` with the shell wrapper announced, so a
    /// missing `cd:` line is the command's choice.
    pub fn create(&self, branch: &str, wt: Option<&str>) -> Output {
        let mut command = Command::new(biscuit_test_harness::bin_exe!("wt"));
        command
            .current_dir(&self.repo)
            .args(["create", branch])
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("NO_COLOR", "1")
            .env("XDG_CACHE_HOME", self.root.path().join("cache"))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env("WT_SHELL_WRAPPER", "1")
            .env_remove("CI")
            .env_remove("WT")
            .stdin(Stdio::null());
        if let Some(wt) = wt {
            command.env("WT", wt);
        }
        command.output().unwrap()
    }

    pub fn repo_state(&self) -> RepoState {
        let mut entries: Vec<String> = fs::read_dir(&self.repo)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        entries.sort();
        RepoState {
            entries,
            branches: git(&self.repo, &["for-each-ref", "--format=%(refname)", "refs/heads"]),
            worktrees: git(&self.repo, &["worktree", "list", "--porcelain"]),
        }
    }
}

/// Asserts the command was refused: exit 1, nothing on stdout (so no `cd:`
/// line), and the repository unchanged. Returns stderr.
pub fn assert_refused(output: &Output, fixture: &Fixture, before: &RepoState) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr).into_owned();
    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(output.stdout.is_empty(), "stdout: {}", String::from_utf8_lossy(&output.stdout));
    assert_eq!(&fixture.repo_state(), before, "{stderr}");
    stderr
}

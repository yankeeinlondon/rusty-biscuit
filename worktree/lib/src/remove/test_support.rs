//! Temporary repositories, with an optional local bare `origin`, for the
//! removal tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

pub struct TestRepo {
    dir: tempfile::TempDir,
}

fn run(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .expect("git should be installed");
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    } else {
        Err(String::from_utf8_lossy(&output.stderr).into_owned())
    }
}

fn configure(path: &Path) {
    for (key, value) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test User"),
        ("commit.gpgsign", "false"),
        ("tag.gpgsign", "false"),
        ("gc.auto", "0"),
    ] {
        run(path, &["config", key, value]).unwrap();
    }
}

impl TestRepo {
    /// A repository on `main` with one commit and no remote.
    pub fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let repo = dir.path().join("repo");
        fs::create_dir(&repo).unwrap();
        run(&repo, &["init", "-b", "main"]).unwrap();
        configure(&repo);
        let this = Self { dir };
        this.commit("README.md");
        this
    }

    /// [`TestRepo::new`] plus a bare `origin` holding `main`.
    pub fn with_origin() -> Self {
        let this = Self::new();
        let origin = this.origin_path();
        run(this.dir.path(), &["init", "--bare", "-b", "main", origin.to_str().unwrap()]).unwrap();
        this.git(&["remote", "add", "origin", origin.to_str().unwrap()]);
        this.git(&["push", "-q", "-u", "origin", "main"]);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().join("repo")
    }

    pub fn origin_path(&self) -> PathBuf {
        self.dir.path().join("origin.git")
    }

    pub fn git(&self, args: &[&str]) -> String {
        self.git_in(&self.path(), args)
    }

    pub fn git_in(&self, dir: &Path, args: &[&str]) -> String {
        run(dir, args).unwrap_or_else(|e| panic!("git {args:?} in {dir:?}: {e}"))
    }

    pub fn try_git(&self, args: &[&str]) -> Result<String, String> {
        run(&self.path(), args)
    }

    pub fn sha(&self, rev: &str) -> String {
        self.git(&["rev-parse", rev])
    }

    /// Commits a new file named `file` on the main checkout's branch.
    pub fn commit(&self, file: &str) -> String {
        self.commit_in(&self.path(), file)
    }

    pub fn commit_in(&self, dir: &Path, file: &str) -> String {
        fs::write(dir.join(file), format!("{file}\n")).unwrap();
        self.git_in(dir, &["add", file]);
        self.git_in(dir, &["commit", "-q", "-m", &format!("add {file}")]);
        self.git_in(dir, &["rev-parse", "HEAD"])
    }

    /// Adds a linked worktree for a new `branch` forked from `start`, in a
    /// sibling directory of the main checkout (never nested inside it).
    pub fn add_worktree(&self, branch: &str, dir_name: &str, start: &str) -> PathBuf {
        let path = self.dir.path().join("wts").join(dir_name);
        self.git(&["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), start]);
        configure(&path);
        path
    }

    /// Commits `file` onto `branch` in origin directly, as another person's
    /// push would, without touching this repository's refs.
    pub fn push_commit_to_origin(&self, branch: &str, file: &str) -> String {
        let pusher = self.dir.path().join("pusher");
        if pusher.exists() {
            self.git_in(&pusher, &["fetch", "-q", "origin"]);
        } else {
            let origin = self.origin_path();
            run(self.dir.path(), &["clone", "-q", origin.to_str().unwrap(), "pusher"]).unwrap();
            configure(&pusher);
        }
        let remote_ref = format!("origin/{branch}");
        if run(&pusher, &["rev-parse", "--verify", "--quiet", &remote_ref]).is_ok() {
            self.git_in(&pusher, &["checkout", "-q", "-B", branch, &remote_ref]);
        } else {
            self.git_in(&pusher, &["checkout", "-q", "-B", branch, "origin/main"]);
        }
        let sha = self.commit_in(&pusher, file);
        self.git_in(&pusher, &["push", "-q", "origin", &format!("HEAD:refs/heads/{branch}")]);
        sha
    }
}

//! Repositories for measuring the `graph gather` and
//! `graph image render (biscuit-terminal)` stages of `wt list --perf`.
//!
//! Each fixture is built once through `git fast-import` (so a 9,000-commit
//! history costs well under a second) and never changes afterward: the
//! before and after measurements of a graph change run on the same shapes.
//! Timestamps are fixed, so SHAs and activity ordering are reproducible.

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use assert_cmd::cargo::cargo_bin;

/// A repository with linked worktrees and the checkout `wt list` runs from.
pub struct GraphFixture {
    pub name: &'static str,
    _root: tempfile::TempDir,
    home: tempfile::TempDir,
    xdg_cache: tempfile::TempDir,
    main: PathBuf,
    run_from: PathBuf,
}

impl GraphFixture {
    /// One commit and no other worktree: the stages' fixed cost, such as a
    /// terminal query that waits out its timeout in a pseudo-terminal that
    /// never answers.
    pub fn floor() -> Self {
        let mut history = History::default();
        history.commit("main", None, None);
        Self::build("floor (one commit)", &history, &[], None)
    }

    /// Three worktrees over 50 commits: `main` plus two unmerged branches
    /// forked 10 and 4 commits back, listed from `main` (the base view).
    pub fn ordinary() -> Self {
        let mut history = History::default();
        let mut main = None;
        for _ in 0..50 {
            main = Some(history.commit("main", main, None));
        }
        let main = main.unwrap();
        let fork_a = history.ancestor(main, 10);
        let fork_b = history.ancestor(main, 4);
        let a = history.chain("feature-a", fork_a, 3);
        let b = history.chain("feature-b", fork_b, 2);
        Self::build("ordinary", &history, &[("feature-a", a), ("feature-b", b)], None)
    }

    /// A branch forked 5,000 first-parent commits back from `main`'s tip and
    /// merged 3,000 back, listed from that branch's own worktree (the focused
    /// view of a merged branch).
    pub fn older_connections() -> Self {
        let mut history = History::default();
        let mut main = None;
        for _ in 0..4_000 {
            main = Some(history.commit("main", main, None));
        }
        let fork = main.unwrap();
        let side = history.chain("old-merged", fork, 3);
        let mut main = fork;
        for _ in 0..1_999 {
            main = history.commit("main", Some(main), None);
        }
        let mut main = history.commit("main", Some(main), Some(side));
        for _ in 0..3_000 {
            main = history.commit("main", Some(main), None);
        }
        let _ = main;
        let fixture = Self::build("older essential connections", &history, &[("old-merged", side)], Some("old-merged"));
        let at = |rev: &str| git_output(&fixture.main, &["rev-parse", rev]);
        assert_eq!(at("main~5000"), at("old-merged~3"), "the fork is 5,000 first parents back");
        assert_eq!(at("main~3000^2"), at("old-merged"), "the merge is 3,000 first parents back");
        fixture
    }

    /// Eight worktrees: `main` plus seven branches, two of them merged into
    /// `main` through merge commits, listed from `main` (the base view).
    pub fn multiple_selected() -> Self {
        let mut history = History::default();
        let mut main = None;
        for _ in 0..20 {
            main = Some(history.commit("main", main, None));
        }
        let mut main = main.unwrap();
        let mut branches = Vec::new();
        for index in 0..2 {
            let name = format!("merged-{index}");
            let tip = history.chain(&name, main, 3);
            for _ in 0..2 {
                main = history.commit("main", Some(main), None);
            }
            main = history.commit("main", Some(main), Some(tip));
            branches.push((name, tip));
        }
        for index in 0..5 {
            let fork = history.ancestor(main, 2 * index);
            let name = format!("open-{index}");
            let tip = history.chain(&name, fork, 1 + index);
            branches.push((name, tip));
        }
        for _ in 0..3 {
            main = history.commit("main", Some(main), None);
        }
        let _ = main;
        let branches: Vec<(&str, usize)> = branches.iter().map(|(name, tip)| (name.as_str(), *tip)).collect();
        let fixture = Self::build("multiple selected branches", &history, &branches, None);
        assert_eq!(git_output(&fixture.main, &["rev-list", "--merges", "--count", "main"]), "2");
        assert_eq!(git_output(&fixture.main, &["worktree", "list", "--porcelain"]).matches("worktree ").count(), 8);
        fixture
    }

    /// The fixtures the graph before/after table reports, floor first.
    pub fn all() -> Vec<Self> {
        vec![Self::floor(), Self::ordinary(), Self::older_connections(), Self::multiple_selected()]
    }

    fn build(name: &'static str, history: &History, branches: &[(&str, usize)], run_from: Option<&str>) -> Self {
        let root = tempfile::tempdir().expect("create fixture root");
        let home = tempfile::tempdir().expect("create temp home");
        let xdg_cache = tempfile::tempdir().expect("create temp xdg cache");
        let main = root.path().join("repo");
        git(root.path(), &["init", "--quiet", "-b", "main", "repo"]);
        git(&main, &["config", "gc.auto", "0"]);
        history.import(&main, branches);
        git(&main, &["reset", "--quiet", "--hard", "main"]);

        let mut run_path = main.clone();
        for (branch, _) in branches {
            let path = root.path().join(format!("repo-{branch}"));
            git(&main, &["worktree", "add", "--quiet", path.to_str().unwrap(), branch]);
            if run_from == Some(*branch) {
                run_path = path;
            }
        }
        Self {
            name,
            _root: root,
            home,
            xdg_cache,
            main,
            run_from: run_path,
        }
    }

    pub fn main(&self) -> &Path {
        &self.main
    }

    /// The checkout `wt list` runs from.
    pub fn run_from(&self) -> &Path {
        &self.run_from
    }

    pub fn home(&self) -> &Path {
        self.home.path()
    }

    pub fn xdg_cache(&self) -> &Path {
        self.xdg_cache.path()
    }

    /// `wt` in [`run_from`](Self::run_from) with isolated cache roots and none
    /// of the user's or the system's git configuration.
    pub fn wt_command(&self) -> Command {
        let global = self.home.path().join("empty.gitconfig");
        fs::write(&global, "").expect("write an empty global git config");
        let mut command = Command::new(cargo_bin("wt"));
        command
            .current_dir(&self.run_from)
            .env("HOME", self.home.path())
            .env("XDG_CACHE_HOME", self.xdg_cache.path())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", global);
        command
    }
}

/// A commit graph written out through `git fast-import`. Commits are indexed
/// in creation order; each commit's time is one second after the previous.
#[derive(Default)]
struct History {
    /// `(branch it was written on, first parent, merged parent)` per commit.
    commits: Vec<(String, Option<usize>, Option<usize>)>,
}

impl History {
    fn commit(&mut self, branch: &str, parent: Option<usize>, merge: Option<usize>) -> usize {
        self.commits.push((branch.to_string(), parent, merge));
        self.commits.len() - 1
    }

    /// `count` commits on `branch` starting from `fork`; returns the tip.
    fn chain(&mut self, branch: &str, fork: usize, count: usize) -> usize {
        (0..count).fold(fork, |parent, _| self.commit(branch, Some(parent), None))
    }

    /// The commit `distance` first parents behind `tip`.
    fn ancestor(&self, tip: usize, distance: usize) -> usize {
        (0..distance).fold(tip, |at, _| self.commits[at].1.expect("history is deep enough"))
    }

    fn import(&self, repo: &Path, branches: &[(&str, usize)]) {
        let mut stream = String::new();
        for (index, (branch, parent, merge)) in self.commits.iter().enumerate() {
            let time = 1_700_000_000 + index;
            stream.push_str(&format!(
                "commit refs/heads/{branch}\nmark :{mark}\nauthor Perf <perf@example.invalid> {time} +0000\ncommitter Perf <perf@example.invalid> {time} +0000\ndata 2\nc\n",
                mark = index + 1
            ));
            if let Some(parent) = parent {
                stream.push_str(&format!("from :{}\n", parent + 1));
            }
            if let Some(merge) = merge {
                stream.push_str(&format!("merge :{}\n", merge + 1));
            }
            stream.push('\n');
        }
        for (branch, tip) in branches {
            stream.push_str(&format!("reset refs/heads/{branch}\nfrom :{}\n\n", tip + 1));
        }
        let mut child = Command::new("git")
            .current_dir(repo)
            .args(["fast-import", "--quiet"])
            .stdin(Stdio::piped())
            .spawn()
            .expect("git fast-import should start");
        child
            .stdin
            .take()
            .expect("fast-import stdin")
            .write_all(stream.as_bytes())
            .expect("write the fast-import stream");
        assert!(child.wait().expect("wait for fast-import").success(), "git fast-import failed");
    }
}

fn git(dir: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(dir)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {dir:?}");
}

fn git_output(dir: &Path, args: &[&str]) -> String {
    let output = Command::new("git").current_dir(dir).args(args).output().expect("git should be installed");
    assert!(output.status.success(), "git {args:?} failed in {dir:?}");
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

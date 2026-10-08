//! `hug` checks for the `--changed` file selection: the parser surface, the
//! acceptance-table selection rows, Git failure exits, the empty-selection
//! representations of every report mode, the cache shared with ordinary
//! scans, and path spellings.
//!
//! Every fixture repository is initialized on an explicit `main` branch and
//! commits are created through the `git2` dev-dependency (no signing, no
//! subprocess), mirroring the helpers of sniff's `changed_paths_tests.rs`.
//! Every helper opens a fresh handle per mutation: libgit2 caches one index
//! per repository handle, so a handle opened before another handle staged a
//! path can commit a tree that lacks it.

mod cache;
mod empty;
mod failure;
mod index_corruption;
mod parser;
mod paths;
mod ownership;
mod selection;
mod type_changes;

use std::path::Path;
use std::process::Output;

use assert_cmd::Command;
use serde_json::Value;
use tempfile::TempDir;

/// The branch every fixture repository is initialized on, so no result
/// depends on the host's `init.defaultBranch`.
pub const MAIN_BRANCH: &str = "refs/heads/main";

fn signature() -> git2::Signature<'static> {
    git2::Signature::now("changed fixture", "fixture@example.com").unwrap()
}

/// A scratch Git repository with commits built through library APIs, plus a
/// test-owned home and temp directory so no `hug` run sees the caller's
/// cache, checkout or configuration.
pub struct Repo {
    pub dir: TempDir,
    pub home: TempDir,
    pub temp: TempDir,
}

impl Repo {
    /// A repository on `main` whose first commit holds `files`.
    pub fn committed(files: &[(&str, &str)]) -> Self {
        let repo = Self::unborn();
        commit_files(repo.dir.path(), "fixture commit", MAIN_BRANCH, files);
        repo
    }

    /// A repository on `main` with no commits.
    pub fn unborn() -> Self {
        let repo = Self {
            dir: TempDir::new().expect("scratch repo"),
            home: TempDir::new().expect("scratch home"),
            temp: TempDir::new().expect("scratch temp"),
        };
        git_init(repo.dir.path());
        repo
    }

    /// Runs `hug` in the repository root.
    pub fn hug(&self, args: &[&str]) -> Output {
        self.hug_in(self.dir.path(), args)
    }

    /// Runs `hug` with the persistent cache under a different temp
    /// directory, keeping this fixture's other isolation.
    pub fn hug_with_temp(&self, temp: &Path, args: &[&str]) -> Output {
        run_hug(temp, self.dir.path(), args)
    }

    /// Runs `hug` in `dir` (a subdirectory, worktree or unrelated path)
    /// with this fixture's isolation.
    pub fn hug_in(&self, dir: &Path, args: &[&str]) -> Output {
        run_hug(self.temp.path(), dir, args)
    }

    /// Stdout of a successful run, failing with stderr otherwise.
    pub fn stdout(&self, args: &[&str]) -> String {
        stdout_of(self.hug(args), args)
    }

    /// The JSON document of a successful run.
    pub fn json(&self, args: &[&str]) -> Value {
        json_of(&self.stdout(args))
    }

    /// The JSON document of a successful run in `dir`.
    pub fn json_in(&self, dir: &Path, args: &[&str]) -> Value {
        json_of(&stdout_of(self.hug_in(dir, args), args))
    }

    /// Writes `relative` with `contents`, creating parent directories.
    pub fn write(&self, relative: &str, contents: &str) {
        write_file(self.dir.path(), relative, contents);
    }

    /// Stages `relative` (`git add <path>`).
    pub fn stage(&self, relative: &str) {
        stage(self.dir.path(), relative);
    }
}

/// One `hug` process with full environment isolation: test-owned `HOME`
/// and config roots (gix reads user configuration), the persistent cache
/// under `temp`, plain output, and no prompts.
fn run_hug(temp: &Path, current_dir: &Path, args: &[&str]) -> Output {
    let config = temp.join("isolation-config");
    let mut command = Command::new(biscuit_test_harness::bin_exe!("hug"));
    command
        .args(args)
        .current_dir(current_dir)
        .env("HOME", temp.join("isolation-home"))
        .env("USERPROFILE", temp.join("isolation-home"))
        .env("GIT_TERMINAL_PROMPT", "0")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("XDG_CONFIG_HOME", &config)
        .env("APPDATA", &config)
        .env("LOCALAPPDATA", &config)
        .env("TMPDIR", temp)
        .env("TMP", temp)
        .env("TEMP", temp)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env("RAYON_NUM_THREADS", "2");
    command.output().expect("run hug")
}

/// Stdout of a successful run, failing with stderr otherwise.
pub fn stdout_of(output: Output, args: &[&str]) -> String {
    assert!(
        output.status.success(),
        "hug {args:?} failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).expect("utf-8 stdout")
}

/// Parses `hug` JSON stdout.
pub fn json_of(stdout: &str) -> Value {
    serde_json::from_str(stdout).unwrap_or_else(|error| panic!("invalid JSON ({error}):\n{stdout}"))
}

/// `(file, record names)` of a `--json` declaration report, in report order.
pub fn files_and_names(report: &Value) -> Vec<(String, Vec<String>)> {
    report["files"]
        .as_array()
        .expect("files")
        .iter()
        .map(|file| {
            (
                file["file"].as_str().expect("file path").to_string(),
                file["symbols"]
                    .as_array()
                    .expect("symbols")
                    .iter()
                    .map(|record| record["identity"]["name"].as_str().expect("name").to_string())
                    .collect(),
            )
        })
        .collect()
}

/// The `--verbose` cache-stats line of a run's stderr.
pub fn cache_stats(output: &Output) -> String {
    let stderr = String::from_utf8_lossy(&output.stderr);
    stderr
        .lines()
        .find(|line| line.starts_with("cache: "))
        .unwrap_or_else(|| panic!("no cache stats line in:\n{stderr}"))
        .to_string()
}

// ─────────────────────────────────────────────────────────────────────────
// git2 fixture helpers (fresh handle per call)
// ─────────────────────────────────────────────────────────────────────────

/// Initializes a repository on the explicit [`MAIN_BRANCH`] branch.
fn git_init(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.initial_head(MAIN_BRANCH);
    git2::Repository::init_opts(dir, &opts).unwrap()
}

/// Initializes a bare repository.
fn git_init_bare(dir: &Path) -> git2::Repository {
    let mut opts = git2::RepositoryInitOptions::new();
    opts.bare(true);
    opts.initial_head(MAIN_BRANCH);
    git2::Repository::init_opts(dir, &opts).unwrap()
}

/// Writes `relative` under `repo_path`, creating parent directories.
fn write_file(repo_path: &Path, relative: &str, contents: &str) {
    let full = repo_path.join(relative);
    if let Some(parent) = full.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(full, contents).unwrap();
}

/// Stages `relative` (`git add <path>`).
fn stage(repo_path: &Path, relative: &str) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.add_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

/// Stages a deletion of `relative` (`git rm --cached`).
fn stage_deletion(repo_path: &Path, relative: &str) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let mut index = repo.index().unwrap();
    index.remove_path(Path::new(relative)).unwrap();
    index.write().unwrap();
}

/// Writes files and commits them on `reference` with HEAD's commit as
/// parent (none for the first commit).
fn commit_files(repo_path: &Path, message: &str, reference: &str, files: &[(&str, &str)]) -> git2::Oid {
    for (path, content) in files {
        write_file(repo_path, path, content);
        stage(repo_path, path);
    }
    commit_index(repo_path, message, reference)
}

/// Commits the current on-disk index on `reference`.
fn commit_index(repo_path: &Path, message: &str, reference: &str) -> git2::Oid {
    let repo = git2::Repository::open(repo_path).unwrap();
    let sig = signature();
    let tree_id = {
        let mut index = repo.index().unwrap();
        index.write_tree().unwrap()
    };
    let tree = repo.find_tree(tree_id).unwrap();
    let parent = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some(reference), &sig, &sig, message, &tree, &parents)
        .unwrap()
}

/// Commits on a named branch stacking on the current HEAD tip, points HEAD
/// at that branch, and resets the on-disk index to HEAD's tree so one
/// branch's staged files do not leak into the next branch's commit.
fn commit_on_branch(repo_path: &Path, branch: &str, message: &str, files: &[(&str, &str)]) -> git2::Oid {
    let reference = format!("refs/heads/{branch}");
    let repo = git2::Repository::open(repo_path).unwrap();
    let head_target = repo.head().unwrap().target().unwrap();
    drop(repo);
    let repo = git2::Repository::open(repo_path).unwrap();
    repo.reference(&reference, head_target, true, "branch fixture")
        .unwrap();
    repo.set_head(&reference).unwrap();
    drop(repo);
    reset_index_to_head(repo_path);
    commit_files(repo_path, message, &reference, files)
}

/// Points HEAD at `branch` and resets the index to that branch's tree.
///
/// The working tree is left alone: SAFE checkout refuses to overwrite
/// untracked files, and every test re-writes (or removes) exactly the
/// files it asserts on.
fn checkout_branch(repo_path: &Path, branch: &str) {
    let reference = format!("refs/heads/{branch}");
    let repo = git2::Repository::open(repo_path).unwrap();
    repo.set_head(&reference).unwrap();
    drop(repo);
    reset_index_to_head(repo_path);
}

/// Resets the on-disk index to HEAD's tree.
fn reset_index_to_head(repo_path: &Path) {
    let repo = git2::Repository::open(repo_path).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    let tree = repo.find_tree(commit.tree_id()).unwrap();
    let mut index = repo.index().unwrap();
    index.read_tree(&tree).unwrap();
    index.write().unwrap();
}

/// Detaches HEAD at `commit`.
fn detach_head(repo_path: &Path, commit: git2::Oid) {
    let repo = git2::Repository::open(repo_path).unwrap();
    repo.set_head_detached(commit).unwrap();
}

/// Writes unmerged index entries at stages 1-3 for `path`.
fn conflict_at(repo: &git2::Repository, path: &str, blob: &str) {
    let oid = repo.blob(blob.as_bytes()).unwrap();
    let mut index = repo.index().unwrap();
    for stage in 1..=3u16 {
        index
            .add(&git2::IndexEntry {
                ctime: git2::IndexTime::new(0, 0),
                mtime: git2::IndexTime::new(0, 0),
                dev: 0,
                ino: 0,
                mode: 0o100644,
                uid: 0,
                gid: 0,
                file_size: 0,
                id: oid,
                flags: stage << 12,
                flags_extended: 0,
                path: path.as_bytes().to_vec(),
            })
            .unwrap();
    }
    index.write().unwrap();
}

/// Creates a linked worktree of the repository at `repo_path` named `name`
/// at `path`, returning nothing (the path is caller-owned).
fn add_worktree(repo_path: &Path, name: &str, path: &Path) {
    let repo = git2::Repository::open(repo_path).unwrap();
    repo.worktree(name, path, None).unwrap();
}

/// The tree id of HEAD, as a base token that resolves to a non-commit.
fn head_tree_id(repo_path: &Path) -> String {
    let repo = git2::Repository::open(repo_path).unwrap();
    let commit = repo.head().unwrap().peel_to_commit().unwrap();
    commit.tree_id().to_string()
}

/// A missing-repository fixture must never silently become a passing skip.
pub fn directory_outside_any_repository() -> TempDir {
    let dir = TempDir::new().unwrap();
    assert!(git2::Repository::discover(dir.path()).is_err(), "fixture temp root must be outside a repository");
    dir
}

//! Temporary repositories, with an optional local bare `origin`, for the
//! removal tests.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Duration;
#[cfg(windows)]
use std::process::Command;

use super::clock::{Clock, Slept};
use super::faults::{FsFaults, FsOp};
use super::mounts::{Mount, MountInfo};

pub struct TestRepo {
    dir: tempfile::TempDir,
}

fn run(cwd: &Path, args: &[&str]) -> Result<String, String> {
    let output = crate::git::command()
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
    crate::test_support::configure(
        path,
        &[
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("tag.gpgsign", "false"),
            ("gc.auto", "0"),
        ],
    );
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
        this.add_and_commit(&this.path(), "README.md");
        this
    }

    /// [`TestRepo::new`] plus a bare `origin` holding `main`, which `main`
    /// tracks, as `git push -u origin main` leaves them.
    pub fn with_origin() -> Self {
        let this = Self::new();
        let origin = this.origin_path();
        // A local bare clone hardlinks the objects in one process; `init
        // --bare` and a push start several (the push's receive-pack and
        // pack-objects among them).
        run(this.dir.path(), &["clone", "-q", "--bare", this.path().to_str().unwrap(), origin.to_str().unwrap()]).unwrap();
        // What `git remote add origin <path>` and `push -u` write.
        crate::test_support::configure(
            &this.path(),
            &[
                ("remote.origin.url", origin.to_str().unwrap()),
                ("remote.origin.fetch", "+refs/heads/*:refs/remotes/origin/*"),
                ("branch.main.remote", "origin"),
                ("branch.main.merge", "refs/heads/main"),
            ],
        );
        this.git(&["update-ref", "refs/remotes/origin/main", "main"]);
        this
    }

    pub fn path(&self) -> PathBuf {
        self.dir.path().join("repo")
    }

    pub fn cache_path(&self) -> PathBuf {
        self.dir.path().join("cache")
    }

    pub fn write_ignored(&self, name: &str, contents: &[u8]) {
        fs::write(self.path().join(name), contents).unwrap();
    }

    pub fn write_include_rules(&self, contents: &[u8]) {
        fs::write(self.path().join(".worktreeinclude"), contents).unwrap();
    }

    pub fn add_linked_worktree(&self, branch: &str) -> PathBuf {
        self.add_worktree(branch, branch, "main")
    }

    pub fn with_global_excludes(&self, contents: &[u8]) -> PathBuf {
        let path = self.dir.path().join("global-excludes");
        fs::write(&path, contents).unwrap();
        crate::test_support::configure(&self.path(), &[("core.excludesFile", path.to_str().unwrap())]);
        path
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
        self.add_and_commit(dir, file);
        // In-process, as `rev-parse HEAD` would answer it (parity-tested in
        // `git_metadata`): one process fewer per commit.
        crate::git_metadata::reference_target(dir, "HEAD")
            .flatten()
            .unwrap_or_else(|| self.git_in(dir, &["rev-parse", "HEAD"]))
    }

    /// What `git push -q origin <branch>` leaves, without its processes
    /// (`receive-pack` and its helpers, several on Windows): origin borrows
    /// this repository's objects (an `alternates` entry) and gets the branch
    /// at its local tip, and the tracking ref follows.
    pub fn publish(&self, branch: &str) {
        let refname = format!("refs/heads/{branch}");
        let tip = crate::git_metadata::reference_target(&self.path(), &refname).flatten().unwrap_or_else(|| self.sha(&refname));
        let alternates = self.origin_path().join("objects").join("info").join("alternates");
        fs::create_dir_all(alternates.parent().unwrap()).unwrap();
        let objects = self.path().join(".git").join("objects");
        fs::write(&alternates, format!("{}\n", objects.to_str().unwrap().replace('\\', "/"))).unwrap();
        for (git_dir, name) in [(self.origin_path(), format!("refs/heads/{branch}")), (self.path().join(".git"), format!("refs/remotes/origin/{branch}"))] {
            let reference = git_dir.join(&name);
            fs::create_dir_all(reference.parent().unwrap()).unwrap();
            fs::write(reference, format!("{tip}\n")).unwrap();
        }
    }

    /// [`TestRepo::commit_in`] without reading back the new SHA.
    fn add_and_commit(&self, dir: &Path, file: &str) {
        fs::write(dir.join(file), format!("{file}\n")).unwrap();
        self.git_in(dir, &["add", file]);
        self.git_in(dir, &["commit", "-q", "-m", &format!("add {file}")]);
    }

    /// Adds a linked worktree for a new `branch` forked from `start`, in a
    /// sibling directory of the main checkout (never nested inside it).
    pub fn add_worktree(&self, branch: &str, dir_name: &str, start: &str) -> PathBuf {
        let path = self.dir.path().join("wts").join(dir_name);
        // No `configure`: a linked worktree shares the main repository's
        // config, which already holds every fixture setting.
        self.git(&["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), start]);
        path
    }

    /// Commits `file` onto `branch` in origin directly, as another person's
    /// push would, without touching this repository's refs. A branch origin
    /// does not have starts from its `main`. One `git fast-import`
    /// ([`crate::test_support::commit_to_origin`]), where a clone, a commit,
    /// and a push start about a dozen processes.
    pub fn push_commit_to_origin(&self, branch: &str, file: &str) -> String {
        crate::test_support::commit_to_origin(&self.origin_path(), branch, file)
    }
}

/// Moves the directory at `path` to its sibling `saved_name` and puts a
/// directory link to it at `path`: a symbolic link, or on Windows a junction,
/// which needs no privilege. Returns the moved directory.
pub fn replace_with_link(path: &Path, saved_name: &str) -> PathBuf {
    let saved = path.with_file_name(saved_name);
    fs::rename(path, &saved).unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&saved, path).unwrap();
    #[cfg(windows)]
    {
        // `path` may be git's spelling (`C:/Users/...`), and `cmd` reads
        // `/Users` as a switch.
        let native = |path: &Path| path.to_string_lossy().replace('/', "\\");
        let status = Command::new("cmd")
            .args(["/C", "mklink", "/J"])
            .arg(native(path))
            .arg(native(&saved))
            .stdout(std::process::Stdio::null())
            .status()
            .expect("cmd runs");
        assert!(status.success(), "mklink /J needs no privilege");
    }
    saved
}

/// Links `link` to the directory `target`: a symlink on Unix; on Windows a
/// symlink where this host may create one, else a junction, which needs no
/// privilege and is removed the same way.
pub fn link_dir(target: &Path, link: &Path) {
    #[cfg(unix)]
    std::os::unix::fs::symlink(target, link).unwrap();
    #[cfg(windows)]
    if std::os::windows::fs::symlink_dir(target, link).is_err() {
        let status = Command::new("cmd")
            .arg("/C")
            .arg("mklink")
            .arg("/J")
            .arg(link)
            .arg(target)
            .stdout(std::process::Stdio::null())
            .status()
            .expect("cmd runs");
        assert!(status.success(), "mklink /J {} {}", link.display(), target.display());
    }
}

/// A [`Clock`] that records each requested sleep
/// instead of taking it, and can cancel from a chosen sleep on.
#[derive(Debug, Default)]
pub struct ManualClock {
    slept: Mutex<Vec<Duration>>,
    /// Sleeps after this many report cancellation.
    cancel_after: Option<usize>,
}

impl ManualClock {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn cancelled_after(sleeps: usize) -> Self {
        Self { cancel_after: Some(sleeps), ..Self::default() }
    }

    /// The sleeps taken, in order; a cancelled one is not listed.
    pub fn slept(&self) -> Vec<Duration> {
        self.slept.lock().unwrap().clone()
    }
}

impl Clock for ManualClock {
    fn sleep(&self, duration: Duration, cancelled: &dyn Fn() -> bool) -> Slept {
        let mut slept = self.slept.lock().unwrap();
        if cancelled() || self.cancel_after.is_some_and(|after| slept.len() >= after) {
            return Slept::Cancelled;
        }
        slept.push(duration);
        Slept::Elapsed
    }
}

/// A fixed mount table.
#[derive(Debug, Clone, Default)]
pub struct FixedMounts(pub Vec<Mount>);

impl MountInfo for FixedMounts {
    fn mounts(&self) -> std::io::Result<Vec<Mount>> {
        Ok(self.0.clone())
    }
}

/// One scripted fault: fires while `remaining` is `None` (always) or above
/// zero.
struct Fault {
    op: FsOp,
    path: PathBuf,
    remaining: Option<usize>,
    fired: usize,
    make: fn() -> std::io::Error,
}

/// [`FsFaults`] from a script of (operation, path)
/// faults, each temporary (a fixed number of times) or permanent.
#[derive(Default)]
pub struct ScriptedFaults {
    faults: Mutex<Vec<Fault>>,
}

impl ScriptedFaults {
    pub fn new() -> Self {
        Self::default()
    }

    /// Fails `op` on `path` the next `count` times, then lets it through.
    pub fn times(self, op: FsOp, path: &Path, count: usize, make: fn() -> std::io::Error) -> Self {
        self.push(op, path, Some(count), make)
    }

    /// Fails every `op` on `path`.
    pub fn always(self, op: FsOp, path: &Path, make: fn() -> std::io::Error) -> Self {
        self.push(op, path, None, make)
    }

    fn push(self, op: FsOp, path: &Path, remaining: Option<usize>, make: fn() -> std::io::Error) -> Self {
        self.faults.lock().unwrap().push(Fault { op, path: path.to_path_buf(), remaining, fired: 0, make });
        self
    }

    /// How many times the fault for `op` on `path` fired.
    pub fn fired(&self, op: FsOp, path: &Path) -> usize {
        self.faults.lock().unwrap().iter().filter(|fault| fault.op == op && fault.path == path).map(|fault| fault.fired).sum()
    }
}

impl FsFaults for ScriptedFaults {
    fn inject(&self, op: FsOp, path: &Path) -> Option<std::io::Error> {
        let mut faults = self.faults.lock().unwrap();
        let fault = faults.iter_mut().find(|fault| fault.op == op && fault.path == path && fault.remaining != Some(0))?;
        if let Some(remaining) = &mut fault.remaining {
            *remaining -= 1;
        }
        fault.fired += 1;
        Some((fault.make)())
    }
}

/// [`Holders`](super::delete::Holders) that answers from a script, one
/// report per query (the last repeats), and records what each query asked
/// about.
#[derive(Default)]
pub struct ScriptedHolders {
    reports: Mutex<Vec<super::delete::HolderReport>>,
    asked: Mutex<Vec<Vec<PathBuf>>>,
}

impl ScriptedHolders {
    /// Answers every query with [`HolderReport::Unknown`](super::delete::HolderReport::Unknown).
    pub fn new() -> Self {
        Self::default()
    }

    pub fn answering(reports: Vec<super::delete::HolderReport>) -> Self {
        Self { reports: Mutex::new(reports), ..Self::default() }
    }

    /// The blocked paths of each query, in order.
    pub fn asked(&self) -> Vec<Vec<PathBuf>> {
        self.asked.lock().unwrap().clone()
    }
}

impl super::delete::Holders for ScriptedHolders {
    fn query(&self, _root: &Path, blocked: &[PathBuf]) -> super::delete::HolderReport {
        self.asked.lock().unwrap().push(blocked.to_vec());
        let mut reports = self.reports.lock().unwrap();
        match reports.len() {
            0 => super::delete::HolderReport::Unknown,
            1 => reports[0].clone(),
            _ => reports.remove(0),
        }
    }
}

/// A [`UsageSource`](super::usage::UsageSource) that sees no other process
/// and complete coverage.
#[derive(Debug, Clone, Copy, Default)]
pub struct ClearUsage;

impl super::usage::UsageSource for ClearUsage {
    fn query(
        &self,
        path: &Path,
    ) -> Result<sniff::filesystem::query::PathUsageReport, sniff::filesystem::query::PathUsageError> {
        Ok(clear_report(path))
    }
}

/// A complete, usable usage report about `path` with no processes.
pub fn clear_report(path: &Path) -> sniff::filesystem::query::PathUsageReport {
    serde_json::from_value(serde_json::json!({
        "schema_version": 1,
        "target": {
            "requested": {"display": path},
            "resolved": {"display": path},
            "kind": "directory",
            "identity": {"kind": "unix", "device": "7", "inode": "3"},
            "recursive": true
        },
        "outcome": "usable",
        "started_at": "2026-10-22T00:00:00Z",
        "finished_at": "2026-10-22T00:00:01Z",
        "budget_ms": 2000,
        "elapsed_us": 1000,
        "budget_exhausted": false,
        "processes": [],
        "coverage": [],
        "limitations": []
    }))
    .expect("a valid usage report")
}

//! The repository the end-to-end parity tests push from: the port of the
//! retired shell suite's `make_publishing_repo`, `stage_fixture_tools`,
//! `make_stacked_repo`, and `stage_ci_local_harness`.
//!
//! The checkout commits the real evidence helpers `ci-push` runs
//! (`scripts/ci/local_evidence.py`, `observation_notes.py`, and the helper
//! modules they import) and, as `scripts/ci/affected_scope.py`, the
//! stub planner in `scripts/ci/fixtures/pre-push/affected_scope_stub.py`: the
//! real planner needs the Cargo workspace it lives in. `main` is on a bare
//! `origin.git`; `feature` is checked out one commit ahead.
//!
//! `ci-push` runs with a cleared environment: `PATH` is the stub directory
//! (`just`, `gh`, `sniff` copies of `ci-push-test-tool`) followed by the
//! directories of the real tools a run needs, and home, temporary, evidence,
//! and constraint directories all live in the fixture.
//!
//! The CI environment is the host's (`ci-push` reads it through sniff), so
//! every plan fixture is relabeled before use: its `macos-latest` and the
//! host's environment trade names. On macOS that is the identity.

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use serde_json::{json, Map, Value};
use tempfile::TempDir;

use crate::fixture::Fixture;

pub const ZERO: &str = "0000000000000000000000000000000000000000";
/// A GitHub-shaped URL for the remote argument; transport still goes to
/// `origin`, which is the bare fixture remote.
pub const GITHUB_URL: &str = "git@github.com:acme/widgets.git";
/// The reason every whole-environment prohibition here states.
pub const PROHIBITION_REASON: &str = "do not rerun WSL for this branch";

const STUB_PLANNER: &str = include_str!("../../ci/fixtures/pre-push/affected_scope_stub.py");

/// The host's CI environment, as `ci-push` derives it.
pub fn host_environment() -> &'static str {
    use sniff::os::OsType;
    let os = sniff::os::detect_os_with_request(&sniff::request::OsRequest::summary())
        .expect("sniff reads this host's operating system");
    match os.os_type {
        OsType::MacOS => "macos-latest",
        OsType::Windows => "windows-latest",
        OsType::Linux if os.kernel.to_ascii_lowercase().contains("microsoft") => "wsl2-ubuntu",
        OsType::Linux => "ubuntu-latest",
        other => panic!("this host ({other:?}) has no CI environment"),
    }
}

/// `sniff os --json` as the `ci-local` recipe reads it, for this host.
fn sniff_os_json(environment: &str) -> String {
    let (os, kernel) = match environment {
        "macos-latest" => ("MacOS", "Darwin"),
        "windows-latest" => ("Windows", "Windows_NT"),
        "wsl2-ubuntu" => ("Linux", "5.15.167.4-microsoft-standard-WSL2"),
        _ => ("Linux", "Linux"),
    };
    json!({ "os_type": os, "kernel": kernel }).to_string()
}

/// `plan` with `macos-latest` and the host's environment trading names.
pub fn relabel(plan: &str) -> String {
    let host = host_environment();
    if host == "macos-latest" {
        return plan.to_owned();
    }
    plan.replace("macos-latest", "\u{0}")
        .replace(host, "macos-latest")
        .replace('\u{0}', host)
}

/// The first program called `name` on the inherited `PATH`, outside any
/// Cygwin directory on Windows. An L1 test needs it, so a missing one fails
/// loudly rather than skipping.
pub fn real_tool(name: &str) -> PathBuf {
    find_tool(name).unwrap_or_else(|| {
        panic!(
            "`{name}` must be on PATH: this L1 test runs the real {name} (CI's runners provide it)"
        )
    })
}

fn find_tool(name: &str) -> Option<PathBuf> {
    let path = std::env::var_os("PATH")?;
    let names: Vec<String> = if cfg!(windows) {
        vec![
            format!("{name}.exe"),
            format!("{name}.cmd"),
            name.to_owned(),
        ]
    } else {
        vec![name.to_owned()]
    };
    std::env::split_paths(&path)
        // A Cygwin program under the recipe's Git Bash (MSYS) mixes two POSIX
        // runtimes; Cygwin's `jq.exe` there died with 139 under parallel load.
        .filter(|dir| !(cfg!(windows) && dir.join("cygwin1.dll").is_file()))
        .flat_map(|dir| names.iter().map(move |file| dir.join(file)))
        .find(|candidate| candidate.is_file())
}

/// The Git programs whose directories end the fixture `PATH`: the real
/// `git`, and on Windows the binary behind its launcher. Git for Windows'
/// `<top>\cmd\git.exe` starts `<top>\<prefix>\bin\git.exe` as a second
/// process for every call, tens of milliseconds each there, and a push runs
/// dozens of Git commands; the fixture runs the binary directly, as a host
/// whose `PATH` names Git's `<prefix>\bin` does, followed by the `usr\bin`
/// the launcher would add (its `sh` runs a local transport).
fn fixture_git() -> Vec<PathBuf> {
    let git = real_tool("git");
    if cfg!(windows) {
        let top = git
            .parent()
            .filter(|directory| directory.file_name().is_some_and(|name| name == "cmd"))
            .and_then(Path::parent);
        if let Some(top) = top {
            let direct = ["mingw64", "ucrt64", "clangarm64"]
                .iter()
                .map(|prefix| top.join(prefix).join("bin").join("git.exe"))
                .find(|candidate| candidate.is_file());
            let sh = top.join("usr").join("bin").join("sh.exe");
            if let Some(direct) = direct.filter(|_| sh.is_file()) {
                return vec![direct, sh];
            }
        }
    }
    vec![git]
}

/// The Bash a `#!/usr/bin/env bash` recipe runs under. On Windows that is
/// Git for Windows' `usr\bin\bash.exe`, found from the real Git, never WSL's
/// `System32\bash.exe`; `just` also needs that directory's `cygpath` to
/// translate the shebang.
fn recipe_bash() -> PathBuf {
    if !cfg!(windows) {
        return real_tool("bash");
    }
    let git = real_tool("git");
    git.ancestors()
        .map(|top| top.join("usr").join("bin").join("bash.exe"))
        .find(|bash| bash.is_file())
        .unwrap_or_else(|| {
            panic!(
                "{} is not part of a Git for Windows installation (no usr\\bin\\bash.exe above it)",
                git.display()
            )
        })
}

/// A Python 3.10 or newer from the inherited `PATH`, as `ci-push` will find
/// it once its directory leads the fixture `PATH`. With `compile`, the same
/// interpreter start also writes the bytecode of every module in that
/// directory beside it (in `__pycache__`, which a fixture checkout ignores).
/// `ci-push` runs its helpers with `PYTHONDONTWRITEBYTECODE=1`, which stops
/// them writing bytecode but not reading it: without this, every helper
/// start recompiles thousands of lines, about 55 ms each on native Windows.
fn real_python(compile: Option<&Path>) -> PathBuf {
    for name in ["python3", "python"] {
        let Some(candidate) = find_tool(name) else {
            continue;
        };
        let mut probe = Command::new(&candidate);
        probe.args([
            "-c",
            "import sys\n\
             if sys.version_info < (3, 10): sys.exit(1)\n\
             if len(sys.argv) > 1:\n\
             \x20   import compileall\n\
             \x20   sys.exit(0 if compileall.compile_dir(sys.argv[1], quiet=1) else 86)\n",
        ]);
        if let Some(directory) = compile {
            probe.arg(directory);
        }
        let output = probe.stdin(Stdio::null()).output();
        match output.map(|output| output.status.code()) {
            Ok(Some(0)) => return candidate,
            Ok(Some(86)) => panic!("{} could not compile {compile:?}", candidate.display()),
            _ => continue,
        }
    }
    panic!("python3 (3.10 or newer) must be on PATH: this L1 test runs the real evidence helpers");
}

/// What a fixture `_test <pkg>` does in the real-recipe runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TestGate {
    Pass,
    /// A complete failure: one failing test and a nonzero exit.
    Fail,
    /// Appends a truncated manifest line and exits 1.
    MalformedManifest,
    /// Records exit 143 in the manifest and exits 1, as a tier recipe that
    /// folds TERM's status does.
    Recorded143,
    Exit130,
}

/// One `ci-push` invocation's result.
pub struct Run {
    pub code: i32,
    pub stdout: String,
    pub stderr: String,
}

impl Run {
    /// Everything the run printed: `ci-push`'s diagnostics are on stderr, the
    /// gates' own output on whichever stream they wrote.
    pub fn all(&self) -> String {
        format!("{}\n{}", self.stdout, self.stderr)
    }

    pub fn says(&self, text: &str) -> bool {
        self.all().contains(text)
    }

    #[track_caller]
    pub fn assert_says(&self, text: &str) {
        assert!(self.says(text), "expected {text:?} in the output:\n{self}");
    }

    #[track_caller]
    pub fn assert_silent_on(&self, text: &str) {
        assert!(
            !self.says(text),
            "expected no {text:?} in the output:\n{self}"
        );
    }

    #[track_caller]
    pub fn assert_exit(&self, code: i32) {
        assert_eq!(self.code, code, "unexpected exit status:\n{self}");
    }
}

impl std::fmt::Display for Run {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "--- exit {} ---\n--- stdout ---\n{}\n--- stderr ---\n{}",
            self.code, self.stdout, self.stderr
        )
    }
}

/// What a gate run leaves for `ci-push` to read: its exit status and every
/// file below `BISCUIT_CI_REPORTS_OUT`, keyed by `/`-separated relative path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateOutcome {
    pub code: i32,
    pub reports: BTreeMap<String, String>,
}

/// What `local_evidence.py verify --cells` decided.
#[derive(Debug)]
pub struct Verified {
    pub accepted: Vec<Value>,
    pub rejections: Vec<String>,
}

impl Verified {
    pub fn accepts(&self, package: &str) -> bool {
        self.accepted.iter().any(|cell| cell["package"] == package)
    }

    #[track_caller]
    pub fn assert_rejects(&self, prefix: &str) {
        assert!(
            self.rejections.iter().any(|rejection| rejection.starts_with(prefix)),
            "no rejection starts with {prefix:?}: {self:#?}"
        );
    }
}

/// One direct run of the real recipe ([`Evidence::run_recipe`]).
pub struct RecipeRun {
    pub run: Run,
    pub outcome: GateOutcome,
    /// What it wrote to `BISCUIT_CI_PLAN_OUT`, if anything.
    pub plan_out: Option<String>,
}

pub struct Evidence {
    root: TempDir,
    pub repo: PathBuf,
    pub origin: PathBuf,
    pub home: PathBuf,
    pub evidence: PathBuf,
    pub constraints: PathBuf,
    pub tools: Fixture,
    pub environment: &'static str,
    pub python: PathBuf,
    /// `main`'s commit, which is also `origin/main` until a test moves it.
    pub main: git2::Oid,
    behaviors: RefCell<Map<String, Value>>,
    planner_plan: RefCell<Option<PathBuf>>,
    extra_path: RefCell<Vec<PathBuf>>,
    planner_hang: RefCell<Option<PathBuf>>,
    /// The recipe's self-test suites, once the harness is staged.
    self_test_suites: RefCell<Vec<String>>,
    /// How many [`Evidence::helpers`] runs have started.
    helper_runs: std::cell::Cell<usize>,
}

impl Evidence {
    /// `make_publishing_repo` with `work.txt` as the feature commit.
    pub fn new() -> Self {
        Self::with_feature("work.txt", "work")
    }

    /// `make_publishing_repo <file> <content>`: `main` holds the helpers, a
    /// README, `pkg/alpha/src/lib.rs`, and the committed policy, and is on
    /// `origin`; `feature` adds `file` and is checked out.
    pub fn with_feature(file: &str, content: &str) -> Self {
        let root = tempfile::tempdir().expect("create the fixture root");
        let repo = root.path().join("repo");
        let origin = root.path().join("origin.git");
        let home = root.path().join("home");
        for directory in [&home, &root.path().join("tmp")] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(root.path().join("gitconfig"), "").unwrap();

        let git = init(&repo);
        copy_ci_helpers(&repo.join("scripts").join("ci"));
        write(&repo, "scripts/ci/affected_scope.py", STUB_PLANNER);
        // The helpers' bytecode must never dirty the tree.
        write(&repo, ".gitignore", "__pycache__/\n");
        write(&repo, "README.md", "base\n");
        write(&repo, "pkg/alpha/src/lib.rs", "base lib\n");
        write(
            &repo,
            ".github/ci/policy.json",
            "{\"preflight_reason\": \"committed policy\"}\n",
        );
        let main = commit_all(&git, "base");
        let mut options = git2::RepositoryInitOptions::new();
        options.bare(true).initial_head("main");
        git2::Repository::init_opts(&origin, &options).unwrap();
        git.remote("origin", origin.to_str().unwrap()).unwrap();
        push(&git, &["refs/heads/main:refs/heads/main"]);
        switch(&git, "feature", main);
        write(&repo, file, &format!("{content}\n"));
        commit_all(&git, "head");

        Self::assemble(root, repo, origin, main)
    }

    /// A fixture for running the real recipe directly ([`Self::run_recipe`]):
    /// the stubs, Python, and isolated environment of [`Self::new`] around an
    /// empty checkout directory with no Git repository or remote, which the
    /// recipe never reads. `main` is the zero object ID, and nothing that
    /// reads Git or pushes may be called.
    pub fn without_repository() -> Self {
        let root = tempfile::tempdir().expect("create the fixture root");
        let repo = root.path().join("repo");
        let origin = root.path().join("origin.git");
        for directory in [&repo, &root.path().join("home"), &root.path().join("tmp")] {
            std::fs::create_dir_all(directory).unwrap();
        }
        std::fs::write(root.path().join("gitconfig"), "").unwrap();
        Self::assemble(root, repo, origin, git2::Oid::zero())
    }

    fn assemble(root: TempDir, repo: PathBuf, origin: PathBuf, main: git2::Oid) -> Self {
        let helpers = repo.join("scripts").join("ci");
        let python = real_python(Some(helpers.as_path()).filter(|helpers| helpers.is_dir()));
        let environment = host_environment();
        let tools = Fixture::new(&["just", "gh", "sniff"], json!({}));
        let mut behaviors = Map::new();
        behaviors.insert("just".into(), json!({ "exit": 0 }));
        behaviors.insert("gh".into(), json!({ "stdout": "[]" }));
        behaviors.insert(
            "sniff".into(),
            json!({ "stdout": sniff_os_json(environment) }),
        );
        let fixture = Evidence {
            evidence: root.path().join("evidence"),
            constraints: root.path().join("constraints"),
            home: root.path().join("home"),
            root,
            repo,
            origin,
            tools,
            environment,
            python,
            main,
            behaviors: RefCell::new(behaviors),
            planner_plan: RefCell::new(None),
            extra_path: RefCell::new(Vec::new()),
            planner_hang: RefCell::new(None),
            self_test_suites: RefCell::new(Vec::new()),
            helper_runs: std::cell::Cell::new(0),
        };
        fixture.write_behaviors();
        fixture
    }

    /// `make_stacked_repo`: `parent` (from main) changes
    /// `pkg/alpha/src/lib.rs`; `child` (from parent) restores it and is
    /// checked out. With `remote_parent_behind`, the remote's `parent` still
    /// holds main's commit. Returns the fixture, parent, and child commits.
    pub fn stacked(remote_parent_behind: bool) -> (Self, git2::Oid, git2::Oid) {
        let fixture = Self::new();
        let git = fixture.git();
        switch(&git, "parent", fixture.main);
        write(&fixture.repo, "pkg/alpha/src/lib.rs", "parent lib\n");
        let parent = commit_all(&git, "parent");
        if remote_parent_behind {
            push(&git, &["refs/heads/main:refs/heads/parent"]);
        } else {
            push(&git, &["refs/heads/parent:refs/heads/parent"]);
        }
        switch(&git, "child", parent);
        write(&fixture.repo, "pkg/alpha/src/lib.rs", "base lib\n");
        let child = commit_all(&git, "child");
        (fixture, parent, child)
    }

    pub fn root(&self) -> &Path {
        self.root.path()
    }

    pub fn git(&self) -> git2::Repository {
        git2::Repository::open(&self.repo).unwrap()
    }

    pub fn head(&self) -> git2::Oid {
        self.git().head().unwrap().peel_to_commit().unwrap().id()
    }

    /// Commits `content` appended to `file` on the checked-out branch.
    pub fn follow_up(&self, file: &str, content: &str) -> git2::Oid {
        let path = self.repo.join(file);
        let mut text = std::fs::read_to_string(&path).unwrap_or_default();
        text.push_str(content);
        text.push('\n');
        write(&self.repo, file, &text);
        commit_all(&self.git(), &format!("follow-up: {file}"))
    }

    /// Pushes local refspecs to `origin` (the way a `--no-verify` push or
    /// another clone would, running no hook).
    pub fn push_refs(&self, refspecs: &[&str]) {
        push(&self.git(), refspecs);
    }

    /// Commits `content` as `file` onto `branch` directly on the remote, as
    /// a push from another clone would; the checkout's tracking ref is left
    /// stale. Returns the new tip.
    pub fn advance_remote(&self, branch: &str, file: &str, content: &str) -> git2::Oid {
        let bare = git2::Repository::open_bare(&self.origin).unwrap();
        let reference = format!("refs/heads/{branch}");
        let parent = bare
            .find_reference(&reference)
            .unwrap()
            .peel_to_commit()
            .unwrap();
        let blob = bare.blob(content.as_bytes()).unwrap();
        let mut update = git2::build::TreeUpdateBuilder::new();
        update.upsert(file, blob, git2::FileMode::Blob);
        let tree = update
            .create_updated(&bare, &parent.tree().unwrap())
            .unwrap();
        let tree = bare.find_tree(tree).unwrap();
        let signature = git2::Signature::now("Fixture", "fixture@example.com").unwrap();
        bare.commit(
            Some(&reference),
            &signature,
            &signature,
            "advance",
            &tree,
            &[&parent],
        )
        .unwrap()
    }

    /// The control entry for one stub (`just`, `gh`, or `sniff`).
    pub fn set_tool(&self, name: &str, behavior: Value) {
        self.behaviors.borrow_mut().insert(name.into(), behavior);
        self.write_behaviors();
    }

    /// `gh pr list` answers these `(number, base branch)` pull requests.
    pub fn open_pull_requests(&self, pulls: &[(u64, &str)]) {
        let reply: Vec<Value> = pulls
            .iter()
            .map(|(number, base)| json!({ "number": number, "baseRefName": base }))
            .collect();
        self.set_tool("gh", json!({ "stdout": Value::Array(reply).to_string() }));
    }

    fn write_behaviors(&self) {
        self.tools
            .configure(Value::Object(self.behaviors.borrow().clone()));
    }

    /// The stub planner emits `plan` for every scope calculation, its base
    /// and head rewritten. A plan whose cells this host runs is passed
    /// through [`relabel`] first.
    pub fn planner_plan(&self, plan: &str) {
        let path = self.root().join("planner-plan.json");
        std::fs::write(&path, plan).unwrap();
        *self.planner_plan.borrow_mut() = Some(path);
    }

    #[cfg(unix)]
    /// Every scope calculation writes its pid to `pid_file` and waits to be
    /// killed (the stub planner's `TEST_PLANNER_HANG`).
    pub fn planner_hangs(&self, pid_file: &Path) {
        *self.planner_hang.borrow_mut() = Some(pid_file.to_path_buf());
    }

    /// A gate that stages one `alpha` L1 report (passing or failing) and the
    /// plan the receipt is recorded against, as the shell suite's
    /// `make_staging_pre_push` / `make_failing_staging_pre_push` did.
    pub fn staging_gate(&self, plan: &str, failing: bool, exit: u8) -> Value {
        json!({
            "write_files": {
                "$BISCUIT_CI_PLAN_OUT": plan,
                "$BISCUIT_CI_REPORTS_OUT/L1/alpha.xml": junit("alpha", failing),
                "$BISCUIT_CI_REPORTS_OUT/manifest.jsonl":
                    manifest_line("alpha", u8::from(failing), self.environment),
            },
            "exit": exit,
        })
    }

    /// A store record prohibiting `environment` until 2099.
    pub fn prohibit(&self, directory: &Path, environment: &str) {
        std::fs::create_dir_all(directory).unwrap();
        let record = json!({
            "environment": environment,
            "reason": PROHIBITION_REASON,
            "owner": "ken",
            "expiry": "2099-01-01",
        });
        std::fs::write(directory.join("current.json"), record.to_string()).unwrap();
    }

    /// `refs/heads/feature <head> refs/heads/feature <zero>`.
    pub fn creating_feature_line(&self) -> String {
        format!(
            "refs/heads/feature {} refs/heads/feature {ZERO}",
            self.head()
        )
    }

    /// Runs `ci-push origin <url>` from the checkout with `lines` (and a
    /// final newline, as Git writes them) on stdin.
    pub fn push(&self, mode: Option<&str>, url: &str, lines: &str) -> Run {
        self.push_with(mode, url, format!("{lines}\n").as_bytes(), |_| {})
    }

    /// As [`Self::push`] with `input` byte for byte and a chance to adjust
    /// the command's environment.
    pub fn push_with(
        &self,
        mode: Option<&str>,
        url: &str,
        input: &[u8],
        adjust: impl FnOnce(&mut Command),
    ) -> Run {
        let mut command = self.ci_push(mode, url);
        adjust(&mut command);
        run(command, input)
    }

    #[cfg(unix)]
    /// Starts `ci-push` with `lines` written to its stdin (then closed) and
    /// its output piped, for a test that signals it while it runs.
    pub fn spawn(&self, mode: Option<&str>, url: &str, lines: &str) -> std::process::Child {
        use std::io::Write;
        let mut child = self
            .ci_push(mode, url)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn ci-push");
        let mut stdin = child.stdin.take().unwrap();
        stdin.write_all(format!("{lines}\n").as_bytes()).unwrap();
        child
    }

    fn ci_push(&self, mode: Option<&str>, url: &str) -> Command {
        let mut command = self.command(biscuit_test_harness::bin_exe!("ci-push"));
        command.args(["origin", url]).current_dir(&self.repo);
        if let Some(mode) = mode {
            command.env("RUSTY_BISCUIT_PRE_PUSH", mode);
        }
        command
    }

    /// Adds another `ci-push-test-tool` stub to the stub directory, acting
    /// as `behavior`.
    pub fn add_stub(&self, name: &str, behavior: Value) {
        self.tools.install(name);
        self.set_tool(name, behavior);
    }

    /// The stub directory alone, as a `PATH`.
    pub fn stub_path(&self) -> OsString {
        self.tools.path()
    }

    /// The default push: `feature` created on `origin` from the checkout.
    pub fn push_feature(&self, mode: Option<&str>) -> Run {
        let url = self.origin_url();
        self.push(mode, &url, &self.creating_feature_line())
    }

    pub fn origin_url(&self) -> String {
        self.origin.to_str().unwrap().to_owned()
    }

    /// A command under the fixture's isolated environment and `PATH`.
    pub fn command(&self, program: impl AsRef<std::ffi::OsStr>) -> Command {
        let mut command = Command::new(program);
        command.env_clear();
        for name in [
            "SYSTEMROOT",
            "SystemRoot",
            "WINDIR",
            "COMSPEC",
            "PATHEXT",
            "LANG",
            // `ci-local` reads it to run a native `jq.exe` with `--binary`.
            "OS",
        ] {
            if let Some(value) = std::env::var_os(name) {
                command.env(name, value);
            }
        }
        let tmp = self.root().join("tmp");
        command
            .env("PATH", self.path())
            .env("HOME", &self.home)
            .env("USERPROFILE", &self.home)
            .env("TMPDIR", &tmp)
            .env("TMP", &tmp)
            .env("TEMP", &tmp)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root().join("gitconfig"))
            .env("BISCUIT_CI_EVIDENCE_DIR", &self.evidence)
            .env("BISCUIT_CI_CONSTRAINTS_DIR", &self.constraints)
            .env("BISCUIT_CI_TARGET_DIR", self.root().join("target"))
            .env("TEST_PLANNER_LOG", self.planner_log_path());
        if let Some(plan) = self.planner_plan.borrow().as_ref() {
            command.env("TEST_PLANNER_PLAN", plan);
        }
        if let Some(pid_file) = self.planner_hang.borrow().as_ref() {
            command.env("TEST_PLANNER_HANG", pid_file);
        }
        command
    }

    /// The real Python under the fixture environment, for checks the shell
    /// suite made with `python3`.
    pub fn python_command(&self) -> Command {
        let mut command = self.command(&self.python);
        command
            .env("PYTHONDONTWRITEBYTECODE", "1")
            .env("PYTHONUTF8", "1");
        command
    }

    /// The stub directory, the real Python's directory, whatever
    /// [`Self::needs_tool`] and [`Self::stage_ci_local_harness`] added, and
    /// Git's directory; on Unix `/usr/bin:/bin` for the coreutils a Bash
    /// recipe runs.
    fn path(&self) -> OsString {
        let mut directories = vec![self.tools.bin()];
        let mut add = |program: &Path| {
            if let Some(directory) = program.parent() {
                if !directories.iter().any(|known| known == directory) {
                    directories.push(directory.to_path_buf());
                }
            }
        };
        add(&self.python);
        for program in self.extra_path.borrow().iter() {
            add(program);
        }
        for program in fixture_git() {
            add(&program);
        }
        if cfg!(unix) {
            add(Path::new("/usr/bin/env"));
            add(Path::new("/bin/sh"));
        }
        std::env::join_paths(directories).unwrap()
    }

    /// Puts the real `name`'s directory on the fixture `PATH`.
    pub fn needs_tool(&self, name: &str) -> PathBuf {
        let tool = real_tool(name);
        self.extra_path.borrow_mut().push(tool.clone());
        tool
    }

    fn planner_log_path(&self) -> PathBuf {
        self.root().join("planner.log")
    }

    /// Every line the stub planners logged, `root …` lines excluded.
    pub fn planner_calls(&self) -> Vec<String> {
        self.planner_log()
            .lines()
            .filter(|line| !line.starts_with("root "))
            .map(str::to_owned)
            .collect()
    }

    pub fn planner_log(&self) -> String {
        std::fs::read_to_string(self.planner_log_path()).unwrap_or_default()
    }

    /// The tree the first planner call lived in.
    pub fn planner_root(&self) -> PathBuf {
        let log = self.planner_log();
        let line = log
            .lines()
            .find_map(|line| line.strip_prefix("root "))
            .unwrap_or_else(|| panic!("the stub planner never ran:\n{log}"));
        PathBuf::from(line.split(" cwd ").next().unwrap())
    }

    /// Forgets one run's planner and stub logs, so the fixture can push again.
    pub fn reset_logs(&self) {
        let _ = std::fs::remove_file(self.planner_log_path());
        self.tools.clear_log();
    }

    /// Each `just` invocation's argv, space-joined, in order.
    pub fn just_calls(&self) -> Vec<String> {
        self.tool_calls("just")
    }

    pub fn tool_calls(&self, tool: &str) -> Vec<String> {
        self.tools
            .invocations()
            .iter()
            .filter(|invocation| invocation["tool"] == tool)
            .map(|invocation| {
                invocation["argv"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|arg| arg.as_str().unwrap())
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .collect()
    }

    pub fn just_ran(&self, prefix: &str) -> bool {
        self.just_calls()
            .iter()
            .any(|call| call.starts_with(prefix))
    }

    /// The validation ref of this host's environment.
    pub fn validation_ref(&self) -> String {
        format!("refs/notes/ci-local/{}", self.environment)
    }

    /// The scope receipt on `commit` in the checkout.
    pub fn scope_receipt(&self, commit: git2::Oid) -> Option<Value> {
        note(&self.repo, "refs/notes/ci-local/scope", commit)
    }

    /// The validation note on `commit` in the checkout.
    pub fn local_validation(&self, commit: git2::Oid) -> Option<Value> {
        note(&self.repo, &self.validation_ref(), commit)
    }

    /// The validation note on `commit` as it sits on the bare remote.
    pub fn remote_validation(&self, commit: git2::Oid) -> Option<Value> {
        note(&self.origin, &self.validation_ref(), commit)
    }

    /// The scope receipt on `commit` as it sits on the bare remote.
    pub fn remote_scope_receipt(&self, commit: git2::Oid) -> Option<Value> {
        note(&self.origin, "refs/notes/ci-local/scope", commit)
    }

    pub fn remote_has_ref(&self, name: &str) -> bool {
        git2::Repository::open_bare(&self.origin)
            .unwrap()
            .find_reference(name)
            .is_ok()
    }

    pub fn local_has_ref(&self, name: &str) -> bool {
        self.git().find_reference(name).is_ok()
    }

    /// No temporary worktree outlived the run: Git lists none and the
    /// fixture's temporary directory holds none.
    #[track_caller]
    pub fn assert_no_leftover_worktree(&self) {
        let listed = self.git().worktrees().unwrap();
        assert_eq!(
            listed.len(),
            0,
            "worktrees outlived the run: {:?}",
            listed.iter().collect::<Vec<_>>()
        );
        let leftovers: Vec<_> = std::fs::read_dir(self.root().join("tmp"))
            .unwrap()
            .map(|entry| entry.unwrap().file_name())
            .filter(|name| {
                name.to_string_lossy()
                    .starts_with(repo_deps::ci_push::worktree::NAME_PREFIX)
            })
            .collect();
        assert!(
            leftovers.is_empty(),
            "temporary trees outlived the run: {leftovers:?}"
        );
    }

    /// `schema.identity(schema.canonical(receipt["plan"]))`, from the
    /// checkout's committed (real) `schema.py`.
    pub fn plan_identity(&self, scope_receipt: &Value) -> String {
        let file = self.root().join("scope-receipt-for-identity.json");
        std::fs::write(&file, scope_receipt.to_string()).unwrap();
        let output = self
            .python_command()
            .arg("-c")
            .arg("import json, sys; sys.path.insert(0, sys.argv[2]); import schema; print(schema.identity(schema.canonical(json.load(open(sys.argv[1]))['plan'])))")
            .arg(&file)
            .arg(self.repo.join("scripts").join("ci"))
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        String::from_utf8(output.stdout).unwrap().trim().to_owned()
    }

    /// The `ci-local` harness (`stage_ci_local_harness`): the real recipe
    /// imported by a harness justfile, the real `constraints.py`, no-op
    /// self-test suites, and a planner that logs `ci-local …` and answers
    /// only `--apply-to`, by running the real planner inside the stub's own
    /// interpreter (so a projection costs one interpreter start, as the
    /// recipe's real call does). `recipe` is the path of the repository's
    /// `just/ci-local.just`, spelled by the calling test.
    pub fn stage_ci_local_harness(&self, recipe: &Path) {
        // Bash's directory leads: on Windows it is Git's `usr\bin`, whose
        // `env`, `cygpath`, and coreutils must win over any a `just` or `jq`
        // directory also holds.
        self.extra_path.borrow_mut().push(recipe_bash());
        for tool in ["just", "jq"] {
            self.needs_tool(tool);
        }
        let harness = self.harness();
        let text = std::fs::read_to_string(recipe)
            .unwrap_or_else(|error| panic!("read {}: {error}", recipe.display()));
        write(&harness, "ci-local.just", &text);
        write(
            &harness,
            "justfile",
            "red := \"\"\ngreen := \"\"\nreset := \"\"\nimport \"ci-local.just\"\n",
        );
        let helpers = biscuit_test_harness::manifest_dir!().join("ci");
        let constraints = std::fs::read_to_string(helpers.join("constraints.py")).unwrap();
        write(&harness, "scripts/ci/constraints.py", &constraints);
        // The recipe's self-test loop: no-op stubs, read from the recipe itself
        // so a suite joining the loop cannot leave the harness behind.
        let suites = text
            .lines()
            .find_map(|line| {
                line.trim()
                    .strip_prefix("for suite in ")
                    .and_then(|rest| rest.strip_suffix("; do"))
                    .filter(|list| list.starts_with("test_"))
            })
            .unwrap_or_else(|| panic!("no self-test suite list in {}", recipe.display()));
        for suite in suites.split_whitespace() {
            write(&harness, &format!("scripts/ci/{suite}"), "");
        }
        *self.self_test_suites.borrow_mut() = suites.split_whitespace().map(str::to_owned).collect();
        // The real planner runs as `__main__` through `run_module`, which,
        // unlike running the file, reads and keeps compiled bytecode beside it
        // (the repository ignores `__pycache__/`): compiling its 6,000 lines
        // costs about 0.15 s per push on native Windows.
        let real_planner =
            serde_json::to_string(helpers.to_str().unwrap()).unwrap();
        write(
            &harness,
            "scripts/ci/affected_scope.py",
            &format!(
                "import os, runpy, sys\n\
                 with open(os.environ[\"TEST_PLANNER_LOG\"], \"a\", encoding=\"utf-8\") as log:\n\
                 \x20   log.write(\"ci-local \" + \" \".join(sys.argv[1:]) + \"\\n\")\n\
                 if \"--apply-to\" not in sys.argv:\n\
                 \x20   raise SystemExit(\"ci-local selected scope although the hook fed it a plan\")\n\
                 sys.path.insert(0, {real_planner})\n\
                 sys.dont_write_bytecode = False\n\
                 runpy.run_module(\"affected_scope\", run_name=\"__main__\", alter_sys=True)\n"
            ),
        );
    }

    /// The directory the real recipe runs in.
    pub fn harness(&self) -> PathBuf {
        self.root().join("harness")
    }

    /// A stub interpreter's behavior, for a `ci-push-test-tool` installed
    /// with [`Self::add_stub`]: after `prefix`, each of the recipe's
    /// self-test suites (no-ops in the harness) exits 0 without starting
    /// Python, and anything else runs under the real Python; without
    /// `prefix` it fails with 96, so a dropped prefix cannot pass unnoticed.
    /// Call after [`Self::stage_ci_local_harness`].
    pub fn interpreter_stub(&self, prefix: &[&str]) -> Value {
        let suites = self.self_test_suites.borrow();
        assert!(!suites.is_empty(), "stage the ci-local harness first");
        let mut dispatch: Vec<Value> = suites
            .iter()
            .map(|suite| {
                let mut args: Vec<String> = prefix.iter().map(|arg| (*arg).to_owned()).collect();
                args.push(format!("scripts/ci/{suite}"));
                json!({ "args": args, "behavior": { "exit": 0 } })
            })
            .collect();
        dispatch.push(json!({ "args": prefix, "behavior": { "delegate": { "program": self.python } } }));
        json!({
            "dispatch": dispatch,
            "stderr": "fixture interpreter: the interpreter prefix is missing\n",
            "exit": 96,
        })
    }

    /// `make_ci_local_just`: `just pre-push` is the real recipe through the
    /// real `just`; `_lint` passes; `_test alpha`/`_test beta` stage a JUnit
    /// report and a manifest line under `BISCUIT_JUNIT_STAGE_DIR` as `alpha`
    /// and `beta` say. Anything else is unexpected (exit 99).
    pub fn recipe_just(&self, alpha: TestGate, beta: TestGate) {
        let real_just = real_tool("just");
        let harness = self.harness();
        let harness = harness.to_str().unwrap();
        let justfile = self.harness().join("justfile");
        let gate = |package: &str, behavior: TestGate| -> Value {
            let report = format!("$BISCUIT_JUNIT_STAGE_DIR/L1/{package}.xml");
            let manifest = "$BISCUIT_JUNIT_STAGE_DIR/manifest.jsonl";
            match behavior {
                TestGate::Pass | TestGate::Fail => {
                    let (xml, line) = self.test_report(package, behavior);
                    json!({
                        "write_files": { report: xml },
                        "append_files": { manifest: line },
                        "exit": u8::from(behavior == TestGate::Fail),
                    })
                }
                TestGate::MalformedManifest => json!({
                    "append_files": { manifest: MALFORMED_MANIFEST_LINE },
                    "exit": 1,
                }),
                TestGate::Recorded143 => json!({
                    "append_files": { manifest: recorded_143_line(package) },
                    "exit": 1,
                }),
                TestGate::Exit130 => json!({ "exit": 130 }),
            }
        };
        self.set_tool(
            "just",
            json!({
                "dispatch": [
                    { "args": ["pre-push"], "behavior": { "delegate": {
                        "program": real_just,
                        "args": ["--justfile", justfile, "--working-directory", harness, "ci-local", "--l2"],
                    } } },
                    { "args": ["_lint"], "behavior": { "exit": 0 } },
                    { "args": ["_test", "alpha"], "behavior": gate("alpha", alpha) },
                    { "args": ["_test", "beta"], "behavior": gate("beta", beta) },
                ],
                "stderr": "fixture just: unexpected subcommand\n",
                "exit": 99,
            }),
        );
    }

    /// The JUnit report and manifest line a fixture `_test <package>` stages
    /// for a `Pass` or `Fail` gate.
    fn test_report(&self, package: &str, behavior: TestGate) -> (String, String) {
        let failing = match behavior {
            TestGate::Pass => false,
            TestGate::Fail => true,
            other => panic!("{other:?} stages no complete report"),
        };
        (
            junit(package, failing),
            manifest_line(package, u8::from(failing), self.environment),
        )
    }

    /// What the real recipe leaves for `ci-push` when the L1 gates of
    /// `cells` run in order, each as its [`TestGate`] says: a complete gate's
    /// report and manifest line; for an interruption, the manifest as the
    /// gate left it and the `interrupted` marker the recipe writes, and no
    /// later gate. The exit status is the recipe's: the interruption's, else
    /// 1 when a gate failed.
    pub fn recipe_outcome(&self, cells: &[(&str, TestGate)]) -> GateOutcome {
        let mut reports = BTreeMap::new();
        let mut manifest = String::new();
        let mut code = 0;
        let mut interruption = None;
        for &(package, behavior) in cells {
            match behavior {
                TestGate::Pass | TestGate::Fail => {
                    let (xml, line) = self.test_report(package, behavior);
                    reports.insert(format!("L1/{package}.xml"), xml);
                    manifest.push_str(&line);
                    if behavior == TestGate::Fail {
                        code = 1;
                    }
                }
                TestGate::MalformedManifest => {
                    manifest.push_str(MALFORMED_MANIFEST_LINE);
                    interruption = Some(("unreadable manifest", 1));
                }
                TestGate::Recorded143 => {
                    manifest.push_str(&recorded_143_line(package));
                    interruption = Some(("143", 143));
                }
                TestGate::Exit130 => interruption = Some(("130", 130)),
            }
            if interruption.is_some() {
                break;
            }
        }
        if !manifest.is_empty() {
            reports.insert("manifest.jsonl".to_owned(), manifest);
        }
        if let Some((marker, status)) = interruption {
            reports.insert("interrupted".to_owned(), format!("{marker}\n"));
            code = status;
        }
        GateOutcome { code, reports }
    }

    /// `just pre-push` without the real recipe: stages `outcome`'s reports,
    /// writes back the plan it was fed as the recipe does, and exits with
    /// `outcome`'s status. Each outcome a test models this way is one case
    /// of `the_real_recipe_stages_the_modeled_outcome_of_a_fed_plan`, which
    /// pins it against the real recipe. The run is recorded for
    /// [`Self::gate_start`] and [`Self::fed_plan`].
    pub fn model_gate(&self, outcome: &GateOutcome) {
        use repo_deps::ci_push::python::{PYTHON_ARGS_FILE_VARIABLE, PYTHON_VARIABLE};
        let staged: Map<String, Value> = outcome
            .reports
            .iter()
            .map(|(path, content)| {
                (
                    format!("$BISCUIT_CI_REPORTS_OUT/{path}"),
                    Value::String(content.clone()),
                )
            })
            .collect();
        let fed = self.fed_plan_path().to_str().unwrap().to_owned();
        let prefix = self.fed_prefix_path().to_str().unwrap().to_owned();
        self.set_tool(
            "just",
            json!({
                "dispatch": [{ "args": ["pre-push"], "behavior": {
                    "copy_files": {
                        "$BISCUIT_CI_PLAN_OUT": "$BISCUIT_CI_PLAN_IN",
                        fed: "$BISCUIT_CI_PLAN_IN",
                        prefix: format!("${PYTHON_ARGS_FILE_VARIABLE}"),
                    },
                    "write_files": staged,
                    "exit": u8::try_from(outcome.code).expect("a modeled exit status fits a byte"),
                } }],
                "record_env": GATE_VARIABLES.iter().chain([&PYTHON_VARIABLE, &PYTHON_ARGS_FILE_VARIABLE]).collect::<Vec<_>>(),
                "stderr": "fixture just: unexpected subcommand\n",
                "exit": 99,
            }),
        );
    }

    /// A [`Self::model_gate`] that stages nothing, so `ci-push` publishes no
    /// validation receipt: for a test whose question is only what the gates
    /// are started with.
    pub fn capture_gate(&self) {
        self.model_gate(&GateOutcome {
            code: 0,
            reports: BTreeMap::new(),
        });
    }

    fn fed_plan_path(&self) -> PathBuf {
        self.root().join("fed-plan.json")
    }

    fn fed_prefix_path(&self) -> PathBuf {
        self.root().join("fed-python-args")
    }

    /// The plan `ci-push` fed the last [`Self::model_gate`] run.
    pub fn fed_plan(&self) -> Value {
        let path = self.fed_plan_path();
        let text = std::fs::read_to_string(&path)
            .unwrap_or_else(|error| panic!("the gates were fed no plan ({}): {error}", path.display()));
        serde_json::from_str(&text).expect("a JSON plan")
    }

    /// How `ci-push` started the one [`Self::model_gate`] run: its logged
    /// invocation (argv, cwd, and the gate variables) and the bytes of the
    /// interpreter prefix file it was handed.
    pub fn gate_start(&self) -> (Value, Vec<u8>) {
        let invocations = self.tools.invocations();
        let gates: Vec<&Value> = invocations
            .iter()
            .filter(|invocation| invocation["tool"] == "just")
            .collect();
        assert_eq!(gates.len(), 1, "{gates:#?}");
        let prefix = std::fs::read(self.fed_prefix_path()).expect("the gates were handed a prefix file");
        (gates[0].clone(), prefix)
    }

    /// Runs `just pre-push` — the real recipe, once [`Self::recipe_just`]
    /// routed it — as `ci-push` starts the gates after a review (pinned by
    /// `the_gates_are_started_as_the_direct_recipe_runs_start_them`): from
    /// the checkout, fed `plan` through `BISCUIT_CI_PLAN_IN`, with the
    /// staging variables, and with `interpreter` and its argument `prefix`
    /// handed over as `BISCUIT_CI_PYTHON` and its NUL-terminated prefix file.
    pub fn run_recipe(&self, plan: &str, interpreter: &Path, prefix: &[&str]) -> RecipeRun {
        use repo_deps::ci_push::python;
        let staging = self.root().join("recipe-staging");
        let reports = staging.join(REPORTS_DIRECTORY);
        std::fs::create_dir_all(&reports).unwrap();
        let plan_in = staging.join("review-plan.json");
        std::fs::write(&plan_in, plan).unwrap();
        let prefix_file = staging.join("python-args");
        let prefix: Vec<OsString> = prefix.iter().map(OsString::from).collect();
        std::fs::write(&prefix_file, python::encode_prefix(&prefix).unwrap()).unwrap();
        let plan_out = staging.join(GATE_PLAN_FILE);
        let mut command = self.command(self.tools.tool("just"));
        command
            .arg("pre-push")
            .current_dir(&self.repo)
            .env("BISCUIT_CI_SCOPE_OUT", staging.join(GATE_SCOPE_FILE))
            .env("BISCUIT_CI_PLAN_OUT", &plan_out)
            .env("BISCUIT_CI_REPORTS_OUT", &reports)
            .env("BISCUIT_CI_PLAN_IN", &plan_in)
            .env(python::PYTHON_VARIABLE, interpreter)
            .env(python::PYTHON_ARGS_FILE_VARIABLE, &prefix_file);
        let run = run(command, b"");
        RecipeRun {
            outcome: GateOutcome {
                code: run.code,
                reports: read_tree(&reports),
            },
            plan_out: std::fs::read_to_string(&plan_out).ok(),
            run,
        }
    }

    /// The plan the checkout's planner resolves for `event` from `base` to
    /// `head`, given the committed paths between them as the review gives
    /// them; returns the plan file.
    pub fn planned(&self, event: &str, base: git2::Oid, head: git2::Oid) -> PathBuf {
        let (plan, call) = self.planner_call(event, base, head);
        self.helpers(&[call]);
        plan
    }

    /// The planner call [`Self::planned`] makes, and the plan file it writes.
    fn planner_call(&self, event: &str, base: git2::Oid, head: git2::Oid) -> (PathBuf, HelperCall) {
        let git = self.git();
        let tree = |commit: git2::Oid| git.find_commit(commit).unwrap().tree().unwrap();
        let diff = git
            .diff_tree_to_tree(Some(&tree(base)), Some(&tree(head)), None)
            .unwrap();
        let plan = self.root().join(format!("plan-{event}-{base}-{head}.json"));
        let mut args = vec!["--event".to_owned(), event.to_owned(), "--plan-out".to_owned(), path_text(&plan)];
        args.extend(["--base", &base.to_string(), "--head", &head.to_string(), "--"].map(str::to_owned));
        args.extend(
            diff.deltas()
                .filter_map(|delta| delta.new_file().path().map(path_text)),
        );
        (plan, HelperCall::new("affected_scope", args))
    }

    /// `local_evidence.py verify --cells` of `plan` for `base`..`head` in the
    /// checkout, as the review runs it.
    pub fn verify(&self, plan: &Path, base: git2::Oid, head: git2::Oid) -> Verified {
        let rejections = self.root().join(format!("rejections-{head}.json"));
        let mut args = vec!["verify".to_owned(), "--cells".to_owned(), "--plan".to_owned(), path_text(plan)];
        args.extend(["--base", &base.to_string(), "--head", &head.to_string(), "--rejections"].map(str::to_owned));
        args.push(path_text(&rejections));
        let accepted = self.helpers(&[HelperCall::new("local_evidence", args)]).remove(0);
        Verified {
            accepted: serde_json::from_slice(&accepted).expect("verify prints a JSON array"),
            rejections: serde_json::from_slice(&std::fs::read(&rejections).unwrap())
                .expect("verify writes a JSON array of rejections"),
        }
    }

    /// Runs each call's `main()` from the checkout's `scripts/ci`, in order,
    /// in one process of the real Python started as `ci-push` starts a
    /// helper (in the checkout, with its environment), so a test pays one
    /// interpreter start for several; returns each call's stdout. A call
    /// that exits nonzero fails the test.
    fn helpers(&self, calls: &[HelperCall]) -> Vec<Vec<u8>> {
        let id = self.helper_runs.get();
        self.helper_runs.set(id + 1);
        let directory = self.root().join(format!("helpers-{id}"));
        std::fs::create_dir_all(&directory).unwrap();
        let outputs: Vec<PathBuf> = (0..calls.len())
            .map(|index| directory.join(format!("stdout-{index}")))
            .collect();
        let spec = json!({
            "scripts": path_text(&self.repo.join("scripts").join("ci")),
            "calls": calls.iter().zip(&outputs).map(|(call, stdout)| json!({
                "module": call.module,
                "argv": call.args,
                "stdout": path_text(stdout),
                "copies": call.stdout_to.iter().map(|file| path_text(file)).collect::<Vec<_>>(),
            })).collect::<Vec<_>>(),
        });
        let spec_file = directory.join("calls.json");
        std::fs::write(&spec_file, spec.to_string()).unwrap();
        let driver = directory.join("driver.py");
        std::fs::write(&driver, HELPER_DRIVER).unwrap();
        let output = self
            .python_command()
            .current_dir(&self.repo)
            .arg(&driver)
            .arg(&spec_file)
            .stdin(Stdio::null())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{calls:#?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        outputs.iter().map(|stdout| std::fs::read(stdout).unwrap()).collect()
    }

    /// Records `outcome` as the validation evidence a hook push of `head`
    /// (reviewed against `base` for `event`) leaves, without a push: the
    /// reports retained under the evidence root, then the evidence helpers
    /// `ci-push` runs after its gates, with the arguments it gives them —
    /// `local_evidence.py record-cells` over the checkout planner's plan and
    /// `observation_notes.py publish-receipt` to `origin`.
    /// `seeded_evidence_is_what_a_hook_push_records` pins the equivalence.
    pub fn record_evidence(&self, event: &str, base: git2::Oid, head: git2::Oid, outcome: &GateOutcome) {
        self.record_runs(&[(event, base, head, outcome)]);
    }

    /// [`Self::record_evidence`] of each `(event, base, head, outcome)` in
    /// order, in one interpreter.
    pub fn record_runs(&self, runs: &[(&str, git2::Oid, git2::Oid, &GateOutcome)]) {
        let mut calls = Vec::new();
        for &(event, base, head, outcome) in runs {
            let staging = self.root().join(format!("seed-{head}"));
            let reports = staging.join(REPORTS_DIRECTORY);
            let retained = self.evidence.join(head.to_string()).join(self.environment);
            std::fs::create_dir_all(&reports).unwrap();
            for (path, content) in &outcome.reports {
                write(&reports, path, content);
                write(&retained, path, content);
            }
            let (plan, planner) = self.planner_call(event, base, head);
            let receipt = staging.join("receipt.json");
            let prior = staging.join("prior-receipt.json");
            std::fs::write(&prior, "").unwrap();
            let (plan, base, head) = (path_text(&plan), base.to_string(), head.to_string());
            calls.push(planner);
            calls.push(
                HelperCall::new(
                    "local_evidence",
                    [
                        "record-cells", "--plan", &plan, "--stage", &path_text(&reports),
                        "--base", &base, "--head", &head, "--environment", self.environment,
                        "--report-dir", &path_text(&retained), "--prior-receipt",
                        &path_text(&prior),
                    ]
                    .map(str::to_owned)
                    .to_vec(),
                )
                .with_stdout_to(&receipt),
            );
            calls.push(HelperCall::new(
                "observation_notes",
                [
                    "publish-receipt", "--receipt", &path_text(&receipt), "--plan", &plan,
                    "--head", &head, "--environment", self.environment, "--remote", "origin",
                ]
                .map(str::to_owned)
                .to_vec(),
            ));
        }
        let outputs = self.helpers(&calls);
        for receipt in outputs.iter().skip(1).step_by(3) {
            assert!(!receipt.is_empty(), "record-cells printed no receipt");
        }
        self.reset_logs();
    }

    /// Removes this environment's validation notes, in the checkout and on
    /// the remote, and every retained report.
    pub fn forget_validation_evidence(&self) {
        let reference = self.validation_ref();
        for repository in [self.git(), git2::Repository::open_bare(&self.origin).unwrap()] {
            if let Ok(mut found) = repository.find_reference(&reference) {
                found.delete().unwrap();
            }
        }
        let _ = std::fs::remove_dir_all(&self.evidence);
    }

    /// The reports retained for `head` are exactly `outcome`'s: no file more
    /// or less, byte for byte.
    #[track_caller]
    pub fn assert_retained_reports(&self, head: git2::Oid, outcome: &GateOutcome) {
        let directory = self
            .evidence
            .join(head.to_string())
            .join(self.environment);
        assert!(directory.is_dir(), "{} was not retained", directory.display());
        assert_eq!(read_tree(&directory), outcome.reports);
    }

    /// `ci-local` ran the planner exactly once, for the projection.
    #[track_caller]
    pub fn assert_ci_local_never_selected(&self) {
        let calls: Vec<String> = self
            .planner_calls()
            .into_iter()
            .filter(|call| call.starts_with("ci-local "))
            .collect();
        assert!(
            calls.len() == 1 && calls[0].starts_with("ci-local --apply-to "),
            "expected ci-local's one planner call to be the projection: {calls:#?}"
        );
    }
}

/// The receipt projection the shell suite's assertions read: the cells of
/// every observation no later observation retries (sorted by package and
/// gate), with the first observation's base, scope identity, and host.
pub fn receipt_view(note: &Value) -> Value {
    let observations = note["observations"].as_array().cloned().unwrap_or_default();
    let retried: Vec<&Value> = observations
        .iter()
        .map(|observation| &observation["retry_of"])
        .collect();
    let mut cells: Vec<Value> = observations
        .iter()
        .filter(|observation| !retried.contains(&&observation["observation_id"]))
        .map(|observation| observation["cell"].clone())
        .collect();
    cells.sort_by_key(|cell| (cell["package"].to_string(), cell["gate"].to_string()));
    let first = observations.first().cloned().unwrap_or(Value::Null);
    json!({
        "head": note["head"],
        "tree": note["tree"],
        "environment": note["environment"],
        "base": first["base"],
        "scope_identity": first["scope_identity"],
        "host": first["producer"]["host"],
        "cells": cells,
    })
}

/// `"package/gate"` for each cell of a receipt view.
pub fn cell_names(view: &Value) -> Vec<String> {
    view["cells"]
        .as_array()
        .unwrap()
        .iter()
        .map(|cell| {
            format!(
                "{}/{}",
                cell["package"].as_str().unwrap(),
                cell["gate"].as_str().unwrap()
            )
        })
        .collect()
}

fn note(repository: &Path, reference: &str, commit: git2::Oid) -> Option<Value> {
    let repository = git2::Repository::open(repository).unwrap();
    let note = repository.find_note(Some(reference), commit).ok()?;
    Some(serde_json::from_str(note.message().expect("a UTF-8 note")).expect("a JSON note"))
}

fn junit(package: &str, failing: bool) -> String {
    if failing {
        format!(
            "<?xml version=\"1.0\"?><testsuites><testsuite name=\"{package}\" tests=\"2\" failures=\"1\" errors=\"0\" skipped=\"0\"><testcase classname=\"{package}\" name=\"one\"/><testcase classname=\"{package}\" name=\"two\"><failure message=\"assertion failed\">boom</failure></testcase></testsuite></testsuites>"
        )
    } else {
        format!(
            "<?xml version=\"1.0\"?><testsuites><testsuite name=\"{package}\" tests=\"2\" failures=\"0\" errors=\"0\" skipped=\"0\"><testcase classname=\"{package}\" name=\"one\"/><testcase classname=\"{package}\" name=\"two\"/></testsuite></testsuites>"
        )
    }
}

fn manifest_line(package: &str, exit_code: u8, environment: &str) -> String {
    format!(
        "{}\n",
        json!({
            "tier": "L1",
            "package": package,
            "xml": format!("L1/{package}.xml"),
            "exit_code": exit_code,
            "environment": environment,
            "duration_s": 3,
            "report_present": true,
        })
    )
}

/// The staging names `ci-push` gives the gates (its `evidence` module's),
/// which [`Evidence::run_recipe`] gives the real recipe.
pub const GATE_SCOPE_FILE: &str = "gate-scope.json";
pub const GATE_PLAN_FILE: &str = "gate-plan.json";
pub const REPORTS_DIRECTORY: &str = "reports";

/// The variables naming the gates' plan input and staging outputs.
pub const GATE_VARIABLES: [&str; 4] = [
    "BISCUIT_CI_PLAN_IN",
    "BISCUIT_CI_PLAN_OUT",
    "BISCUIT_CI_SCOPE_OUT",
    "BISCUIT_CI_REPORTS_OUT",
];

/// One helper `main()` for [`Evidence::helpers`]: the module in the
/// checkout's `scripts/ci` and its arguments.
#[derive(Debug)]
struct HelperCall {
    module: &'static str,
    args: Vec<String>,
    /// Where `ci-push` would have written the call's stdout, if anywhere.
    stdout_to: Option<PathBuf>,
}

impl HelperCall {
    fn new(module: &'static str, args: Vec<String>) -> Self {
        Self { module, args, stdout_to: None }
    }

    fn with_stdout_to(mut self, file: &Path) -> Self {
        self.stdout_to = Some(file.to_path_buf());
        self
    }
}

/// Runs helper modules' `main()` in turn, each with its own `sys.argv` and
/// captured stdout (written where the spec says, and also wherever a later
/// call reads it from), as [`Evidence::helpers`] describes.
const HELPER_DRIVER: &str = r#"import contextlib, importlib, io, json, sys
with open(sys.argv[1], encoding="utf-8") as handle:
    spec = json.load(handle)
sys.path.insert(0, spec["scripts"])
for call in spec["calls"]:
    module = importlib.import_module(call["module"])
    sys.argv = [module.__file__, *call["argv"]]
    out = io.StringIO()
    try:
        with contextlib.redirect_stdout(out):
            module.main()
    except SystemExit as exit:
        if exit.code not in (None, 0):
            sys.stderr.write(f"{call['module']} {call['argv']}: exited {exit.code!r}\n")
            raise
    for target in [call["stdout"], *call.get("copies", [])]:
        with open(target, "w", encoding="utf-8", newline="") as handle:
            handle.write(out.getvalue())
"#;

fn path_text(path: &Path) -> String {
    path.to_str().expect("a UTF-8 fixture path").to_owned()
}

/// The truncated manifest line a [`TestGate::MalformedManifest`] gate appends.
const MALFORMED_MANIFEST_LINE: &str = "{\"tier\":";

/// The manifest line a [`TestGate::Recorded143`] gate appends.
fn recorded_143_line(package: &str) -> String {
    format!(
        "{{\"tier\":\"L1\",\"package\":\"{package}\",\"xml\":\"L1/{package}.xml\",\"exit_code\":143,\"report_present\":false}}\n"
    )
}

/// Every file below `directory`, keyed by its `/`-separated relative path.
fn read_tree(directory: &Path) -> BTreeMap<String, String> {
    let mut files = BTreeMap::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(current) = pending.pop() {
        for entry in std::fs::read_dir(&current)
            .unwrap_or_else(|error| panic!("read {}: {error}", current.display()))
        {
            let path = entry.unwrap().path();
            if path.is_dir() {
                pending.push(path);
                continue;
            }
            let relative: Vec<String> = path
                .strip_prefix(directory)
                .unwrap()
                .components()
                .map(|part| part.as_os_str().to_string_lossy().into_owned())
                .collect();
            files.insert(relative.join("/"), std::fs::read_to_string(&path).unwrap());
        }
    }
    files
}

fn run(mut command: Command, input: &[u8]) -> Run {
    use std::io::Write;
    let mut child = command
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("spawn ci-push");
    let mut stdin = child.stdin.take().unwrap();
    let bytes = input.to_vec();
    // ci-push reads all of stdin first; writing from a thread keeps a large
    // input from deadlocking against its output.
    let writer = std::thread::spawn(move || {
        let _ = stdin.write_all(&bytes);
    });
    let output = child.wait_with_output().unwrap();
    writer.join().unwrap();
    finished(&output)
}

/// The [`Run`] of a finished `ci-push`.
pub fn finished(output: &Output) -> Run {
    Run {
        code: exit_code(&output.status),
        stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
        stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
    }
}

/// The status as a shell reports it: 128 + the signal for a signal death.
pub fn exit_code(status: &std::process::ExitStatus) -> i32 {
    #[cfg(unix)]
    if let Some(signal) = std::os::unix::process::ExitStatusExt::signal(status) {
        return 128 + signal;
    }
    status.code().unwrap_or(-1)
}

fn init(path: &Path) -> git2::Repository {
    let mut options = git2::RepositoryInitOptions::new();
    options.initial_head("main");
    let git = git2::Repository::init_opts(path, &options).unwrap();
    let mut config = git.config().unwrap();
    config.set_str("user.name", "hook-test").unwrap();
    config.set_str("user.email", "hook@example.com").unwrap();
    config.set_bool("commit.gpgsign", false).unwrap();
    config.set_bool("core.autocrlf", false).unwrap();
    git
}

/// The real helpers `ci-push` calls (`local_evidence.py` and
/// `observation_notes.py`) and every helper module they import, found by
/// following their `import` statements. The rest of `scripts/ci` is never
/// run from the checkout, and every committed file costs a fresh write, read,
/// and object on Windows (about 15 ms each there).
fn copy_ci_helpers(destination: &Path) {
    let helpers = biscuit_test_harness::manifest_dir!().join("ci");
    std::fs::create_dir_all(destination).unwrap();
    let mut pending = vec!["local_evidence".to_owned(), "observation_notes".to_owned()];
    let mut copied = std::collections::BTreeSet::new();
    while let Some(module) = pending.pop() {
        // The checkout's planner is the stub, written separately.
        if module == "affected_scope" || !copied.insert(module.clone()) {
            continue;
        }
        let file = format!("{module}.py");
        let text = std::fs::read_to_string(helpers.join(&file))
            .unwrap_or_else(|error| panic!("read helper {file}: {error}"));
        for line in text.lines() {
            let statement = line.trim_start();
            let imported = statement
                .strip_prefix("import ")
                .or_else(|| statement.strip_prefix("from "))
                .and_then(|rest| rest.split([' ', ',', '.']).next());
            if let Some(name) = imported {
                if helpers.join(format!("{name}.py")).is_file() {
                    pending.push(name.to_owned());
                }
            }
        }
        std::fs::write(destination.join(&file), text).unwrap();
    }
}

pub fn write(root: &Path, relative: &str, content: &str) {
    let path = root.join(relative);
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, content).unwrap();
}

/// Stages every change (deletions included) and commits on HEAD's branch.
pub fn commit_all(git: &git2::Repository, message: &str) -> git2::Oid {
    let mut index = git.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.update_all(["*"], None).unwrap();
    index.write().unwrap();
    let tree = git.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = git.signature().unwrap();
    let parent = git.head().ok().and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    git.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parents,
    )
    .unwrap()
}

/// Creates (or resets) `branch` at `at` and checks it out.
pub fn switch(git: &git2::Repository, branch: &str, at: git2::Oid) {
    let commit = git.find_commit(at).unwrap();
    git.branch(branch, &commit, true).unwrap();
    git.checkout_tree(
        commit.as_object(),
        Some(git2::build::CheckoutBuilder::new().force()),
    )
    .unwrap();
    git.set_head(&format!("refs/heads/{branch}")).unwrap();
}

fn push(git: &git2::Repository, refspecs: &[&str]) {
    git.find_remote("origin")
        .unwrap()
        .push(refspecs, None)
        .unwrap();
}

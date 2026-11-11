//! The repository the end-to-end parity tests push from: the port of the
//! retired shell suite's `make_publishing_repo`, `stage_fixture_tools`,
//! `make_stacked_repo`, and `stage_ci_local_harness`.
//!
//! The checkout commits the real `scripts/ci/*.py` helpers (evidence, schema,
//! observation notes, constraints) and, as `scripts/ci/affected_scope.py`, the
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

/// The first program called `name` on the inherited `PATH`. An L1 test needs
/// it, so a missing one fails loudly rather than skipping.
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
        .flat_map(|dir| names.iter().map(move |file| dir.join(file)))
        .find(|candidate| candidate.is_file())
}

/// A Python 3.10 or newer from the inherited `PATH`, as `ci-push` will find
/// it once its directory leads the fixture `PATH`.
pub fn real_python() -> PathBuf {
    for name in ["python3", "python"] {
        let Some(candidate) = find_tool(name) else {
            continue;
        };
        let proved = Command::new(&candidate)
            .args([
                "-c",
                "import sys; sys.exit(0 if sys.version_info >= (3, 10) else 1)",
            ])
            .stdin(Stdio::null())
            .output()
            .is_ok_and(|output| output.status.success());
        if proved {
            return candidate;
        }
    }
    panic!("python3 (3.10 or newer) must be on PATH: this L1 test runs the real evidence helpers");
}

#[cfg(unix)]
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
            root,
            repo,
            origin,
            home,
            tools,
            environment,
            python: real_python(),
            main,
            behaviors: RefCell::new(behaviors),
            planner_plan: RefCell::new(None),
            extra_path: RefCell::new(Vec::new()),
            planner_hang: RefCell::new(None),
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

    /// Adds another `ci-push-test-tool` copy to the stub directory, acting
    /// as `behavior`.
    pub fn add_stub(&self, name: &str, behavior: Value) {
        let tool = biscuit_test_harness::bin_exe!("ci-push-test-tool");
        std::fs::copy(&tool, self.tools.tool(name)).unwrap();
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

    /// The stub directory, then the real Python's and Git's directories,
    /// then whatever [`Self::needs_tool`] added; on Unix `/usr/bin:/bin` for
    /// the coreutils a Bash recipe runs.
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
        add(&real_tool("git"));
        if cfg!(unix) {
            add(Path::new("/usr/bin/env"));
            add(Path::new("/bin/sh"));
        }
        std::env::join_paths(directories).unwrap()
    }

    #[cfg(unix)]
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

    #[cfg(unix)]
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

    #[cfg(unix)]
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

    #[cfg(unix)]
    /// The `ci-local` harness (`stage_ci_local_harness`): the real recipe
    /// imported by a harness justfile, the real `constraints.py`, no-op
    /// self-test suites, and a planner that logs `ci-local …` and answers
    /// only `--apply-to`, through the real planner. `recipe` is the path of
    /// the repository's `just/ci-local.just`, spelled by the calling test.
    pub fn stage_ci_local_harness(&self, recipe: &Path) {
        for tool in ["just", "jq", "bash"] {
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
        let real_planner =
            serde_json::to_string(helpers.join("affected_scope.py").to_str().unwrap()).unwrap();
        write(
            &harness,
            "scripts/ci/affected_scope.py",
            &format!(
                "import os, subprocess, sys\n\
                 with open(os.environ[\"TEST_PLANNER_LOG\"], \"a\", encoding=\"utf-8\") as log:\n\
                 \x20   log.write(\"ci-local \" + \" \".join(sys.argv[1:]) + \"\\n\")\n\
                 if \"--apply-to\" not in sys.argv:\n\
                 \x20   raise SystemExit(\"ci-local selected scope although the hook fed it a plan\")\n\
                 raise SystemExit(subprocess.run([sys.executable, {real_planner}, *sys.argv[1:]], check=False).returncode)\n"
            ),
        );
    }

    #[cfg(unix)]
    fn harness(&self) -> PathBuf {
        self.root().join("harness")
    }

    #[cfg(unix)]
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
                    let failing = behavior == TestGate::Fail;
                    json!({
                        "write_files": { report: junit(package, failing) },
                        "append_files": { manifest: manifest_line(package, u8::from(failing), self.environment) },
                        "exit": u8::from(failing),
                    })
                }
                TestGate::MalformedManifest => json!({
                    "append_files": { manifest: "{\"tier\":" },
                    "exit": 1,
                }),
                TestGate::Recorded143 => json!({
                    "append_files": { manifest: format!(
                        "{{\"tier\":\"L1\",\"package\":\"{package}\",\"xml\":\"L1/{package}.xml\",\"exit_code\":143,\"report_present\":false}}\n"
                    ) },
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

    #[cfg(unix)]
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

#[cfg(unix)]
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

/// The real helpers `ci-push` and the gates call, without their test suites.
fn copy_ci_helpers(destination: &Path) {
    let helpers = biscuit_test_harness::manifest_dir!().join("ci");
    std::fs::create_dir_all(destination).unwrap();
    for entry in std::fs::read_dir(&helpers).unwrap() {
        let entry = entry.unwrap();
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.ends_with(".py")
            && !name.starts_with("test_")
            && entry.file_type().unwrap().is_file()
        {
            std::fs::copy(entry.path(), destination.join(&*name)).unwrap();
        }
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

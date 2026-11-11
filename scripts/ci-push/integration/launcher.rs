//! `.githooks/pre-push`, the Cargo launcher, reached the way Git reaches it:
//! an actual `git push` to a bare remote runs the real dispatcher from the
//! shared hooks directory, the dispatcher runs the pushing worktree's
//! launcher, and the launcher runs `cargo`, a compiled stand-in first on
//! `PATH` that records what it was given or forwards to the real `ci-push`.
//!
//! Every checkout path holds a space and a non-ASCII letter.

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::OnceLock;

use serde_json::{json, Value};
use tempfile::TempDir;

use crate::fixture::{same_path, Fixture};

/// What the launcher runs, before Git's arguments.
const CARGO_RUN: [&str; 10] = [
    "run",
    "--quiet",
    "-p",
    "repo-deps",
    "--no-default-features",
    "--features",
    "local-tools",
    "--bin",
    "ci-push",
    "--",
];

/// Inherited variables that would point the fixture's Git at another
/// repository, or choose the hook's mode for it.
const SCRUBBED: [&str; 12] = [
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_PREFIX",
    "GIT_NAMESPACE",
    "GIT_CONFIG_PARAMETERS",
    "RUSTY_BISCUIT_PRE_PUSH",
    "RUSTY_BISCUIT_PRE_PUSH_AREAS",
    "BISCUIT_CI_PLAN_IN",
];

pub(crate) const HOOK: &str = ".githooks/pre-push";

/// The real `git` the fixture pushes with: the first on the test's `PATH`.
pub(crate) fn real_git() -> &'static Path {
    static GIT: OnceLock<PathBuf> = OnceLock::new();
    GIT.get_or_init(|| {
        let name = if cfg!(windows) { "git.exe" } else { "git" };
        let path = std::env::var_os("PATH").unwrap_or_default();
        std::env::split_paths(&path)
            .map(|directory| directory.join(name))
            .find(|candidate| candidate.is_file())
            .expect("git is on PATH")
    })
}

/// The Git for Windows installation `real_git` belongs to.
#[cfg(windows)]
fn git_for_windows() -> &'static Path {
    static TOP: OnceLock<PathBuf> = OnceLock::new();
    TOP.get_or_init(|| {
        real_git()
            .ancestors()
            .find(|top| top.join("usr").join("bin").join("sh.exe").is_file())
            .unwrap_or_else(|| {
                panic!(
                    "{} is not part of a Git for Windows installation (no usr\\bin\\sh.exe above it)",
                    real_git().display()
                )
            })
            .to_path_buf()
    })
}

/// Git for Windows' own `sh`, which is what Git runs a hook with.
#[cfg(windows)]
pub(crate) fn git_sh() -> PathBuf {
    git_for_windows().join("usr").join("bin").join("sh.exe")
}

/// Git Bash: the `bin\bash.exe` of the installation `real_git` belongs to,
/// never WSL's `System32\bash.exe`.
#[cfg(windows)]
fn git_bash() -> PathBuf {
    let bash = git_for_windows().join("bin").join("bash.exe");
    assert!(
        bash.is_file(),
        "Git Bash is missing: {} does not exist",
        bash.display()
    );
    bash
}

/// Starts no console window for a child of the test.
pub(crate) fn windowless(command: &mut Command) -> &mut Command {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        command.creation_flags(CREATE_NO_WINDOW);
    }
    command
}

#[cfg(unix)]
pub(crate) fn make_executable(file: &Path) {
    use std::os::unix::fs::PermissionsExt;
    std::fs::set_permissions(file, std::fs::Permissions::from_mode(0o755)).unwrap();
}

/// Git for Windows decides executability from the `#!` line.
#[cfg(windows)]
pub(crate) fn make_executable(_file: &Path) {}

/// Writes `text` as `worktree`'s `.githooks/pre-push`, executable.
pub(crate) fn install_hook(worktree: &Path, text: &str) {
    let hook = worktree.join(HOOK);
    std::fs::create_dir_all(hook.parent().unwrap()).unwrap();
    std::fs::write(&hook, text).unwrap();
    make_executable(&hook);
}

/// Commits every file in `worktree` on its `HEAD`.
pub(crate) fn commit(worktree: &Path, message: &str) -> git2::Oid {
    let repository = git2::Repository::open(worktree).unwrap();
    let mut index = repository.index().unwrap();
    index
        .add_all(["*"], git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree = repository.find_tree(index.write_tree().unwrap()).unwrap();
    let signature = repository.signature().unwrap();
    let parent = repository
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let parents: Vec<&git2::Commit<'_>> = parent.iter().collect();
    repository
        .commit(
            Some("HEAD"),
            &signature,
            &signature,
            message,
            &tree,
            &parents,
        )
        .unwrap()
}

/// Edits `README.md` in `worktree` and commits it.
pub(crate) fn commit_change(worktree: &Path, message: &str) -> git2::Oid {
    std::fs::write(worktree.join("README.md"), format!("{message}\n")).unwrap();
    commit(worktree, message)
}

/// A main checkout whose `main` is on a bare `origin`, with the real
/// dispatcher in the shared hooks directory and an isolated Git
/// configuration for every command the test starts.
pub(crate) struct HookRepository {
    root: TempDir,
    main: PathBuf,
    remote: PathBuf,
    /// `main`'s first commit, on the remote.
    pub(crate) first: git2::Oid,
}

impl HookRepository {
    /// Commits `files` (plus a `README.md` and a `sub/` directory) as `main`'s
    /// first commit and pushes it to the remote without running any hook.
    pub(crate) fn new(dispatcher: &str, files: &[(&str, &str)]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let main = root.path().join("main checkout ü");
        let remote = root.path().join("remote.git");
        std::fs::create_dir_all(root.path().join("home")).unwrap();
        std::fs::write(root.path().join("gitconfig"), "").unwrap();
        std::fs::create_dir_all(main.join("sub")).unwrap();
        std::fs::write(main.join("sub").join("notes.txt"), "notes\n").unwrap();
        std::fs::write(main.join("README.md"), "first\n").unwrap();
        for (name, text) in files {
            let file = main.join(name);
            std::fs::create_dir_all(file.parent().unwrap()).unwrap();
            std::fs::write(&file, text).unwrap();
            if *name == HOOK {
                make_executable(&file);
            }
        }

        let mut options = git2::RepositoryInitOptions::new();
        options.initial_head("main");
        let repository = git2::Repository::init_opts(&main, &options).unwrap();
        let mut config = repository.config().unwrap();
        config.set_str("user.name", "Fixture").unwrap();
        config.set_str("user.email", "fixture@example.com").unwrap();
        config.set_bool("commit.gpgsign", false).unwrap();
        let first = commit(&main, "first");
        options.bare(true);
        git2::Repository::init_opts(&remote, &options).unwrap();
        repository
            .remote("origin", remote.to_str().unwrap())
            .unwrap()
            .push(&["refs/heads/main:refs/heads/main"], None)
            .unwrap();

        let hooks = repository.path().join("hooks");
        std::fs::create_dir_all(&hooks).unwrap();
        std::fs::write(hooks.join("pre-push"), dispatcher).unwrap();
        make_executable(&hooks.join("pre-push"));
        HookRepository {
            root,
            main,
            remote,
            first,
        }
    }

    pub(crate) fn main(&self) -> &Path {
        &self.main
    }

    /// The installed dispatcher, shared by every worktree.
    pub(crate) fn dispatcher(&self) -> PathBuf {
        self.main.join(".git").join("hooks").join("pre-push")
    }

    pub(crate) fn url(&self) -> &str {
        self.remote.to_str().unwrap()
    }

    pub(crate) fn home(&self) -> PathBuf {
        self.root.path().join("home")
    }

    /// Adds a linked worktree on a new branch `linked`, checked out from
    /// `main`'s `HEAD`.
    pub(crate) fn add_linked(&self) -> PathBuf {
        let linked = self.root.path().join("linked worktree ü");
        let repository = git2::Repository::open(&self.main).unwrap();
        repository.worktree("linked", &linked, None).unwrap();
        let shared = git2::Repository::open(&linked).unwrap();
        assert!(
            same_path(shared.commondir(), repository.path()),
            "the linked worktree shares the main checkout's hooks directory"
        );
        if linked.join(HOOK).is_file() {
            make_executable(&linked.join(HOOK));
        }
        linked
    }

    /// The tip of `reference` on the remote, if it exists.
    pub(crate) fn remote_tip(&self, reference: &str) -> Option<git2::Oid> {
        git2::Repository::open_bare(&self.remote)
            .unwrap()
            .refname_to_id(reference)
            .ok()
    }

    /// `PATH` for a push whose children are only Git, `sh`, and fixture
    /// stand-ins: `bin`, then the real Git's directory, then what `sh` needs.
    /// Git for Windows' layouts vary, so Windows keeps the inherited tail.
    pub(crate) fn hermetic_path(&self, bin: &[PathBuf]) -> OsString {
        let mut path = bin.to_vec();
        path.push(real_git().parent().unwrap().to_path_buf());
        if cfg!(windows) {
            path.extend(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ));
        } else {
            path.extend([PathBuf::from("/usr/bin"), PathBuf::from("/bin")]);
        }
        std::env::join_paths(path).unwrap()
    }

    /// `PATH` for a push that reaches the real `ci-push`, which needs the
    /// host's Python: `bin` and the real Git first, then the inherited `PATH`.
    pub(crate) fn host_path(&self, bin: &[PathBuf]) -> OsString {
        let mut path = bin.to_vec();
        path.push(real_git().parent().unwrap().to_path_buf());
        path.extend(std::env::split_paths(
            &std::env::var_os("PATH").unwrap_or_default(),
        ));
        std::env::join_paths(path).unwrap()
    }

    /// Isolates `command` from the host's Git configuration and repository.
    pub(crate) fn isolate<'c>(&self, command: &'c mut Command, path: &OsString) -> &'c mut Command {
        for name in SCRUBBED {
            command.env_remove(name);
        }
        command
            .env("PATH", path)
            .env("HOME", self.home())
            .env("USERPROFILE", self.home())
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CONFIG_GLOBAL", self.root.path().join("gitconfig"))
            .env("GIT_TERMINAL_PROMPT", "0")
            .env(
                "BISCUIT_CI_CONSTRAINTS_DIR",
                self.home().join("constraints"),
            )
            .env("BISCUIT_CI_EVIDENCE_DIR", self.home().join("evidence"));
        // Git's own messages are asserted on; Windows keeps its default
        // locale because MSYS spells non-ASCII paths through it.
        if cfg!(unix) {
            command.env("LC_ALL", "C");
        }
        windowless(command)
    }

    /// Runs `git push <args>` from `cwd`, started as `launch` says.
    pub(crate) fn push(
        &self,
        launch: Launch,
        cwd: &Path,
        path: &OsString,
        args: &[&str],
        env: &[(&str, &str)],
    ) -> Output {
        let mut command = match launch {
            Launch::Direct => {
                let mut command = Command::new(real_git());
                command.arg("push").args(args);
                command
            }
            #[cfg(windows)]
            Launch::GitBash => {
                // A script file, not `-c`: MSYS re-splits a `-c` script's
                // nested quotes (see the `os` skill's windows.md).
                let script = self.root.path().join("push.sh");
                std::fs::write(&script, "exec git push \"$@\"\n").unwrap();
                let mut command = Command::new(git_bash());
                command
                    .args(["--noprofile", "--norc"])
                    .arg(script.to_str().unwrap().replace('\\', "/"))
                    .args(args);
                command
            }
            #[cfg(windows)]
            Launch::PowerShell => {
                let mut command = Command::new("powershell.exe");
                command.args([
                    "-NoProfile",
                    "-NonInteractive",
                    "-Command",
                    &format!("git push {}; exit $LASTEXITCODE", args.join(" ")),
                ]);
                command
            }
        };
        self.isolate(&mut command, path)
            .current_dir(cwd)
            .envs(env.iter().copied());
        command
            .stdin(Stdio::null())
            .output()
            .unwrap_or_else(|error| panic!("start {launch:?} for `git push`: {error}"))
    }
}

/// How the test starts `git push`.
#[derive(Debug, Clone, Copy)]
pub(crate) enum Launch {
    /// The real `git`, directly.
    Direct,
    /// Non-interactive Git Bash.
    #[cfg(windows)]
    GitBash,
    /// Windows PowerShell, non-interactive.
    #[cfg(windows)]
    PowerShell,
}

fn stderr(output: &Output) -> String {
    String::from_utf8_lossy(&output.stderr).into_owned()
}

fn assert_pushed(output: &Output) {
    assert!(
        output.status.success(),
        "the push failed: {}",
        stderr(output)
    );
}

fn cargo_argv(repository: &HookRepository) -> Value {
    let mut argv: Vec<&str> = CARGO_RUN.to_vec();
    argv.extend(["origin", repository.url()]);
    json!(argv)
}

fn update_line(reference: &str, local: git2::Oid, remote: &str) -> String {
    format!("refs/heads/{reference} {local} refs/heads/{reference} {remote}\n")
}

fn recorded_cwd(invocation: &Value) -> PathBuf {
    PathBuf::from(invocation["cwd"].as_str().expect("a recorded cwd"))
}

fn recording_cargo() -> Fixture {
    Fixture::new(&["cargo"], json!({ "cargo": { "read_stdin": true } }))
}

#[test]
fn a_push_from_a_subdirectory_runs_cargo_at_the_top_level_with_gits_arguments_and_stdin() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[(HOOK, include_str!("../../../.githooks/pre-push"))],
    );
    let second = commit_change(repository.main(), "second");
    let cargo = recording_cargo();

    let output = repository.push(
        Launch::Direct,
        &repository.main().join("sub"),
        &repository.hermetic_path(&[cargo.bin()]),
        &["origin", "main"],
        &[],
    );

    assert_pushed(&output);
    let call = cargo.only_invocation();
    assert_eq!(call["argv"], cargo_argv(&repository));
    assert_eq!(
        call["stdin"],
        json!(update_line("main", second, &repository.first.to_string()))
    );
    assert!(
        same_path(&recorded_cwd(&call), repository.main()),
        "{call:#}"
    );
    assert_eq!(repository.remote_tip("refs/heads/main"), Some(second));
}

#[test]
fn a_failing_cargo_blocks_the_push_and_leaves_the_remote_ref_unchanged() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[(HOOK, include_str!("../../../.githooks/pre-push"))],
    );
    commit_change(repository.main(), "second");
    let cargo = Fixture::new(
        &["cargo"],
        json!({ "cargo": { "exit": 3, "stderr": "cargo stand-in refused the push" } }),
    );

    let output = repository.push(
        Launch::Direct,
        repository.main(),
        &repository.hermetic_path(&[cargo.bin()]),
        &["origin", "main"],
        &[],
    );

    let stderr = stderr(&output);
    assert!(!output.status.success(), "{stderr}");
    assert!(
        stderr.contains("cargo stand-in refused the push"),
        "{stderr}"
    );
    assert!(stderr.contains("failed to push some refs"), "{stderr}");
    assert_eq!(cargo.invocations().len(), 1);
    assert_eq!(
        repository.remote_tip("refs/heads/main"),
        Some(repository.first)
    );
}

#[test]
fn every_push_runs_cargo_again() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[(HOOK, include_str!("../../../.githooks/pre-push"))],
    );
    let cargo = recording_cargo();
    let path = repository.hermetic_path(&[cargo.bin()]);

    let second = commit_change(repository.main(), "second");
    assert_pushed(&repository.push(
        Launch::Direct,
        repository.main(),
        &path,
        &["origin", "main"],
        &[],
    ));
    let third = commit_change(repository.main(), "third");
    assert_pushed(&repository.push(
        Launch::Direct,
        repository.main(),
        &path,
        &["origin", "main"],
        &[],
    ));

    let stdin: Vec<Value> = cargo
        .invocations()
        .iter()
        .map(|call| call["stdin"].clone())
        .collect();
    assert_eq!(
        stdin,
        [
            json!(update_line("main", second, &repository.first.to_string())),
            json!(update_line("main", third, &second.to_string())),
        ]
    );
    assert_eq!(repository.remote_tip("refs/heads/main"), Some(third));
}

#[test]
fn each_worktree_runs_cargo_in_its_own_checkout() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[(HOOK, include_str!("../../../.githooks/pre-push"))],
    );
    let linked = repository.add_linked();
    let cargo = recording_cargo();
    let path = repository.hermetic_path(&[cargo.bin()]);
    let on_linked = commit_change(&linked, "linked change");
    let on_main = commit_change(repository.main(), "main change");

    assert_pushed(&repository.push(Launch::Direct, &linked, &path, &["origin", "linked"], &[]));
    assert_pushed(&repository.push(
        Launch::Direct,
        repository.main(),
        &path,
        &["origin", "main"],
        &[],
    ));

    let calls = cargo.invocations();
    assert_eq!(calls.len(), 2, "{calls:#?}");
    let zero = "0".repeat(on_linked.to_string().len());
    assert_eq!(
        calls[0]["stdin"],
        json!(update_line("linked", on_linked, &zero))
    );
    assert!(
        same_path(&recorded_cwd(&calls[0]), &linked),
        "{:#}",
        calls[0]
    );
    assert_eq!(
        calls[1]["stdin"],
        json!(update_line("main", on_main, &repository.first.to_string()))
    );
    assert!(
        same_path(&recorded_cwd(&calls[1]), repository.main()),
        "{:#}",
        calls[1]
    );
    assert_eq!(repository.remote_tip("refs/heads/linked"), Some(on_linked));
    assert_eq!(repository.remote_tip("refs/heads/main"), Some(on_main));
}

/// A `cargo` that runs the real `ci-push` the way `cargo run ... -- <args>`
/// would, and a `just` for the gates.
fn forwarding_cargo() -> Fixture {
    let ci_push = biscuit_test_harness::bin_exe!("ci-push");
    Fixture::new(
        &["cargo", "just"],
        json!({
            "cargo": {
                "forward": ci_push,
                "read_stdin": true,
                "record_env": ["RUSTY_BISCUIT_PRE_PUSH"],
            },
            "just": {},
        }),
    )
}

#[test]
fn the_real_ci_push_lets_a_tag_push_through_and_blocks_an_invalid_mode() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[(HOOK, include_str!("../../../.githooks/pre-push"))],
    );
    let second = commit_change(repository.main(), "second");
    git2::Repository::open(repository.main())
        .unwrap()
        .reference("refs/tags/v1", second, false, "tag")
        .unwrap();
    let tools = forwarding_cargo();
    let path = repository.host_path(&[tools.bin()]);

    let tag = repository.push(
        Launch::Direct,
        repository.main(),
        &path,
        &["origin", "v1"],
        &[],
    );
    let refused = repository.push(
        Launch::Direct,
        repository.main(),
        &path,
        &["origin", "main"],
        &[("RUSTY_BISCUIT_PRE_PUSH", "bogus")],
    );

    assert_pushed(&tag);
    assert!(
        stderr(&tag).contains("This push updates no branch"),
        "{}",
        stderr(&tag)
    );
    assert_eq!(repository.remote_tip("refs/tags/v1"), Some(second));
    let refusal = stderr(&refused);
    assert!(!refused.status.success(), "{refusal}");
    assert!(
        refusal.contains("unknown RUSTY_BISCUIT_PRE_PUSH value \"bogus\""),
        "{refusal}"
    );
    assert!(refusal.contains("failed to push some refs"), "{refusal}");
    assert_eq!(
        repository.remote_tip("refs/heads/main"),
        Some(repository.first)
    );
    let modes: Vec<Value> = tools
        .invocations()
        .iter()
        .filter(|call| call["tool"] == "cargo")
        .map(|call| call["env"]["RUSTY_BISCUIT_PRE_PUSH"].clone())
        .collect();
    assert_eq!(modes, [Value::Null, json!("bogus")]);
}

/// The planner stand-in: `--apply-to` copies the reviewed plan, any other
/// call writes the committed `plan.json`.
const AFFECTED_SCOPE: &str = r#"import os, sys
args = sys.argv[1:]
def value(flag):
    return args[args.index(flag) + 1]
if "--apply-to" in args:
    with open(value("--apply-to")) as source, open(value("--plan-out"), "w") as target:
        target.write(source.read())
else:
    with open(os.path.join(os.path.dirname(os.path.abspath(__file__)), "plan.json")) as plan, open(value("--plan-out"), "w") as out:
        out.write(plan.read())
    print('{"scope": "document"}')
"#;

/// The evidence stand-in: `verify` rejects nothing, `scope-record` answers
/// a receipt.
const LOCAL_EVIDENCE: &str = r#"import json, sys
args = sys.argv[1:]
def value(flag):
    return args[args.index(flag) + 1]
if args[0] == "verify":
    with open(value("--rejections"), "w") as rejections:
        rejections.write("[]")
    print("[]")
elif args[0] == "scope-record":
    print(json.dumps({"receipt": "scope", "base": value("--base"), "head": value("--head")}))
else:
    sys.exit("unexpected " + args[0])
"#;

/// Through `launch`: an auto-mode push of a branch no CI run follows is
/// reviewed and runs no gate; a strict push of `main` runs the gates.
fn the_real_ci_push_reviews_auto_and_strict_pushes(launch: Launch, repository: &HookRepository) {
    let second = commit_change(repository.main(), "second");
    let topic = git2::Repository::open(repository.main())
        .unwrap()
        .reference("refs/heads/topic", second, false, "topic")
        .unwrap()
        .target()
        .unwrap();
    let tools = forwarding_cargo();
    let path = repository.host_path(&[tools.bin()]);
    let gates = |tools: &Fixture| {
        tools
            .invocations()
            .iter()
            .filter(|call| call["tool"] == "just")
            .count()
    };

    let auto = repository.push(launch, repository.main(), &path, &["origin", "topic"], &[]);
    assert_pushed(&auto);
    assert!(stderr(&auto).contains("No gate runs"), "{}", stderr(&auto));
    assert_eq!(gates(&tools), 0);
    assert_eq!(repository.remote_tip("refs/heads/topic"), Some(topic));

    let strict = repository.push(
        launch,
        repository.main(),
        &path,
        &["origin", "main"],
        &[("RUSTY_BISCUIT_PRE_PUSH", "strict")],
    );
    assert_pushed(&strict);
    assert!(
        stderr(&strict).contains("Pre-push validation passed."),
        "{}",
        stderr(&strict)
    );
    assert_eq!(gates(&tools), 1);
    assert_eq!(repository.remote_tip("refs/heads/main"), Some(second));
}

#[test]
fn the_real_ci_push_reviews_an_auto_push_without_gates_and_a_strict_push_with_them() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[
            (HOOK, include_str!("../../../.githooks/pre-push")),
            ("scripts/ci/affected_scope.py", AFFECTED_SCOPE),
            ("scripts/ci/local_evidence.py", LOCAL_EVIDENCE),
            (
                "scripts/ci/plan.json",
                include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json"),
            ),
        ],
    );
    the_real_ci_push_reviews_auto_and_strict_pushes(Launch::Direct, &repository);
}

#[cfg(windows)]
#[test]
fn windows_git_bash_pushes_through_the_launcher_to_the_real_ci_push() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[
            (HOOK, include_str!("../../../.githooks/pre-push")),
            ("scripts/ci/affected_scope.py", AFFECTED_SCOPE),
            ("scripts/ci/local_evidence.py", LOCAL_EVIDENCE),
            (
                "scripts/ci/plan.json",
                include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json"),
            ),
        ],
    );
    the_real_ci_push_reviews_auto_and_strict_pushes(Launch::GitBash, &repository);
}

#[cfg(windows)]
#[test]
fn windows_powershell_pushes_through_the_launcher_to_the_real_ci_push() {
    let repository = HookRepository::new(
        include_str!("../../../.githooks/pre-push-dispatcher"),
        &[
            (HOOK, include_str!("../../../.githooks/pre-push")),
            ("scripts/ci/affected_scope.py", AFFECTED_SCOPE),
            ("scripts/ci/local_evidence.py", LOCAL_EVIDENCE),
            (
                "scripts/ci/plan.json",
                include_str!("../../ci/fixtures/pre-push/plans/plan-macos-executing.json"),
            ),
        ],
    );
    the_real_ci_push_reviews_auto_and_strict_pushes(Launch::PowerShell, &repository);
}

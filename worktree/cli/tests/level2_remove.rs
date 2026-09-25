//! Level 2: `wt remove` in a real terminal (a detached tmux pane, which never
//! takes focus).
//!
//! - Each question starts after exactly one blank line below the report
//!   (item 1), including the Not safe menu.
//! - Move-first removal through the generated bash, zsh, and fish wrappers:
//!   the prompt works while the wrapper captures stdout, the shell lands in the
//!   fork parent's worktree or the base repo, and a failed `cd`, an expired
//!   token, or a changed branch tip between the two runs leaves the worktree
//!   intact.
//!
//! Each scenario runs a script in the pane and reads its exit code and final
//! directory from a marker file. A `wt` shim on `PATH` runs
//! `WT_TEST_BETWEEN` just before the wrapper's `wt remove --handoff` call,
//! which is how state changes "between the runs".
#![cfg(unix)]

use std::fs;
use std::os::unix::fs::PermissionsExt as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

fn git(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .expect("git should be installed");
    assert!(
        out.status.success(),
        "git {args:?} in {dir:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

fn canonical(path: &Path) -> PathBuf {
    fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf())
}

fn quote(path: &Path) -> String {
    quote_text(&path.display().to_string())
}

fn quote_text(text: &str) -> String {
    format!("'{}'", text.replace('\'', r"'\''"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Shell {
    Bash,
    Zsh,
    Fish,
}

impl Shell {
    fn name(self) -> &'static str {
        match self {
            Shell::Bash => "bash",
            Shell::Zsh => "zsh",
            Shell::Fish => "fish",
        }
    }

    fn installed(self) -> bool {
        Command::new(self.name())
            .arg("-c")
            .arg("exit 0")
            .output()
            .is_ok_and(|out| out.status.success())
    }

    /// Runs `script` without any startup files.
    fn command_line(self, script: &Path) -> String {
        match self {
            Shell::Bash => format!("bash --noprofile --norc {}", quote(script)),
            Shell::Zsh => format!("zsh -f {}", quote(script)),
            Shell::Fish => format!("fish --no-config {}", quote(script)),
        }
    }
}

struct Scene {
    root: tempfile::TempDir,
}

struct Outcome {
    code: i32,
    pwd: PathBuf,
}

impl Scene {
    /// `root/repo` on `main` with a committed `docs/`, plus the `wt` shim.
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir_all(repo.join("docs")).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("gc.auto", "0"),
        ] {
            git(&repo, &["config", key, value]);
        }
        fs::write(repo.join("docs/guide.md"), "guide\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "initial"]);

        let bin = root.path().join("bin");
        fs::create_dir(&bin).unwrap();
        let shim = bin.join("wt");
        fs::write(
            &shim,
            format!(
                "#!/bin/sh\n\
                if [ \"$1\" = remove ] && [ \"$2\" = --handoff ] && [ -n \"${{WT_TEST_BETWEEN:-}}\" ]; then\n\
                \x20   WT_TOKEN=\"$3\" sh -c \"$WT_TEST_BETWEEN\"\n\
                fi\n\
                exec {} \"$@\"\n",
                quote(&cargo_bin("wt"))
            ),
        )
        .unwrap();
        fs::set_permissions(&shim, fs::Permissions::from_mode(0o755)).unwrap();
        Self { root }
    }

    fn repo(&self) -> PathBuf {
        self.root.path().join("repo")
    }

    fn add_worktree(&self, branch: &str, dir_name: &str) -> PathBuf {
        let path = self.root.path().join("wts").join(dir_name);
        git(&self.repo(), &["worktree", "add", "-q", "-b", branch, path.to_str().unwrap(), "main"]);
        path
    }

    fn record_fork(&self, branch: &str, parent: &str) {
        let store = worktree::fork_origin::fork_origin_path(&self.repo()).unwrap();
        worktree::fork_origin::record(
            &store,
            branch,
            worktree::fork_origin::ForkOrigin {
                base_branch: parent.to_string(),
                base_sha: git(&self.repo(), &["rev-parse", parent]),
                created_at: 0,
            },
        )
        .unwrap();
    }

    fn branch_exists(&self, branch: &str) -> bool {
        Command::new("git")
            .current_dir(self.repo())
            .args(["rev-parse", "--verify", "--quiet", &format!("refs/heads/{branch}")])
            .status()
            .unwrap()
            .success()
    }

    /// Writes a script that sources the generated wrapper (unless `wrapped`
    /// is false), moves to `cwd`, runs `command`, and records the result.
    fn script(&self, shell: Shell, wrapped: bool, cwd: &Path, command: &str, between: &str) -> PathBuf {
        let wrapper = self.root.path().join(format!("wrapper.{}", shell.name()));
        let generated = Command::new(cargo_bin("wt"))
            .args(["--completions", shell.name()])
            .output()
            .unwrap();
        fs::write(&wrapper, generated.stdout).unwrap();
        let marker = self.marker();
        let _ = fs::remove_file(&marker);
        let bin = self.root.path().join("bin");
        let source = if wrapped {
            format!(". {}\n", quote(&wrapper))
        } else {
            String::new()
        };
        let body = match shell {
            Shell::Bash | Shell::Zsh => format!(
                "export PATH={bin}:\"$PATH\"\n\
                export NO_COLOR=1\n\
                unset CI WT_SHELL_WRAPPER\n\
                export WT_TEST_BETWEEN={between}\n\
                compdef() {{ :; }}\n\
                {source}\
                cd {cwd} || exit 90\n\
                {command}\n\
                rc=$?\n\
                printf 'rc=%s\\npwd=%s\\n' \"$rc\" \"$(pwd -P)\" > {marker}\n",
                bin = quote(&bin),
                between = quote_text(between),
                cwd = quote(cwd),
                marker = quote(&marker),
            ),
            Shell::Fish => format!(
                "set -gx PATH {bin} $PATH\n\
                set -gx NO_COLOR 1\n\
                set -e CI\n\
                set -e WT_SHELL_WRAPPER\n\
                set -gx WT_TEST_BETWEEN {between}\n\
                {source}\
                cd {cwd}; or exit 90\n\
                {command}\n\
                set rc $status\n\
                printf 'rc=%s\\npwd=%s\\n' $rc (pwd -P) > {marker}\n",
                bin = quote(&bin),
                between = quote_text(between),
                cwd = quote(cwd),
                marker = quote(&marker),
            ),
        };
        let path = self.root.path().join(format!("scene.{}", shell.name()));
        fs::write(&path, body).unwrap();
        path
    }

    fn marker(&self) -> PathBuf {
        self.root.path().join("marker")
    }

    fn start(&self, harness: &mut TmuxHarness, shell: Shell, script: &Path) {
        harness
            .send_text(format!("clear; {}\n", shell.command_line(script)).as_bytes())
            .expect("send script");
    }

    fn wait_outcome(&self) -> Outcome {
        let deadline = Instant::now() + Duration::from_secs(30);
        loop {
            if let Ok(text) = fs::read_to_string(self.marker())
                && text.ends_with('\n')
                && text.contains("pwd=")
            {
                let value = |key: &str| {
                    text.lines()
                        .find_map(|line| line.strip_prefix(key))
                        .unwrap_or_default()
                        .to_string()
                };
                return Outcome {
                    code: value("rc=").parse().unwrap_or(-1),
                    pwd: PathBuf::from(value("pwd=")),
                };
            }
            assert!(Instant::now() < deadline, "the scene never finished");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
}

impl Scene {
    /// The handoff record path up to the token: `<cache>/<hash>.handoff-`.
    fn handoff_prefix(&self) -> String {
        let zeros = "0".repeat(32);
        let probe = worktree::remove::handoff::handoff_path(&self.repo(), &zeros).unwrap();
        probe
            .to_string_lossy()
            .trim_end_matches(&format!("{zeros}.json"))
            .to_string()
    }
}

impl Drop for Scene {
    fn drop(&mut self) {
        if let Ok(path) = worktree::fork_origin::fork_origin_path(&self.repo()) {
            let _ = fs::remove_file(path);
        }
        // A scene whose wrapper never reached the handoff leaves its record.
        let prefix = PathBuf::from(self.handoff_prefix());
        if let (Some(dir), Some(stem)) = (prefix.parent(), prefix.file_name()) {
            let stem = stem.to_string_lossy().into_owned();
            for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
                if entry.file_name().to_string_lossy().starts_with(&stem) {
                    let _ = fs::remove_file(entry.path());
                }
            }
        }
    }
}

fn wait_for_text(harness: &mut TmuxHarness, needle: &str) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(frame) = harness.capture()
            && frame.plain.contains(needle)
        {
            // One more beat so the prompt is fully drawn.
            std::thread::sleep(Duration::from_millis(150));
            return harness.capture().map(|f| f.plain).unwrap_or(frame.plain);
        }
        assert!(Instant::now() < deadline, "{needle:?} never appeared");
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// Asserts the line holding `needle` follows exactly one blank line, which
/// itself follows text.
fn assert_one_blank_line_before(plain: &str, needle: &str) {
    let lines: Vec<&str> = plain.lines().collect();
    let at = lines
        .iter()
        .position(|line| line.contains(needle))
        .unwrap_or_else(|| panic!("{needle:?} not on screen:\n{plain}"));
    assert!(at >= 2, "{needle:?} too close to the top:\n{plain}");
    assert!(lines[at - 1].trim().is_empty(), "no blank line before {needle:?}:\n{plain}");
    assert!(!lines[at - 2].trim().is_empty(), "more than one blank line before {needle:?}:\n{plain}");
}

fn harness() -> TmuxHarness {
    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn_shell failed");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    harness
}

#[test]
#[serial(level2_terminal)]
fn level2_remove_files_question_follows_one_blank_line() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/x", "feat-x");
    fs::write(wt.join("lib.rs"), "fn a() {}\n").unwrap();
    let mut harness = harness();

    let script = scene.script(Shell::Bash, false, &scene.repo(), "wt remove feat-x", "");
    scene.start(&mut harness, Shell::Bash, &script);
    let plain = wait_for_text(&mut harness, "Discard the files listed above");
    assert!(plain.contains("Uncommitted files (1):"), "{plain}");
    assert!(plain.contains("lib.rs"), "{plain}");
    assert_one_blank_line_before(&plain, "Discard the files listed above");

    // Default No: nothing is removed and the run succeeds.
    harness.send_text(b"n\n").unwrap();
    let outcome = scene.wait_outcome();
    assert_eq!(outcome.code, 0);
    assert!(wt.join("lib.rs").exists());
    assert!(scene.branch_exists("feat/x"));
}

#[test]
#[serial(level2_terminal)]
fn level2_remove_not_safe_menu_defaults_to_keeping_the_branch() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/y", "feat-y");
    fs::write(wt.join("only.txt"), "x\n").unwrap();
    git(&wt, &["add", "only.txt"]);
    git(&wt, &["commit", "-q", "-m", "only here"]);
    let mut harness = harness();

    let script = scene.script(Shell::Bash, false, &scene.repo(), "wt remove feat-y", "");
    scene.start(&mut harness, Shell::Bash, &script);
    let plain = wait_for_text(&mut harness, "Delete it and lose 1 commit");
    assert!(plain.contains("Keep the branch"), "{plain}");
    assert!(plain.contains("only here"), "the lost commit is listed:\n{plain}");
    assert_one_blank_line_before(&plain, "Deleting the branch would lose 1 commit:");
    assert_one_blank_line_before(&plain, "is not safe to delete");

    harness.send_key("Enter").unwrap();
    let outcome = scene.wait_outcome();
    assert_eq!(outcome.code, 0);
    assert!(!wt.exists(), "the worktree is removed");
    assert!(scene.branch_exists("feat/y"), "Enter keeps the branch");
}

fn shells() -> Vec<Shell> {
    [Shell::Bash, Shell::Zsh, Shell::Fish]
        .into_iter()
        .filter(|shell| {
            let ok = shell.installed();
            if !ok {
                eprintln!("skipping {}: not installed", shell.name());
            }
            ok
        })
        .collect()
}

#[test]
#[serial(level2_terminal)]
fn level2_move_first_through_each_wrapper_lands_in_the_fork_parent() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = harness();
    for shell in shells() {
        let scene = Scene::new();
        let parent = scene.add_worktree("feat/theme", "feat-theme");
        let wt = scene.add_worktree("feat/x", "feat-x");
        scene.record_fork("feat/x", "feat/theme");
        fs::write(wt.join("notes.txt"), "draft\n").unwrap();
        let inside = wt.join("docs");

        let script = scene.script(shell, true, &inside, "wt remove feat-x", "");
        scene.start(&mut harness, shell, &script);
        // The prompt works while the wrapper captures stdout.
        let plain = wait_for_text(&mut harness, "Discard the files listed above");
        assert_one_blank_line_before(&plain, "Discard the files listed above");
        harness.send_text(b"y\n").unwrap();

        let outcome = scene.wait_outcome();
        let plain = harness.capture().unwrap().plain;
        assert_eq!(outcome.code, 0, "{shell:?}:\n{plain}");
        assert_eq!(canonical(&outcome.pwd), canonical(&parent.join("docs")), "{shell:?}");
        assert!(plain.contains("Moving you to the feat/theme worktree"), "{shell:?}:\n{plain}");
        assert!(plain.contains("Removed worktree"), "{shell:?}:\n{plain}");
        assert!(!wt.exists(), "{shell:?}");
        assert!(!scene.branch_exists("feat/x"), "{shell:?}: on main, so Safe");
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_move_first_lands_in_the_base_repo_without_a_fork_record() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = harness();
    for shell in shells() {
        let scene = Scene::new();
        let wt = scene.add_worktree("feat/x", "feat-x");

        let script = scene.script(shell, true, &wt, "wt remove feat-x", "");
        scene.start(&mut harness, shell, &script);
        let outcome = scene.wait_outcome();
        let plain = harness.capture().unwrap().plain;
        assert_eq!(outcome.code, 0, "{shell:?}:\n{plain}");
        assert_eq!(canonical(&outcome.pwd), canonical(&scene.repo()), "{shell:?}");
        assert!(plain.contains("Moving you to the base repo"), "{shell:?}:\n{plain}");
        assert!(!wt.exists(), "{shell:?}");
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_move_first_failures_leave_the_worktree_intact() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);
    let mut harness = harness();
    for shell in shells() {
        // A failed `cd`: the landing directory (the fork parent's worktree)
        // is gone, so the wrapper stops before the handoff.
        let scene = Scene::new();
        let parent = scene.add_worktree("feat/theme", "feat-theme");
        let wt = scene.add_worktree("feat/x", "feat-x");
        scene.record_fork("feat/x", "feat/theme");
        fs::remove_dir_all(&parent).unwrap();
        let script = scene.script(shell, true, &wt, "wt remove feat-x", "");
        scene.start(&mut harness, shell, &script);
        let outcome = scene.wait_outcome();
        assert_ne!(outcome.code, 0, "{shell:?}: failed cd");
        assert_eq!(canonical(&outcome.pwd), canonical(&wt), "{shell:?}: the shell stays");
        assert!(wt.exists(), "{shell:?}: failed cd");

        // An expired token.
        let scene = Scene::new();
        let wt = scene.add_worktree("feat/x", "feat-x");
        let expire = format!(
            "f=\"{}$WT_TOKEN.json\"; \
            sed 's/\"created_at\": [0-9]*/\"created_at\": 1/' \"$f\" > \"$f.tmp\" && mv \"$f.tmp\" \"$f\"",
            scene.handoff_prefix()
        );
        let script = scene.script(shell, true, &wt, "wt remove feat-x", &expire);
        scene.start(&mut harness, shell, &script);
        let outcome = scene.wait_outcome();
        let plain = harness.capture().unwrap().plain;
        assert_eq!(outcome.code, 4, "{shell:?}: expired:\n{plain}");
        assert!(plain.contains("expired"), "{shell:?}:\n{plain}");
        assert!(wt.exists(), "{shell:?}: expired");

        // A changed branch tip.
        let scene = Scene::new();
        let wt = scene.add_worktree("feat/x", "feat-x");
        let commit = format!(
            "git -C {} commit -q --allow-empty -m late",
            quote(&wt)
        );
        let script = scene.script(shell, true, &wt, "wt remove feat-x", &commit);
        scene.start(&mut harness, shell, &script);
        let outcome = scene.wait_outcome();
        let plain = harness.capture().unwrap().plain;
        assert_eq!(outcome.code, 3, "{shell:?}: changed tip:\n{plain}");
        assert!(plain.contains("checked-out commit"), "{shell:?}:\n{plain}");
        assert!(wt.exists(), "{shell:?}: changed tip");
        assert!(scene.branch_exists("feat/x"), "{shell:?}");
    }
}

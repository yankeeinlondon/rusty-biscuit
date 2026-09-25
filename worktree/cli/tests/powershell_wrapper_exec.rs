//! Runs the generated PowerShell wrapper against the real `wt` on Windows.
//!
//! PowerShell is launched with the worktree as its working directory, the
//! case that holds the directory on Windows: the wrapper must move both the
//! location and the process's current directory before the handoff call can
//! remove it. The scenarios ask no questions, so no terminal is needed.
#![cfg(windows)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin;
use worktree::remove::handoff::canonical;

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

struct Scene {
    root: tempfile::TempDir,
}

impl Scene {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        for (key, value) in [
            ("user.email", "test@example.com"),
            ("user.name", "Test User"),
            ("commit.gpgsign", "false"),
            ("gc.auto", "0"),
        ] {
            git(&repo, &["config", key, value]);
        }
        fs::write(repo.join("README.md"), "readme\n").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-q", "-m", "initial"]);
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

    /// Runs `command` through the wrapper in a PowerShell started inside
    /// `cwd`, returning the exit code and final location it recorded.
    fn run(&self, cwd: &Path, command: &str) -> (i32, PathBuf, String) {
        let wrapper = self.root.path().join("wrapper.ps1");
        let generated = Command::new(cargo_bin("wt"))
            .args(["--completions", "powershell"])
            .output()
            .unwrap();
        fs::write(&wrapper, generated.stdout).unwrap();
        let marker = self.root.path().join("marker.txt");
        let _ = fs::remove_file(&marker);
        let script = self.root.path().join("scene.ps1");
        fs::write(
            &script,
            format!(
                ". '{wrapper}'\r\n\
                $global:LASTEXITCODE = 0\r\n\
                {command}\r\n\
                $rc = $global:LASTEXITCODE\r\n\
                \"rc=$rc\" | Set-Content -Encoding ascii '{marker}'\r\n\
                \"pwd=$((Get-Location).ProviderPath)\" | Add-Content -Encoding ascii '{marker}'\r\n",
                wrapper = wrapper.display(),
                marker = marker.display(),
            ),
        )
        .unwrap();
        let output = Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-File"])
            .arg(&script)
            .current_dir(cwd)
            .env_remove("WT_SHELL_WRAPPER")
            .env_remove("CI")
            .env("NO_COLOR", "1")
            .output()
            .expect("powershell.exe is present on Windows");
        let log = format!(
            "stdout:\n{}\nstderr:\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let text = fs::read_to_string(&marker).unwrap_or_else(|_| panic!("no marker\n{log}"));
        let value = |key: &str| {
            text.lines()
                .find_map(|line| line.strip_prefix(key))
                .unwrap_or_default()
                .to_string()
        };
        (value("rc=").parse().unwrap_or(-1), PathBuf::from(value("pwd=")), log)
    }
}

#[test]
fn move_first_removes_the_worktree_powershell_was_launched_inside() {
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/x", "feat-x");

    let (code, pwd, log) = scene.run(&wt, "wt remove feat-x");

    assert_eq!(code, 0, "{log}");
    assert_eq!(canonical(&pwd), canonical(&scene.repo()), "{log}");
    assert!(!wt.exists(), "the worktree is removed\n{log}");
    assert!(log.contains("Moving you to the base repo"), "{log}");
}

#[test]
fn a_directory_held_by_another_program_exits_4_with_nothing_removed() {
    let scene = Scene::new();
    let wt = scene.add_worktree("feat/x", "feat-x");
    // Spawned directly, not through `cmd /C`: killing `cmd` would leave its
    // `ping` child holding the directory and the test's output pipe.
    let mut holder = Command::new("ping")
        .args(["-n", "60", "127.0.0.1"])
        .current_dir(&wt)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();
    std::thread::sleep(std::time::Duration::from_millis(300));

    let output = Command::new(cargo_bin("wt"))
        .args(["remove", "feat-x"])
        .current_dir(scene.repo())
        .env_remove("WT_SHELL_WRAPPER")
        .env_remove("CI")
        .env("NO_COLOR", "1")
        .output()
        .unwrap();

    let _ = holder.kill();
    let _ = holder.wait();
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(4), "{stderr}");
    assert!(stderr.contains("in use by another program"), "{stderr}");
    assert!(wt.join("README.md").exists(), "files survive");
    assert!(
        git(&scene.repo(), &["worktree", "list", "--porcelain"]).contains("feat/x"),
        "still registered"
    );
    assert!(
        git(&scene.repo(), &["branch", "--list", "feat/x"]).contains("feat/x"),
        "branch kept"
    );
}

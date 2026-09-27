use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git").current_dir(repo).args(args)
        .env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

struct Fixture {
    root: tempfile::TempDir,
    repo: PathBuf,
    base: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let root = tempfile::tempdir().unwrap();
        let repo = root.path().join("repo");
        let base = root.path().join("wts");
        fs::create_dir(&repo).unwrap();
        fs::create_dir(&base).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        for (key, value) in [("user.name", "Test User"), ("user.email", "test@example.com"), ("commit.gpgsign", "false")] {
            git(&repo, &["config", key, value]);
        }
        fs::write(repo.join("README.md"), b"initial\n").unwrap();
        git(&repo, &["add", "README.md"]);
        git(&repo, &["commit", "-q", "-m", "initial"]);
        Self { root, repo, base }
    }

    fn run(&self, branch: &str, wrapper: bool) -> Output {
        let mut command = Command::new(biscuit_test_harness::bin_exe!("wt"));
        command.current_dir(&self.repo).args(["create", branch])
            .env("WT", &self.base).env("NO_COLOR", "1")
            .env("XDG_CACHE_HOME", self.root.path().join("cache"))
            .env("GIT_TERMINAL_PROMPT", "0").env_remove("CI");
        if wrapper { command.env("WT_SHELL_WRAPPER", "1"); }
        else { command.env_remove("WT_SHELL_WRAPPER"); }
        command.output().unwrap()
    }

    fn destination(&self, branch: &str) -> PathBuf {
        self.base.join("repo").join(branch)
    }
}

#[test]
fn create_reports_copied_paths_on_stderr_and_preserves_stdout_protocol() {
    let fixture = Fixture::new();
    fs::write(fixture.repo.join(".gitignore"), "*.env\n").unwrap();
    fs::write(fixture.repo.join(".worktreeinclude"), "*.env\n").unwrap();
    fs::write(fixture.repo.join("a space.env"), b"secret").unwrap();
    let output = fixture.run("first", false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("Copied from main: a\\ space.env"), "{stderr}");
    assert_eq!(fs::read(fixture.destination("first").join("a space.env")).unwrap(), b"secret");
    let wrapped = fixture.run("second", true);
    assert!(wrapped.status.success(), "{}", String::from_utf8_lossy(&wrapped.stderr));
    assert_eq!(String::from_utf8_lossy(&wrapped.stdout), format!("cd:{}\n", fixture.destination("second").join("").display()));
    assert!(String::from_utf8_lossy(&wrapped.stderr).contains("Copied from main: a\\ space.env"));
}

#[test]
fn create_warns_after_success_when_rules_are_invalid() {
    let fixture = Fixture::new();
    fs::create_dir(fixture.repo.join(".worktreeinclude")).unwrap();
    let output = fixture.run("warning", true);
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("cd:{}\n", fixture.destination("warning").join("").display()));
    assert!(String::from_utf8_lossy(&output.stderr).contains("cannot read .worktreeinclude"));
    assert!(fixture.destination("warning").exists());
}

#[cfg(unix)]
#[test]
fn create_makes_control_characters_visible_in_the_report() {
    let fixture = Fixture::new();
    fs::write(fixture.repo.join(".gitignore"), "*.env\n").unwrap();
    fs::write(fixture.repo.join(".worktreeinclude"), "*.env\n").unwrap();
    fs::write(fixture.repo.join("line\nbreak.env"), b"secret").unwrap();
    fs::write(fixture.repo.join("<tag>.env"), b"tag secret").unwrap();
    let output = fixture.run("control", false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("line\\nbreak.env"), "{stderr}");
    assert!(stderr.contains("<tag>.env"), "{stderr}");
    assert_eq!(fs::read(fixture.destination("control").join("line\nbreak.env")).unwrap(), b"secret");
    assert_eq!(fs::read(fixture.destination("control").join("<tag>.env")).unwrap(), b"tag secret");
}

#[cfg(unix)]
#[test]
fn create_reports_a_file_copy_failure_and_keeps_the_worktree() {
    use std::os::unix::fs::PermissionsExt;
    let fixture = Fixture::new();
    fs::write(fixture.repo.join(".gitignore"), "*.env\n").unwrap();
    fs::write(fixture.repo.join(".worktreeinclude"), "*.env\n").unwrap();
    let source = fixture.repo.join("unreadable.env");
    fs::write(&source, b"secret").unwrap();
    fs::set_permissions(&source, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::File::open(&source).is_ok() {
        fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
        return;
    }
    let output = fixture.run("file-failure", true);
    fs::set_permissions(&source, fs::Permissions::from_mode(0o600)).unwrap();
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert_eq!(String::from_utf8_lossy(&output.stdout), format!("cd:{}\n", fixture.destination("file-failure").join("").display()));
    assert!(String::from_utf8_lossy(&output.stderr).contains("Could not copy unreadable.env"));
    assert!(fixture.destination("file-failure").exists());
    assert!(!fixture.destination("file-failure").join("unreadable.env").exists());
}

#[cfg(unix)]
#[test]
fn create_reports_an_unsupported_link_without_copying_its_target() {
    use std::os::unix::fs::symlink;
    let fixture = Fixture::new();
    fs::write(fixture.repo.join(".gitignore"), "*.env\n").unwrap();
    fs::write(fixture.repo.join(".worktreeinclude"), "*.env\n").unwrap();
    let target = fixture.root.path().join("outside");
    fs::create_dir(&target).unwrap();
    fs::write(target.join("secret.txt"), b"secret").unwrap();
    symlink(&target, fixture.repo.join("linked.env")).unwrap();
    let output = fixture.run("linked", false);
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported included path: linked.env"));
    assert!(!fixture.destination("linked").join("linked.env").exists());
    assert_eq!(fs::read(target.join("secret.txt")).unwrap(), b"secret");
}

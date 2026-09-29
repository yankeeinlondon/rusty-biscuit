use std::path::{Path, PathBuf};
use std::process::Command;
use std::io::Write;

use crate::error::WorktreeError;

#[derive(Debug, Clone)]
pub struct RepoInfo {
    /// Name of the repository (directory name of the root)
    pub name: String,
    /// Absolute path to the repository root
    pub root: PathBuf,
    /// Relative path from repo root to CWD
    pub relative_path: PathBuf,
}

/// Verify git is installed and available on PATH.
pub fn ensure_git() -> Result<(), WorktreeError> {
    which::which("git").map_err(|_| WorktreeError::GitNotFound)?;
    Ok(())
}

/// Get repository info for the current working directory.
///
/// ## Errors
///
/// Returns `NotInGitRepo` if the current directory is not inside a git repository.
pub fn repo_info() -> Result<RepoInfo, WorktreeError> {
    ensure_git()?;

    let root = git_rev_parse("--show-toplevel")?;
    let root = PathBuf::from(root);

    // Derive the repo name from the main worktree path, not the current checkout.
    // Inside a linked worktree, --show-toplevel returns the worktree dir (e.g.
    // /tmp/wt/rusty-biscuit/feat-xyz), so file_name() would give "feat-xyz".
    // The first entry from `git worktree list` is always the main checkout.
    let name = main_worktree_name().unwrap_or_else(|_| {
        root.file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default()
    });

    let cwd = std::env::current_dir()?;
    let relative_path = cwd
        .strip_prefix(&root)
        .unwrap_or_else(|_| std::path::Path::new(""))
        .to_path_buf();

    Ok(RepoInfo {
        name,
        root,
        relative_path,
    })
}

/// Derive the repository name from the main worktree path.
///
/// `git worktree list --porcelain` always lists the main checkout first,
/// so its directory name is the canonical repo name even when called from
/// inside a linked worktree.
fn main_worktree_name() -> Result<String, WorktreeError> {
    let output = git_command(&["worktree", "list", "--porcelain"])?;
    let main_path = output
        .lines()
        .find_map(|line| line.strip_prefix("worktree "))
        .ok_or_else(|| WorktreeError::GitParse("cannot find main worktree".into()))?;
    PathBuf::from(main_path)
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .ok_or_else(|| WorktreeError::GitParse("cannot determine repo name".into()))
}

/// Run `git rev-parse` with the given argument and return trimmed stdout.
fn git_rev_parse(arg: &str) -> Result<String, WorktreeError> {
    let output = Command::new("git")
        .args(["rev-parse", arg])
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        return Err(WorktreeError::NotInGitRepo);
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run an arbitrary git command and return trimmed stdout.
pub fn git_command(args: &[&str]) -> Result<String, WorktreeError> {
    git_command_status(args, false).map(Option::unwrap_or_default)
}

/// Like [`git_command`], but exit code 1 is `Ok(None)`: the answer "no" of
/// commands such as `merge-base --is-ancestor`, or `merge-base` finding no
/// common ancestor. Any other failure is still `Err`.
pub fn git_command_allow_no_match(args: &[&str]) -> Result<Option<String>, WorktreeError> {
    git_command_status(args, true)
}

fn git_command_status(args: &[&str], allow_no_match: bool) -> Result<Option<String>, WorktreeError> {
    #[cfg(any(test, feature = "count-git"))]
    {
        recorder::record(args);
        if recorder::injected_failure(args) {
            return Err(WorktreeError::GitCommand("injected failure".to_string()));
        }
    }

    let output = Command::new("git")
        .args(args)
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        if allow_no_match && output.status.code() == Some(1) {
            return Ok(None);
        }
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorktreeError::GitCommand(stderr));
    }

    Ok(Some(String::from_utf8_lossy(&output.stdout).trim().to_string()))
}

/// Run a git command in a specific directory.
pub fn git_command_in(dir: &std::path::Path, args: &[&str]) -> Result<String, WorktreeError> {
    #[cfg(any(test, feature = "count-git"))]
    recorder::record(args);

    let output = Command::new("git")
        .current_dir(dir)
        .args(args)
        .output()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorktreeError::GitCommand(stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// Run `git -C <dir> <args>` with `base` as the process's working directory,
/// returning trimmed stdout.
///
/// Removal code addresses a worktree only through `-C` and runs from the base
/// repository, so no git process (or orphan of one) holds the worktree as its
/// current directory; on Windows that would block deleting it.
pub fn git_from(base: &Path, dir: &Path, args: &[&str]) -> Result<String, WorktreeError> {
    git_from_raw(base, dir, args).map(|out| out.trim().to_string())
}

/// [`git_from`] without trimming, for NUL-separated (`-z`) output.
///
/// Invalid UTF-8 becomes U+FFFD, so distinct paths can read as one; use
/// [`git_from_bytes`] where a path's identity matters.
pub fn git_from_raw(base: &Path, dir: &Path, args: &[&str]) -> Result<String, WorktreeError> {
    git_from_bytes(base, dir, args, None).map(|out| String::from_utf8_lossy(&out).into_owned())
}

/// [`git_from`]'s stdout exactly as git wrote it.
pub fn git_from_bytes(base: &Path, dir: &Path, args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>, WorktreeError> {
    git_from_bytes_status(base, dir, args, stdin, false)
}

/// Like [`git_from_bytes`], but accepts Git's no-matches exit code.
pub(crate) fn git_from_bytes_allow_no_match(base: &Path, dir: &Path, args: &[&str], stdin: Option<&[u8]>) -> Result<Vec<u8>, WorktreeError> {
    git_from_bytes_status(base, dir, args, stdin, true)
}

fn git_from_bytes_status(base: &Path, dir: &Path, args: &[&str], stdin: Option<&[u8]>, allow_no_match: bool) -> Result<Vec<u8>, WorktreeError> {
    #[cfg(any(test, feature = "count-git"))]
    recorder::record(args);

    let mut command = Command::new("git");
    command.current_dir(base).arg("-C").arg(dir).args(args)
        .stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped());
    if stdin.is_some() {
        command.stdin(std::process::Stdio::piped());
    }
    let mut child = command
        .spawn()
        .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;
    let output = std::thread::scope(|scope| {
        let writer = stdin.map(|bytes| {
            let mut pipe = child.stdin.take().expect("piped stdin");
            scope.spawn(move || pipe.write_all(bytes))
        });
        let output = child.wait_with_output()
            .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;
        if let Some(writer) = writer {
            writer.join().expect("git stdin writer panicked")
                .map_err(|e| WorktreeError::GitCommand(e.to_string()))?;
        }
        Ok::<_, WorktreeError>(output)
    })?;

    if !output.status.success() && !(allow_no_match && output.status.code() == Some(1)) {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(WorktreeError::GitCommand(stderr));
    }

    Ok(output.stdout)
}

#[cfg(any(test, feature = "count-git"))]
pub mod recorder {
    use std::sync::Mutex;

    #[derive(Default)]
    pub struct GitCallLog {
        pub calls: Vec<Vec<String>>,
    }

    static RECORDER: Mutex<Option<GitCallLog>> = Mutex::new(None);

    pub fn start_recording() {
        *RECORDER.lock().expect("recorder mutex poisoned") = Some(GitCallLog::default());
    }

    pub fn finish_recording() -> Vec<Vec<String>> {
        RECORDER
            .lock()
            .expect("recorder mutex poisoned")
            .take()
            .map(|log| log.calls)
            .unwrap_or_default()
    }

    pub fn record(args: &[&str]) {
        let mut guard = RECORDER.lock().expect("recorder mutex poisoned");
        if let Some(ref mut log) = *guard {
            log.calls.push(args.iter().map(|s| s.to_string()).collect());
        }
    }

    /// Decides which calls [`fail_matching`] fails.
    type FailWhen = Box<dyn Fn(&[&str]) -> bool + Send>;

    static FAIL_WHEN: Mutex<Option<FailWhen>> = Mutex::new(None);

    /// Makes every [`git_command`](super::git_command) and
    /// [`git_command_allow_no_match`](super::git_command_allow_no_match) call
    /// whose arguments `predicate` matches fail without running Git, until the
    /// returned guard drops. Other runners are unaffected.
    #[must_use = "the failure injection ends when the guard drops"]
    pub fn fail_matching(predicate: impl Fn(&[&str]) -> bool + Send + 'static) -> FailureGuard {
        *FAIL_WHEN.lock().expect("recorder mutex poisoned") = Some(Box::new(predicate));
        FailureGuard(())
    }

    /// Ends a [`fail_matching`] injection when dropped.
    pub struct FailureGuard(());

    impl Drop for FailureGuard {
        fn drop(&mut self) {
            *FAIL_WHEN.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = None;
        }
    }

    pub fn injected_failure(args: &[&str]) -> bool {
        FAIL_WHEN
            .lock()
            .expect("recorder mutex poisoned")
            .as_ref()
            .is_some_and(|predicate| predicate(args))
    }

    pub fn count_matching(calls: &[Vec<String>], predicate: impl Fn(&[String]) -> bool) -> usize {
        calls.iter().filter(|args| predicate(args)).count()
    }

    pub fn has_any_matching(calls: &[Vec<String>], predicate: impl Fn(&[String]) -> bool) -> bool {
        count_matching(calls, predicate) > 0
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::process::Command;

    use super::*;
    use crate::worktree::default_branch;

    fn run_git(repo: &Path, args: &[&str]) {
        let status = Command::new("git")
            .current_dir(repo)
            .args(args)
            .status()
            .expect("git should be installed");
        assert!(status.success(), "git {:?} failed in {:?}", args, repo);
    }

    /// RAII guard that changes CWD for the duration of a test and restores it on drop.
    struct DirGuard {
        old: PathBuf,
    }

    impl DirGuard {
        fn enter(dir: &Path) -> Self {
            let old = std::env::current_dir().expect("get cwd");
            std::env::set_current_dir(dir).expect("set cwd");
            DirGuard { old }
        }
    }

    impl Drop for DirGuard {
        fn drop(&mut self) {
            let _ = std::env::set_current_dir(&self.old);
        }
    }

    /// Create a temporary git repo with a known branch structure for
    /// deterministic subprocess-count assertions.
    fn temp_repo() -> tempfile::TempDir {
        let dir = tempfile::tempdir().expect("create temp dir");
        let path = dir.path();

        run_git(path, &["init", "-b", "main"]);
        run_git(path, &["config", "user.email", "test@example.com"]);
        run_git(path, &["config", "user.name", "Test User"]);
        run_git(path, &["config", "commit.gpgsign", "false"]);
        // Suppress background/detached git work so nextest leak detection
        // sees no lingering child processes after the test returns.
        run_git(path, &["config", "gc.auto", "0"]);
        run_git(path, &["config", "core.fsmonitor", "false"]);
        run_git(path, &["config", "core.commitGraph", "false"]);

        fs::write(path.join("file.txt"), "1\n").unwrap();
        run_git(path, &["add", "."]);
        run_git(path, &["commit", "-m", "commit 1"]);

        dir
    }

    #[test]
    fn ensure_git_succeeds() {
        assert!(ensure_git().is_ok());
    }

    #[test]
    #[serial_test::serial]
    fn allow_no_match_separates_a_no_from_a_failure_by_exit_code() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());
        fs::write(repo.path().join("file.txt"), "2\n").unwrap();
        run_git(repo.path(), &["commit", "-qam", "commit 2"]);

        assert_eq!(
            git_command_allow_no_match(&["merge-base", "--is-ancestor", "HEAD~1", "HEAD"]).unwrap(),
            Some(String::new())
        );
        assert_eq!(
            git_command_allow_no_match(&["merge-base", "--is-ancestor", "HEAD", "HEAD~1"]).unwrap(),
            None,
            "exit 1 is the answer no"
        );
        assert!(
            git_command_allow_no_match(&["merge-base", "--is-ancestor", "HEAD", "no-such-ref"]).is_err(),
            "exit 128 stays an error"
        );
        assert!(git_command(&["merge-base", "--is-ancestor", "HEAD", "HEAD~1"]).is_err());
    }

    #[test]
    #[serial_test::serial]
    fn repo_info_from_current_repo() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());

        let info = repo_info().unwrap();
        assert_eq!(
            info.name,
            repo.path().file_name().unwrap().to_string_lossy()
        );
        assert_eq!(info.root.canonicalize().unwrap(), repo.path().canonicalize().unwrap());
        assert!(info.relative_path.as_os_str().is_empty());
    }

    #[test]
    #[serial_test::serial]
    fn recorder_captures_default_branch_call() {
        let repo = temp_repo();
        let _guard = DirGuard::enter(repo.path());

        recorder::start_recording();
        let result = default_branch();
        let calls = recorder::finish_recording();

        assert!(result.is_ok(), "default_branch should succeed: {result:?}");

        let symbolic_ref_count = recorder::count_matching(&calls, |args| {
            args.first().map(String::as_str) == Some("symbolic-ref")
        });
        assert_eq!(
            symbolic_ref_count, 1,
            "expected exactly one symbolic-ref call, got {calls:?}"
        );
    }

    #[test]
    fn git_from_bytes_preserves_nul_input_and_output() {
        let repo = temp_repo();
        fs::write(repo.path().join(".gitignore"), b"*.env\n").unwrap();
        let input = b"a.env\0not ignored\0";
        let output = git_from_bytes(repo.path(), repo.path(),
            &["check-ignore", "--stdin", "-z", "--verbose", "--non-matching"], Some(input)).unwrap();
        assert!(output.windows(b"a.env\0".len()).any(|part| part == b"a.env\0"));
        assert!(output.windows(b"not ignored\0".len()).any(|part| part == b"not ignored\0"));
        assert!(output.contains(&0));
    }
}

//! The background refresh of a stale PR answer: `wt list` launches a detached
//! `wt internal-refresh-prs <main checkout>` and never waits for it.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use worktree::git::git_from;
use worktree::pull_requests::{
    OpenPrSource, REFRESH_DEADLINE, SniffOpenPrSource, pr_store_path, refresh, unix_now,
};

/// The hidden subcommand's name, shared by the launcher and `args.rs`.
pub const SUBCOMMAND: &str = "internal-refresh-prs";

/// Starts the worker for the repository whose main checkout is `main`.
///
/// The worker runs from `main`, so it never holds a linked worktree open, and
/// has no standard streams: on Windows an inherited pipe would make a caller
/// capturing `wt list`'s output wait for the worker. A spawn failure is
/// ignored; the stale answer stays shown.
pub fn launch(main: &Path) {
    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let mut command = Command::new(exe);
    command
        .arg(SUBCOMMAND)
        .arg(main)
        .current_dir(main)
        .env_remove("WT_SHELL_WRAPPER")
        .env_remove("COMPLETE")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    sniff::process::configure_detached_child(&mut command);
    // Dropping the handle neither waits for nor kills the worker.
    let _ = command.spawn();
}

/// The worker: refreshes the stored answer for `repo` when it is a main
/// checkout, and does nothing otherwise. It prints nothing and never fails,
/// since nobody reads its output or exit status.
pub fn run(repo: &Path) {
    let Some(main) = main_checkout(repo) else {
        return;
    };
    let Ok(store) = pr_store_path(&main) else {
        return;
    };
    refresh(&store, &main, unix_now, worker_source);
}

fn worker_source(origin: &str) -> Box<dyn OpenPrSource> {
    Box::new(SniffOpenPrSource {
        remote_url: origin.to_string(),
        deadline: REFRESH_DEADLINE,
    })
}

/// `repo` itself when it is the top level of a repository's main checkout
/// (not a linked worktree, not a subdirectory).
fn main_checkout(repo: &Path) -> Option<PathBuf> {
    let canonical = std::fs::canonicalize(repo).ok()?;
    let out = git_from(
        &canonical,
        &canonical,
        &["rev-parse", "--path-format=absolute", "--show-toplevel", "--git-dir", "--git-common-dir"],
    )
    .ok()?;
    let mut lines = out.lines().map(|line| std::fs::canonicalize(line.trim()).ok());
    let (Some(Some(top)), Some(Some(git_dir)), Some(Some(common))) = (lines.next(), lines.next(), lines.next()) else {
        return None;
    };
    (top == canonical && git_dir == common).then_some(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git(dir: &Path, args: &[&str]) {
        let status = Command::new("git").current_dir(dir).args(args).status().expect("git");
        assert!(status.success(), "git {args:?}");
    }

    #[test]
    fn only_the_top_level_of_a_main_checkout_is_accepted() {
        let root = tempfile::tempdir().expect("temp dir");
        let main = root.path().join("main");
        std::fs::create_dir_all(main.join("sub")).unwrap();
        git(&main, &["init", "-q", "-b", "main"]);
        git(&main, &["-c", "user.email=t@example.com", "-c", "user.name=T", "-c", "commit.gpgsign=false", "commit", "-q", "--allow-empty", "-m", "c"]);
        let linked = root.path().join("linked");
        git(&main, &["worktree", "add", "-q", "-b", "feat", linked.to_str().unwrap()]);

        assert_eq!(main_checkout(&main), Some(std::fs::canonicalize(&main).unwrap()));
        assert_eq!(main_checkout(&linked), None, "a linked worktree");
        assert_eq!(main_checkout(&main.join("sub")), None, "a subdirectory");
        assert_eq!(main_checkout(&root.path().join("missing")), None, "a missing path");
        assert_eq!(main_checkout(root.path()), None, "not a repository");
    }
}

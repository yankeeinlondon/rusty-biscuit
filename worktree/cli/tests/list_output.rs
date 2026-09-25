//! `wt list` output through the real binary, with the user cache (comparison
//! cache, fork-origin records, PR store) isolated in a temporary home.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

fn init_repo(path: &Path) {
    run_git(path, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.email", "test@example.com"),
        ("user.name", "Test User"),
        ("commit.gpgsign", "false"),
        ("gc.auto", "0"),
        ("core.fsmonitor", "false"),
        ("core.commitGraph", "false"),
    ] {
        run_git(path, &["config", key, value]);
    }
}

fn commit(path: &Path, file: &str, contents: &str) {
    fs::write(path.join(file), contents).expect("write file");
    run_git(path, &["add", "."]);
    run_git(path, &["commit", "-q", "-m", file]);
}

/// A temporary home, so no store in the user's real cache is read or written.
struct Home {
    dir: tempfile::TempDir,
}

impl Home {
    fn new() -> Self {
        Self {
            dir: tempfile::tempdir().expect("temp home"),
        }
    }

    fn wt(&self, cwd: &Path, args: &[&str]) -> Output {
        Command::new(biscuit_test_harness::bin_exe!("wt"))
            .current_dir(cwd)
            .args(args)
            .env("HOME", self.dir.path())
            .env("XDG_CACHE_HOME", self.dir.path().join("cache"))
            .env_remove("TERM_PROGRAM")
            .env_remove("KITTY_WINDOW_ID")
            .env_remove("WT_SHELL_WRAPPER")
            .env("NO_COLOR", "1")
            .env_remove("FORCE_COLOR")
            .env_remove("CLICOLOR_FORCE")
            .output()
            .expect("wt should run")
    }
}

#[test]
fn list_output_is_the_redesigned_table() {
    let repo = tempfile::tempdir().expect("create temp dir");
    let main = repo.path().join("main");
    let feature = repo.path().join("feature-a");
    fs::create_dir(&main).expect("create main repo dir");
    init_repo(&main);
    commit(&main, "file.txt", "1\n");
    commit(&main, "file.txt", "2\n");
    run_git(&main, &["checkout", "-q", "-b", "feature-a"]);
    commit(&main, "a.txt", "a\n");
    run_git(&main, &["checkout", "-q", "main"]);
    run_git(&main, &["worktree", "add", "-q", feature.to_str().unwrap(), "feature-a"]);

    let output = Home::new().wt(&main, &["list"]);

    assert!(output.status.success(), "wt list should succeed");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "\n┌─────────────┬───────────┬───────────┬───────────┐\n\
│ Worktree    │ Branch    │ ->  main  │ -> parent │\n\
├─────────────┼───────────┼───────────┼───────────┤\n\
│ ○ base repo │  main     │ —         │ —         │\n\
│ ○ feature-a │ feature-a │ clean     │ —         │\n\
└─────────────┴───────────┴───────────┴───────────┘\n\
\n\
\x20Worktree   ○ clean    ● uncommitted files    ● uncommitted source files\n\
\x20Branch     ├─ merges cleanly into parent    ├─ conflicts with parent    └┄ parent deleted\n"
    );
}

/// `wt create --from` writes the fork record; `wt list` reads it, nests the
/// branch under its parent, answers `-> parent`, and prunes the record of a
/// deleted branch on the way.
#[test]
fn create_from_records_the_parent_that_list_draws() {
    let root = tempfile::tempdir().expect("temp dir");
    let main = root.path().join("repo");
    let worktrees = root.path().join("wts");
    fs::create_dir_all(&main).unwrap();
    fs::create_dir_all(&worktrees).unwrap();
    init_repo(&main);
    commit(&main, "file.txt", "base\n");
    let home = Home::new();
    let create = |args: &[&str]| {
        let mut command = Command::new(biscuit_test_harness::bin_exe!("wt"));
        command
            .current_dir(&main)
            .arg("create")
            .args(args)
            .env("HOME", home.dir.path())
            .env("XDG_CACHE_HOME", home.dir.path().join("cache"))
            .env("WT", &worktrees)
            .env_remove("WT_SHELL_WRAPPER");
        let output = command.output().expect("wt create should run");
        assert!(output.status.success(), "wt create {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    };

    create(&["feat/theme"]);
    let theme: PathBuf = worktrees.join("repo").join("feat-theme");
    commit(&theme, "file.txt", "light\n");
    create(&["feat/dark", "--from", "feat/theme"]);
    let dark = worktrees.join("repo").join("feat-dark");
    commit(&dark, "file.txt", "dark\n");
    // The parent moves on with a conflicting change.
    commit(&theme, "file.txt", "light 2\n");
    create(&["short-lived"]);
    run_git(&main, &["worktree", "remove", worktrees.join("repo").join("short-lived").to_str().unwrap()]);
    run_git(&main, &["branch", "-D", "short-lived"]);

    let output = home.wt(&main, &["list"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{stderr}");

    let row = |needle: &str| {
        stderr
            .lines()
            .find(|line| line.contains(needle))
            .unwrap_or_else(|| panic!("no row with {needle:?}:\n{stderr}"))
            .to_string()
    };
    assert!(row("feat/theme").contains("└─ feat/theme"), "{stderr}");
    let dark_row = row("feat/dark");
    assert!(dark_row.contains("   └─ feat/dark"), "nested under feat/theme:\n{stderr}");
    let cells: Vec<&str> = dark_row.split('│').map(str::trim).collect();
    assert_eq!(cells[3], "clean", "-> main: {dark_row}");
    assert_eq!(cells[4], "conflicts", "-> parent: {dark_row}");

    let store = fork_store(home.dir.path(), &main);
    let records = fs::read_to_string(&store).expect("fork store");
    assert!(records.contains("\"feat/dark\""), "{records}");
    assert!(!records.contains("short-lived"), "the deleted branch's record is pruned: {records}");
}

/// The fork-origin store `wt` wrote. The child's user cache is under the
/// temporary home, except on Windows, where the cache directory does not
/// follow `HOME`.
fn fork_store(home: &Path, main: &Path) -> PathBuf {
    fn find(dir: &Path) -> Option<PathBuf> {
        for entry in fs::read_dir(dir).ok()?.flatten() {
            let path = entry.path();
            if path.is_dir() {
                if let Some(found) = find(&path) {
                    return Some(found);
                }
            } else if path.to_string_lossy().ends_with(".fork-origins.json") {
                return Some(path);
            }
        }
        None
    }
    find(home).unwrap_or_else(|| worktree::fork_origin::fork_origin_path(main).expect("store path"))
}

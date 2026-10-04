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
    // The last column widens the table toward the legend's width, up to the
    // 80-column terminal.
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "\n┌─────────────┬───────────┬───────────┬────────────────────────────────────────┐\n\
│ Worktree    │ Branch    │ ->  main  │ -> parent                              │\n\
├─────────────┼───────────┼───────────┼────────────────────────────────────────┤\n\
│ ○ base repo │  main     │ —         │ —                                      │\n\
│ ○ feature-a │ feature-a │ clean     │ —                                      │\n\
└─────────────┴───────────┴───────────┴────────────────────────────────────────┘\n\
\n\
\x20Worktree   ○ clean    ● uncommitted files    ● uncommitted source files\n\
\x20Branch     └─ merges cleanly into parent     └─ conflicts with parent    └┄ parent deleted\n"
    );
}

/// `-w` sizes only the graph, never the counts' 100-column gate: captured
/// output renders below 100 columns, and a wide `-w` does not add counts.
#[test]
fn a_wide_width_flag_does_not_show_the_counts() {
    let repo = tempfile::tempdir().expect("create temp dir");
    let main = repo.path().join("main");
    let feature = repo.path().join("feature-a");
    fs::create_dir(&main).expect("create main repo dir");
    init_repo(&main);
    commit(&main, "file.txt", "1\n");
    run_git(&main, &["checkout", "-q", "-b", "feature-a"]);
    commit(&main, "a.txt", "a\n");
    run_git(&main, &["checkout", "-q", "main"]);
    run_git(&main, &["worktree", "add", "-q", feature.to_str().unwrap(), "feature-a"]);

    let home = Home::new();
    for args in [&["list"][..], &["list", "-w", "200"], &["-w", "100%", "list"]] {
        let output = home.wt(&main, args);
        assert!(output.status.success(), "wt {args:?} should succeed");
        let stderr = String::from_utf8_lossy(&output.stderr);
        let row = stderr.lines().find(|line| line.contains("○ feature-a")).expect("feature-a row");
        let cells: Vec<&str> = row.split('│').map(str::trim).collect();
        assert_eq!(cells[3], "clean", "wt {args:?}: {stderr}");
    }
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

/// The `lhg-before` shape (a detached linked worktree whose `.git` file is
/// gone) and a worktree whose directory is gone, beside a healthy one, through
/// the real binary: `✕`, never `○`, an accurate note for each, and nothing on
/// disk or in Git's records changed by listing.
#[test]
fn worktrees_git_cannot_read_are_crossed_and_explained_without_repair() {
    let repo = tempfile::tempdir().expect("create temp dir");
    let main = repo.path().join("main");
    fs::create_dir(&main).expect("create main repo dir");
    init_repo(&main);
    commit(&main, "file.txt", "1\n");
    let unlinked = repo.path().join("lhg-before");
    let missing = repo.path().join("feat-gone");
    let healthy = repo.path().join("feat-ok");
    run_git(&main, &["worktree", "add", "-q", "--detach", unlinked.to_str().unwrap(), "HEAD"]);
    run_git(&main, &["worktree", "add", "-q", "-b", "feat/gone", missing.to_str().unwrap()]);
    run_git(&main, &["worktree", "add", "-q", "-b", "feat/ok", healthy.to_str().unwrap()]);
    fs::write(unlinked.join("kept.txt"), "work\n").expect("write");
    fs::remove_file(unlinked.join(".git")).expect("unlink");
    fs::remove_dir_all(&missing).expect("remove directory");
    let porcelain = || {
        let output = Command::new("git").current_dir(&main).args(["worktree", "list", "--porcelain"]).output().unwrap();
        String::from_utf8(output.stdout).unwrap()
    };
    let records_before = porcelain();
    assert_eq!(records_before.matches("prunable").count(), 2, "{records_before}");
    // Git's own spelling of each path, which the notes show.
    let recorded = |name: &str| {
        records_before
            .lines()
            .filter_map(|line| line.strip_prefix("worktree "))
            .find(|path| path.ends_with(name))
            .unwrap_or_else(|| panic!("{name} in {records_before}"))
            .to_string()
    };
    let base_path = records_before.lines().next().unwrap().strip_prefix("worktree ").unwrap().to_string();

    let output = Home::new().wt(&main, &["list"]);

    assert!(output.status.success(), "wt list should succeed: {output:?}");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let cell = |name: &str| {
        let line = stderr.lines().find(|line| line.contains(&format!(" {name} "))).unwrap_or_else(|| panic!("{name}: {stderr}"));
        line.split('│').nth(1).unwrap().trim().to_string()
    };
    assert_eq!(cell("lhg-before"), "✕ lhg-before", "{stderr}");
    assert_eq!(cell("feat-gone"), "✕ feat-gone", "{stderr}");
    assert_eq!(cell("feat-ok"), "○ feat-ok", "{stderr}");
    assert!(stderr.contains("✕ git can't read this worktree"), "{stderr}");
    assert!(!stderr.contains("couldn't check"), "no readable row is unknown: {stderr}");

    // Notes wrap at the captured width, so compare their words.
    let words: String = stderr.split_whitespace().collect::<Vec<_>>().join(" ");
    // Paths may be single-quoted (a Windows short name's `~` needs it).
    let squeezed: String = stderr.split_whitespace().collect::<String>().replace('\'', "");
    assert!(
        words.contains("lhg-before: its .git file is missing; wt remove lhg-before attempts to restore the link before checking its files."),
        "{words}"
    );
    assert!(
        squeezed.contains(&format!("run git-C{base_path}worktreerepair{}.", recorded("lhg-before")).replace(' ', "")),
        "{stderr}"
    );
    assert!(
        words.contains("feat-gone: its directory is gone; wt remove feat/gone checks whether its remaining Git record can be removed safely."),
        "{words}"
    );
    let lhg = words.find("lhg-before: its").unwrap();
    let gone = words.find("feat-gone: its").unwrap();
    assert!(gone < lhg, "notes follow table row order: {stderr}");

    // Listing repaired and removed nothing.
    assert!(fs::symlink_metadata(unlinked.join(".git")).is_err(), "the link was not restored");
    assert_eq!(fs::read_to_string(unlinked.join("kept.txt")).unwrap(), "work\n");
    assert!(fs::symlink_metadata(&missing).is_err(), "nothing was recreated");
    assert_eq!(porcelain(), records_before, "Git's records are unchanged");
}

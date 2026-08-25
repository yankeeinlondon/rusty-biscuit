use std::fs;
use std::path::Path;
use std::process::Command;

fn run_git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .current_dir(repo)
        .args(args)
        .status()
        .expect("git should be installed");
    assert!(status.success(), "git {args:?} failed in {repo:?}");
}

#[test]
fn list_output_is_byte_for_byte_unchanged() {
    let repo = tempfile::tempdir().expect("create temp dir");
    let main = repo.path().join("main");
    let feature = repo.path().join("feature-a");
    fs::create_dir(&main).expect("create main repo dir");

    run_git(&main, &["init", "-b", "main"]);
    run_git(&main, &["config", "user.email", "test@example.com"]);
    run_git(&main, &["config", "user.name", "Test User"]);
    run_git(&main, &["config", "commit.gpgsign", "false"]);
    run_git(&main, &["config", "gc.auto", "0"]);
    run_git(&main, &["config", "core.fsmonitor", "false"]);
    run_git(&main, &["config", "core.commitGraph", "false"]);

    fs::write(main.join("file.txt"), "1\n").expect("write first revision");
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "commit 1"]);
    fs::write(main.join("file.txt"), "2\n").expect("write second revision");
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "commit 2"]);
    run_git(&main, &["checkout", "-b", "feature-a"]);
    fs::write(main.join("a.txt"), "a\n").expect("write feature revision");
    run_git(&main, &["add", "."]);
    run_git(&main, &["commit", "-m", "feature a"]);
    run_git(&main, &["checkout", "main"]);
    run_git(
        &main,
        &["worktree", "add", feature.to_str().unwrap(), "feature-a"],
    );

    let output = Command::new(biscuit_test_harness::bin_exe!("wt"))
        .current_dir(&main)
        .arg("list")
        .env_remove("TERM_PROGRAM")
        .env_remove("KITTY_WINDOW_ID")
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .output()
        .expect("wt list should run");

    assert!(output.status.success(), "wt list should succeed");
    assert_eq!(String::from_utf8_lossy(&output.stdout), "");
    assert_eq!(
        String::from_utf8_lossy(&output.stderr),
        "\n┌──────────┬───────────────┬───────────┬───────┬─────────┐\n\
│ Worktree │ Worktree Name │ Branch    │ Merge │ Commits │\n\
├──────────┼───────────────┼───────────┼───────┼─────────┤\n\
│  Clean   │ main::(main)  │ main      │       │         │\n\
│  Clean   │ feature-a     │ feature-a │ clean │      +1 │\n\
└──────────┴───────────────┴───────────┴───────┴─────────┘\n"
    );
}

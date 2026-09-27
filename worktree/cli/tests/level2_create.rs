//! Level 2 coverage for the `wt create` copy report in a narrow terminal.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

use assert_cmd::cargo::cargo_bin;
use biscuit_test_harness::TerminalHarness;
use biscuit_test_harness::tmux::TmuxHarness;
use serial_test::serial;
use test_toolkit::{Backend, Level, require_level};

fn git(repo: &Path, args: &[&str]) {
    let output = Command::new("git").current_dir(repo).args(args)
        .env("GIT_TERMINAL_PROMPT", "0").output().unwrap();
    assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
}

fn quote(path: &Path) -> String {
    format!("'{}'", path.display().to_string().replace('\'', r"'\''"))
}

fn wait_for_file(path: &Path) -> String {
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        if let Ok(contents) = fs::read_to_string(path)
            && contents.ends_with('\n') {
            return contents;
        }
        assert!(Instant::now() < deadline, "timed out waiting for {}", path.display());
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[test]
#[serial(level2_terminal)]
fn level2_create_report_wraps_copied_names_and_warning_in_tmux() {
    require_level!(Level::L2, TmuxHarness::available(), Backend::Tmux);

    let root = tempfile::tempdir().unwrap();
    let repo = root.path().join("repo");
    let base = root.path().join("wts");
    fs::create_dir(&repo).unwrap();
    fs::create_dir(&base).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for (key, value) in [
        ("user.name", "Test User"),
        ("user.email", "test@example.com"),
        ("commit.gpgsign", "false"),
        ("gc.auto", "0"),
    ] { git(&repo, &["config", key, value]); }
    fs::write(repo.join("README.md"), "initial\n").unwrap();
    git(&repo, &["add", "README.md"]);
    git(&repo, &["commit", "-q", "-m", "initial"]);

    git(&repo, &["checkout", "-q", "-b", "feature"]);
    fs::write(repo.join("destination-owned.env"), "tracked\n").unwrap();
    git(&repo, &["add", "destination-owned.env"]);
    git(&repo, &["commit", "-q", "-m", "track destination file"]);
    git(&repo, &["checkout", "-q", "main"]);

    fs::write(repo.join(".gitignore"), "*.env\n").unwrap();
    fs::write(repo.join(".worktreeinclude"), "*.env\n").unwrap();
    let names = [
        "north-region-credentials.env",
        "south-region-credentials.env",
        "west-region-credentials.env",
    ];
    for name in names { fs::write(repo.join(name), name).unwrap(); }
    fs::write(repo.join("destination-owned.env"), "ignored source\n").unwrap();

    let stdout = root.path().join("protocol.stdout");
    let marker = root.path().join("exit-code");
    let script = root.path().join("create.sh");
    fs::write(&script, format!(
        "#!/bin/sh\ncd {} || exit 90\nWT={} XDG_CACHE_HOME={} WT_SHELL_WRAPPER=1 GIT_TERMINAL_PROMPT=0 {} create feature > {}\nprintf '%s\\n' \"$?\" > {}\n",
        quote(&repo), quote(&base), quote(&root.path().join("cache")), quote(&cargo_bin("wt")),
        quote(&stdout), quote(&marker),
    )).unwrap();

    let mut harness = TmuxHarness::new();
    harness.spawn_shell().expect("spawn tmux shell");
    harness.resize(48, 40).expect("resize narrow pane");
    assert_eq!(harness.pane_cols().unwrap(), 48);
    harness.send_text(format!("clear; sh {}\n", quote(&script)).as_bytes()).expect("run create scene");
    assert_eq!(wait_for_file(&marker), "0\n");
    let _ = biscuit_test_harness::wait_for_prompt(&mut harness);
    let frame = harness.capture().expect("capture create report");
    let lines: Vec<_> = frame.plain.lines().collect();
    let copied = lines.iter().position(|line| line.contains("Copied from base:"))
        .unwrap_or_else(|| panic!("missing copied report:\n{}", frame.plain));
    let warning = lines.iter().position(|line| line.contains("Skipped destination-owned.env:"))
        .unwrap_or_else(|| panic!("missing partial-copy warning:\n{}", frame.plain));
    assert!(warning > copied + 1, "copied names did not wrap:\n{}", frame.plain);
    let copied_text = lines[copied..warning].join("").replace(' ', "");
    for name in names {
        assert!(copied_text.contains(name),
            "missing {name} in wrapped copy report:\n{}", frame.plain);
    }
    let warning_text = lines[warning..].join(" ").split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(warning_text.contains("tracked in the destination"),
        "warning did not remain visible:\n{}", frame.plain);
    assert_eq!(lines[warning + 1].trim(), "destination", "warning did not wrap:\n{}", frame.plain);
    for line in &lines[copied..=warning + 1] {
        assert!(line.chars().count() <= 48, "report exceeds pane width: {line:?}");
    }

    let destination: PathBuf = base.join("repo").join("feature");
    assert_eq!(fs::read_to_string(stdout).unwrap(), format!("cd:{}\n", destination.join("").display()));
    for name in names { assert_eq!(fs::read(destination.join(name)).unwrap(), name.as_bytes()); }
    assert_eq!(fs::read(destination.join("destination-owned.env")).unwrap(), b"tracked\n");
}

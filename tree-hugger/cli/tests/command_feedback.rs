//! What `hug` prints on stderr when a command fails, and when a lint policy
//! flag names a rule the registry does not know.

use std::path::Path;
use std::process::{Command, Output};

use biscuit_test_harness::bin_exe;
use tempfile::TempDir;

/// An empty scratch repository: `hug` finds its root from the `.git`
/// directory alone, so no `git` process is needed.
fn scratch(files: &[(&str, &str)]) -> TempDir {
    let dir = TempDir::new().expect("scratch repository");
    std::fs::create_dir(dir.path().join(".git")).expect("create .git");
    for (name, contents) in files {
        std::fs::write(dir.path().join(name), contents).expect("write fixture");
    }
    dir
}

fn hug(dir: &Path, args: &[&str]) -> Output {
    let home = dir.join(".git");
    Command::new(bin_exe!("hug"))
        .args(args)
        .current_dir(dir)
        .env("HOME", &home)
        .env("USERPROFILE", &home)
        .env("XDG_CONFIG_HOME", &home)
        .env("NO_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .env("RAYON_NUM_THREADS", "2")
        .output()
        .expect("run hug")
}

fn stderr_of(output: &Output) -> String {
    String::from_utf8(output.stderr.clone()).expect("utf-8 stderr")
}

#[test]
fn a_failing_command_prints_its_display_message_and_exits_1() {
    let dir = scratch(&[]);
    let scratch_name = dir.path().file_name().unwrap().to_str().unwrap().to_string();
    let no_sources = "No supported source files found in `";
    let cases: [(&[&str], &str); 5] = [
        (&["symbols", "--no-cache", "missing/**"], no_sources),
        (&["refs", "--no-cache", "foo", "missing/**"], no_sources),
        (&["lint", "--no-cache", "missing/**"], no_sources),
        (&["imports", "--no-cache", "missing/**"], no_sources),
        (
            &["god-files", "nope-dir"],
            "Failed to read `nope-dir`: god-files scan directory does not exist",
        ),
    ];

    for (args, message) in cases {
        let output = hug(dir.path(), args);
        let stderr = stderr_of(&output);
        assert_eq!(output.status.code(), Some(1), "hug {args:?}: {stderr}");
        assert!(stderr.starts_with("error: "), "hug {args:?}: {stderr}");
        assert!(stderr.contains(message), "hug {args:?}: {stderr}");
        if message == no_sources {
            assert!(stderr.contains(&scratch_name), "hug {args:?} names the scan root: {stderr}");
        }
        // The old `Debug` dump: `Error: NoSourceFiles { path: … }`.
        assert!(!stderr.contains('{') && !stderr.contains("Error:"), "hug {args:?}: {stderr}");
        assert_eq!(stderr.lines().count(), 1, "one line, no source chain: {stderr}");
    }
}

#[cfg(unix)] // Windows file names cannot hold control characters.
#[test]
fn a_control_character_in_a_failing_path_is_escaped() {
    let dir = scratch(&[]);
    let output = hug(dir.path(), &["god-files", "bad\u{1b}[31mdir"]);
    let stderr = stderr_of(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(stderr.starts_with("error: "), "{stderr}");
    assert!(stderr.contains("bad\\u{1b}[31mdir"), "{stderr}");
    assert!(!stderr.contains('\u{1b}'), "raw control character in: {stderr:?}");
}

const CLEAN: (&str, &str) = ("probe.rs", "pub fn ok() {}\n");

/// Runs `hug lint` on the clean probe with `policy` flags, returning the exit
/// code and every unknown-rule warning line.
fn lint_warnings(dir: &Path, policy: &[&str]) -> (Option<i32>, Vec<String>) {
    let mut args = vec!["lint", "--no-cache", "--plain"];
    args.extend_from_slice(policy);
    args.push("probe.rs");
    let output = hug(dir, &args);
    let warnings = stderr_of(&output)
        .lines()
        .filter(|line| line.starts_with("warning: unknown rule"))
        .map(str::to_string)
        .collect();
    (output.status.code(), warnings)
}

#[test]
fn an_unknown_rule_name_warns_once_and_keeps_the_exit_status() {
    let dir = scratch(&[CLEAN]);

    // A rule the registry no longer has, and a typo of `unwrap-call`.
    assert_eq!(
        lint_warnings(dir.path(), &["--warn", "unused-variable"]),
        (Some(0), vec!["warning: unknown rule 'unused-variable'".to_string()])
    );
    assert_eq!(
        lint_warnings(dir.path(), &["--deny", "unwrap-cal"]),
        (Some(0), vec!["warning: unknown rule 'unwrap-cal'".to_string()])
    );
    assert_eq!(
        lint_warnings(dir.path(), &["--allow", "no-such-rule"]),
        (Some(0), vec!["warning: unknown rule 'no-such-rule'".to_string()])
    );
}

#[test]
fn a_repeated_unknown_rule_name_warns_once_per_run() {
    let dir = scratch(&[CLEAN]);
    let (status, warnings) = lint_warnings(
        dir.path(),
        &[
            "--deny", "unwrap-cal", "--deny", "unwrap-cal", "--warn", "unwrap-cal", "--allow",
            "unwrap-cal", "--warn", "unused-variable",
        ],
    );

    assert_eq!(status, Some(0));
    assert_eq!(
        warnings,
        [
            "warning: unknown rule 'unwrap-cal'",
            "warning: unknown rule 'unused-variable'",
        ]
    );
}

#[test]
fn known_rules_aliases_and_special_selectors_do_not_warn() {
    let dir = scratch(&[CLEAN]);
    for policy in [
        &["--deny", "unwrap-call"][..],
        // `unreachable-code` is an alias of `dead-code`.
        &["--warn", "unreachable-code"],
        &["--deny", "all"],
        &["--allow", "ALL"],
        &["--warn", "category:correctness"],
    ] {
        assert_eq!(lint_warnings(dir.path(), policy), (Some(0), Vec::new()), "{policy:?}");
    }
}

#[test]
fn an_unknown_rule_name_with_a_control_character_is_escaped() {
    let dir = scratch(&[CLEAN]);
    let (status, warnings) = lint_warnings(dir.path(), &["--warn", "bad\u{1b}[31mrule"]);

    assert_eq!(status, Some(0));
    assert_eq!(warnings, ["warning: unknown rule 'bad\\u{1b}[31mrule'"]);
}

#[test]
fn lint_findings_still_decide_the_status_beside_an_unknown_rule() {
    let dir = scratch(&[("probe.rs", "fn main() {\n    Some(1).unwrap();\n}\n")]);
    let output = hug(
        dir.path(),
        &["lint", "--no-cache", "--plain", "--deny", "unwrap-call", "--deny", "unwrap-cal", "probe.rs"],
    );
    let stderr = stderr_of(&output);

    assert_eq!(output.status.code(), Some(1), "{stderr}");
    assert!(String::from_utf8_lossy(&output.stdout).contains("[unwrap-call]"));
    assert_eq!(stderr.matches("warning: unknown rule 'unwrap-cal'").count(), 1, "{stderr}");
}

//! The `--changed` parser surface: optional value, equals form, the token
//! after a bare flag staying positional, argument errors, command
//! availability, and `--json-full` compatibility.

use crate::{Repo, files_and_names};

/// Asserts `hug args` is an argument error (exit 2) whose stderr names
/// `needle`, with nothing on stdout.
fn assert_argument_error(output: std::process::Output, args: &[&str], needle: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(
        output.status.code(),
        Some(2),
        "hug {args:?} is an argument error: {stderr}"
    );
    assert!(stderr.contains(needle), "hug {args:?}: `{needle}` in\n{stderr}");
    assert!(output.stdout.is_empty(), "hug {args:?} prints nothing on stdout");
}

#[test]
fn the_bare_flag_and_the_equals_form_both_run() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("staged.rs", "fn staged() {}\n");
    repo.stage("staged.rs");

    // Bare flag: uncommitted changes only, so the committed clean.rs stays
    // out and the staged file is reported.
    let report = repo.json(&["--json", "symbols", "--changed"]);
    assert_eq!(
        files_and_names(&report),
        vec![("staged.rs".to_string(), vec!["staged".to_string()])],
    );

    // Equals form with the branch's own tip: the committed comparison is
    // empty (base equals HEAD), so the same uncommitted set is reported.
    let equals = repo.json(&["--json", "symbols", "--changed=main"]);
    assert_eq!(files_and_names(&equals), files_and_names(&report));
}

#[test]
fn a_token_after_the_bare_flag_stays_a_positional_filter() {
    let repo = Repo::committed(&[("lib.rs", "fn committed_only() {}\n")]);
    repo.write("keep.rs", "fn matches_filter() {}\nfn keep_other() {}\n");

    // `matches` is not a reference: if clap had consumed it as the flag's
    // value the run would exit 1 with an unresolved reference. Instead it
    // is a smart-case name filter that narrows the changed file's records.
    let report = repo.json(&["--json", "symbols", "--changed", "matches"]);
    assert_eq!(
        files_and_names(&report),
        vec![("keep.rs".to_string(), vec!["matches_filter".to_string()])],
    );

    // With no matching name the same run is a successful empty report, not
    // a reference failure.
    let empty = repo.json(&["--json", "symbols", "--changed", "nomatch"]);
    assert_eq!(empty["files"].as_array().map(Vec::len), Some(0));
    assert_eq!(empty["errors"].as_array().map(Vec::len), Some(0));
}

#[test]
fn an_empty_value_is_an_argument_error() {
    let repo = Repo::unborn();
    assert_argument_error(
        repo.hug(&["symbols", "--changed="]),
        &["symbols", "--changed="],
        "needs a reference after the equals sign",
    );
}

#[test]
fn a_repeated_flag_is_an_argument_error() {
    let repo = Repo::unborn();
    assert_argument_error(
        repo.hug(&["symbols", "--changed", "--changed"]),
        &["symbols", "--changed", "--changed"],
        "cannot be used multiple times",
    );
}

#[test]
fn the_flag_conflicts_with_prelude() {
    let repo = Repo::unborn();
    assert_argument_error(
        repo.hug(&["symbols", "--changed", "--prelude"]),
        &["symbols", "--changed", "--prelude"],
        "cannot be used with '--prelude'",
    );
}

#[test]
fn only_the_declaration_commands_accept_the_flag() {
    let repo = Repo::committed(&[("lib.rs", "fn one() {}\n")]);
    for args in [
        vec!["imports", "--changed"],
        vec!["lint", "--changed"],
        vec!["god-files", "--changed"],
        vec!["completions", "bash", "--changed"],
    ] {
        assert_argument_error(repo.hug(&args), &args, "unexpected argument '--changed'");
    }
}

#[test]
fn all_four_declaration_commands_accept_the_flag() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("staged.rs", "fn staged() {}\nstruct Staged;\n");
    repo.stage("staged.rs");
    for (command, expected) in [("functions", "1\n"), ("types", "1\n"), ("symbols", "2\n"), ("classes", "1\n")] {
        let args = [command, "--changed", "--count"];
        let output = repo.hug(&args);
        assert!(
            output.status.success(),
            "hug {args:?}:\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8(output.stdout).unwrap();
        assert_eq!(stdout, expected, "hug {command} --changed reports only staged declarations");
    }
}

#[test]
fn json_full_combines_with_the_flag() {
    let repo = Repo::committed(&[("clean.rs", "fn clean() {}\n")]);
    repo.write("staged.rs", "fn staged() {}\n");
    repo.stage("staged.rs");

    let report = repo.json(&["--json-full", "symbols", "--changed"]);
    let indexes = report["indexes"].as_array().expect("indexes");
    assert_eq!(indexes.len(), 1, "one complete index for the changed file");
    assert_eq!(report["errors"].as_array().map(Vec::len), Some(0));
    assert_eq!(
        indexes[0]["file"].as_str().expect("file"),
        "staged.rs",
        "the unchanged committed file is not selected"
    );
}

#[test]
fn declaration_help_explains_changed_selection() {
    let repo = Repo::unborn();
    for command in ["functions", "types", "symbols", "classes"] {
        let help = repo.stdout(&[command, "--help"]);
        let words = help.split_whitespace().collect::<Vec<_>>().join(" ");
        for phrase in ["--changed[=<REF>]", "--changed=main", "current on-disk", "directly to `HEAD`", "Cannot be combined with --prelude"] {
            assert!(words.contains(phrase), "hug {command} --help lacks {phrase}: {help}");
        }
        let short = repo.stdout(&[command, "-h"]);
        assert!(short.contains("--changed[=<REF>]"), "hug {command} -h: {short}");
    }
}

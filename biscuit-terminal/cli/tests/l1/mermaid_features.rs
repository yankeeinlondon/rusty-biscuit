//! `--feature` on `bt`'s Mermaid commands, through the real binary.
//!
//! Every run is piped and `--plain`, so images are unavailable and code
//! blocks are plain fences: a text drawing opens with ```` ```txt ```` and
//! the source with ```` ```mermaid ````. Highlighting is tested in-process
//! (`commands::mermaid::tests`).

use std::process::{Command, Output};

const FLOWCHART: &str = "A --> B";
const VALID_NAMES: [&str; 3] = [
    "no-mermaid-fallbacks",
    "no-mermaid-partial-fallbacks",
    "no-mermaid-partial-warning",
];

/// The nine commands that render through Mermaid.
const MERMAID_COMMANDS: [&str; 9] = [
    "flowchart",
    "state-diagram",
    "erd",
    "pie-chart",
    "quadrant",
    "timeline",
    "bar-chart",
    "line-chart",
    "git-graph",
];

/// `bt` with the host's graphics, color, and fallback settings removed.
fn bt() -> Command {
    let mut command = Command::new(biscuit_test_harness::bin_exe!("bt"));
    for name in [
        "BISCUIT_GRAPHICS",
        "BISCUIT_GRAPHICS_TRANSPORT",
        "FORCE_COLOR",
        "CLICOLOR_FORCE",
        "MERMAID_FALLBACK",
        "COMPLETE",
    ] {
        command.env_remove(name);
    }
    command
}

/// `bt --plain flowchart <features…> "A --> B"`, with `MERMAID_FALLBACK`
/// set to `variable` when given.
fn flowchart(features: &[&str], variable: Option<&str>) -> Output {
    let mut command = bt();
    command.args(["--plain", "flowchart"]).args(features).arg(FLOWCHART);
    if let Some(value) = variable {
        command.env("MERMAID_FALLBACK", value);
    }
    command.output().expect("run bt flowchart")
}

fn stdout(output: &Output) -> String {
    assert!(output.status.success(), "bt failed: {output:?}");
    String::from_utf8_lossy(&output.stdout).into_owned()
}

fn is_text(out: &str) -> bool {
    out.starts_with("```txt\n")
}

fn is_source(out: &str) -> bool {
    out.starts_with("```mermaid\n")
}

#[test]
fn without_a_flag_or_the_variable_the_diagram_is_drawn_as_text() {
    let out = stdout(&flowchart(&[], None));

    assert!(is_text(&out), "{out}");
    assert!(out.contains("mermaid-partial-warning"), "note missing: {out}");
}

#[test]
fn without_a_flag_mermaid_fallback_code_block_shows_the_source() {
    let out = stdout(&flowchart(&[], Some("Code-Block")));

    assert!(is_source(&out), "{out}");
}

#[test]
fn a_flag_shows_the_source_with_the_variable_unset() {
    let out = stdout(&flowchart(&["--feature", "no-mermaid-fallbacks"], None));

    assert!(is_source(&out), "{out}");
}

#[test]
fn no_partial_fallbacks_shows_the_source() {
    let out = stdout(&flowchart(&["--feature", "no-mermaid-partial-fallbacks"], None));

    assert!(is_source(&out), "{out}");
}

#[test]
fn no_partial_warning_hides_the_note() {
    let out = stdout(&flowchart(&["--feature", "no-mermaid-partial-warning"], None));

    assert!(is_text(&out), "{out}");
    assert!(!out.contains("mermaid-partial-warning"), "note shown: {out}");
}

#[test]
fn comma_separated_features_equal_repeated_flags() {
    let combined = stdout(&flowchart(
        &["--feature", "no-mermaid-partial-warning,no-mermaid-fallbacks"],
        None,
    ));
    let repeated = stdout(&flowchart(
        &[
            "--feature",
            "no-mermaid-partial-warning",
            "--feature",
            "no-mermaid-fallbacks",
        ],
        None,
    ));

    assert!(is_source(&combined), "second name ignored: {combined}");
    assert_eq!(combined, repeated);
}

#[test]
fn feature_names_match_case_insensitively() {
    let out = stdout(&flowchart(&["--feature", "No-Mermaid-FALLBACKS"], None));

    assert!(is_source(&out), "{out}");
}

#[test]
fn an_unknown_or_empty_name_is_a_usage_error_listing_the_names() {
    for value in [
        "bogus",
        "no-mermaid-fallbacks,,no-mermaid-partial-warning",
        "",
        // `md render` only: every Mermaid fence stays code there.
        "no-mermaid-rendering",
    ] {
        let output = flowchart(&["--feature", value], None);

        let stderr = String::from_utf8_lossy(&output.stderr);
        assert_eq!(output.status.code(), Some(2), "--feature {value:?}: {output:?}");
        for name in VALID_NAMES {
            assert!(stderr.contains(name), "--feature {value:?} omits {name}: {stderr}");
        }
        assert!(output.stdout.is_empty(), "--feature {value:?} printed output");
    }
}

#[test]
fn every_mermaid_command_accepts_the_feature_flag() {
    for command in MERMAID_COMMANDS {
        let output = bt()
            .args([command, "--help"])
            .output()
            .expect("run bt --help");

        let help = stdout(&output);
        assert!(help.contains("--feature <NAME>"), "{command} lacks --feature");
        for name in VALID_NAMES {
            assert!(help.contains(name), "{command} --help omits {name}");
        }
    }
}

#[test]
fn completion_offers_the_feature_names() {
    let output = bt()
        .env("COMPLETE", "fish")
        .args(["--", "bt", "flowchart", "--feature", ""])
        .output()
        .expect("complete bt");

    let offered: Vec<String> = stdout(&output)
        .lines()
        .map(|line| line.split('\t').next().unwrap_or_default().to_string())
        .collect();
    assert_eq!(offered, VALID_NAMES, "{offered:?}");
}

#[test]
fn the_static_completion_script_lists_the_feature_names() {
    let output = bt()
        .args(["--completions", "bash"])
        .output()
        .expect("bt --completions bash");

    let script = stdout(&output);
    for name in VALID_NAMES {
        assert!(script.contains(name), "bash completion omits {name}");
    }
    assert!(!script.contains("no-mermaid-rendering"));
}

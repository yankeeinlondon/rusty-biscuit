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

/// A flowchart with one missing closing bracket: Merman cannot parse it.
const BROKEN_FLOWCHART: &str = "A[unclosed --> B";

/// `bt --plain <args…>`, which must fail with a Mermaid parse error on
/// stderr and print nothing on stdout.
fn assert_render_failure(args: &[&str]) {
    let output = bt().arg("--plain").args(args).output().expect("run bt");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "{args:?} succeeded: {output:?}");
    assert!(stderr.contains("Error"), "{args:?} reported no error: {stderr}");
    assert!(stderr.contains(BROKEN_FLOWCHART), "{args:?} omits the source: {stderr}");
    assert!(output.stdout.is_empty(), "{args:?} printed a fallback: {output:?}");
}

#[test]
fn without_images_a_source_that_does_not_parse_is_an_error() {
    assert_render_failure(&["flowchart", BROKEN_FLOWCHART]);
}

/// The policy forbids text, so no text request reports the error; the
/// source is still parsed.
#[test]
fn without_images_or_text_a_source_that_does_not_parse_is_an_error() {
    assert_render_failure(&["flowchart", "--feature", "no-mermaid-fallbacks", BROKEN_FLOWCHART]);
}

/// A math label parses, but the image route refuses it (Merman's math
/// support is not built in). With text forbidden by the variable, the
/// source shown in its place still reports that error and exits
/// unsuccessfully. In-process tests cover the flags and `--width`.
#[test]
fn without_images_or_text_a_math_label_is_an_error() {
    let output = bt()
        .args(["--plain", "flowchart", "A[\"$$x^2$$\"] --> B"])
        .env("MERMAID_FALLBACK", "code-block")
        .output()
        .expect("run bt");

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!output.status.success(), "succeeded: {output:?}");
    assert!(stderr.contains("lacks capability `math`"), "{stderr}");
    assert!(output.stdout.is_empty(), "printed a fallback: {output:?}");
}

/// `--title` is written into the frontmatter as a quoted scalar, so a title
/// that is not valid as bare YAML (`[broken`) still leaves the frontmatter
/// readable and the flowchart is drawn as text, led by that title. No bt command takes raw
/// Mermaid with its own frontmatter, so malformed frontmatter is reached
/// only in-process (biscuit-visualized's text, component, and hook tests).
#[test]
fn without_images_a_title_yaml_would_misread_is_drawn_as_text() {
    let output = bt()
        .args(["--plain", "flowchart", "--title", "[broken", "A --> B"])
        .output()
        .expect("run bt flowchart");

    let out = stdout(&output);
    let drawing = out.strip_prefix("[broken\n\n").unwrap_or_else(|| panic!("the title leads: {out}"));
    assert!(is_text(drawing), "{out}");
}

/// `bt --plain <command> --title Release <args…> <features…>`'s stdout.
fn titled(command: &str, args: &[&str], features: &[&str]) -> String {
    let output = bt()
        .args(["--plain", command, "--title", "Release"])
        .args(features)
        .args(args)
        .output()
        .expect("run bt");
    stdout(&output)
}

/// `--title` is shown exactly once on every route without images: leading
/// the text drawing (which never draws a frontmatter title), and inside the
/// source (frontmatter or a `title` statement), with no second copy.
#[test]
fn without_images_a_title_is_shown_once_on_the_text_and_source_routes() {
    let drawn: [(&str, &[&str]); 3] = [
        ("flowchart", &["A --> B"]),
        ("state-diagram", &["[*] --> Idle"]),
        ("erd", &["A ||--o{ B : has"]),
    ];
    for (command, args) in drawn {
        let out = titled(command, args, &[]);
        assert!(out.starts_with("Release\n\n```txt\n"), "{command}: {out}");
        assert_eq!(out.matches("Release").count(), 1, "{command}: {out}");

        let out = titled(command, args, &["--feature", "no-mermaid-fallbacks"]);
        assert!(is_source(&out), "{command}: {out}");
        assert_eq!(out.matches("Release").count(), 1, "{command}: {out}");
    }

    let source_only: [(&str, &[&str]); 6] = [
        ("pie-chart", &["Dogs:386"]),
        ("quadrant", &["A: [0.3, 0.7]"]),
        ("timeline", &["2002: LinkedIn"]),
        ("bar-chart", &["1", "2", "3"]),
        ("line-chart", &["1", "2", "3"]),
        ("git-graph", &["commit"]),
    ];
    for (command, args) in source_only {
        let out = titled(command, args, &[]);
        assert!(is_source(&out), "{command}: {out}");
        assert_eq!(out.matches("Release").count(), 1, "{command}: {out}");
    }
}

/// A pie chart is never drawn as text: its source is shown, successfully.
#[test]
fn without_images_a_family_not_drawn_as_text_shows_its_source() {
    let output = bt()
        .args(["--plain", "pie-chart", "Dogs:386", "Cats:85"])
        .output()
        .expect("run bt pie-chart");

    let out = stdout(&output);
    assert!(is_source(&out), "{out}");
    assert!(out.contains("Dogs"), "{out}");
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

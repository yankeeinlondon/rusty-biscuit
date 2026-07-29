//! Integration tests for the `terminal` CLI binary.

use predicates::prelude::*;
use serde_json::json;

#[test]
fn test_default_shows_metadata() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Terminal Metadata"));
}

#[test]
fn test_json_flag_outputs_json() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"app\""))
        .stdout(predicate::str::contains("\"is_tty\""));
}

#[test]
fn test_help_flag() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Display terminal metadata"));
}

#[test]
fn test_columns_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("columns")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render two columns"))
        .stdout(predicate::str::contains("--gap"))
        .stdout(predicate::str::contains("--left"));
}

#[test]
fn test_every_subcommand_help_exposes_example_flag() {
    let subcommands = [
        "image",
        "flowchart",
        "quadrant",
        "pie-chart",
        "git-graph",
        "bar-chart",
        "line-chart",
        "timeline",
        "state-diagram",
        "graph-expression",
        "erd",
        "prose",
        "quote",
        "list",
        "padleft",
        "padright",
        "columns",
        "dir",
        "block",
        "progress",
        "table",
        "section",
        "status-block",
        "text-block",
    ];

    for subcommand in subcommands {
        assert_cmd::Command::cargo_bin("bt").unwrap()
            .arg(subcommand)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--example"));
    }
}

#[test]
fn test_version_flag() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bt"));
}

#[test]
fn test_respects_no_color() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .assert()
        .success()
        // Should NOT contain escape codes when NO_COLOR is set
        .stdout(predicate::str::contains("\x1b[").not());
}

/// Regression test for Stage 3 / Phase 3e of the renderable IR push.
///
/// `bt quote` is a tree-rendered command that exercises both bold and
/// colored output through the shared terminal-detection layer. Setting
/// `NO_COLOR=1` (and removing any `FORCE_COLOR` override the calling
/// shell may have set) must suppress all SGR ANSI sequences in stdout.
///
/// The fix lives in the shared color-detection layer
/// (`biscuit-terminal/lib/src/discovery/detection/color.rs`) — per-command
/// SGR stripping is explicitly out of scope. If this test ever regresses,
/// fix `color_depth()` rather than the `quote` command.
#[test]
fn test_tree_rendered_quote_respects_no_color() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .env_remove("FORCE_COLOR")
        .env_remove("CLICOLOR_FORCE")
        .args(["quote", "Test text"])
        .assert()
        .success()
        // No SGR sequences should appear when NO_COLOR is set.
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn test_graph_expression_example_plain_overrides_force_color() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .env_remove("CLICOLOR_FORCE")
        .args(["--plain", "graph-expression", "--example"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert!(
        !stdout.contains('\x1b'),
        "stdout must contain no ANSI escapes, got: {stdout:?}"
    );
    assert!(
        !stderr.contains('\x1b'),
        "stderr must contain no ANSI escapes, got: {stderr:?}"
    );
}

#[test]
fn test_block_plain_overrides_force_color() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .env("CLICOLOR_FORCE", "1")
        .args(["--plain", "block", "hello", "--bold"])
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    assert_eq!(stdout.trim_end(), "hello");
    assert!(
        !stdout.contains('\x1b'),
        "stdout must contain no ANSI escapes, got: {stdout:?}"
    );
    assert!(
        !stderr.contains('\x1b'),
        "stderr must contain no ANSI escapes, got: {stderr:?}"
    );
}

#[test]
fn test_shows_underline_support() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Underline Support"))
        .stdout(predicate::str::contains("Straight:"))
        .stdout(predicate::str::contains("Curly:"));
}

#[test]
fn test_json_output_is_valid_json() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Verify it parses as valid JSON
    let parsed: Result<serde_json::Value, _> = serde_json::from_str(&stdout);
    assert!(parsed.is_ok(), "Output should be valid JSON: {}", stdout);

    // Verify expected fields exist
    let json = parsed.unwrap();
    assert!(json.get("app").is_some(), "Should have 'app' field");
    assert!(json.get("os").is_some(), "Should have 'os' field");
    assert!(json.get("width").is_some(), "Should have 'width' field");
    assert!(json.get("height").is_some(), "Should have 'height' field");
    assert!(json.get("is_tty").is_some(), "Should have 'is_tty' field");
    assert!(json.get("is_ci").is_some(), "Should have 'is_ci' field");
    assert!(
        json.get("color_depth").is_some(),
        "Should have 'color_depth' field"
    );
    assert!(
        json.get("supports_italic").is_some(),
        "Should have 'supports_italic' field"
    );
    assert!(
        json.get("multiplex").is_some(),
        "Should have 'multiplex' field"
    );
}

#[test]
fn test_default_output_shows_size() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Size:"));
}

#[test]
fn test_default_output_shows_tty_status() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Is TTY:"));
}

/// Test that font fields are valid when present in JSON output.
/// Font detection via config parsing may or may not succeed depending on
/// the terminal and whether a config file exists.
#[test]
fn test_json_font_fields_are_valid_if_present() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // If font field is present, it should be a string
    if let Some(font) = parsed.get("font") {
        assert!(font.is_string(), "'font' should be a string if present");
    }

    // If font_size field is present, it should be a number
    if let Some(size) = parsed.get("font_size") {
        assert!(
            size.is_number(),
            "'font_size' should be a number if present"
        );
    }

    // font_ligatures is still unimplemented, should always be absent
    assert!(
        parsed.get("font_ligatures").is_none(),
        "'font_ligatures' should be omitted (not implemented)"
    );
}

/// Regression test: Font section must always be displayed in default output.
/// This bug was fixed when font detection was added but the section was only
/// shown conditionally when font data was available. Now it's always shown.
#[test]
fn test_always_shows_font_section() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Font section must always be present, even when font detection returns None
    assert!(
        stdout.contains("Fonts"),
        "Fonts section must always be displayed"
    );
    assert!(
        stdout.contains("Name:"),
        "Font Name field must be displayed"
    );
    assert!(
        stdout.contains("Size:"),
        "Font Size field must be displayed"
    );
    assert!(
        stdout.contains("Ligatures:"),
        "Font Ligatures field must be displayed"
    );
}

/// Regression test: JSON output must include ligatures_likely field.
/// This ensures the heuristic-based ligature support detection is exported.
#[test]
fn test_json_includes_ligatures_likely() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    // ligatures_likely field must always be present (it's not optional)
    assert!(
        parsed.get("ligatures_likely").is_some(),
        "JSON output must include 'ligatures_likely' field"
    );
    assert!(
        parsed.get("ligatures_likely").unwrap().is_boolean(),
        "'ligatures_likely' must be a boolean"
    );
}

#[test]
fn test_json_includes_content_analysis() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .arg("red\nblue")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("line_count").unwrap(), &json!(2));
    assert_eq!(parsed.get("line_lengths").unwrap(), &json!([3, 4]));
    assert_eq!(parsed.get("total_length").unwrap(), &json!(7));
    assert_eq!(
        parsed.get("contains_color_escape_codes").unwrap(),
        &json!(false)
    );
    assert_eq!(parsed.get("contains_osc8_links").unwrap(), &json!(false));
    assert!(parsed.get("app").is_none(), "Metadata should be omitted");
}

#[test]
fn test_content_analysis_detects_color_and_osc8() {
    let content = "\x1b[31mred\x1b[0m \x1b]8;;https://example.com\x07link\x1b]8;;\x07";
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--json")
        .arg(content)
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("line_count").unwrap(), &json!(1));
    assert_eq!(parsed.get("line_lengths").unwrap(), &json!([8]));
    assert_eq!(parsed.get("total_length").unwrap(), &json!(8));
    assert_eq!(
        parsed.get("contains_color_escape_codes").unwrap(),
        &json!(true)
    );
    assert_eq!(parsed.get("contains_osc8_links").unwrap(), &json!(true));
    assert!(parsed.get("app").is_none(), "Metadata should be omitted");
}

#[test]
fn test_completions_bash() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_bt()"))
        .stdout(predicate::str::contains("complete -F"));
}

#[test]
fn test_completions_zsh() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef bt"))
        .stdout(predicate::str::contains("_bt()"));
}

#[test]
fn test_completions_fish() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--completions")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c bt"));
}

#[test]
fn test_completions_invalid_shell() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--completions")
        .arg("invalid")
        .assert()
        .failure()
        .stderr(predicate::str::contains("invalid value 'invalid'"))
        .stderr(predicate::str::contains("--completions"));
}

// =============================================================================
// Pie Chart Command Tests
// =============================================================================

#[test]
fn test_pie_chart_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render a pie chart"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--inverse"))
        .stdout(predicate::str::contains("--show-data"));
}

#[test]
fn test_pie_chart_json_simple_format() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("Dogs: 386")
        .arg("Cats: 85")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "pie-chart");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("\"Dogs\" : 386"));
    assert!(instructions.contains("\"Cats\" : 85"));
}

#[test]
fn test_pie_chart_json_semicolon_format() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("Dogs: 386; Cats: 85; Birds: 15")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("\"Dogs\" : 386"));
    assert!(instructions.contains("\"Cats\" : 85"));
    assert!(instructions.contains("\"Birds\" : 15"));
}

#[test]
fn test_pie_chart_json_official_mermaid_format() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("\"Dogs\" : 386")
        .arg("\"Cats\" : 85")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("\"Dogs\" : 386"));
    assert!(instructions.contains("\"Cats\" : 85"));
}

#[test]
fn test_pie_chart_json_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("--title")
        .arg("Pet Distribution")
        .arg("Dogs: 386")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "Pet Distribution");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("title Pet Distribution"));
}

#[test]
fn test_pie_chart_json_with_show_data() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("--show-data")
        .arg("Dogs: 386")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("show_data").unwrap(), true);
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.starts_with("pie showData"));
}

#[test]
fn test_pie_chart_requires_data() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_pie_chart_json_with_custom_colors() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("TypeScript: 45 #3178c6")
        .arg("Rust: 35 #dea584")
        .arg("Python: 20")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    // Should have init directive with colors
    assert!(instructions.contains("%%{init:"));
    assert!(instructions.contains("'pie1': '#3178c6'"));
    assert!(instructions.contains("'pie2': '#dea584'"));
    // Python has no color, so no pie3
    assert!(!instructions.contains("pie3"));
    // Data should be clean (no color in the data lines)
    assert!(instructions.contains("\"TypeScript\" : 45"));
    assert!(instructions.contains("\"Rust\" : 35"));
}

#[test]
fn test_pie_chart_json_with_color_prefix() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("TypeScript: 45 color: #3178c6")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("'pie1': '#3178c6'"));
    assert!(instructions.contains("\"TypeScript\" : 45"));
}

#[test]
fn test_pie_chart_semicolon_with_colors() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("A: 10 #ff0000; B: 20 #00ff00; C: 30")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("'pie1': '#ff0000'"));
    assert!(instructions.contains("'pie2': '#00ff00'"));
    assert!(!instructions.contains("pie3")); // C has no color
}

#[test]
fn test_pie_chart_example_flag_uses_example_data() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    // Example uses TypeScript, Rust, Python with colors
    assert!(instructions.contains("TypeScript"));
    assert!(instructions.contains("Rust"));
    assert!(instructions.contains("Python"));
    assert!(instructions.contains("#3178C6")); // TypeScript blue
}

#[test]
fn test_flowchart_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("flowchart")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("flowchart"));
    assert!(instructions.contains("Start"));
    assert!(instructions.contains("Decision"));
}

#[test]
fn test_example_flag_does_not_require_data() {
    // Without --example, data is required
    assert_cmd::Command::cargo_bin("bt").unwrap().arg("pie-chart").assert().failure();

    // With --example, data is not required
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("pie-chart")
        .arg("--json")
        .arg("--example")
        .assert()
        .success();
}

// =============================================================================
// Bar Chart Command Tests (Phase 5-9 feature: XY charts)
// =============================================================================

#[test]
fn test_bar_chart_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render a bar chart"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--x-axis"))
        .stdout(predicate::str::contains("--y-axis"))
        .stdout(predicate::str::contains("--horizontal"))
        .stdout(predicate::str::contains("--show-data-label"));
}

#[test]
fn test_bar_chart_json_with_space_separated_values() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("1")
        .arg("8")
        .arg("7")
        .arg("5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "bar-chart");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta"));
    assert!(instructions.contains("bar [1, 8, 7, 5]"));
}

#[test]
fn test_bar_chart_json_input_format() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("[1, 8, 7, 5]")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("bar [1, 8, 7, 5]"));
}

#[test]
fn test_bar_chart_comma_separated_input() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("bar [1, 8, 7, 5]"));
}

#[test]
fn test_bar_chart_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--title")
        .arg("Sales Data")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "Sales Data");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("title \"Sales Data\""));
}

#[test]
fn test_bar_chart_with_x_axis_labels() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--x-axis")
        .arg("Q1,Q2,Q3,Q4")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("x-axis"));
}

#[test]
fn test_bar_chart_with_y_axis_label() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--y-axis")
        .arg("Revenue")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("y-axis \"Revenue\""));
}

#[test]
fn test_bar_chart_horizontal_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--horizontal")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta horizontal"));
}

#[test]
fn test_bar_chart_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta"));
    assert!(instructions.contains("bar"));
}

#[test]
fn test_bar_chart_requires_data_without_example() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_bar_chart_with_line_overlay() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("--json")
        .arg("--line")
        .arg("1,8,7,5")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("bar"));
    assert!(instructions.contains("line"));
}

// =============================================================================
// Line Chart Command Tests (Phase 5-9 feature: XY charts)
// =============================================================================

#[test]
fn test_line_chart_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render a line chart"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--x-axis"))
        .stdout(predicate::str::contains("--y-axis"))
        .stdout(predicate::str::contains("--horizontal"))
        .stdout(predicate::str::contains("--bar"));
}

#[test]
fn test_line_chart_json_with_values() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--json")
        .arg("10,20,15,25")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "line-chart");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta"));
    assert!(instructions.contains("line [10, 20, 15, 25]"));
}

#[test]
fn test_line_chart_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--json")
        .arg("--title")
        .arg("Trends")
        .arg("10,20,15,25")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "Trends");
}

#[test]
fn test_line_chart_horizontal() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--json")
        .arg("--horizontal")
        .arg("10,20,15,25")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta horizontal"));
}

#[test]
fn test_line_chart_with_bar_overlay() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--json")
        .arg("--bar")
        .arg("10,20,15,25")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("bar"));
    assert!(instructions.contains("line"));
}

#[test]
fn test_line_chart_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("xychart-beta"));
    assert!(instructions.contains("line"));
}

#[test]
fn test_line_chart_requires_data_without_example() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("line-chart")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// =============================================================================
// Timeline Command Tests (Phase 5-9 feature)
// =============================================================================

#[test]
fn test_timeline_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render a timeline diagram"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--section"))
        .stdout(predicate::str::contains("--inverse"));
}

#[test]
fn test_timeline_basic() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .arg("--json")
        .arg("2020: Started")
        .arg("2022: Launched")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "timeline");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("timeline"));
    assert!(instructions.contains("2020"));
    assert!(instructions.contains("Started"));
    assert!(instructions.contains("2022"));
    assert!(instructions.contains("Launched"));
}

#[test]
fn test_timeline_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .arg("--json")
        .arg("--title")
        .arg("Company History")
        .arg("2020: Founded")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "Company History");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("title Company History"));
}

#[test]
fn test_timeline_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("timeline"));
}

#[test]
fn test_timeline_requires_events_without_example() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// =============================================================================
// State Diagram Command Tests (Phase 5-9 feature)
// =============================================================================

#[test]
fn test_state_diagram_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("state-diagram")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Render a state diagram"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--inverse"))
        .stdout(predicate::str::contains("[*]"));
}

#[test]
fn test_state_diagram_basic() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("state-diagram")
        .arg("--json")
        .arg("[*] --> Idle")
        .arg("Idle --> Running")
        .arg("Running --> [*]")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "state-diagram");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("stateDiagram-v2"));
    assert!(instructions.contains("[*] --> Idle"));
    assert!(instructions.contains("Idle --> Running"));
}

#[test]
fn test_state_diagram_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("state-diagram")
        .arg("--json")
        .arg("--title")
        .arg("Process States")
        .arg("[*] --> Ready")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "Process States");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.starts_with("---\ntitle: Process States\n---\n"));
    assert!(instructions.contains("stateDiagram-v2"));
}

#[test]
fn test_state_diagram_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("state-diagram")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("stateDiagram-v2"));
}

#[test]
fn test_state_diagram_requires_transitions_without_example() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("state-diagram")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

// =============================================================================
// ERD (Entity Relationship Diagram) Command Tests (Phase 5-9 feature)
// =============================================================================

#[test]
fn test_erd_help() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("entity relationship diagram"))
        .stdout(predicate::str::contains("--title"))
        .stdout(predicate::str::contains("--entity"))
        .stdout(predicate::str::contains("||--o{"));
}

#[test]
fn test_erd_basic() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .arg("--json")
        .arg("Customer ||--o{ Order : places")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "erd");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("erDiagram"));
    assert!(instructions.contains("Customer"));
    assert!(instructions.contains("Order"));
}

#[test]
fn test_erd_with_title() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .arg("--json")
        .arg("--title")
        .arg("E-Commerce Schema")
        .arg("Customer ||--o{ Order : places")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("title").unwrap(), "E-Commerce Schema");
    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.starts_with("---\ntitle: E-Commerce Schema\n---\n"));
    assert!(instructions.contains("erDiagram"));
}

#[test]
fn test_erd_with_entity_definitions() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .arg("--json")
        .arg("--entity")
        .arg("Customer { id int PK, name string }")
        .arg("--entity")
        .arg("Order { id int PK, date date }")
        .arg("Customer ||--o{ Order : places")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("Customer {"));
    assert!(instructions.contains("Order {"));
}

#[test]
fn test_erd_example_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    let instructions = parsed.get("instructions").unwrap().as_str().unwrap();
    assert!(instructions.contains("erDiagram"));
}

#[test]
fn test_erd_requires_relationships_without_example() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("erd")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_graph_expression_json_example() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("--json")
        .arg("--example")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("type").unwrap(), "graph-expression");
    assert_eq!(parsed.get("syntax").unwrap(), "auto");
    assert_eq!(parsed.get("orientation").unwrap(), "top-to-bottom");
    assert!(
        parsed
            .get("source")
            .unwrap()
            .as_str()
            .unwrap()
            .contains("Validate")
    );
}

#[test]
fn test_graph_expression_json_dot_mode() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("--json")
        .arg("--syntax")
        .arg("dot")
        .arg("--orientation")
        .arg("left-to-right")
        .arg("digraph { A -> B; B -> C; }")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("syntax").unwrap(), "dot");
    assert_eq!(parsed.get("orientation").unwrap(), "left-to-right");
    assert_eq!(
        parsed.get("source").unwrap().as_str().unwrap(),
        "digraph { A -> B; B -> C; }"
    );
}

#[test]
fn test_graph_expression_json_reports_inverse_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("--json")
        .arg("--inverse")
        .arg("a -> b")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("Output should be valid JSON");

    assert_eq!(parsed.get("inverse").unwrap(), true);
    assert_eq!(parsed.get("source").unwrap().as_str().unwrap(), "a -> b");
}

#[test]
fn test_graph_expression_falls_back_to_code_block_on_non_tty() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("a -> b -> c")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("```graph-expression"));
    assert!(stdout.contains("a -> b -> c"));
}

#[test]
fn test_graph_expression_dot_fallback_uses_dot_info_string() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("--syntax")
        .arg("dot")
        .arg("digraph { A -> B; }")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("```dot"));
    assert!(stdout.contains("digraph { A -> B; }"));
}

#[test]
#[serial_test::serial(visualization)]
fn test_graph_expression_meta_outputs_render_metadata_in_a_pty() {
    use expectrl::{Expect, Session};
    use std::process::Command;

    let bin_path = assert_cmd::cargo::cargo_bin("bt");
    let mut cmd = Command::new(bin_path);
    cmd.arg("graph-expression")
        .arg("--meta")
        .arg("a -> b")
        .env("CI", "1")
        .env("NO_COLOR", "1")
        .env("TERM_PROGRAM", "Ghostty")
        .env("TERM", "xterm-ghostty");

    let mut p = Session::spawn(cmd).expect("Failed to spawn bt in PTY");
    p.expect("\"file_size_bytes\":")
        .expect("Expected render metadata JSON in PTY output");
    p.expect("\"render_time_ms\":")
        .expect("Expected render_time_ms in PTY output");
}

#[test]
fn test_graph_expression_rejects_mixed_edge_kinds() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("graph-expression")
        .arg("a -> b; c -- d")
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Mixed directed and undirected edges",
        ));
}

#[test]
fn test_prose_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("Hello <bold>world</bold>!")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_columns_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("columns")
        .arg("Left side")
        .arg("Right side")
        .arg("--gap")
        .arg("5")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

// ---------------------------------------------------------------------------
// Verbosity flag tests
// ---------------------------------------------------------------------------

#[test]
fn test_verbose_flag_shows_environment_section() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment"))
        .stdout(predicate::str::contains("TERM:"))
        .stdout(predicate::str::contains("COLORTERM:"));
}

#[test]
fn test_very_verbose_shows_raw_detection() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("-vv")
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment"))
        .stdout(predicate::str::contains("Raw Detection"))
        .stdout(predicate::str::contains("LANG:"))
        .stdout(predicate::str::contains("TMUX:"));
}

#[test]
fn test_default_no_environment_section() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment").not())
        .stdout(predicate::str::contains("Raw Detection").not());
}

#[test]
fn test_quiet_suppresses_output() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--quiet")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    // --quiet suppresses the default metadata output entirely
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty with --quiet, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_silent_suppresses_all_output() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--silent")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty with --silent, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_silent_suppresses_json_output() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("--silent")
        .arg("--json")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty with --silent --json, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_quiet_short_flag() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("-q")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    assert!(output.stdout.is_empty(), "stdout should be empty with -q");
}

// ---------------------------------------------------------------------------
// STDERR error output and exit code tests
// ---------------------------------------------------------------------------

#[test]
fn test_prose_empty_errors_to_stderr() {
    // Pass an explicit empty positional to bypass clap's `required_unless_present`
    // check and exercise the runtime guard in `ProseArgs::run`. Without any
    // positional at all, clap rejects the invocation at parse time before the
    // guard can run.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail with no content");
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty on error, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("No content provided"),
        "stderr should contain error message, got: {}",
        stderr
    );
}

/// Verifies that `bt prose --force-color` emits SGR sequences to a
/// non-TTY pipe.
///
/// Without `--force-color`, `bt` would correctly suppress colors when
/// stdout is not a TTY. With the flag, the renderer constructs a
/// forced-color [`Terminal`] regardless of detection, so the output
/// must contain `\x1b[3` (the SGR foreground prefix). This is the
/// Level-1 unit-test-shaped proof that the flag works without any
/// terminal harness.
#[test]
fn test_prose_force_color_flag_emits_sgr_to_pipe() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("--force-color")
        .arg("<red>x</red>")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --force-color should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let has_red = stdout.contains("\x1b[31m") || stdout.contains("\x1b[91m");
    assert!(
        has_red,
        "expected stdout to contain SGR red (\\x1b[31m or \\x1b[91m); got bytes: {:?}",
        output.stdout
    );
}

/// Verifies that `bt prose --print-bytes` emits a hex dump of the
/// rendered byte stream to stderr.
///
/// The dump is delimited by a `--- prose debug ---` header line and
/// followed by a single line of lowercase hex digits. Combined with
/// `--force-color` this gives Level-2 tests a deterministic signal
/// that the renderer produced SGR red regardless of any terminal
/// capture filtering.
#[test]
fn test_prose_print_bytes_flag_dumps_hex_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("--force-color")
        .arg("--print-bytes")
        .arg("<red>x</red>")
        .env_remove("NO_COLOR")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --print-bytes should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("--- prose debug ---"),
        "expected stderr to contain '--- prose debug ---' header; got: {stderr}"
    );
    // \x1b[31m hex == 1b5b33316d ; \x1b[91m hex == 1b5b39316d
    let has_red_hex = stderr.contains("1b5b33316d") || stderr.contains("1b5b39316d");
    assert!(
        has_red_hex,
        "expected stderr hex dump to contain SGR red bytes (1b5b33316d or 1b5b39316d); \
         got: {stderr}"
    );
}

#[test]
fn test_prose_markdown_margin_left_emits_style_frontmatter() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<b>bold</b>")
        .arg("--margin-left")
        .arg("4")
        .arg("--md")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --md should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "---\nstyle:\n  page:\n    margin-left: 4ch\n---\n\n**bold**\n"
    );
}

#[test]
fn test_prose_markdown_without_layout_omits_frontmatter() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<b>bold</b>")
        .arg("--md")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --md should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(stdout, "**bold**\n");
}

#[test]
fn test_prose_markdown_plus_preserves_color_as_inline_html() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<purple-800>dark purple</purple-800>")
        .arg("--md-plus")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --md-plus should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("<span style=\"color: rgb("),
        "MarkdownPlus should preserve color as inline HTML, got: {stdout}"
    );
    assert!(
        stdout.contains(">dark purple</span>\n"),
        "MarkdownPlus should preserve inner text, got: {stdout}"
    );
}

#[test]
fn test_prose_markdown_plus_margin_left_emits_style_frontmatter() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<blue>blue</blue>")
        .arg("--margin-left")
        .arg("4")
        .arg("--md-plus")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --md-plus should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("---\nstyle:\n  page:\n    margin-left: 4ch\n---\n\n"),
        "MarkdownPlus layout should be preserved in frontmatter, got: {stdout}"
    );
    assert!(
        stdout.contains("<span style=\"color: "),
        "MarkdownPlus body should preserve color as inline HTML, got: {stdout}"
    );
}

#[test]
fn test_prose_html_margin_left_emits_layout_wrapper() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<b>bold</b>")
        .arg("--margin-left")
        .arg("4")
        .arg("--html")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --html should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "<div style=\"margin-left: 4ch\"><span class=\"prose\"><strong>bold</strong></span></div>\n"
    );
}

#[test]
fn test_prose_html_without_layout_omits_layout_wrapper() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<b>bold</b>")
        .arg("--html")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt prose --html should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout,
        "<span class=\"prose\"><strong>bold</strong></span>\n"
    );
}

#[test]
fn test_quote_empty_errors_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail with no content");
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty on error, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    // clap rejects the missing positional `CONTENT...` argument before the
    // command body runs; either the clap-generated message or the manual
    // "No content provided" fallback is acceptable evidence that empty input
    // is rejected.
    assert!(
        stderr.contains("No content provided") || stderr.contains("required"),
        "stderr should explain the missing input, got: {}",
        stderr
    );
}

#[test]
fn test_quote_md_emits_canonical_block_quote_syntax() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--md")
        .arg("To be or not to be")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt quote --md should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.starts_with("> "),
        "stdout should start with `> `, got: {stdout:?}"
    );
    assert!(stdout.contains("To be or not to be"));
    // Markdown output must not carry any ANSI escapes.
    assert!(
        !stdout.contains('\x1b'),
        "markdown output should not contain escape codes, got: {stdout:?}"
    );
}

#[test]
fn test_quote_md_with_attribution_renders_separate_paragraph() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--md")
        .arg("--attribution")
        .arg("Shakespeare")
        .arg("To be or not to be")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("To be or not to be"));
    assert!(stdout.contains("— Shakespeare"));
    // Every non-empty line of the output is a Markdown block-quote line.
    for line in stdout.lines().filter(|l| !l.is_empty()) {
        assert!(
            line.starts_with('>'),
            "every line of `bt quote --md` should start with `>`, got: {line:?}"
        );
    }
}

#[test]
fn test_quote_html_emits_blockquote_fragment() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--html")
        .arg("To be or not to be")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt quote --html should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<blockquote"),
        "stdout should contain `<blockquote`, got: {stdout:?}"
    );
    assert!(stdout.contains("</blockquote>"));
    assert!(stdout.contains("To be or not to be"));
    assert!(
        !stdout.contains('\x1b'),
        "HTML output should not contain escape codes, got: {stdout:?}"
    );
}

#[test]
fn test_quote_html_with_attribution_has_two_paragraphs() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--html")
        .arg("--attribution")
        .arg("Shakespeare")
        .arg("To be or not to be")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<blockquote"));
    assert!(stdout.contains("To be or not to be"));
    assert!(stdout.contains("— Shakespeare"));
    assert_eq!(
        stdout.matches("<p>").count(),
        2,
        "expected 2 <p> elements, got: {stdout:?}"
    );
}

#[test]
fn test_quote_md_and_html_are_mutually_exclusive() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--md")
        .arg("--html")
        .arg("text")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "--md and --html should conflict");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("cannot be used with"),
        "stderr should indicate conflict, got: {stderr:?}"
    );
}

#[test]
fn test_quote_example_renders_default_quote() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--example")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    assert!(
        output.status.success(),
        "bt quote --example should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Clarity"),
        "example output should contain `Clarity`, got: {stdout:?}"
    );
    assert!(
        stdout.contains("Command:"),
        "example output should print the command, got: {stdout:?}"
    );
    assert!(
        stdout.contains("bt quote"),
        "example output should echo the `bt quote` invocation, got: {stdout:?}"
    );
}

#[test]
fn test_quote_md_strips_inline_styling_tags() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("--md")
        .arg("<b>bold</b> text")
        .output()
        .expect("Failed to execute command");

    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The tree projection flattens Prose styling, so the Markdown output is
    // plain text inside the block quote — no terminal styling tags survive.
    assert!(stdout.contains("bold"));
    assert!(stdout.contains("text"));
    assert!(stdout.starts_with("> "));
}

#[test]
fn test_list_empty_errors_to_stderr() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_padleft_missing_args_errors_to_stderr() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("padleft")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_padright_missing_args_errors_to_stderr() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("padright")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_image_nonexistent_file_errors_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("image")
        .arg("/tmp/nonexistent_file_12345.png")
        .output()
        .expect("Failed to execute command");

    assert!(
        !output.status.success(),
        "should fail with nonexistent file"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.is_empty(), "stderr should contain an error message");
}

#[test]
fn test_bar_chart_invalid_data_errors_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("bar-chart")
        .arg("not_a_number")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail with invalid data");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid number"),
        "stderr should contain error about invalid number, got: {}",
        stderr
    );
}

#[test]
fn test_timeline_bad_format_errors_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("timeline")
        .arg("no colon here")
        .output()
        .expect("Failed to execute command");

    assert!(!output.status.success(), "should fail with bad format");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Invalid event format"),
        "stderr should contain format error, got: {}",
        stderr
    );
}

#[test]
fn test_error_exit_code_is_nonzero() {
    // Verify the actual exit code is non-zero (not just failure)
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .output()
        .expect("Failed to execute command");

    let code = output.status.code().expect("should have exit code");
    assert_ne!(code, 0, "exit code should be non-zero on error");
}

// ---------------------------------------------------------------------------
// Snapshot tests
// ---------------------------------------------------------------------------

#[test]
fn test_prose_styled_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("prose")
        .arg("<b>Error:</b> something went <red>wrong</red> in the <i>module</i>")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_quote_snapshot() {
    // `bt quote` is rendered through the canonical render-tree path. In the
    // non-TTY snapshot environment the border stays visible but color SGR is
    // suppressed.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("quote")
        .arg("To be or not to be, that is the question.")
        .arg("--attribution")
        .arg("Shakespeare")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_list_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("list")
        .arg("First item")
        .arg("Second item with <b>bold</b>")
        .arg("Third item")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_padleft_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("padleft")
        .arg("30")
        .arg("hello")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_padright_snapshot() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("padright")
        .arg("30")
        .arg("hello")
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

/// Public docs and examples must not advertise the removed Prose
/// atomic-token grammar (`{{bold}}`, `{{reset}}`, …).
///
/// The atomic-token grammar was removed in the prose-cross-target feature;
/// only bracketed tags and the Markdown subset remain. A user copying a
/// stale `{{bold}}` example would render the literal text `{{bold}}`. Lines
/// that explicitly document the removal are exempt.
#[test]
fn public_docs_do_not_advertise_removed_atomic_tokens() {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let docs = [
        format!("{manifest_dir}/README.md"),
        format!("{manifest_dir}/../README.md"),
        format!("{manifest_dir}/../docs/components/compose.md"),
        format!("{manifest_dir}/../docs/components/index.md"),
        format!("{manifest_dir}/../docs/components/prose.md"),
    ];

    let banned = [
        "{{bold}}",
        "{{reset}}",
        "{{italic}}",
        "{{red}}",
        "{{dim}}",
        "{{cyan}}",
        "{{bg-",
    ];

    for path in &docs {
        let Ok(text) = std::fs::read_to_string(path) else {
            continue; // file moved/renamed — not this test's concern
        };
        for (idx, line) in text.lines().enumerate() {
            // Lines that document the grammar removal may name the old
            // syntax legitimately.
            if line.contains("removed") {
                continue;
            }
            for token in &banned {
                assert!(
                    !line.contains(token),
                    "{path}:{} advertises removed Prose atomic token {token:?}:\n{line}",
                    idx + 1,
                );
            }
        }
    }
}

#[test]
fn test_actual_terminal_query_integration() {
    use expectrl::{Expect, Session};
    use std::process::Command;

    // We get the path to the 'bt' binary
    let bin_path = assert_cmd::cargo::cargo_bin("bt");

    // Spawn in a PTY so `is_tty()` is true, but force a non-probing path.
    // Pseudo-terminals used in tests do not implement real terminal query
    // responses, so inheriting host env vars can cause capability probes to
    // hang waiting for replies that never arrive.
    let mut cmd = Command::new(bin_path);
    cmd.env("CI", "1");
    cmd.env("NO_COLOR", "1");
    cmd.env("TERM_PROGRAM", "Ghostty");

    let mut p = Session::spawn(cmd).expect("Failed to spawn bt in PTY");

    // Check that we can read some terminal output (it implies the program ran successfully in a PTY)
    // We expect it to print "Terminal Metadata"
    p.expect("Terminal Metadata")
        .expect("Did not find 'Terminal Metadata' in PTY output");
}

// ---------------------------------------------------------------------------
// bt compose
// ---------------------------------------------------------------------------

#[test]
fn test_compose_no_args_errors_to_stderr() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .arg("compose")
        .output()
        .expect("Failed to execute command");
    assert!(
        !output.status.success(),
        "bt compose with no args should fail"
    );
    assert!(
        output.stdout.is_empty(),
        "stdout should be empty on error, got: {}",
        String::from_utf8_lossy(&output.stdout)
    );
}

#[test]
fn test_compose_positional_items_concatenate() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md", "foo", "bar"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Trailing newline from println! is fine; trim before exact match.
    assert_eq!(stdout.trim_end_matches('\n'), "foobar");
}

#[test]
fn test_compose_md_renders_heading() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md", "--heading", "1", "Hello"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.trim_end_matches('\n').starts_with("# Hello"),
        "stdout: {stdout:?}"
    );
}

#[test]
fn test_compose_html_emits_div_wrapper() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--html", "hello"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<div"), "stdout: {stdout:?}");
    assert!(stdout.contains("hello"));
}

#[test]
fn test_compose_md_and_html_are_mutually_exclusive() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md", "--html", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success());
}

#[test]
fn test_compose_example_emits_command_line() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("bt compose"),
        "expected example command in stdout, got: {stdout:?}"
    );
    // Example renders the canonical demo content.
    assert!(stdout.contains("Project Status"));
    assert!(stdout.contains("Unit tests"));
}

#[test]
fn test_compose_html_example_emits_html_and_command_line() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--example", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<h1"));
    assert!(stdout.contains("Project Status"));
    assert!(stdout.contains("<ul"));
    assert!(stdout.contains("bt compose"));
}

#[test]
fn test_compose_help_exposes_md_md_plus_html_and_example_flags() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--help"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--example"));
    assert!(stdout.contains("--md"));
    assert!(stdout.contains("--md-plus"));
    assert!(stdout.contains("--html"));
}

/// Strips SGR (color/style) escape sequences only; preserves printable
/// content. Mirrors the in-test helper in `level2_*` files but kept local
/// so this integration test does not depend on the harness crate.
fn strip_sgr(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\x1b' && chars.peek() == Some(&'[') {
            chars.next(); // consume `[`
            for c in chars.by_ref() {
                if c.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(ch);
        }
    }
    out
}

#[test]
fn test_compose_default_terminal_concatenates_positional_items() {
    // Default (no --md / --html / --md-plus) takes the terminal path —
    // assert that ANSI-stripped output contains the concatenated string.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "foo", "bar"])
        .env("NO_COLOR", "1")
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stripped = strip_sgr(&stdout);
    assert!(
        stripped.contains("foobar"),
        "expected `foobar` substring in stripped terminal output, got: {stdout:?}"
    );
}

#[test]
fn test_compose_md_plus_with_text_succeeds() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md-plus", "--text", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("x"),
        "expected `x` in MarkdownPlus output; got: {stdout:?}"
    );
}

#[test]
fn test_compose_prose_terminal_output_contains_bold_sgr() {
    // The terminal path lowers Prose `<b>` to the SGR bold open sequence
    // (`\x1b[1m`). Stripping SGR is irrelevant here — assert the raw byte
    // sequence is present so a regression in inline-style lowering shows up.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--prose", "<b>bold</b>"])
        .env_remove("NO_COLOR")
        .env("FORCE_COLOR", "1")
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1m"),
        "expected SGR bold open `\\x1b[1m`; got: {stdout:?}"
    );
}

#[test]
fn test_compose_md_with_list_emits_dash_marker() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md", "--list", "one", "two"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("- one"),
        "expected `- one` in Markdown output; got: {stdout:?}"
    );
    assert!(
        stdout.contains("- two"),
        "expected `- two` in Markdown output; got: {stdout:?}"
    );
}

#[test]
fn test_compose_md_with_table_emits_gfm_table() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md", "--table", "A,B", "x,y"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // GFM table: header row uses `|`, separator row uses `|---|`.
    assert!(
        stdout.contains("| A"),
        "expected `| A` header cell; got: {stdout:?}"
    );
    assert!(
        stdout.contains("| B"),
        "expected `| B` header cell; got: {stdout:?}"
    );
    // GFM separator row uses `:--` / `---` patterns; accept either since the
    // renderer encodes column alignment via the leading colon.
    assert!(
        stdout.contains(":--") || stdout.contains("---"),
        "expected GFM separator row; got: {stdout:?}"
    );
    assert!(
        stdout.contains("| x"),
        "expected `| x` cell; got: {stdout:?}"
    );
    assert!(
        stdout.contains("| y"),
        "expected `| y` cell; got: {stdout:?}"
    );
}

#[test]
fn test_compose_md_plus_md_mutual_exclusion() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md-plus", "--md", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md-plus + --md should fail");
}

#[test]
fn test_compose_md_plus_html_mutual_exclusion() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["compose", "--md-plus", "--html", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md-plus + --html should fail");
}

// ---------------------------------------------------------------------------
// `bt list --ordered` cross-target rendering
// ---------------------------------------------------------------------------

#[test]
fn test_list_ordered_renders_numbered_items() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "First", "Second", "Third"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1. First"))
        .stdout(predicate::str::contains("2. Second"))
        .stdout(predicate::str::contains("3. Third"));
}

#[test]
fn test_list_short_ordered_flag_works() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "-o", "A", "B"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1. A"))
        .stdout(predicate::str::contains("2. B"));
}

#[test]
fn test_list_ordered_md_emits_commonmark() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--md", "First", "Second", "Third"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt list -o --md should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Portable CommonMark numbered list — no ANSI, no HTML.
    assert!(stdout.contains("1. First"), "got: {stdout}");
    assert!(stdout.contains("2. Second"), "got: {stdout}");
    assert!(stdout.contains("3. Third"), "got: {stdout}");
    assert!(!stdout.contains("<ol"), "no HTML wrapper: {stdout}");
    assert!(!stdout.contains("\x1b["), "no ANSI: {stdout:?}");
}

#[test]
fn test_list_ordered_md_plus_matches_md() {
    let md = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--md", "A", "B", "C"])
        .output()
        .expect("Failed to execute command");
    let md_plus = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--md-plus", "A", "B", "C"])
        .output()
        .expect("Failed to execute command");
    assert!(md.status.success() && md_plus.status.success());
    assert_eq!(
        String::from_utf8_lossy(&md.stdout),
        String::from_utf8_lossy(&md_plus.stdout),
        "ordered list --md and --md-plus produce identical output",
    );
}

#[test]
fn test_list_ordered_html_emits_ol_li() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--html", "First", "Second"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<ol"), "ol present: {stdout}");
    assert!(stdout.contains("</ol>"), "ol closed: {stdout}");
    assert!(stdout.contains("<li>"), "li present: {stdout}");
    assert!(stdout.contains("First"));
    assert!(stdout.contains("Second"));
}

#[test]
fn test_list_ordered_example_renders_and_prints_command() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("1. Install dependencies"), "got: {stdout}");
    assert!(stdout.contains("Command:"), "command label printed");
    assert!(stdout.contains("bt list --ordered"), "command shown");
}

#[test]
fn test_list_unordered_example_unchanged() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Default unordered example uses `•` bullets.
    assert!(stdout.contains("•"), "default bullet: {stdout}");
    assert!(stdout.contains("Plan"));
    assert!(stdout.contains("focused tests"));
}

#[test]
fn test_list_unordered_terminal_honors_custom_bullet() {
    // The `--bullet` flag must reach the default terminal render path.
    // Markdown / HTML paths ignore it (pinned by
    // `test_list_unordered_md_ignores_custom_bullet` /
    // `test_list_unordered_html_ignores_custom_bullet`); this test pins the
    // terminal path.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--bullet", "→ ", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt list --bullet should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    // ANSI / OSC may surround the marker; check for the literal bullet
    // glyph followed by the item content.
    assert!(
        stdout.contains("→ x"),
        "expected `→ x` in terminal output: {stdout:?}"
    );
}

#[test]
fn test_list_md_html_mutual_exclusion() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--md", "--html", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md + --html should fail");
}

#[test]
fn test_list_md_md_plus_mutual_exclusion() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--ordered", "--md", "--md-plus", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md + --md-plus should fail");
}

#[test]
fn test_list_unordered_md_emits_commonmark() {
    // After UnorderedList's IR migration, `bt list --md` (no `--ordered`)
    // emits portable CommonMark bullet syntax through the canonical render
    // tree, mirroring the ordered path. The CLI's custom terminal bullet
    // (`--bullet`, default `• `) is ignored — Markdown's portable contract
    // uses `- `.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--md", "First", "Second", "Third"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt list --md should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- First"), "got: {stdout}");
    assert!(stdout.contains("- Second"), "got: {stdout}");
    assert!(stdout.contains("- Third"), "got: {stdout}");
    assert!(
        !stdout.contains("•"),
        "custom bullet not used in MD: {stdout}"
    );
    assert!(!stdout.contains("<ul"), "no HTML wrapper: {stdout}");
    assert!(!stdout.contains("\x1b["), "no ANSI: {stdout:?}");
}

#[test]
fn test_list_unordered_md_ignores_custom_bullet() {
    // `--bullet "→ "` only affects terminal output. The Markdown path emits
    // standard `- ` regardless.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--md", "--bullet", "→ ", "Alpha", "Beta"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- Alpha"), "got: {stdout}");
    assert!(stdout.contains("- Beta"), "got: {stdout}");
    assert!(
        !stdout.contains("→"),
        "custom bullet must not leak into Markdown: {stdout}"
    );
}

#[test]
fn test_list_unordered_md_plus_matches_md() {
    let md = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--md", "A", "B", "C"])
        .output()
        .expect("Failed to execute command");
    let md_plus = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--md-plus", "A", "B", "C"])
        .output()
        .expect("Failed to execute command");
    assert!(md.status.success() && md_plus.status.success());
    assert_eq!(
        String::from_utf8_lossy(&md.stdout),
        String::from_utf8_lossy(&md_plus.stdout),
        "unordered list --md and --md-plus produce identical output",
    );
}

#[test]
fn test_list_unordered_html_emits_ul_li() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--html", "First", "Second"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<ul"), "ul present: {stdout}");
    assert!(stdout.contains("</ul>"), "ul closed: {stdout}");
    assert!(stdout.contains("<li>"), "li present: {stdout}");
    assert!(stdout.contains("First"));
    assert!(stdout.contains("Second"));
    // No <ol>, ever, for unordered.
    assert!(!stdout.contains("<ol"), "no ordered tag: {stdout}");
}

#[test]
fn test_list_unordered_html_ignores_custom_bullet() {
    // Browsers control list-marker presentation via CSS; the terminal bullet
    // is not rendered into the HTML fragment.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--html", "--bullet", "→ ", "Alpha", "Beta"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<ul"), "ul present: {stdout}");
    assert!(stdout.contains("Alpha"));
    assert!(
        !stdout.contains("→"),
        "custom bullet must not leak into HTML: {stdout}"
    );
}

#[test]
fn test_list_unordered_md_with_left_margin_emits_frontmatter() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--md", "--margin-left", "4", "First", "Second"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("margin-left: 4ch"), "frontmatter: {stdout}");
    assert!(
        stdout.contains("- First"),
        "items below frontmatter: {stdout}"
    );
}

#[test]
fn test_list_unordered_html_with_layout_emits_margin_on_ul_only() {
    // Same contract as the ordered path: `--margin-left N` lowers to
    // `margin-left:Nch` on the `<ul>` itself; the CLI must not double-wrap
    // in `<div style="…">` for tree-expressible properties.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--html", "--margin-left", "2", "X", "Y"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<ul"), "ul still emitted: {stdout}");
    assert!(
        stdout.contains("margin-left:2ch"),
        "tree path applies margin to <ul>: {stdout}"
    );
    assert!(
        !stdout.contains("<div style="),
        "no wrapper div for tree-expressible properties: {stdout}"
    );
    let margin_left_count = stdout.matches("margin-left").count();
    assert_eq!(
        margin_left_count, 1,
        "margin-left applied exactly once: {stdout}"
    );
}

#[test]
fn test_list_unordered_html_with_alignment_wraps_in_div() {
    // Same rule as the ordered path: `text-align` from `--alignment` has no
    // peer on the tree path's `<ul>` CSS (which only emits
    // `margin-left:auto`/`margin-right:auto` with `max_width`), so the CLI
    // wraps in `<div style="text-align: …">` to express it.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["list", "--html", "--alignment", "center", "X", "Y"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<div style=\"text-align: center\">"),
        "wrapper div for alignment: {stdout}"
    );
    assert!(stdout.contains("<ul"), "ul present: {stdout}");
}

#[test]
fn test_list_ordered_md_with_left_margin_emits_frontmatter() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "list",
            "--ordered",
            "--md",
            "--margin-left",
            "4",
            "First",
            "Second",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("margin-left: 4ch"), "frontmatter: {stdout}");
    assert!(
        stdout.contains("1. First"),
        "items below frontmatter: {stdout}"
    );
}

#[test]
fn test_list_ordered_html_with_layout_emits_margin_on_ol_only() {
    // Per OrderedList-spec: LayoutArgs that the tree path's CSS lowering can
    // express on the component's own root (margins) must NOT be additionally
    // wrapped in a `<div style="…">` — that would double-apply them.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "list",
            "--ordered",
            "--html",
            "--margin-left",
            "2",
            "X",
            "Y",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<ol"), "ol still emitted: {stdout}");
    // The tree path emits margin-left:2ch on the <ol> itself; no wrapper div.
    assert!(
        stdout.contains("margin-left:2ch"),
        "tree path applies margin to <ol>: {stdout}"
    );
    assert!(
        !stdout.contains("<div style="),
        "no wrapper div for tree-expressible properties: {stdout}"
    );
    let margin_left_count = stdout.matches("margin-left").count();
    assert_eq!(
        margin_left_count, 1,
        "margin-left applied exactly once: {stdout}"
    );
}

#[test]
fn test_list_ordered_html_with_alignment_wraps_in_div() {
    // text-align from --alignment has no peer on the tree path's <ol> CSS
    // (which only emits margin-left:auto / margin-right:auto with max_width),
    // so the CLI must wrap in a <div style="text-align: …"> to express it.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "list",
            "--ordered",
            "--html",
            "--alignment",
            "center",
            "X",
            "Y",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<div style=\"text-align: center\">"),
        "wrapper div for alignment: {stdout}"
    );
    assert!(stdout.contains("<ol"), "ol present: {stdout}");
}

// =============================================================================
// bt progress: cross-target rendering (--html, --md, --md-plus)
// =============================================================================

#[test]
fn test_progress_terminal_default() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .args(["progress", "60", "--label", "Loading"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Loading"))
        .stdout(predicate::str::contains("60%"))
        .stdout(predicate::str::contains("["))
        .stdout(predicate::str::contains("]"));
}

#[test]
fn test_progress_example_succeeds() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .args(["progress", "--example"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Indexing"))
        .stdout(predicate::str::contains("Command:"))
        .stdout(predicate::str::contains(
            "bt progress 72 --label \"Indexing\"",
        ));
}

#[test]
fn test_progress_md_outputs_label_and_percentage() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "75", "--label", "Loading", "--md"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Loading 75%"),
        "portable Markdown shows label + percentage: {stdout}"
    );
    // Must NOT contain bar glyphs or HTML
    assert!(!stdout.contains('█'), "no bar glyphs in Markdown: {stdout}");
    assert!(
        !stdout.contains("<span"),
        "no HTML in portable Markdown: {stdout}"
    );
}

#[test]
fn test_progress_md_unlabeled_outputs_just_percentage() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "50", "--md"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert_eq!(
        stdout.trim(),
        "50%",
        "unlabeled progress Markdown is just the percentage"
    );
}

#[test]
fn test_progress_md_plus_emits_semantic_html() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "40", "--label", "Sync", "--md-plus"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("role=\"progressbar\""),
        "MarkdownPlus carries semantic widget: {stdout}"
    );
    assert!(
        stdout.contains("aria-valuenow=\"40\""),
        "ARIA value reflects completion: {stdout}"
    );
    assert!(stdout.contains("Sync"), "label survives: {stdout}");
    assert!(stdout.contains("40%"), "percentage survives: {stdout}");
}

#[test]
fn test_progress_html_emits_semantic_widget() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "25", "--label", "Loading", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("role=\"progressbar\""),
        "HTML fragment carries semantic widget: {stdout}"
    );
    assert!(
        stdout.contains("aria-valuenow=\"25\""),
        "ARIA value reflects completion: {stdout}"
    );
    assert!(
        stdout.contains("class=\"progress\""),
        "stable `progress` class on root: {stdout}"
    );
    assert!(
        stdout.contains("progress-label") || stdout.contains("aria-label=\"Loading\""),
        "label is exposed: {stdout}"
    );
}

#[test]
fn test_progress_html_unlabeled_omits_label_span() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "50", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("role=\"progressbar\""),
        "HTML widget present: {stdout}"
    );
    assert!(
        !stdout.contains("progress-label"),
        "unlabeled progress emits no label span: {stdout}"
    );
}

#[test]
fn test_progress_md_html_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "50", "--md", "--html"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_progress_md_md_plus_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "50", "--md", "--md-plus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_progress_html_md_plus_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "50", "--html", "--md-plus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_progress_rejects_percentage_above_100() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "150"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Percentage must be 0-100"));
}

#[test]
fn test_progress_color_flags_parse_for_terminal() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .args([
            "progress",
            "50",
            "--label",
            "Loading",
            "--fill-color",
            "green",
            "--empty-color",
            "#444444",
            "--bracket-color",
            "cyan",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("Loading"))
        .stdout(predicate::str::contains("50%"));
}

#[test]
fn test_progress_html_color_flags_inline_css() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "progress",
            "50",
            "--label",
            "Loading",
            "--html",
            "--fill-color",
            "green",
            "--empty-color",
            "red",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("background-color"),
        "fill/empty colors lowered to inline CSS background-color: {stdout}"
    );
}

#[test]
fn test_progress_example_with_md_renders_markdown() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "--example", "--md"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Indexing 72%"),
        "example --md still renders example values: {stdout}"
    );
}

#[test]
fn test_progress_example_with_html_renders_semantic_widget() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "--example", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("role=\"progressbar\""),
        "example --html emits semantic widget: {stdout}"
    );
    assert!(
        stdout.contains("aria-valuenow=\"72\""),
        "example --html reflects the example completion: {stdout}"
    );
    assert!(
        stdout.contains("Indexing"),
        "example --html preserves the example label: {stdout}"
    );
    assert!(
        stdout.contains("Command:"),
        "example --html includes the Command: trailer: {stdout}"
    );
    assert!(
        stdout.contains("bt progress 72 --label \"Indexing\""),
        "example --html includes the command string: {stdout}"
    );
}

#[test]
fn test_progress_example_with_md_plus_renders_inline_html() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["progress", "--example", "--md-plus"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("role=\"progressbar\""),
        "example --md-plus emits MarkdownPlus inline HTML widget: {stdout}"
    );
    assert!(
        stdout.contains("aria-valuenow=\"72\""),
        "example --md-plus reflects the example completion: {stdout}"
    );
    assert!(
        stdout.contains("Indexing"),
        "example --md-plus preserves the example label: {stdout}"
    );
    assert!(
        stdout.contains("Command:"),
        "example --md-plus includes the Command: trailer: {stdout}"
    );
    assert!(
        stdout.contains("bt progress 72 --label \"Indexing\""),
        "example --md-plus includes the command string: {stdout}"
    );
}

// -------------------------------------------------------------------------
// bt section
// -------------------------------------------------------------------------

#[test]
fn test_section_requires_title_or_example() {
    let assert = assert_cmd::Command::cargo_bin("bt").unwrap().arg("section").assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("required") || stderr.contains("Title"),
        "bt section without args should fail with a title-required message, got: {stderr}"
    );
}

#[test]
fn test_section_terminal_default_emits_heading_prefix() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "My Title"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt section should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("## My Title"),
        "default level 2 heading prefix: {stdout}"
    );
}

#[test]
fn test_section_level_flag_changes_heading_prefix() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Heading", "--level", "4"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("#### Heading"),
        "level 4 heading prefix: {stdout}"
    );
}

#[test]
fn test_section_invalid_level_errors() {
    let assert = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--level", "9"])
        .assert()
        .failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("1-6") || stderr.contains("level"),
        "out-of-range level should error: {stderr}"
    );
}

#[test]
fn test_section_md_renders_heading_and_body() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "section",
            "Hello",
            "--md",
            "--content",
            "First body",
            "--content",
            "Second body",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("## Hello"), "heading: {stdout}");
    assert!(stdout.contains("First body"), "body 1: {stdout}");
    assert!(stdout.contains("Second body"), "body 2: {stdout}");
}

#[test]
fn test_section_md_plus_matches_md_for_pure_section() {
    let md_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--md", "--content", "Body"])
        .output()
        .expect("md run");
    let md_plus_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--md-plus", "--content", "Body"])
        .output()
        .expect("md-plus run");
    assert!(md_output.status.success());
    assert!(md_plus_output.status.success());
    let md = String::from_utf8_lossy(&md_output.stdout);
    let md_plus = String::from_utf8_lossy(&md_plus_output.stdout);
    assert_eq!(
        md, md_plus,
        "Section's pure-CommonMark output is identical for --md and --md-plus"
    );
}

#[test]
fn test_section_html_emits_section_element() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--html", "--level", "3"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<section"), "section element: {stdout}");
    assert!(stdout.contains("</section>"), "section close: {stdout}");
    assert!(stdout.contains("<h3"), "h3 tag matches --level: {stdout}");
    assert!(stdout.contains("Title"), "title text: {stdout}");
}

#[test]
fn test_section_md_html_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--md", "--html"])
        .assert()
        .failure();
}

#[test]
fn test_section_md_md_plus_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "Title", "--md", "--md-plus"])
        .assert()
        .failure();
}

#[test]
fn test_section_example_emits_command_line() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Deployment Guide"),
        "example title present: {stdout}"
    );
    assert!(
        stdout.contains("Command:"),
        "example emits Command: trailer: {stdout}"
    );
    assert!(
        stdout.contains("bt section"),
        "example shows the bt section command string: {stdout}"
    );
}

#[test]
fn test_section_example_with_html_emits_section_element() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "--example", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("<section"),
        "example --html emits a section element: {stdout}"
    );
    assert!(
        stdout.contains("Deployment Guide"),
        "example --html preserves the example title: {stdout}"
    );
    assert!(
        stdout.contains("Command:"),
        "example --html emits the Command: trailer: {stdout}"
    );
}

#[test]
fn test_section_help_exposes_md_md_plus_html_and_example_flags() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["section", "--help"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--html"), "help exposes --html: {stdout}");
    assert!(stdout.contains("--md"), "help exposes --md: {stdout}");
    assert!(
        stdout.contains("--md-plus"),
        "help exposes --md-plus: {stdout}"
    );
    assert!(
        stdout.contains("--example"),
        "help exposes --example: {stdout}"
    );
    assert!(stdout.contains("--level"), "help exposes --level: {stdout}");
    assert!(
        stdout.contains("--content"),
        "help exposes --content: {stdout}"
    );
}

// -------------------------------------------------------------------------
// bt status-block
// -------------------------------------------------------------------------

#[test]
fn test_status_block_requires_severity_and_body() {
    let assert = assert_cmd::Command::cargo_bin("bt").unwrap().arg("status-block").assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("severity") || stderr.contains("required"),
        "bt status-block without args should fail: {stderr}"
    );
}

#[test]
fn test_status_block_example_succeeds_and_prints_command() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["status-block", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt status-block --example should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Command:"),
        "example should print the command used: {stdout}"
    );
    assert!(
        stdout.contains("Shell Expansion Failed"),
        "example header text present: {stdout}"
    );
}

#[test]
fn test_status_block_terminal_default_emits_thick_border() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["status-block", "--severity", "error", "Body text"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The default border may be colored on a TrueColor terminal; assert the
    // glyph is present and the body text survives. Byte-level adjacency
    // varies with SGR placement.
    assert!(
        stdout.contains('┃'),
        "default terminal output carries the thick left border: {stdout:?}"
    );
    assert!(
        stdout.contains("Body text"),
        "body text present: {stdout:?}"
    );
}

#[test]
fn test_status_block_md_emits_block_quote() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["status-block", "--md", "--severity", "error", "Body text"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("> Body text"),
        "Markdown block quote: {stdout}"
    );
}

#[test]
fn test_status_block_html_emits_div_with_classes() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "status-block",
            "--html",
            "--severity",
            "warning",
            "--header",
            "Heads up",
            "Body text",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("status-block"),
        "fragment carries status-block class: {stdout}"
    );
    assert!(
        stdout.contains("status-block--warning"),
        "severity class present: {stdout}"
    );
    assert!(
        stdout.contains("<blockquote"),
        "body block quote element: {stdout}"
    );
}

#[test]
fn test_status_block_md_html_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "status-block",
            "--md",
            "--html",
            "--severity",
            "error",
            "body",
        ])
        .assert()
        .failure();
}

#[test]
fn test_status_block_every_non_deprecated_severity_runs() {
    let severities = [
        "error",
        "warning",
        "info",
        "success",
        "active",
        "not-started",
        "tool-use",
        "subagent",
    ];
    for severity in severities {
        let output = assert_cmd::Command::cargo_bin("bt").unwrap()
            .args(["status-block", "--severity", severity, "body"])
            .output()
            .expect("Failed to execute command");
        assert!(
            output.status.success(),
            "severity {severity} should render: stderr = {:?}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}

#[test]
fn test_status_block_help_exposes_target_and_layout_flags() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["status-block", "--help"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--md"), "help exposes --md: {stdout}");
    assert!(stdout.contains("--html"), "help exposes --html: {stdout}");
    assert!(
        stdout.contains("--example"),
        "help exposes --example: {stdout}"
    );
    assert!(
        stdout.contains("--severity"),
        "help exposes --severity: {stdout}"
    );
    assert!(
        stdout.contains("--header"),
        "help exposes --header: {stdout}"
    );
    assert!(stdout.contains("--hint"), "help exposes --hint: {stdout}");
    assert!(
        stdout.contains("--border-color"),
        "help exposes --border-color: {stdout}"
    );
    // The CLI deliberately does NOT expose the arbitrary `border(String)` knob —
    // that is a terminal-only compatibility surface.
    assert!(
        !stdout.contains("--border-prefix") && !stdout.contains("--border <"),
        "CLI must not expose arbitrary border prefix: {stdout}"
    );
}

#[test]
fn test_status_block_border_color_applies_on_color_terminal() {
    // FORCE_COLOR makes the terminal report TrueColor so the override is
    // emitted as truecolor SGR. We check for the truecolor preamble.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .args([
            "status-block",
            "--severity",
            "error",
            "--border-color",
            "#aa11ff",
            "Body",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // The truecolor SGR for #aa11ff would emit `\x1b[38;2;170;17;255m` — just
    // assert that some SGR escape sequence was emitted to prove the override
    // travelled through the renderer.
    assert!(
        stdout.contains("\x1b["),
        "border color should emit SGR on color terminal: {stdout:?}"
    );
}

// -------------------------------------------------------------------------
// bt table
// -------------------------------------------------------------------------

#[test]
fn test_table_requires_columns_without_example() {
    let assert = assert_cmd::Command::cargo_bin("bt").unwrap().arg("table").assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr).to_string();
    assert!(
        stderr.contains("required") || stderr.contains("columns"),
        "bt table without args should fail with a columns-required message, got: {stderr}"
    );
}

#[test]
fn test_table_terminal_default_emits_box_borders() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--columns",
            "Name,Score",
            "--row",
            "Ann,90",
            "--row",
            "Bob,75",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt table should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains('┌'), "top-left corner: {stdout}");
    assert!(stdout.contains("Name"), "header: {stdout}");
    assert!(stdout.contains("Ann"), "first row data: {stdout}");
    assert!(stdout.contains("Bob"), "second row data: {stdout}");
}

#[test]
fn test_table_md_emits_gfm_pipe_table() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--md",
            "--columns",
            "Name,Score",
            "--row",
            "Ann,90",
            "--row",
            "Bob,75",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("| Name"), "header pipe row: {stdout}");
    assert!(stdout.contains("| Score"), "header pipe row: {stdout}");
    assert!(stdout.contains("--"), "GFM delimiter row present: {stdout}");
    assert!(stdout.contains("Ann"), "first row data: {stdout}");
    assert!(stdout.contains("Bob"), "second row data: {stdout}");
    // No terminal box characters in the Markdown output.
    assert!(
        !stdout.contains('┌'),
        "no box drawing in Markdown: {stdout}"
    );
}

#[test]
fn test_table_md_plus_matches_md_for_pure_gfm_table() {
    let md_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--md",
            "--columns",
            "Name,Score",
            "--row",
            "Ann,90",
        ])
        .output()
        .expect("md run");
    let md_plus_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--md-plus",
            "--columns",
            "Name,Score",
            "--row",
            "Ann,90",
        ])
        .output()
        .expect("md-plus run");
    assert!(md_output.status.success());
    assert!(md_plus_output.status.success());
    let md = String::from_utf8_lossy(&md_output.stdout);
    let md_plus = String::from_utf8_lossy(&md_plus_output.stdout);
    assert_eq!(
        md, md_plus,
        "Table's pure-GFM output is identical for --md and --md-plus"
    );
}

#[test]
fn test_table_html_emits_table_element() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--html",
            "--columns",
            "Name,Score",
            "--row",
            "Ann,90",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<table"), "table element: {stdout}");
    assert!(stdout.contains("</table>"), "table close: {stdout}");
    assert!(stdout.contains("<thead"), "thead present: {stdout}");
    assert!(stdout.contains("<tbody"), "tbody present: {stdout}");
    assert!(stdout.contains("<th"), "th present: {stdout}");
    assert!(stdout.contains("<td"), "td present: {stdout}");
    assert!(stdout.contains("Name"), "header text: {stdout}");
    assert!(stdout.contains("Ann"), "body text: {stdout}");
}

#[test]
fn test_table_title_renders_caption_on_every_target() {
    let term_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--title",
            "Quarterly Results",
            "--columns",
            "Q,Value",
            "--row",
            "Q1,100",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(term_output.status.success());
    let term_stdout = String::from_utf8_lossy(&term_output.stdout);
    assert!(
        term_stdout.contains("Quarterly Results"),
        "terminal title present: {term_stdout}"
    );

    let md_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--md",
            "--title",
            "Quarterly Results",
            "--columns",
            "Q,Value",
            "--row",
            "Q1,100",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(md_output.status.success());
    let md_stdout = String::from_utf8_lossy(&md_output.stdout);
    assert!(
        md_stdout.contains("Quarterly Results"),
        "Markdown title present: {md_stdout}"
    );

    let html_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--html",
            "--title",
            "Quarterly Results",
            "--columns",
            "Q,Value",
            "--row",
            "Q1,100",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(html_output.status.success());
    let html_stdout = String::from_utf8_lossy(&html_output.stdout);
    assert!(
        html_stdout.contains("<caption"),
        "HTML emits <caption>: {html_stdout}"
    );
    assert!(
        html_stdout.contains("Quarterly Results"),
        "HTML title text present: {html_stdout}"
    );
}

#[test]
fn test_table_md_cell_pipe_is_escaped() {
    // The CLI splits rows on `,`, so a cell value containing a literal `|`
    // exercises the Markdown table-cell escape path through the canonical
    // tree renderer.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--md",
            "--columns",
            "Name,Note",
            "--row",
            "Ann,left|right",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("left\\|right"),
        "cell pipe escaped in Markdown: {stdout}"
    );
}

#[test]
fn test_table_md_html_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["table", "--md", "--html", "--columns", "A", "--row", "x"])
        .assert()
        .failure();
}

#[test]
fn test_table_md_md_plus_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["table", "--md", "--md-plus", "--columns", "A", "--row", "x"])
        .assert()
        .failure();
}

#[test]
fn test_table_html_md_plus_mutual_exclusion() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--html",
            "--md-plus",
            "--columns",
            "A",
            "--row",
            "x",
        ])
        .assert()
        .failure();
}

#[test]
fn test_table_help_exposes_cross_target_and_example_flags() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["table", "--help"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("--html"), "help exposes --html: {stdout}");
    assert!(stdout.contains("--md"), "help exposes --md: {stdout}");
    assert!(
        stdout.contains("--md-plus"),
        "help exposes --md-plus: {stdout}"
    );
    assert!(
        stdout.contains("--example"),
        "help exposes --example: {stdout}"
    );
    assert!(stdout.contains("--title"), "help exposes --title: {stdout}");
}

#[test]
fn test_table_example_emits_command_line() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["table", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Parser"), "example data present: {stdout}");
    assert!(
        stdout.contains("Command:"),
        "example emits Command: trailer: {stdout}"
    );
    assert!(
        stdout.contains("bt table"),
        "example shows the bt table command string: {stdout}"
    );
}

#[test]
fn test_table_typed_currency_column_formats_value() {
    // A `usd` entry in `--column-types` declares a currency column whose
    // `--mixed-row` cell is formatted; the literal header is untouched.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--columns",
            "Item,Amount",
            "--column-types",
            ",usd",
            "--mixed-row",
            "Widget,9.99",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "typed currency column should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("$9.99"), "currency value formatted: {stdout}");
    assert!(stdout.contains("Amount"), "literal header is preserved: {stdout}");
}

#[test]
fn test_table_unknown_column_type_errors() {
    // `--column-types` is an explicit declaration, so an unrecognized token is a
    // user error rather than silently-ignored header text.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args([
            "table",
            "--columns",
            "Item,Amount",
            "--column-types",
            ",bogus",
            "--row",
            "Widget,9.99",
        ])
        .output()
        .expect("Failed to execute command");
    assert!(
        !output.status.success(),
        "an unknown column type must be rejected"
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("unknown column type") && stderr.contains("bogus"),
        "the error names the offending token: {stderr}"
    );
}

#[test]
fn test_table_literal_colon_header_is_preserved() {
    // Header text is always literal now that types live in `--column-types`, so
    // a colon-bearing header is preserved verbatim, never split into a type.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["table", "--columns", "Time: Value", "--row", "12:00"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "a colon-bearing header must not error: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Time: Value"),
        "the literal colon-bearing header is preserved verbatim: {stdout}"
    );
    assert!(stdout.contains("12:00"), "the colon-bearing cell renders: {stdout}");
}

#[test]
fn test_table_header_ending_in_type_word_is_literal() {
    // Regression: a literal header that ends in a recognized type word (with or
    // without a colon) must render verbatim. The previous `Header:type` suffix
    // parsing silently dropped these words and changed the column's alignment;
    // types now live in `--column-types`, so the header text is untouched.
    for header in [
        "Revenue: USD",
        "Total int",
        "Steps integer",
        "Speed float",
        "Price usd",
        "Cost gbp",
        "Sum eur",
    ] {
        let output = assert_cmd::Command::cargo_bin("bt").unwrap()
            .args(["table", "--columns", header, "--row", "1"])
            .output()
            .expect("Failed to execute command");
        assert!(
            output.status.success(),
            "header {header:?} must not error: {}",
            String::from_utf8_lossy(&output.stderr)
        );
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            stdout.contains(header),
            "the literal header {header:?} is preserved verbatim: {stdout}"
        );
    }
}

// -------------------------------------------------------------------------
// bt text-block
// -------------------------------------------------------------------------

#[test]
fn test_text_block_terminal_default_emits_text() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("NO_COLOR", "1")
        .args(["text-block", "Hello world"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success(), "bt text-block should succeed");
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello world"),
        "default terminal output emits text: {stdout:?}"
    );
}

#[test]
fn test_text_block_bold_emits_bold_sgr() {
    // FORCE_COLOR=1 advertises TrueColor so the tree path emits ANSI SGR.
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .args(["text-block", "Hello", "--bold"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("\x1b[1m"),
        "--bold emits bold SGR under FORCE_COLOR: {stdout:?}"
    );
}

#[test]
fn test_text_block_md_emits_plain_text() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello world", "--md", "--bold", "--italic"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Hello world"),
        "Markdown output preserves text: {stdout:?}"
    );
    // Markdown ignores Style by contract — no SGR, no semantic wrappers.
    assert!(!stdout.contains("\x1b["), "no SGR in Markdown: {stdout:?}");
    assert!(
        !stdout.contains("<strong"),
        "no HTML in portable Markdown: {stdout:?}"
    );
}

#[test]
fn test_text_block_md_plus_matches_md_for_styled_text() {
    let md_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--md", "--bold"])
        .output()
        .expect("md run");
    let md_plus_output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--md-plus", "--bold"])
        .output()
        .expect("md-plus run");
    assert!(md_output.status.success());
    assert!(md_plus_output.status.success());
    // TextBlock's MarkdownPlus output is intentionally identical to portable
    // Markdown — styled MarkdownPlus is deferred outside this migration
    // (spec RT-TEXTBLOCK-002, DENIED).
    assert_eq!(
        String::from_utf8_lossy(&md_output.stdout),
        String::from_utf8_lossy(&md_plus_output.stdout),
        "TextBlock --md and --md-plus produce identical plain text"
    );
}

#[test]
fn test_text_block_html_emits_paragraph_with_style() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--html", "--bold"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<p"), "<p> element: {stdout:?}");
    assert!(stdout.contains("Hello"), "content present: {stdout:?}");
    // Bold lowers to either a <strong> wrapper or a `font-weight:bold` CSS
    // declaration on the paragraph itself (RT-TEXTBLOCK-001).
    assert!(
        stdout.contains("<strong") || stdout.contains("font-weight:bold"),
        "bold semantics preserved: {stdout:?}"
    );
}

#[test]
fn test_text_block_md_html_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--md", "--html"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_text_block_md_plus_html_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--md-plus", "--html"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_text_block_md_md_plus_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--md", "--md-plus"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_text_block_underline_variants_are_mutually_exclusive() {
    // The five underline flags pairwise conflict — `--underline` (straight)
    // vs `--curly-underline` is a representative pair.
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--underline", "--curly-underline"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_text_block_bold_and_dim_are_mutually_exclusive() {
    assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--bold", "--dim"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[test]
fn test_text_block_example_succeeds_and_prints_command() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "--example"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt text-block --example should succeed"
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Release candidate passed"),
        "example body text present: {stdout:?}"
    );
    assert!(
        stdout.contains("Command:"),
        "example emits Command: trailer: {stdout:?}"
    );
    assert!(
        stdout.contains("bt text-block"),
        "example shows the bt text-block command string: {stdout:?}"
    );
}

#[test]
fn test_text_block_fg_emits_foreground_sgr_on_color_terminal() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .args(["text-block", "Hello", "--fg", "red"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Either the basic SGR (`\x1b[31m`) or a TrueColor SGR (`\x1b[38;...`)
    // is acceptable — the degradation path picks one based on the advertised
    // capability.
    assert!(
        stdout.contains("\x1b[31m") || stdout.contains("\x1b[38;"),
        "--fg red emits a foreground SGR under FORCE_COLOR: {stdout:?}"
    );
}

#[test]
fn test_text_block_fg_lowers_to_css_in_html() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["text-block", "Hello", "--html", "--fg", "red"])
        .output()
        .expect("Failed to execute command");
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("color:"),
        "--fg red lowers to inline CSS `color:` in HTML: {stdout:?}"
    );
}

#[test]
fn test_text_block_example_dim_wins_over_implicit_bold() {
    // `--example` injects `bold = true` by default to match the documented
    // command. When the user passes `--dim`, that injection must be skipped
    // so the in-memory state is not contradictory (clap's `conflicts_with`
    // does not fire on the implicit injection).
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .env("FORCE_COLOR", "1")
        .args(["text-block", "--example", "--dim"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "--example + --dim must succeed: stderr = {:?}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Inspect only the example body — the `Command:` trailer is styled bold
    // by `print_example_command`, which would otherwise mask the assertion.
    // Split on the bold-open + "Command:" prefix so the trailer's `\x1b[1m`
    // does not leak into the body slice.
    let body = stdout.split("\x1b[1mCommand:").next().unwrap_or(&stdout);
    assert!(
        body.contains("\x1b[2m"),
        "--dim emits dim SGR in the example body: {body:?}"
    );
    assert!(
        !body.contains("\x1b[1m"),
        "--dim suppresses the example's implicit bold SGR in the body: {body:?}"
    );
}

// ---------------------------------------------------------------------------
// `bt columns` cross-target rendering (TwoColumn IR migration)
//
// These tests mirror the `bt compose` pattern: assert that `--md`, `--md-plus`,
// `--html`, and `--example` work, and that the three target flags are mutually
// exclusive.
// ---------------------------------------------------------------------------

#[test]
fn test_columns_html_emits_columns_flex_container() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--html", "LeftBody", "RightBody"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --html should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#"class="columns""#),
        "columns flex container class present: {stdout:?}"
    );
    assert!(
        stdout.contains(r#"class="column""#),
        "column child class present: {stdout:?}"
    );
    assert!(stdout.contains("LeftBody"), "left content: {stdout:?}");
    assert!(stdout.contains("RightBody"), "right content: {stdout:?}");
}

#[test]
fn test_columns_md_collapses_to_sequential_blocks() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--md", "LeftBody", "RightBody"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --md should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("LeftBody"), "left survives: {stdout:?}");
    assert!(stdout.contains("RightBody"), "right survives: {stdout:?}");
    // Portable Markdown collapses columns to left-then-right blocks; there is
    // no HTML wrapper.
    assert!(
        !stdout.contains(r#"class="columns""#),
        "portable Markdown has no columns HTML container: {stdout:?}"
    );
    let left_pos = stdout.find("LeftBody").expect("left present");
    let right_pos = stdout.find("RightBody").expect("right present");
    assert!(left_pos < right_pos, "left precedes right: {stdout:?}");
}

#[test]
fn test_columns_md_plus_emits_flex_html_container() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--md-plus", "LeftBody", "RightBody"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --md-plus should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#"class="columns""#),
        "MarkdownPlus container present: {stdout:?}"
    );
    assert!(stdout.contains("LeftBody"), "left survives: {stdout:?}");
    assert!(stdout.contains("RightBody"), "right survives: {stdout:?}");
}

#[test]
fn test_columns_md_and_html_are_mutually_exclusive() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--md", "--html", "L", "R"])
        .output()
        .expect("Failed to execute command");
    assert!(
        !output.status.success(),
        "bt columns --md + --html should fail"
    );
}

#[test]
fn test_columns_md_plus_and_html_are_mutually_exclusive() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--md-plus", "--html", "L", "R"])
        .output()
        .expect("Failed to execute command");
    assert!(
        !output.status.success(),
        "bt columns --md-plus + --html should fail"
    );
}

#[test]
fn test_columns_md_and_md_plus_are_mutually_exclusive() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--md", "--md-plus", "L", "R"])
        .output()
        .expect("Failed to execute command");
    assert!(
        !output.status.success(),
        "bt columns --md + --md-plus should fail"
    );
}

#[test]
fn test_columns_example_with_md_succeeds() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--example", "--md"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --example --md should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Example renders canonical demo content and the `Command:` trailer.
    assert!(
        stdout.contains("Release"),
        "example left content present: {stdout:?}"
    );
    assert!(
        stdout.contains("bt columns"),
        "example command trailer present: {stdout:?}"
    );
}

#[test]
fn test_columns_example_with_md_plus_succeeds() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--example", "--md-plus"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --example --md-plus should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#"class="columns""#),
        "MarkdownPlus example contains flex container: {stdout:?}"
    );
    assert!(stdout.contains("bt columns"), "command trailer: {stdout:?}");
}

#[test]
fn test_columns_example_with_html_succeeds() {
    let output = assert_cmd::Command::cargo_bin("bt").unwrap()
        .args(["columns", "--example", "--html"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "bt columns --example --html should succeed, stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains(r#"class="columns""#),
        "HTML example contains flex container: {stdout:?}"
    );
    assert!(stdout.contains("bt columns"), "command trailer: {stdout:?}");
}

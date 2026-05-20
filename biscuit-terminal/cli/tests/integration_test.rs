//! Integration tests for the `terminal` CLI binary.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use serde_json::json;

#[test]
fn test_default_shows_metadata() {
    cargo_bin_cmd!("bt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Terminal Metadata"));
}

#[test]
fn test_json_flag_outputs_json() {
    cargo_bin_cmd!("bt")
        .arg("--json")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"app\""))
        .stdout(predicate::str::contains("\"is_tty\""));
}

#[test]
fn test_help_flag() {
    cargo_bin_cmd!("bt")
        .arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Display terminal metadata"));
}

#[test]
fn test_columns_help() {
    cargo_bin_cmd!("bt")
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
    ];

    for subcommand in subcommands {
        cargo_bin_cmd!("bt")
            .arg(subcommand)
            .arg("--help")
            .assert()
            .success()
            .stdout(predicate::str::contains("--example"));
    }
}

#[test]
fn test_version_flag() {
    cargo_bin_cmd!("bt")
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("bt"));
}

#[test]
fn test_respects_no_color() {
    cargo_bin_cmd!("bt")
        .env("NO_COLOR", "1")
        .assert()
        .success()
        // Should NOT contain escape codes when NO_COLOR is set
        .stdout(predicate::str::contains("\x1b[").not());
}

#[test]
fn test_shows_underline_support() {
    cargo_bin_cmd!("bt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Underline Support"))
        .stdout(predicate::str::contains("Straight:"))
        .stdout(predicate::str::contains("Curly:"));
}

#[test]
fn test_json_output_is_valid_json() {
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Size:"));
}

#[test]
fn test_default_output_shows_tty_status() {
    cargo_bin_cmd!("bt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Is TTY:"));
}

/// Test that font fields are valid when present in JSON output.
/// Font detection via config parsing may or may not succeed depending on
/// the terminal and whether a config file exists.
#[test]
fn test_json_font_fields_are_valid_if_present() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("--completions")
        .arg("bash")
        .assert()
        .success()
        .stdout(predicate::str::contains("_bt()"))
        .stdout(predicate::str::contains("complete -F"));
}

#[test]
fn test_completions_zsh() {
    cargo_bin_cmd!("bt")
        .arg("--completions")
        .arg("zsh")
        .assert()
        .success()
        .stdout(predicate::str::contains("#compdef bt"))
        .stdout(predicate::str::contains("_bt()"));
}

#[test]
fn test_completions_fish() {
    cargo_bin_cmd!("bt")
        .arg("--completions")
        .arg("fish")
        .assert()
        .success()
        .stdout(predicate::str::contains("complete -c bt"));
}

#[test]
fn test_completions_invalid_shell() {
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("pie-chart")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_pie_chart_json_with_custom_colors() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt").arg("pie-chart").assert().failure();

    // With --example, data is not required
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("bar-chart")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_bar_chart_with_line_overlay() {
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("erd")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"));
}

#[test]
fn test_graph_expression_json_example() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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

    let bin_path = assert_cmd::cargo::cargo_bin!("bt");
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
    cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("-v")
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment"))
        .stdout(predicate::str::contains("TERM:"))
        .stdout(predicate::str::contains("COLORTERM:"));
}

#[test]
fn test_very_verbose_shows_raw_detection() {
    cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .assert()
        .success()
        .stdout(predicate::str::contains("Environment").not())
        .stdout(predicate::str::contains("Raw Detection").not());
}

#[test]
fn test_quiet_suppresses_output() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
        .arg("prose")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .arg("list")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_padleft_missing_args_errors_to_stderr() {
    cargo_bin_cmd!("bt")
        .arg("padleft")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_padright_missing_args_errors_to_stderr() {
    cargo_bin_cmd!("bt")
        .arg("padright")
        .assert()
        .failure()
        .stderr(predicate::str::contains("required"))
        .stdout(predicate::str::is_empty());
}

#[test]
fn test_image_nonexistent_file_errors_to_stderr() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    // `bt quote` is rendered through the canonical render-tree path, which
    // emits truecolor SGR for the left border regardless of `NO_COLOR`
    // — there is no NO_COLOR-aware downgrade in that path today (see the
    // BlockQuote review notes). Asserting the styled snapshot is the
    // honest contract; revisit if/when the tree renderer learns to honor
    // NO_COLOR end to end.
    let output = cargo_bin_cmd!("bt")
        .arg("quote")
        .arg("To be or not to be, that is the question.")
        .arg("--attribution")
        .arg("Shakespeare")
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);
    insta::assert_snapshot!(stdout);
}

#[test]
fn test_list_snapshot() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let bin_path = assert_cmd::cargo::cargo_bin!("bt");

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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
        .args(["compose", "--md", "--html", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success());
}

#[test]
fn test_compose_example_emits_command_line() {
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
        .args(["compose", "--md-plus", "--text", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("x"), "expected `x` in MarkdownPlus output; got: {stdout:?}");
}

#[test]
fn test_compose_prose_terminal_output_contains_bold_sgr() {
    // The terminal path lowers Prose `<b>` to the SGR bold open sequence
    // (`\x1b[1m`). Stripping SGR is irrelevant here — assert the raw byte
    // sequence is present so a regression in inline-style lowering shows up.
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
        .args(["compose", "--md", "--list", "one", "two"])
        .output()
        .expect("Failed to execute command");
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("- one"), "expected `- one` in Markdown output; got: {stdout:?}");
    assert!(stdout.contains("- two"), "expected `- two` in Markdown output; got: {stdout:?}");
}

#[test]
fn test_compose_md_with_table_emits_gfm_table() {
    let output = cargo_bin_cmd!("bt")
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
    assert!(stdout.contains("| A"), "expected `| A` header cell; got: {stdout:?}");
    assert!(stdout.contains("| B"), "expected `| B` header cell; got: {stdout:?}");
    // GFM separator row uses `:--` / `---` patterns; accept either since the
    // renderer encodes column alignment via the leading colon.
    assert!(
        stdout.contains(":--") || stdout.contains("---"),
        "expected GFM separator row; got: {stdout:?}"
    );
    assert!(stdout.contains("| x"), "expected `| x` cell; got: {stdout:?}");
    assert!(stdout.contains("| y"), "expected `| y` cell; got: {stdout:?}");
}

#[test]
fn test_compose_md_plus_md_mutual_exclusion() {
    let output = cargo_bin_cmd!("bt")
        .args(["compose", "--md-plus", "--md", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md-plus + --md should fail");
}

#[test]
fn test_compose_md_plus_html_mutual_exclusion() {
    let output = cargo_bin_cmd!("bt")
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
    cargo_bin_cmd!("bt")
        .args(["list", "--ordered", "First", "Second", "Third"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1. First"))
        .stdout(predicate::str::contains("2. Second"))
        .stdout(predicate::str::contains("3. Third"));
}

#[test]
fn test_list_short_ordered_flag_works() {
    cargo_bin_cmd!("bt")
        .args(["list", "-o", "A", "B"])
        .assert()
        .success()
        .stdout(predicate::str::contains("1. A"))
        .stdout(predicate::str::contains("2. B"));
}

#[test]
fn test_list_ordered_md_emits_commonmark() {
    let output = cargo_bin_cmd!("bt")
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
    let md = cargo_bin_cmd!("bt")
        .args(["list", "--ordered", "--md", "A", "B", "C"])
        .output()
        .expect("Failed to execute command");
    let md_plus = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
    let output = cargo_bin_cmd!("bt")
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
fn test_list_md_html_mutual_exclusion() {
    let output = cargo_bin_cmd!("bt")
        .args(["list", "--ordered", "--md", "--html", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md + --html should fail");
}

#[test]
fn test_list_md_md_plus_mutual_exclusion() {
    let output = cargo_bin_cmd!("bt")
        .args(["list", "--ordered", "--md", "--md-plus", "x"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success(), "--md + --md-plus should fail");
}

#[test]
fn test_list_unordered_md_returns_error() {
    // UnorderedList's cross-target rendering is pending its IR migration —
    // the CLI rejects --md / --html / --md-plus without --ordered to avoid
    // shipping an inconsistent surface.
    let output = cargo_bin_cmd!("bt")
        .args(["list", "--md", "A", "B"])
        .output()
        .expect("Failed to execute command");
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("require --ordered"),
        "explanatory error: {stderr}"
    );
}

#[test]
fn test_list_ordered_md_with_left_margin_emits_frontmatter() {
    let output = cargo_bin_cmd!("bt")
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
    assert!(stdout.contains("1. First"), "items below frontmatter: {stdout}");
}

#[test]
fn test_list_ordered_html_with_layout_wraps_in_div() {
    let output = cargo_bin_cmd!("bt")
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
    assert!(stdout.contains("<div style=\""), "wrapper div: {stdout}");
    assert!(stdout.contains("margin-left: 2ch"), "css: {stdout}");
    assert!(stdout.contains("<ol"), "ol still emitted: {stdout}");
}

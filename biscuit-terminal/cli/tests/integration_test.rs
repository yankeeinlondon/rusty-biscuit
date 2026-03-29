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
fn test_completions_help() {
    cargo_bin_cmd!("bt")
        .arg("--completions")
        .arg("help")
        .assert()
        .success()
        .stdout(predicate::str::contains("Shell Completions Setup"))
        .stdout(predicate::str::contains("DYNAMIC COMPLETIONS"))
        .stdout(predicate::str::contains("STATIC COMPLETIONS"));
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
        .stderr(predicate::str::contains("invalid shell"))
        .stderr(predicate::str::contains("Valid shells:"));
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
        .arg("Hello {{bold}}world{{reset}}!")
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

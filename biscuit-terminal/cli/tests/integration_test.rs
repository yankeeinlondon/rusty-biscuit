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

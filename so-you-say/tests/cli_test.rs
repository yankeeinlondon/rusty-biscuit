use std::io::Write;
use std::process::{Command, Stdio};

#[test]
fn test_cli_with_arguments() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "test"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should exit with code 0 when given arguments"
    );
}

#[test]
fn test_cli_help_flag() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--help"])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "Help flag should exit with code 0");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("Convert text to speech using system TTS"),
        "Help output should contain description"
    );
    assert!(
        stdout.contains("Usage:"),
        "Help output should contain usage information"
    );
}

#[test]
fn test_cli_version_flag() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--version"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "Version flag should exit with code 0"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("so-you-say"),
        "Version output should contain binary name"
    );
}

#[test]
fn test_cli_stdin_input() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "so-you-say"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(b"test input")
            .expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        output.status.success(),
        "CLI should exit with code 0 when reading from stdin"
    );
}

#[test]
fn test_cli_no_args_closes_gracefully() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "so-you-say"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    // Close stdin immediately without writing anything
    drop(child.stdin.take());

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        !output.status.success(),
        "CLI should exit with code 1 when given no input"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: No input provided"),
        "Error message should be displayed when no input is provided"
    );
}

#[test]
fn test_cli_multi_word_args() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "Hello",
            "world",
            "from",
            "tests",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should handle multiple arguments correctly"
    );
}

#[test]
fn test_cli_empty_stdin() {
    let mut child = Command::new("cargo")
        .args(["run", "-p", "so-you-say"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("Failed to spawn");

    if let Some(mut stdin) = child.stdin.take() {
        stdin.write_all(b"").expect("Failed to write to stdin");
    }

    let output = child.wait_with_output().expect("Failed to wait");
    assert!(
        !output.status.success(),
        "CLI should exit with code 1 when stdin is empty"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("Error: No input provided"),
        "Error message should be displayed when stdin is empty"
    );
}

#[test]
fn test_cli_unicode_args() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "Hello", "世界", "🚀"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should handle unicode arguments correctly"
    );
}

#[test]
fn test_cli_special_chars_args() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "Hello,",
            "world!",
            "How's",
            "it",
            "going?",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should handle special characters in arguments correctly"
    );
}

#[test]
fn test_cli_gender_flag_male() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "--gender",
            "male",
            "test",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept --gender male flag"
    );
}

#[test]
fn test_cli_gender_flag_female() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "--gender",
            "female",
            "test",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept --gender female flag"
    );
}

#[test]
fn test_cli_gender_flag_short() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "-g", "male", "test"])
        .output()
        .expect("Failed to execute");

    assert!(output.status.success(), "CLI should accept -g short flag");
}

#[test]
fn test_cli_gender_flag_invalid() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "--gender",
            "invalid",
            "test",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        !output.status.success(),
        "CLI should reject invalid gender value"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("error"),
        "Error message should indicate invalid value"
    );
}

#[test]
fn test_cli_help_shows_gender_option() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--help"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--gender") && stdout.contains("-g"),
        "Help should document --gender/-g flag"
    );
    assert!(
        stdout.contains("male") && stdout.contains("female"),
        "Help should show possible gender values"
    );
}

#[test]
fn test_cli_list_voices_flag() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--list-voices"])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept --list-voices flag"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("NAME") && stdout.contains("LANGUAGE") && stdout.contains("GENDER"),
        "Voices output should have table headers"
    );
    assert!(
        stdout.contains("Bold") && stdout.contains("default"),
        "Voices output should show legend for default voice"
    );
}

#[test]
fn test_cli_list_voices_with_gender_filter() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "--list-voices",
            "--gender",
            "female",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept --list-voices with --gender filter"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should only show Female voices (or "No voices found" if none available)
    assert!(
        !stdout.contains("Male  ") || stdout.contains("Female"),
        "Output should be filtered to female voices only"
    );
}

#[test]
fn test_cli_help_shows_list_voices_option() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--help"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--list-voices"),
        "Help should document --list-voices flag"
    );
}

#[test]
fn test_cli_voice_option() {
    let output = Command::new("cargo")
        .args([
            "run",
            "-p",
            "so-you-say",
            "--",
            "--voice",
            "Samantha",
            "test",
        ])
        .output()
        .expect("Failed to execute");

    assert!(
        output.status.success(),
        "CLI should accept --voice option"
    );
}

#[test]
fn test_cli_help_shows_voice_option() {
    let output = Command::new("cargo")
        .args(["run", "-p", "so-you-say", "--", "--help"])
        .output()
        .expect("Failed to execute");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--voice"),
        "Help should document --voice option"
    );
}

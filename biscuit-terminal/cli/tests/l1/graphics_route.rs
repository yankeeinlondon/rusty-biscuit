//! The graphics route as `bt` reports it and acts on it with piped output.
//!
//! Every spawned process has piped stdout and stderr, so the route resolves
//! to no pixels and hints (stderr-only, terminal-only) never print.

use std::path::Path;

const TINY_PNG: &[u8] = include_bytes!("../fixtures/tiny.png");

/// `bt` with the graphics overrides removed so the host's settings never
/// leak into a test.
fn bt() -> assert_cmd::Command {
    let mut command = assert_cmd::Command::cargo_bin("bt").expect("bt binary");
    command
        .env_remove("BISCUIT_GRAPHICS")
        .env_remove("BISCUIT_GRAPHICS_TRANSPORT");
    command
}

fn json_report(command: &mut assert_cmd::Command) -> (serde_json::Value, String) {
    let output = command.arg("--json").output().expect("run bt --json");
    assert!(output.status.success(), "bt --json failed: {output:?}");
    let report = serde_json::from_slice(&output.stdout).expect("stdout is JSON");
    (report, String::from_utf8_lossy(&output.stderr).into_owned())
}

fn write_file(dir: &Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write fixture");
    path.to_string_lossy().into_owned()
}

#[test]
fn json_report_without_override_has_no_pixel_route_when_piped() {
    let (report, stderr) = json_report(&mut bt());

    let route = &report["graphics_route"];
    assert_eq!(route["format"], "None", "{route}");
    assert_eq!(route["reason"]["kind"], "NotATerminal", "{route}");
    assert!(route["reason"]["message"].is_string(), "{route}");
    assert!(report.get("image_support").is_some(), "image_support was dropped");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}

/// The library decides "not a terminal" before it validates the override, so
/// a piped run reports `NotATerminal` even for an invalid value; what this
/// pins is that the invalid value neither breaks the JSON nor writes a hint.
/// The `InvalidInput` wording is covered by the `hints` unit tests.
#[test]
fn json_report_with_invalid_override_stays_parseable_and_quiet() {
    let (report, stderr) = json_report(bt().env("BISCUIT_GRAPHICS", "bogus"));

    let reason = &report["graphics_route"]["reason"];
    assert_eq!(reason["kind"], "NotATerminal", "{reason}");
    assert!(stderr.is_empty(), "unexpected stderr: {stderr}");
}

#[test]
fn image_prints_fallback_text_without_graphics_controls_when_piped() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_file(dir.path(), "route-fixture.png", TINY_PNG);

    let output = bt().arg("image").arg(&path).output().expect("run bt image");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "bt image failed: {output:?}");
    assert!(stdout.contains("route-fixture.png"), "{stdout:?}");
    assert!(!stdout.contains("\x1b_G"), "Kitty controls in {stdout:?}");
    assert!(!stdout.contains("\x1b]1337"), "iTerm2 controls in {stdout:?}");
    assert!(output.stderr.is_empty(), "unexpected stderr: {:?}", output.stderr);
}

#[test]
fn image_with_corrupt_png_fails_even_without_a_pixel_route() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_file(dir.path(), "corrupt.png", b"\x89PNG\r\n\x1a\nnot really a png");

    let output = bt().arg("image").arg(&path).output().expect("run bt image");

    assert!(!output.status.success(), "corrupt image succeeded: {output:?}");
    assert!(!output.stderr.is_empty(), "no error message on stderr");
    assert!(
        !String::from_utf8_lossy(&output.stdout).contains("corrupt.png"),
        "fallback text printed for a corrupt image"
    );
}

/// Forcing color makes `bt` treat stdout as a truecolor terminal, but the
/// route still knows it is a pipe: plain text, never half-block color cells.
#[test]
fn image_prints_plain_text_not_half_blocks_when_piped_with_forced_color() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = write_file(dir.path(), "forced-fixture.png", TINY_PNG);

    let output = bt()
        .env("FORCE_COLOR", "1")
        .env("COLORTERM", "truecolor")
        .env("TERM_PROGRAM", "Apple_Terminal")
        .arg("image")
        .arg(&path)
        .output()
        .expect("run bt image");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(output.status.success(), "bt image failed: {output:?}");
    assert_eq!(stdout.trim_end(), "[Image: forced-fixture.png]");
    assert!(!stdout.contains('\x1b'), "escape sequences in {stdout:?}");
    assert!(output.stderr.is_empty(), "unexpected stderr: {:?}", output.stderr);
}

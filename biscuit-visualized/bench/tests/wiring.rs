//! Spawns the benchmark binary for the AC-V2 wiring checks: a control run
//! passes, a doctored baseline and a doctored limit each flag. One family and
//! three iterations keep each run short; the limits are lifted wherever a test
//! is about another rule, because this binary is a debug build.

use std::path::Path;
use std::process::{Command, Output};

const BENCH: &str = env!("CARGO_BIN_EXE_biscuit-visualized-bench");

/// A flowchart baseline with both medians set to `median_ms`.
fn baseline(dir: &Path, median_ms: f64) -> std::path::PathBuf {
    let path = dir.join("baseline.json");
    let stats = format!(r#"{{"median_ms": {median_ms}, "p95_ms": {median_ms}}}"#);
    let text = format!(
        r#"{{"reference_host": "not-this-host", "families": {{"flowchart": {{"warm": {stats}, "cold": {stats}}}}}}}"#
    );
    std::fs::write(&path, text).expect("write baseline");
    path
}

fn bench(baseline: &Path, extra: &[&str]) -> Output {
    Command::new(BENCH)
        .args(["--families", "flowchart", "--iterations", "3", "--baseline"])
        .arg(baseline)
        .args(extra)
        .output()
        .expect("spawn the benchmark")
}

fn stdout(output: &Output) -> String {
    String::from_utf8_lossy(&output.stdout).into_owned()
}

const NO_LIMITS: &[&str] = &["--warm-limit-ms", "600000", "--cold-limit-ms", "600000"];

#[test]
fn control_run_passes_against_a_generous_baseline() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = baseline(dir.path(), 600_000.0);
    let output = bench(&path, &[&["--reference"], NO_LIMITS].concat());
    assert_eq!(
        output.status.code(),
        Some(0),
        "{}{}",
        stdout(&output),
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(stdout(&output).contains("no rule fired"));
}

#[test]
fn doctored_baseline_flags_a_regression() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = baseline(dir.path(), 0.001);
    let output = bench(&path, &[&["--reference"], NO_LIMITS].concat());
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("flowchart: warm median"), "{text}");
    assert!(text.contains("slower than the baseline 0.00 ms"), "{text}");
}

#[test]
fn doctored_limit_flags() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = baseline(dir.path(), 600_000.0);
    let output = bench(
        &path,
        &[
            "--reference",
            "--warm-limit-ms",
            "0.001",
            "--cold-limit-ms",
            "600000",
        ],
    );
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(text.contains("flowchart: warm median"), "{text}");
    assert!(text.contains("is over the 0.001 ms limit"), "{text}");
}

#[test]
fn family_without_baseline_flags_on_the_reference_host() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("baseline.json");
    std::fs::write(
        &path,
        r#"{"reference_host": "not-this-host", "families": {}}"#,
    )
    .expect("write baseline");
    let output = bench(&path, &[&["--reference"], NO_LIMITS].concat());
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(1), "{text}");
    assert!(
        text.contains("flowchart: no baseline recorded for this family"),
        "{text}"
    );
}

#[test]
fn off_the_reference_host_a_doctored_baseline_is_informational() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = baseline(dir.path(), 0.001);
    let output = bench(&path, NO_LIMITS);
    let text = stdout(&output);
    assert_eq!(output.status.code(), Some(0), "{text}");
    assert!(text.contains("would flag (informational)"), "{text}");
}

#[test]
fn invalid_baseline_is_an_error_and_is_not_overwritten() {
    let dir = tempfile::tempdir().expect("temp dir");
    let path = dir.path().join("baseline.json");
    let text = r#"{"reference_host": "h", "families": {}, "families": {}}"#;
    std::fs::write(&path, text).expect("write baseline");
    let output = bench(&path, &["--reference", "--write-baseline"]);
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert_eq!(output.status.code(), Some(2), "{stderr}");
    assert!(stderr.contains("families: duplicate key"), "{stderr}");
    assert_eq!(std::fs::read_to_string(&path).expect("read baseline"), text);
}

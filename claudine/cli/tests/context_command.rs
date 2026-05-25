use assert_cmd::cargo::cargo_bin_cmd;
use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..").canonicalize().unwrap()
}

#[test]
fn context_default_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "default context should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column; got: {stdout}"
    );
    assert!(
        stdout.contains("Type"),
        "should have Type column; got: {stdout}"
    );
    assert!(
        stdout.contains("Description"),
        "should have Description column; got: {stdout}"
    );
    assert!(
        stdout.contains("Date and Time Information"),
        "should have H3 grouping; got: {stdout}"
    );
}

#[test]
fn context_default_works_outside_repo() {
    let temp_dir = std::env::temp_dir();
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(&temp_dir)
        .args(["context"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(
        stdout.contains("ctx.today"),
        "should show ctx.today outside repo; got: {stdout}"
    );
    assert!(
        stdout.contains("Property"),
        "should have Property column outside repo; got: {stdout}"
    );
}

#[test]
fn context_values_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--values should produce stdout");
    assert!(
        stdout.contains("ctx.today"),
        "should display ctx.today; got: {stdout}"
    );
    assert!(
        stdout.contains("Value"),
        "should have Value column; got: {stdout}"
    );
    assert!(
        !stdout.contains("Description"),
        "should NOT have Description column; got: {stdout}"
    );
}

/// Regression: review-2 found that documented aliases (`utc`, `dow`,
/// `dow_abbr`) rendered as `null` under `--values` because the Darkmatter
/// runtime captured only canonical keys. Aliases are now populated; ensure
/// they appear as non-null values alongside their canonical counterparts.
#[test]
fn context_values_resolves_documented_aliases() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    // Locate each alias row and confirm the value cell is not the literal
    // `null` text. Rows are rendered as `| ctx.<key> | <Type> | <value> |`.
    for alias in ["ctx.utc", "ctx.dow", "ctx.dow_abbr"] {
        let row = stdout
            .lines()
            .find(|line| line.contains(alias))
            .unwrap_or_else(|| panic!("expected a row for {alias}; got stdout:\n{stdout}"));
        assert!(
            !row.contains("null"),
            "{alias} row must contain a real value, not `null`; row: {row}",
        );
    }
}

/// Regression: `--values` must also surface canonical values, not just
/// aliases. `ctx.today` always has a value, so use it as a sentinel.
#[test]
fn context_values_renders_non_null_for_canonical_keys() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let today_row = stdout
        .lines()
        .find(|line| line.contains("ctx.today"))
        .unwrap_or_else(|| panic!("expected ctx.today row; got stdout:\n{stdout}"));
    assert!(
        !today_row.contains("null"),
        "ctx.today must have a real value; row: {today_row}",
    );
}

#[test]
fn context_expressions_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "--expressions should produce stdout");
    assert!(
        stdout.contains("Operator Precedence"),
        "should show precedence; got: {stdout}"
    );
    assert!(
        stdout.contains("Truthiness"),
        "should show truthiness; got: {stdout}"
    );
    assert!(
        stdout.contains("Unary Operators"),
        "should show unary operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Comparison Operators"),
        "should show comparison operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Arithmetic Operators"),
        "should show arithmetic operators; got: {stdout}"
    );
    assert!(
        stdout.contains("Variable Access"),
        "should show variable access; got: {stdout}"
    );
    assert!(
        stdout.contains("Null Propagation"),
        "should show null propagation; got: {stdout}"
    );
    assert!(
        stdout.contains("Functions"),
        "should show functions; got: {stdout}"
    );
}

#[test]
fn context_side_effects_exits_zero_and_produces_stdout() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stdout.contains("not implemented yet"),
        "--side-effects should show placeholder on stdout; got: {stdout}"
    );
    // The footer messages must remain on stderr — separate from the
    // placeholder text that goes to stdout.
    assert!(
        !stderr.contains("not implemented yet"),
        "placeholder must not leak to stderr; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr must carry the --expressions hint; got: {stderr}"
    );
}

/// Regression: review-2 finding "tables are parsed with the first data row
/// as the header" — verify the rendered output now contains the real
/// `Value` / `Falsy` headers from the Truthiness table rather than the
/// first data row's contents being promoted to a header.
#[test]
fn context_expressions_truthiness_table_uses_real_headers() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);

    // Find the Truthiness section, then look for `Value` and `Falsy` in the
    // bytes that follow it. These are the documented column headers; if the
    // parser regressed they would be replaced by the first data row's
    // contents (`null` / `yes`).
    let truthiness_idx = stdout
        .find("Truthiness")
        .unwrap_or_else(|| panic!("expected Truthiness heading; got stdout:\n{stdout}"));
    let after = &stdout[truthiness_idx..];

    assert!(
        after.contains("Value"),
        "Truthiness table must carry the documented `Value` header; \
         output after heading:\n{after}",
    );
    assert!(
        after.contains("Falsy"),
        "Truthiness table must carry the documented `Falsy` header; \
         output after heading:\n{after}",
    );
}

#[test]
fn context_default_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_values_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--values"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        !stderr.contains("--values"),
        "stderr should NOT mention --values when already using --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_expressions_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--expressions"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

#[test]
fn context_side_effects_writes_footer_to_stderr() {
    let assert = cargo_bin_cmd!("claudine")
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["context", "--side-effects"])
        .assert()
        .success();

    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    assert!(
        stderr.contains("--values"),
        "stderr should mention --values; got: {stderr}"
    );
    assert!(
        stderr.contains("--expressions"),
        "stderr should mention --expressions; got: {stderr}"
    );
    assert!(
        stderr.contains("--side-effects"),
        "stderr should mention --side-effects; got: {stderr}"
    );
}

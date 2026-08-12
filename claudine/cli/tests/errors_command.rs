use std::path::PathBuf;

fn repo_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir.join("../..").canonicalize().unwrap()
}

/// Strips ANSI CSI escape sequences so a captured heading line compares as plain
/// text even if a trailing reset survives under `NO_COLOR`.
fn strip_ansi(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let mut chars = input.chars();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            for next in chars.by_ref() {
                if ('@'..='~').contains(&next) {
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
fn errors_default_exits_zero_and_lists_representative_codes() {
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("COLUMNS", "200")
        .current_dir(repo_root())
        .args(["errors"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    assert!(!stdout.is_empty(), "errors report should produce stdout");

    // Representative codes from across the registry.
    for code in [
        "cap.plan_limit",
        "composition.invalid_file_reference",
        "io.read_failed",
    ] {
        assert!(
            stdout.contains(code),
            "errors report should list `{code}`; got:\n{stdout}"
        );
    }

    // Column headers mirror the documented contract.
    for header in ["Code", "Disposition", "Origin", "Detail"] {
        assert!(
            stdout.contains(header),
            "errors report should have `{header}` column; got:\n{stdout}"
        );
    }

    // At least one detail field name is surfaced (the `err.detail.*` keys).
    assert!(
        stdout.contains("reset_at"),
        "errors report should show the `reset_at` detail field; got:\n{stdout}"
    );
    assert!(
        stdout.contains("reference"),
        "errors report should show the `reference` detail field; got:\n{stdout}"
    );
}

#[test]
fn errors_default_lists_every_code_contiguously() {
    // Each `err.code` is the copyable string an author pastes into a `when:`
    // clause, so the human report must contain it as one unbroken substring —
    // never wrapped across the `Code` cell. Guards against any future code being
    // silently split out of the introspection surface.
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("COLUMNS", "200")
        .current_dir(repo_root())
        .args(["errors"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    for code in claudine::diagnostics::CODES.iter().map(|spec| spec.code) {
        assert!(
            stdout.contains(code),
            "errors report must list `{code}` as a contiguous string; got:\n{stdout}"
        );
    }
}

#[test]
fn errors_default_groups_by_category() {
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .env("COLUMNS", "200")
        .current_dir(repo_root())
        .args(["errors"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    // Category headings render as standalone lines (not just code prefixes).
    for category in ["cap", "composition", "io", "internal"] {
        assert!(
            stdout
                .lines()
                .any(|line| strip_ansi(line).trim() == category),
            "errors report should have a `{category}` category heading; got:\n{stdout}"
        );
    }
}

#[test]
fn errors_json_emits_valid_array_with_known_code() {
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["errors", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(&stdout).expect("errors --json must emit valid JSON");

    let array = parsed.as_array().expect("errors --json must be an array");
    assert!(!array.is_empty(), "errors --json array must not be empty");

    let plan_limit = array
        .iter()
        .find(|entry| entry["code"] == "cap.plan_limit")
        .expect("errors --json must contain cap.plan_limit");
    assert_eq!(plan_limit["category"], "cap");
    assert_eq!(plan_limit["disposition"], "throttled");
    assert!(
        plan_limit["detail"]
            .as_array()
            .unwrap()
            .iter()
            .any(|f| f == "reset_at"),
        "cap.plan_limit detail must carry reset_at; got: {plan_limit}"
    );
}

#[test]
fn errors_json_covers_every_registered_code() {
    let assert = assert_cmd::Command::cargo_bin("claudine")
        .unwrap()
        .env("NO_COLOR", "1")
        .current_dir(repo_root())
        .args(["errors", "--json"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&assert.get_output().stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).unwrap();
    let count = parsed.as_array().unwrap().len();
    assert_eq!(
        count,
        claudine::diagnostics::CODES.len(),
        "errors --json must surface every code in the registry"
    );
}

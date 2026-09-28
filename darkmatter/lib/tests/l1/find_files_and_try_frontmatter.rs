//! `find_files(pattern)` and `try_frontmatter(file)`, read through the
//! public composition result: each row composes a document whose frontmatter
//! value `v` calls the function, and asserts the composed value or the
//! composition error.

use std::path::Path;
use std::process::Command;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeOptions;
use serde_json::{Value, json};

/// A Git repository with fixes under two lifecycle directories.
fn repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("temp dir");
    let status = Command::new("git")
        .args(["init", "-q"])
        .current_dir(dir.path())
        .status()
        .expect("run git");
    assert!(status.success());
    for (path, content) in [
        ("area/fixes/2026-01-01-a/spec.md", "---\nstatus: draft\n---\n"),
        ("area/fixes/_completed/2026-01-02-b/spec.md", "---\nstatus: completed\n---\n"),
        ("area/fixes/_completed/2026-01-02-b/plan.md", "plan\n"),
        ("area/other/2026-01-01-a/spec.md", "---\n---\n"),
        ("docs/dup.md", "---\nstatus: a\nstatus: b\n---\n"),
        ("docs/invalid.md", "---\nfixed: [F1\n---\n"),
        ("docs/unterminated.md", "---\nstatus: draft\nno closing fence\n"),
        ("docs/ok.md", "---\nstatus: draft\nfixed: [F1]\n---\nBody\n"),
    ] {
        let path = dir.path().join(path);
        std::fs::create_dir_all(path.parent().expect("parent")).expect("mkdir");
        std::fs::write(path, content).expect("write fixture");
    }
    dir
}

/// Composes `docs/probe.md` with `v: "{{ <expression> }}"` and returns `v`.
fn value(dir: &Path, expression: &str) -> Result<Value, String> {
    let path = dir.join("docs/probe.md");
    let yaml = format!("{{{{ {expression} }}}}").replace('\'', "''");
    std::fs::write(&path, format!("---\nv: '{yaml}'\n---\nBody\n")).expect("write probe");
    let document = Markdown::try_from(path.as_path()).map_err(|error| error.to_string())?;
    let (composed, _report) = document
        .compose_with(ComposeOptions::new().with_source_file(&path))
        .map_err(|error| error.to_string())?;
    Ok(composed.frontmatter().as_map().get("v").cloned().unwrap_or(Value::Null))
}

fn portable(dir: &Path, relative: &str) -> Value {
    let canonical = std::fs::canonicalize(dir.join(relative)).expect("fixture exists");
    json!(biscuit_file::to_portable_string(&canonical))
}

/// Paths as the function reports them, canonicalized so a symlinked temp
/// directory (macOS `/var` → `/private/var`) compares equal.
fn canonical_list(value: Value) -> Value {
    Value::Array(
        value
            .as_array()
            .unwrap_or_else(|| panic!("expected an array, got {value}"))
            .iter()
            .map(|path| {
                let path = std::fs::canonicalize(path.as_str().expect("path string")).expect("match exists");
                json!(biscuit_file::to_portable_string(&path))
            })
            .collect(),
    )
}

#[test]
fn find_files_reports_every_match_of_a_glob_reference() {
    let dir = repo();
    let d = dir.path();
    let spec_a = portable(d, "area/fixes/2026-01-01-a/spec.md");
    let spec_b = portable(d, "area/fixes/_completed/2026-01-02-b/spec.md");
    let rows: [(&str, Value); 9] = [
        // Control: one active match, found by directory identity.
        ("find_files('&area/fixes/**/2026-01-01-a/spec.md')", json!([spec_a])),
        // Relocated under a lifecycle directory: still found.
        ("find_files('&area/fixes/**/2026-01-02-b/spec.md')", json!([spec_b])),
        // Two matches are both reported, sorted.
        ("find_files('&area/fixes/**/spec.md')", json!([spec_a, spec_b])),
        // `**` matches zero directories as well as several.
        ("find_files('&area/**/2026-01-01-a/spec.md')", json!([portable(d, "area/fixes/2026-01-01-a/spec.md"), portable(d, "area/other/2026-01-01-a/spec.md")])),
        // `*` stays within one segment.
        ("find_files('&area/fixes/*/spec.md')", json!([spec_a])),
        // A pattern with no wildcard names one file.
        ("find_files('&area/fixes/2026-01-01-a/spec.md')", json!([spec_a])),
        // A bare sigil searches its root.
        ("find_files('&**/2026-01-02-b/plan.md')", json!([portable(d, "area/fixes/_completed/2026-01-02-b/plan.md")])),
        // A pattern relative to the document's directory.
        ("find_files('**/ok.md')", json!([portable(d, "docs/ok.md")])),
        // Nothing matches.
        ("find_files('&area/fixes/**/2026-09-09-none/spec.md')", json!([])),
    ];
    for (expression, expected) in rows {
        let found = value(d, expression).unwrap_or_else(|error| panic!("{expression}: {error}"));
        assert_eq!(canonical_list(found), expected, "{expression}");
    }
}

#[test]
fn find_files_input_matrix() {
    let dir = repo();
    let d = dir.path();
    // Absent directory and absent file: empty, never an error.
    assert_eq!(value(d, "find_files('&missing/**/spec.md')").unwrap(), json!([]));
    assert_eq!(value(d, "find_files('&area/fixes/none.md')").unwrap(), json!([]));
    // Explicit null: empty.
    assert_eq!(value(d, "find_files(null)").unwrap(), json!([]));
    // Wrong type, missing argument, invalid glob, a `%` reference, a URL: errors.
    for (expression, fragment) in [
        ("find_files(3)", "find_files"),
        ("find_files()", "find_files"),
        ("find_files('&area/[')", "invalid glob"),
        ("find_files('%&spec.md')", "not a `%` recursive reference"),
        ("find_files('https://example.com/*.md')", "cannot search"),
    ] {
        let error = value(d, expression).expect_err(expression);
        assert!(error.contains(fragment), "{expression}: expected `{fragment}` in: {error}");
    }
}

#[test]
fn try_frontmatter_reports_a_failure_as_a_value() {
    let dir = repo();
    let d = dir.path();
    assert_eq!(
        value(d, "try_frontmatter('&docs/ok.md')").unwrap(),
        json!({"error": null, "ok": true, "value": {"status": "draft", "fixed": ["F1"]}})
    );
    for (file, fragment) in [
        ("&docs/missing.md", "missing.md"),
        ("&docs/dup.md", "duplicate"),
        ("&docs/invalid.md", "parse"),
    ] {
        let outcome = value(d, &format!("try_frontmatter('{file}')")).unwrap();
        assert_eq!(outcome["ok"], json!(false), "{file}: {outcome}");
        assert_eq!(outcome["value"], Value::Null, "{file}: {outcome}");
        let error = outcome["error"].as_str().unwrap_or_default();
        assert!(error.contains(fragment), "{file}: expected `{fragment}` in: {error}");
    }
    // An opening fence that never closes is body text, as `frontmatter()` reads it.
    assert_eq!(
        value(d, "try_frontmatter('&docs/unterminated.md')").unwrap(),
        json!({"error": null, "ok": true, "value": {}})
    );
    assert_eq!(value(d, "try_frontmatter(null)").unwrap(), Value::Null);
    assert!(value(d, "try_frontmatter(3)").is_err());
}

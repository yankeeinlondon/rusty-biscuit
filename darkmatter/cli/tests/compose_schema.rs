//! Integration tests for schema validation in the `md compose` pipeline.
//!
//! Exercises the binary (not the in-process library) so that exit status,
//! styled `BlockError` rendering, stderr routing, and the top-level error
//! handler are all covered end-to-end.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    cargo_bin_cmd!("md")
}

fn write_file(dir: &TempDir, name: &str, content: &str) -> PathBuf {
    let path = dir.path().join(name);
    let mut f = fs::File::create(&path).unwrap();
    f.write_all(content.as_bytes()).unwrap();
    path
}

fn read_to_string(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

// ── High #2: planner-prompt CLI regression ──────────────────────────────────

/// The originating bug: a document with `$schema` requiring `spec`, an empty
/// `spec: ""`, and `dir: "$(dirname '{{ spec }}')"` used to surface as a
/// cryptic `dirname` shell error. With the always-on schema-validation stage,
/// `md compose` must exit non-zero with the styled `BlockError`, mention the
/// failing property (`spec`), and never reach shell expansion (so a sentinel
/// command in `$(...)` must NOT leave any side effect, and `dirname` must NOT
/// appear in the rendered output).
#[test]
fn compose_fails_fast_with_schema_block_before_shell_expansion() {
    let tmp = TempDir::new().unwrap();
    let sentinel = tmp.path().join("SENTINEL");
    let sentinel_arg = sentinel.to_string_lossy().to_string();

    // Use a sentinel command that would create a file if shell expansion ran.
    let content = format!(
        "---\n\
        $schema:\n  spec: 'string(min(1); required)'\n\
        spec: \"\"\n\
        dir: \"$(touch {} && dirname '{{{{ spec }}}}')\"\n\
        ---\nBody\n",
        sentinel_arg,
    );
    let doc = write_file(&tmp, "planner.md", &content);

    let assert = md_cmd().args(["compose"]).arg(&doc).assert().failure();
    let output = assert.get_output();
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(
        stderr.contains("schema validation failed"),
        "expected schema-validation header on stderr, got stderr:\n{stderr}\nstdout:\n{stdout}",
    );
    assert!(
        stderr.contains("spec"),
        "expected failing property `spec` to appear on stderr, got:\n{stderr}",
    );
    // Shell expansion must not have run — the rendered block must not contain
    // any `dirname` message and the sentinel file must not exist.
    assert!(
        !stderr.contains("dirname") && !stdout.contains("dirname"),
        "expected shell expansion to never run, but `dirname` leaked into output:\nstderr:\n{stderr}\nstdout:\n{stdout}",
    );
    assert!(
        !sentinel.exists(),
        "shell sentinel `{}` was created — schema validation did not fail fast before shell expansion",
        sentinel.display(),
    );

    // The styled block lists the source path so authors can jump to it.
    assert!(
        stderr.contains("planner.md"),
        "expected source path on stderr, got:\n{stderr}",
    );
}

#[test]
fn compose_with_required_property_supplied_succeeds() {
    let tmp = TempDir::new().unwrap();
    let target = write_file(&tmp, "design.md", "# Design\n");
    let _ = target;
    let content = "---\n\
        $schema:\n  spec: 'string(min(1); required)'\n\
        spec: design.md\n\
        ---\nBody\n";
    let doc = write_file(&tmp, "planner-ok.md", content);

    md_cmd()
        .current_dir(tmp.path())
        .args(["compose"])
        .arg(&doc)
        .assert()
        .success()
        .stdout(predicate::str::contains("Body"));
}

// ── Medium #4: md compose ↔ md schema validate parity ───────────────────────

/// Both `md compose` and `md schema validate` consume the same input file and
/// the same underlying `DarkmatterSchemas::validate`. They must agree:
/// - both fail (non-zero exit) for the same failing property when it is
///   missing/blank;
/// - both succeed when the failing property is supplied via positional
///   `key=value` setter.
#[test]
fn compose_and_schema_validate_agree_on_same_document() {
    let tmp = TempDir::new().unwrap();
    let content = "---\n\
        $schema:\n  title: 'string(min(1); required)'\n\
        ---\nBody\n";
    let doc = write_file(&tmp, "post.md", content);

    // Both fail without the required property.
    let validate_failure = md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .assert()
        .failure();
    let validate_stdout = String::from_utf8_lossy(&validate_failure.get_output().stdout).to_string();
    assert!(
        validate_stdout.contains("title"),
        "schema validate should mention failing property `title`, got:\n{validate_stdout}",
    );

    let compose_failure = md_cmd().args(["compose"]).arg(&doc).assert().failure();
    let compose_stderr =
        String::from_utf8_lossy(&compose_failure.get_output().stderr).to_string();
    assert!(
        compose_stderr.contains("title"),
        "compose should mention failing property `title`, got:\n{compose_stderr}",
    );

    // Both succeed when the required property is supplied via positional
    // `title=...` setter (compose uses the same shorthand-set machinery via
    // its positional ARGS; schema validate has parity via its assignment
    // positional support).
    md_cmd()
        .args(["schema", "validate"])
        .arg(&doc)
        .arg("title=Hello")
        .assert()
        .success();

    md_cmd()
        .args(["compose"])
        .arg(&doc)
        .arg("title=Hello")
        .assert()
        .success();
}

// ── Schema-preparation failure surfaces the diagnostic ──────────────────────

/// Schema preparation errors (malformed `$schema`, unresolved reference)
/// arrive at the renderer with an empty problem list and the diagnostic in
/// `summary`. The styled block must surface that diagnostic so users see
/// the root cause instead of just the path.
#[test]
fn compose_renders_summary_for_schema_preparation_failure() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "bad-schema.md",
        "---\n$schema: ./does-not-exist.yaml\ntitle: hi\n---\nBody\n",
    );

    let assert = md_cmd().args(["compose"]).arg(&doc).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);

    assert!(
        stderr.contains("schema validation failed"),
        "expected schema-validation header on stderr, got:\n{stderr}",
    );
    // The underlying diagnostic must be visible — not just the path.
    assert!(
        stderr.to_lowercase().contains("schema could not be prepared")
            || stderr.to_lowercase().contains("could not")
            || stderr.to_lowercase().contains("resolve")
            || stderr.to_lowercase().contains("not found"),
        "expected preparation diagnostic on stderr (e.g. resolution failure), got:\n{stderr}",
    );
}

// ── Source-path link on styled error ────────────────────────────────────────

/// Ensure the source file path appears in the rendered block (the styled
/// renderer wraps it in OSC8, but the literal path text is still emitted).
#[test]
fn compose_block_includes_source_path() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "needs-title.md",
        "---\n$schema:\n  title: 'string(required)'\n---\nBody\n",
    );

    let assert = md_cmd().args(["compose"]).arg(&doc).assert().failure();
    let stderr = String::from_utf8_lossy(&assert.get_output().stderr);
    let _ = read_to_string(&doc);

    assert!(
        stderr.contains("needs-title.md"),
        "expected file name in styled block stderr, got:\n{stderr}",
    );
}

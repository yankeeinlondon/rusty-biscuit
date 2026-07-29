//! Integration tests for schema validation in the `md compose` pipeline.
//!
//! Exercises the binary (not the in-process library) so that exit status,
//! styled `BlockError` rendering, stderr routing, and the top-level error
//! handler are all covered end-to-end.

use darkmatter::markdown::Markdown;
use predicates::prelude::*;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::TempDir;

fn md_cmd() -> assert_cmd::Command {
    assert_cmd::Command::cargo_bin("md").unwrap()
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

// ── review-1 Medium #3: `md compose --frontmatter` write-back serialization ──
//
// The library unit tests in `compose::schema_validation` prove that
// `schema_validation::run` mutates the in-memory frontmatter. These CLI tests
// close the gap the review flagged: that the coerced values actually serialize
// into the composed document after the *full* pipeline — the user-visible
// output. Each composes with `--frontmatter`, re-parses the emitted document,
// and asserts the serialized frontmatter holds real JSON types. Re-parsing (vs
// matching raw YAML text) is robust against the serializer's quote-style
// choices while still proving the on-disk bytes round-trip to the right type.

/// Composes `doc` with `--frontmatter` and re-parses the emitted document so
/// callers can inspect the *serialized* frontmatter's real JSON types.
fn composed_frontmatter(doc: &Path) -> serde_json::Map<String, serde_json::Value> {
    let assert = md_cmd()
        .args(["compose", "--frontmatter"])
        .arg(doc)
        .assert()
        .success();
    let stdout = String::from_utf8_lossy(&assert.get_output().stdout).to_string();
    let reparsed: Markdown = stdout.as_str().into();
    reparsed
        .frontmatter()
        .as_map()
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect()
}

#[test]
fn compose_frontmatter_serializes_real_boolean() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "boolean.md",
        "---\n$schema:\n  has_spec: boolean\nhas_spec: \"true\"\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("has_spec"), Some(&serde_json::json!(true)));
}

#[test]
fn compose_frontmatter_serializes_real_number() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "number.md",
        "---\n$schema:\n  n: number\nn: \"42\"\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("n"), Some(&serde_json::json!(42)));
}

#[test]
fn compose_frontmatter_serializes_string_reverse_coercion() {
    let tmp = TempDir::new().unwrap();
    // A real number `42` against a `string` field is reverse-coerced to "42";
    // the composed output must store (and serialize) it as a real string.
    let doc = write_file(
        &tmp,
        "string.md",
        "---\n$schema:\n  spec: 'string(required)'\nspec: 42\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("spec"), Some(&serde_json::json!("42")));
}

#[test]
fn compose_frontmatter_serializes_boolish_normalization() {
    let tmp = TempDir::new().unwrap();
    // `boolish` accepts the `True` spelling and normalizes it to a real bool.
    let doc = write_file(
        &tmp,
        "boolish.md",
        "---\n$schema:\n  flag: boolish\nflag: \"True\"\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("flag"), Some(&serde_json::json!(true)));
}

#[test]
fn compose_frontmatter_serializes_numberlike_normalization() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "numberlike.md",
        "---\n$schema:\n  n: numberlike\nn: \"42\"\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("n"), Some(&serde_json::json!(42)));
}

#[test]
fn compose_frontmatter_serializes_typed_array() {
    let tmp = TempDir::new().unwrap();
    let doc = write_file(
        &tmp,
        "array.md",
        "---\n$schema:\n  flags: 'boolean[]'\nflags:\n  - \"true\"\n  - \"false\"\n---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("flags"), Some(&serde_json::json!([true, false])));
}

#[test]
fn compose_frontmatter_serializes_root_union_write_back() {
    let tmp = TempDir::new().unwrap();
    // A 3-arm root union (mirrors prompts/implement.md) where every arm types
    // the `has_*` trio as boolean. The frontmatter holds them as strings; the
    // composed output must serialize them as real booleans via the
    // first-validating-arm coercion path.
    let doc = write_file(
        &tmp,
        "union.md",
        "---\n\
         $schema:\n\
        \x20 - review: string(required)\n\
        \x20   has_spec: boolean\n\
        \x20   has_plan: boolean\n\
        \x20   has_review: boolean\n\
        \x20 - spec: string(required)\n\
        \x20   has_spec: boolean\n\
        \x20   has_plan: boolean\n\
        \x20   has_review: boolean\n\
        \x20 - plan: string(required)\n\
        \x20   has_spec: boolean\n\
        \x20   has_plan: boolean\n\
        \x20   has_review: boolean\n\
         spec: design.md\n\
         has_spec: \"true\"\n\
         has_plan: \"false\"\n\
         has_review: \"false\"\n\
         ---\nBody\n",
    );
    let fm = composed_frontmatter(&doc);
    assert_eq!(fm.get("has_spec"), Some(&serde_json::json!(true)));
    assert_eq!(fm.get("has_plan"), Some(&serde_json::json!(false)));
    assert_eq!(fm.get("has_review"), Some(&serde_json::json!(false)));
}

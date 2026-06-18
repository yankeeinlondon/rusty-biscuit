mod common;

use common::{md_cmd, md_file};
use predicates::prelude::*;
use std::io::Write;

const FM_DOC: &str = "---\n\
title: Hello World\n\
author: Alice\n\
count: 42\n\
tags:\n\
  - rust\n\
  - cli\n\
---\n\
# Document\n";

const FM_DOC_TAB_INDENT: &str = "---\n\
\tlast_updated: 2026-02-27\n\
---\n\
# Document\n";

#[test]
fn test_get_single_property_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"Hello World\""));
}

#[test]
fn test_get_single_property_number() {
    md_cmd()
        .args(["get", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_single_property_array() {
    md_cmd()
        .args(["get", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_get_missing_property_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "nonexistent"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_multiple_properties_returns_object() {
    md_cmd()
        .args(["get", "-", "title", "author"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"author\""))
        .stdout(predicate::str::contains("\"Alice\""));
}

#[test]
fn test_get_multiple_with_missing_includes_empty_string() {
    md_cmd()
        .args(["get", "-", "title", "missing"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\""))
        .stdout(predicate::str::contains("\"Hello World\""))
        .stdout(predicate::str::contains("\"missing\""))
        .stdout(predicate::str::contains("\"\""));
}

#[test]
fn test_get_json5_output() {
    md_cmd()
        .args(["get", "--json5", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        // JSON5 uses unquoted keys for valid identifiers
        .stdout(predicate::str::contains("title:"))
        .stdout(predicate::str::contains("count:"));
}

#[test]
fn test_get_yaml_output() {
    md_cmd()
        .args(["get", "--yaml", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("Hello World"));
}

#[test]
fn test_get_toml_output() {
    md_cmd()
        .args(["get", "--toml", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title"))
        .stdout(predicate::str::contains("Hello World"))
        .stdout(predicate::str::contains("count"))
        .stdout(predicate::str::contains("42"));
}

#[test]
fn test_get_from_file() {
    let tmp = md_file("---\nversion: 2\n---\n# Doc\n");

    md_cmd()
        .args(["get"])
        .arg(tmp.path())
        .arg("version")
        .assert()
        .success()
        .stdout(predicate::str::contains("2"));
}

#[test]
fn test_get_no_frontmatter_returns_empty_string() {
    md_cmd()
        .args(["get", "-", "title"])
        .write_stdin("# No frontmatter")
        .assert()
        .success()
        .stdout(predicate::str::contains("\"\""));
}

/// Regression: malformed frontmatter (a quoted scalar followed by trailing
/// unquoted text) used to be silently treated as "no frontmatter", so
/// `md get phases` returned `""` even when the file clearly defined `phases`.
/// The fix surfaces a `MarkdownError::FrontmatterParse` with the offending
/// YAML line in the rendered StatusBlock.
#[test]
fn test_get_malformed_frontmatter_renders_status_block_with_offending_line() {
    use darkmatter::testing::strip_ansi_codes;

    let yaml = "---\nphases: 5\nfindings:\n  - id: '@' magic lookup emits results\n---\n# Doc\n";

    let output = md_cmd()
        .args(["get", "-", "phases"])
        .write_stdin(yaml)
        .output()
        .expect("md get should run");

    assert!(!output.status.success(), "expected a failure exit status");

    // The offending YAML line is syntax-highlighted, so its characters are
    // interleaved with SGR escapes in the raw stderr; strip ANSI before
    // asserting on the visible text.
    let stderr = strip_ansi_codes(&String::from_utf8_lossy(&output.stderr));
    assert!(stderr.contains("MarkdownError"), "stderr: {stderr}");
    assert!(stderr.contains("frontmatter parse failed"), "stderr: {stderr}");
    assert!(
        stderr.contains("'@' magic lookup emits results"),
        "offending line must be shown. stderr: {stderr}"
    );
}

#[test]
fn test_get_tab_indented_frontmatter_property_is_populated() {
    md_cmd()
        .args(["get", "-", "last_updated"])
        .write_stdin(FM_DOC_TAB_INDENT)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"2026-02-27\""));
}

// -- raw flag tests --

#[test]
fn test_get_raw_string_unquoted() {
    md_cmd()
        .args(["get", "--raw", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("Hello World\n");
}

#[test]
fn test_get_raw_number() {
    md_cmd()
        .args(["get", "--raw", "-", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("42\n");
}

#[test]
fn test_get_raw_null_returns_empty() {
    md_cmd()
        .args(["get", "--raw", "-", "nonexistent"])
        .write_stdin("---\nnonexistent: null\n---\n# Doc")
        .assert()
        .success()
        .stdout("\n");
}

#[test]
fn test_get_raw_array_one_per_line() {
    md_cmd()
        .args(["get", "--raw", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("rust\ncli\n");
}

#[test]
fn test_get_raw_object_key_value_lines() {
    md_cmd()
        .args(["get", "--raw", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Hello World"))
        .stdout(predicate::str::contains("count: 42"));
}

// -- compact flag tests --

#[test]
fn test_get_compact_array() {
    md_cmd()
        .args(["get", "--compact", "-", "tags"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("[\"rust\",\"cli\"]\n");
}

#[test]
fn test_get_compact_object() {
    md_cmd()
        .args(["get", "--compact", "-", "title", "count"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout(predicate::str::contains("\"title\":\"Hello World\""))
        .stdout(predicate::str::contains("\"count\":42"));
}

#[test]
fn test_get_compact_scalar_unchanged() {
    md_cmd()
        .args(["get", "--compact", "-", "title"])
        .write_stdin(FM_DOC)
        .assert()
        .success()
        .stdout("\"Hello World\"\n");
}

// =============================================================================
//                          SET SUBCOMMAND TESTS
// =============================================================================

#[test]
fn test_set_string_value_via_stdin() {
    md_cmd()
        .args(["set", "-", "title", "New Title"])
        .write_stdin("---\ntitle: Old Title\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: New Title"))
        .stdout(predicate::str::contains("# Content"));
}

#[test]
fn test_set_adds_new_property_via_stdin() {
    md_cmd()
        .args(["set", "-", "author", "Alice"])
        .write_stdin("---\ntitle: Hello\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("author: Alice"))
        .stdout(predicate::str::contains("title: Hello"));
}

#[test]
fn test_set_numeric_value() {
    md_cmd()
        .args(["set", "-", "count", "42"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("count: 42"));
}

#[test]
fn test_set_boolean_value() {
    md_cmd()
        .args(["set", "-", "draft", "true"])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("draft: true"));
}

#[test]
fn test_set_json_array_value() {
    md_cmd()
        .args(["set", "-", "tags", r#"["rust","cli"]"#])
        .write_stdin("---\ntitle: Test\n---\n# Content\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("rust"))
        .stdout(predicate::str::contains("cli"));
}

#[test]
fn test_set_creates_frontmatter_when_none_exists() {
    md_cmd()
        .args(["set", "-", "title", "Brand New"])
        .write_stdin("# No Frontmatter\n\nJust content.\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Brand New"))
        .stdout(predicate::str::contains("# No Frontmatter"));
}

#[test]
fn test_set_updates_file_in_place() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated", "--save"])
        .assert()
        .success()
        .stdout(predicate::str::is_empty());

    let updated = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(updated.contains("title: Updated"));
    assert!(updated.contains("# Content"));
    assert!(!updated.contains("Original"));
}

#[test]
fn test_set_without_save_does_not_mutate_file() {
    let mut tmp = tempfile::NamedTempFile::new().unwrap();
    write!(tmp, "---\ntitle: Original\n---\n# Content\n").unwrap();

    md_cmd()
        .arg("set")
        .arg(tmp.path())
        .args(["title", "Updated"])
        .assert()
        .success()
        .stdout(predicate::str::contains("title: Updated"))
        .stdout(predicate::str::contains("# Content"));

    // File should be unchanged
    let on_disk = std::fs::read_to_string(tmp.path()).unwrap();
    assert!(on_disk.contains("title: Original"));
}

#[test]
fn test_set_preserves_body_content() {
    let input =
        "---\ntitle: Test\n---\n# Heading\n\nParagraph with **bold** text.\n\n- list item\n";
    md_cmd()
        .args(["set", "-", "version", "2"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("# Heading"))
        .stdout(predicate::str::contains("**bold**"))
        .stdout(predicate::str::contains("- list item"));
}

#[test]
fn test_get_requires_at_least_one_prop() {
    md_cmd()
        .args(["get", "-"])
        .write_stdin(FM_DOC)
        .assert()
        .failure();
}


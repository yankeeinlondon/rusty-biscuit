//! `md clean` rule precedence through the real binary: an explicit flag beats
//! the document's `style.markdown`, which beats the runtime default, and an
//! invalid `style.markdown` object is ignored as a whole.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;

/// A frontmatter block holding only `style.markdown` with `rules` (each line
/// indented four spaces).
fn rules(rules: &str) -> String {
    format!("---\nstyle:\n  markdown:\n{rules}---\n")
}

/// Cleans `source` from stdin with `args`, asserting success and an empty
/// stderr, and returns stdout.
fn clean(fixture: &CliProcessFixture, source: &str, args: &[&str]) -> String {
    let assert = fixture
        .command()
        .arg("clean")
        .args(args)
        .arg("-")
        .write_stdin(source)
        .assert()
        .success();
    let output = assert.get_output();
    assert_eq!(String::from_utf8_lossy(&output.stderr), "", "args: {args:?}");
    String::from_utf8(output.stdout.clone()).unwrap()
}

#[test]
fn document_rules_beat_the_defaults() {
    let fixture = CliProcessFixture::named("clean-rules-document-rules-beat-the-defaults");
    let body = "- a\n  - b\n\n```\nx\n```\n";

    assert_eq!(
        clean(&fixture, body, &[]),
        "- a\n    - b\n\n```text\nx\n```\n",
        "defaults: 4-space nesting and `text` fences"
    );

    let document = format!(
        "{}{body}",
        rules("    indentation: 2\n    default-language: rust\n")
    );
    let expected = format!(
        "{}- a\n  - b\n\n```rust\nx\n```\n",
        rules("    indentation: 2\n    default-language: rust\n")
    );
    assert_eq!(clean(&fixture, &document, &[]), expected);
}

#[test]
fn indent_flag_beats_the_document() {
    let fixture = CliProcessFixture::named("clean-rules-indent-flag-beats-the-document");
    let frontmatter = rules("    indentation: 2\n");
    let document = format!("{frontmatter}- a\n    - b\n");

    assert_eq!(
        clean(&fixture, &document, &[]),
        format!("{frontmatter}- a\n  - b\n")
    );
    assert_eq!(
        clean(&fixture, &document, &["--indent", "8"]),
        format!("{frontmatter}-    a\n        - b\n")
    );
}

#[test]
fn compact_and_loose_flags_beat_the_document() {
    let fixture = CliProcessFixture::named("clean-rules-compact-and-loose-flags-beat-the-document");
    let one = rules("    list-item-padding: one\n");
    let none = rules("    list-item-padding: none\n");

    assert_eq!(clean(&fixture, &format!("{one}- a\n- b\n"), &[]), format!("{one}- a\n\n- b\n"));
    assert_eq!(
        clean(&fixture, &format!("{one}- a\n- b\n"), &["--compact"]),
        format!("{one}- a\n- b\n")
    );
    assert_eq!(clean(&fixture, &format!("{none}- a\n\n- b\n"), &[]), format!("{none}- a\n- b\n"));
    assert_eq!(
        clean(&fixture, &format!("{none}- a\n\n- b\n"), &["--loose"]),
        format!("{none}- a\n\n- b\n")
    );
}

#[test]
fn compact_and_loose_match_document_list_item_padding() {
    let fixture =
        CliProcessFixture::named("clean-rules-compact-and-loose-match-document-list-item-padding");
    let body = "- a\n\n- b\n- c\n\n1. x\n2. y\n\n3. z\n";

    for (flag, value) in [("--compact", "none"), ("--loose", "one")] {
        let frontmatter = rules(&format!("    list-item-padding: {value}\n"));
        let from_document = clean(&fixture, &format!("{frontmatter}{body}"), &[]);
        let from_flag = clean(&fixture, body, &[flag]);
        assert_eq!(
            from_document.strip_prefix(&frontmatter),
            Some(from_flag.as_str()),
            "{flag} vs list-item-padding: {value}"
        );
        assert_ne!(from_flag, body, "{flag} changed the padding");
    }
}

#[test]
fn fixed_width_flag_beats_the_document_width() {
    let fixture = CliProcessFixture::named("clean-rules-fixed-width-flag-beats-the-document-width");
    let frontmatter = rules("    fixed-width: 20\n");
    let document = format!("{frontmatter}one two three four five six seven eight nine ten\n");

    assert_eq!(
        clean(&fixture, &document, &[]),
        format!("{frontmatter}one two three four\nfive six seven eight\nnine ten\n")
    );
    assert_eq!(
        clean(&fixture, &document, &["--fixed-width", "30"]),
        format!("{frontmatter}one two three four five six\nseven eight nine ten\n")
    );
}

#[test]
fn ignore_incidental_newlines_overrides_the_document_width_and_unwrap() {
    let fixture = CliProcessFixture::named(
        "clean-rules-ignore-incidental-newlines-overrides-the-document-width-and-unwrap",
    );
    let frontmatter = rules("    fixed-width: 20\n    unwrap: true\n");
    let document = format!(
        "{frontmatter}one two three four five six\nseven eight nine ten eleven twelve thirteen\n"
    );

    assert_ne!(clean(&fixture, &document, &[]), document, "the document width applies");
    assert_eq!(
        clean(&fixture, &document, &["--ignore-incidental-newlines"]),
        document
    );
}

/// `indentation: 3` invalidates the whole object, so the valid
/// `default-language: rust` beside it is ignored too; flags and defaults
/// still apply.
#[test]
fn invalid_document_rules_are_ignored_as_a_whole() {
    let fixture = CliProcessFixture::named("clean-rules-invalid-document-rules-are-ignored-as-a-whole");
    let frontmatter = rules("    indentation: 3\n    default-language: rust\n");
    let document = format!("{frontmatter}- a\n  - b\n\n```\nx\n```\n");

    let default = fixture
        .command()
        .args(["clean", "-"])
        .write_stdin(document.as_str())
        .assert()
        .success();
    assert_eq!(
        String::from_utf8_lossy(&default.get_output().stdout),
        format!("{frontmatter}- a\n    - b\n\n```text\nx\n```\n")
    );
    let stderr = String::from_utf8_lossy(&default.get_output().stderr);
    assert!(stderr.contains("markdown cleanup"), "{stderr}");
    assert!(stderr.contains("clean.invalid-rules"), "{stderr}");

    let flagged = fixture
        .command()
        .args(["clean", "--indent", "2", "-"])
        .write_stdin(document.as_str())
        .assert()
        .success();
    assert_eq!(
        String::from_utf8_lossy(&flagged.get_output().stdout),
        format!("{frontmatter}- a\n  - b\n\n```text\nx\n```\n")
    );

    let json = fixture
        .command()
        .args(["clean", "--json", "-"])
        .write_stdin(document.as_str())
        .assert()
        .success();
    assert!(json.get_output().stderr.is_empty(), "JSON mode keeps stderr empty");
    let report: Value = serde_json::from_slice(&json.get_output().stdout).unwrap();
    let finding = report["diagnostics"]
        .as_array()
        .unwrap()
        .iter()
        .find(|entry| entry["code"] == "clean.invalid-rules")
        .unwrap_or_else(|| panic!("no clean.invalid-rules finding: {report:#}"));
    let value_offset = document.find("3\n").unwrap();
    assert_eq!(finding["span"]["start"], value_offset);
    assert_eq!(finding["repairs"], Value::Array(vec![]));
    assert_eq!(report["applied"], Value::Array(vec![]));
    assert_eq!(report["changed"], Value::Bool(true));
}

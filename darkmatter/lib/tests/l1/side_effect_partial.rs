//! The shipped side-effect partial (`claudine/docs/schemas/partials/side-effect.yaml`)
//! after the `any(required)` migration: `set_frontmatter.value` is a
//! null-admitting union of every native value, and `merge_frontmatter` takes a
//! required object `obj` in a closed argument shape. Each test copies the real
//! partial into a temporary directory and validates documents that import its
//! `side_effect` type, one edit away from a passing control.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::DarkmatterSchemas;

use crate::request_support::context_at;

#[derive(Clone, Copy, Debug, PartialEq)]
enum Outcome {
    Valid,
    Invalid,
    /// The document's frontmatter does not load, or its schema does not resolve.
    LoadError,
}

const SCHEMA: &str = "$schema:\n  effect: side_effect@./side-effect.yaml\n";

/// A temporary directory holding `partial` as `side-effect.yaml` and the
/// `target.md` file the `file` arguments name.
fn fixture(partial: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("side-effect.yaml"), partial).expect("write partial");
    std::fs::write(root.join("target.md"), "---\ntitle: Target\n---\nBody\n").expect("write target");
    (dir, root)
}

/// Loads `doc.md` from disk the way `md schema validate` does and validates it.
fn outcome(dir: &Path, frontmatter: &str) -> Outcome {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n{SCHEMA}{frontmatter}---\nBody\n")).expect("write document");
    let Ok(markdown) = Markdown::try_from(path.as_path()) else {
        return Outcome::LoadError;
    };
    let markdown = markdown.with_source(ComposeSource::File(path));
    match DarkmatterSchemas::new(context_at(dir)).validate(&markdown) {
        Ok(result) if result.valid => Outcome::Valid,
        Ok(_) => Outcome::Invalid,
        Err(_) => Outcome::LoadError,
    }
}

fn assert_rows(dir: &Path, rows: &[(&str, &str, Outcome)]) {
    let failures: Vec<String> = rows
        .iter()
        .filter_map(|(label, frontmatter, expected)| {
            let actual = outcome(dir, frontmatter);
            (actual != *expected).then(|| format!("{label}: expected {expected:?}, got {actual:?}"))
        })
        .collect();
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn set_frontmatter_value_admits_null_omission_and_every_native_value() {
    let partial = include_str!("../../../../claudine/docs/schemas/partials/side-effect.yaml");
    let (_guard, dir) = fixture(partial);
    let set = |body: &str| format!("effect:\n  set_frontmatter:\n{body}");
    let with_value = |value: &str| set(&format!("    file: ./target.md\n    prop: title\n    value: {value}\n"));

    use Outcome::{Invalid, LoadError, Valid};
    let rows: Vec<(&str, String, Outcome)> = vec![
        ("control: string", with_value("hello"), Valid),
        ("value absent", set("    file: ./target.md\n    prop: title\n"), Valid),
        ("value null", with_value("null"), Valid),
        ("value number", with_value("42"), Valid),
        ("value float", with_value("1.5"), Valid),
        ("value boolean", with_value("false"), Valid),
        ("value object", with_value("{ a: 1, b: { c: null } }"), Valid),
        ("value array", with_value("[a, b]"), Valid),
        ("value mixed array", with_value("[1, a, true, null, { k: v }]"), Valid),
        ("value all-number array", with_value("[1, 2]"), Valid),
        ("value empty string", with_value("''"), Valid),
        ("value empty array", with_value("[]"), Valid),
        ("value empty object", with_value("{}"), Valid),
        ("file absent", set("    prop: title\n    value: hello\n"), Invalid),
        ("file null", set("    file: null\n    prop: title\n    value: hello\n"), Invalid),
        ("file wrong type", set("    file: [./target.md]\n    prop: title\n    value: hello\n"), Invalid),
        ("prop absent", set("    file: ./target.md\n    value: hello\n"), Invalid),
        ("prop null", set("    file: ./target.md\n    prop: null\n    value: hello\n"), Invalid),
        ("prop wrong type", set("    file: ./target.md\n    prop: { a: 1 }\n    value: hello\n"), Invalid),
        ("unexpected key", set("    file: ./target.md\n    prop: title\n    value: hello\n    obj: {}\n"), Invalid),
        (
            "duplicate value key",
            set("    file: ./target.md\n    prop: title\n    value: a\n    value: b\n"),
            LoadError,
        ),
        (
            "nested duplicate key",
            with_value("{ a: 1, a: 2 }"),
            LoadError,
        ),
        ("trailing invalid content", format!("{}  ]\n", with_value("hello")), LoadError),
    ];
    let rows: Vec<(&str, &str, Outcome)> =
        rows.iter().map(|(label, frontmatter, expected)| (*label, frontmatter.as_str(), *expected)).collect();
    assert_rows(&dir, &rows);
}

#[test]
fn merge_frontmatter_requires_an_object_and_has_no_prop_argument() {
    let partial = include_str!("../../../../claudine/docs/schemas/partials/side-effect.yaml");
    let (_guard, dir) = fixture(partial);
    let merge = |body: &str| format!("effect:\n  merge_frontmatter:\n{body}");
    let with_obj = |obj: &str| merge(&format!("    file: ./target.md\n    obj: {obj}\n"));

    use Outcome::{Invalid, LoadError, Valid};
    let rows: Vec<(&str, String, Outcome)> = vec![
        ("control: object without prop", with_obj("{ title: New, tags: [a, 1, null] }"), Valid),
        ("obj empty object", with_obj("{}"), Valid),
        ("obj nested values unconstrained", with_obj("{ a: { b: [1, { c: null }] } }"), Valid),
        ("obj absent", merge("    file: ./target.md\n"), Invalid),
        ("obj null", with_obj("null"), Invalid),
        ("obj string", with_obj("title"), Invalid),
        ("obj number", with_obj("3"), Invalid),
        ("obj boolean", with_obj("true"), Invalid),
        ("obj array", with_obj("[{ title: New }]"), Invalid),
        ("obj empty array", with_obj("[]"), Invalid),
        ("removed prop key", merge("    file: ./target.md\n    prop: title\n    obj: { title: New }\n"), Invalid),
        ("removed value key", merge("    file: ./target.md\n    value: New\n    obj: { title: New }\n"), Invalid),
        ("file absent", merge("    obj: { title: New }\n"), Invalid),
        ("file null", merge("    file: null\n    obj: { title: New }\n"), Invalid),
        (
            "duplicate obj key",
            merge("    file: ./target.md\n    obj: { a: 1 }\n    obj: { b: 2 }\n"),
            LoadError,
        ),
        ("nested duplicate key", with_obj("{ a: 1, a: 2 }"), LoadError),
        ("trailing invalid content", format!("{}  ]\n", with_obj("{ a: 1 }")), LoadError),
    ];
    let rows: Vec<(&str, &str, Outcome)> =
        rows.iter().map(|(label, frontmatter, expected)| (*label, frontmatter.as_str(), *expected)).collect();
    assert_rows(&dir, &rows);
}

//! `md schema validate` through the normal invocation path, on the files a
//! document's `$schema` may and may not target: a schema file carrying a
//! `description` or `categories` beside its `$schema` applies like any other, and a
//! `types`-only file is refused with the named-import hint.

use crate::common;

use common::CliProcessFixture;

/// Runs `md schema validate --format json` on `doc` from a repository at the
/// fixture's working directory, returning the exit code and the report.
fn validate(process: &CliProcessFixture, doc: &std::path::Path) -> (Option<i32>, serde_json::Value) {
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json"])
        .arg(doc)
        .output()
        .unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let report = serde_json::from_str(&stdout).unwrap_or_else(|error| panic!("{error}: stdout {stdout:?}"));
    (output.status.code(), report)
}

#[test]
fn a_described_schema_file_applies_its_schema() {
    let process = CliProcessFixture::new();
    assert!(process.initialize_repository(), "git init");
    process.write_file(
        "schemas/review.yaml",
        "description: Feature reviews\n$schema:\n  title: string(required)\n",
    );
    let valid = process.write_file("docs/valid.md", "---\n$schema: ../schemas/review.yaml\ntitle: Ship it\n---\nBody\n");
    let invalid = process.write_file("docs/invalid.md", "---\n$schema: ../schemas/review.yaml\n---\nBody\n");

    let (code, report) = validate(&process, &valid);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], true, "{report}");

    let (code, report) = validate(&process, &invalid);
    assert_eq!(code, Some(1), "the schema's `required` applies: {report}");
    assert!(report.to_string().contains("title"), "{report}");
}

/// An informal schema may declare `categories`, as a tagged one may.
#[test]
fn an_informal_schema_file_with_categories_applies_its_schema() {
    let process = CliProcessFixture::new();
    assert!(process.initialize_repository(), "git init");
    process.write_file(
        "schemas/subject.yaml",
        "categories: [Core]\n$schema:\n  title: string\ntypes:\n  Id: string\n",
    );
    let doc = process.write_file("docs/doc.md", "---\n$schema: ../schemas/subject.yaml\ntitle: Ship it\n---\nBody\n");

    let (code, report) = validate(&process, &doc);
    assert_eq!(code, Some(0), "{report}");
    assert_eq!(report["valid"], true, "{report}");
}

#[test]
fn a_types_only_schema_file_is_refused_with_the_import_hint() {
    let process = CliProcessFixture::new();
    assert!(process.initialize_repository(), "git init");
    let types = process.write_file("schemas/types.yaml", "kind: schema\ntypes:\n  title: string(required)\n");
    let doc = process.write_file("docs/doc.md", "---\n$schema: ../schemas/types.yaml\ntitle: Ship it\n---\nBody\n");

    let (code, report) = validate(&process, &doc);
    assert_eq!(code, Some(2), "{report}");
    let message = report["message"].as_str().expect("an error message");
    let display = biscuit_file::canonicalize_simplified(&types).unwrap().display().to_string();
    for fragment in [display.as_str(), "is not a valid schema definition", "use a named-type import (`Name@file.yaml`)"] {
        assert!(message.contains(fragment), "missing {fragment:?} in {message}");
    }
}

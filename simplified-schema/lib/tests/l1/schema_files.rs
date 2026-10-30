//! The `SimplifiedSchema` file factories (feature `file`): one argument, a
//! raw path or a context-bound source; one read; typed errors; source
//! relative resolution; supplied text for unsaved buffers.
//!
//! Every test binds its own temp-dir context, or moves the process into a
//! temp dir before a raw-path capture, so no capture walks this repository.

#![cfg(feature = "file")]

use std::path::{Path, PathBuf};

use biscuit_file::{FileReference, FileResolutionContext};
use serde_json::json;
use serial_test::serial;
use simplified_schema::{SchemaError, SchemaSource, SimplifiedSchema, SimplifiedSchemaError, ValidationFailure};

const PERSON_YAML: &str = "$schema:\n  name: string(required)\n  age: number(min(0); integer)\n";

fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
    let path = dir.join(name);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&path, text).unwrap();
    path
}

/// A source bound to a context anchored at `dir`.
fn bound(path: &Path, dir: &Path) -> SchemaSource {
    SchemaSource::from(path).bound(FileResolutionContext::new(dir))
}

fn canonical(path: &Path) -> PathBuf {
    biscuit_file::canonicalize_simplified(path).unwrap()
}

#[test]
fn yaml_and_markdown_files_with_the_same_payload_are_the_same_schema() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = write(dir.path(), "person.yaml", PERSON_YAML);
    let markdown = write(dir.path(), "person.md", &format!("---\ntitle: P\n{PERSON_YAML}---\n# Body\n\n$schema: ignored\n"));
    let from_yaml = SimplifiedSchema::try_from_yaml(bound(&yaml, dir.path())).unwrap();
    let from_markdown = SimplifiedSchema::try_from_markdown(bound(&markdown, dir.path())).unwrap();
    assert_eq!(from_yaml.to_json_schema().unwrap(), from_markdown.to_json_schema().unwrap());
    assert_eq!(from_yaml.property("age").unwrap().def(), from_markdown.property("age").unwrap().def());
    assert_eq!(from_yaml.provenance().path(), Some(yaml.as_path()));
    assert_eq!(from_markdown.provenance().path(), Some(markdown.as_path()));
    // And equal to the same text parsed as content.
    let content = SimplifiedSchema::try_from(PERSON_YAML).unwrap();
    assert_eq!(content.to_json_schema().unwrap(), from_yaml.to_json_schema().unwrap());
}

#[test]
fn crlf_files_parse_like_lf() {
    let dir = tempfile::tempdir().unwrap();
    let lf = write(dir.path(), "lf.yaml", PERSON_YAML);
    let crlf = write(dir.path(), "crlf.md", &format!("---\r\n{}---\r\nbody\r\n", PERSON_YAML.replace('\n', "\r\n")));
    let lf = SimplifiedSchema::try_from_yaml(bound(&lf, dir.path())).unwrap();
    let crlf = SimplifiedSchema::try_from_markdown(bound(&crlf, dir.path())).unwrap();
    assert_eq!(lf.to_json_schema().unwrap(), crlf.to_json_schema().unwrap());
}

#[test]
fn factory_errors_are_typed() {
    let dir = tempfile::tempdir().unwrap();
    let missing = dir.path().join("missing.yaml");
    assert!(matches!(
        SimplifiedSchema::try_from_yaml(bound(&missing, dir.path())),
        Err(SimplifiedSchemaError::MissingFile { path }) if path == missing
    ));
    let reference = FileReference::new("./missing.yaml").unwrap();
    assert!(matches!(
        SimplifiedSchema::try_from_yaml(SchemaSource::from(&reference).bound(FileResolutionContext::new(dir.path()))),
        Err(SimplifiedSchemaError::MissingFile { .. })
    ));
    let plain = write(dir.path(), "plain.md", "# No frontmatter\n");
    assert!(matches!(
        SimplifiedSchema::try_from_markdown(bound(&plain, dir.path())),
        Err(SimplifiedSchemaError::MissingSchema { reason, .. }) if reason.contains("no frontmatter")
    ));
    let no_schema = write(dir.path(), "no-schema.md", "---\ntitle: x\n---\n");
    assert!(matches!(
        SimplifiedSchema::try_from_markdown(bound(&no_schema, dir.path())),
        Err(SimplifiedSchemaError::MissingSchema { .. })
    ));
    let ordinary = write(dir.path(), "ordinary.yaml", "title: x\n");
    assert!(matches!(
        SimplifiedSchema::try_from_yaml(bound(&ordinary, dir.path())),
        Err(SimplifiedSchemaError::MissingSchema { source_name, .. }) if source_name.contains("ordinary.yaml")
    ));
    let malformed = write(dir.path(), "malformed.yaml", "$schema:\n  age: 'number(min('\n");
    assert!(matches!(
        SimplifiedSchema::try_from_yaml(bound(&malformed, dir.path())),
        Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { path, .. })) if path == malformed
    ));
    let duplicate = write(dir.path(), "duplicate.yaml", "$schema:\n  a: string\n  a: number\n");
    assert!(matches!(SimplifiedSchema::try_from_yaml(bound(&duplicate, dir.path())), Err(SimplifiedSchemaError::Schema(_))));
}

/// The raw JSON Schema loader: control, then one edit per document shape.
#[test]
fn json_schema_files_go_through_the_resolvers_json_loader() {
    let dir = tempfile::tempdir().unwrap();
    let control = r#"{"type": "object", "properties": {"count": {"type": "number"}}}"#;
    let path = write(dir.path(), "count.json", control);
    let schema = SimplifiedSchema::try_from_json_schema(bound(&path, dir.path())).unwrap();
    assert!(schema.validate_strict(&json!({ "count": 2 })).is_ok());
    assert!(schema.validate_strict(&json!({ "count": "2" })).is_err());
    assert!(matches!(schema.to_simplified_yaml(), Err(SimplifiedSchemaError::ExportUnsupported { .. })));
    for (name, text) in [
        ("trailing.json", format!("{control} junk")),
        ("duplicate.json", r#"{"type": "object", "type": "string"}"#.to_string()),
        ("array.json", "[1, 2]".to_string()),
    ] {
        let path = write(dir.path(), name, &text);
        assert!(
            matches!(SimplifiedSchema::try_from_json_schema(bound(&path, dir.path())), Err(SimplifiedSchemaError::Schema(_))),
            "{name}"
        );
    }
}

#[test]
fn imports_and_file_values_resolve_from_the_schema_files_directory() {
    let dir = tempfile::tempdir().unwrap();
    let people = write(dir.path(), "schemas/people.yaml", "$schema:\n  Person:\n    name: string(required)\n");
    let main = write(dir.path(), "schemas/main.yaml", "$schema:\n  owner: Person@./people.yaml\n  notes: file(eager)\n");
    write(dir.path(), "schemas/notes.md", "notes\n");
    // The context is anchored at the root, not the schema's folder.
    let schema = SimplifiedSchema::try_from_yaml(bound(&main, dir.path())).unwrap();
    assert_eq!(schema.provenance().imports(), [canonical(&people)]);
    assert_eq!(schema.query("owner.name").unwrap().unwrap().type_text(), "string(required)");
    assert!(schema.validate_strict(&json!({ "owner": { "name": "Ada" }, "notes": "./notes.md" })).is_ok());
    match schema.validate_strict(&json!({ "owner": { "name": "Ada" }, "notes": "./absent.md" })) {
        Err(ValidationFailure::InvalidData { problems }) => assert_eq!(problems[0].path.as_str(), "/notes"),
        other => panic!("a missing file is invalid data with a file context, got {other:?}"),
    }
}

#[test]
fn a_bound_source_with_supplied_text_reads_no_file() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "people.yaml", "$schema:\n  Person:\n    name: string\n");
    // An unsaved buffer: the file does not exist.
    let draft = dir.path().join("draft.md");
    let text = "---\n$schema:\n  owner: Person@./people.yaml\n---\nunsaved body\n";
    let schema = {
        let source = bound(&draft, dir.path()).with_text(text);
        SimplifiedSchema::try_from_markdown(source).unwrap()
    };
    assert!(!draft.exists());
    assert_eq!(schema.provenance().path(), Some(draft.as_path()));
    assert_eq!(schema.query("owner.name").unwrap().unwrap().type_text(), "string");
    assert!(schema.validate_strict(&json!({ "owner": { "name": "x" } })).is_ok());
}

#[test]
fn remote_references_are_unsupported() {
    for reference in ["https://example.com/schema.yaml", "http://example.com/s.md"] {
        assert!(matches!(
            SimplifiedSchema::try_from_yaml(reference),
            Err(SimplifiedSchemaError::Schema(SchemaError::RemoteUnsupported { .. }))
        ));
        assert!(matches!(
            block_on(SimplifiedSchema::resolve(reference)),
            Err(SimplifiedSchemaError::Schema(SchemaError::RemoteUnsupported { .. }))
        ));
    }
}

#[test]
fn resolve_selects_the_loader_by_extension() {
    let dir = tempfile::tempdir().unwrap();
    let yaml = write(dir.path(), "p.yaml", PERSON_YAML);
    let markdown = write(dir.path(), "p.md", &format!("---\n{PERSON_YAML}---\n"));
    let json = write(dir.path(), "p.json", r#"{"type": "object", "properties": {"name": {"type": "string"}}}"#);
    let from_yaml = block_on(SimplifiedSchema::resolve(bound(&yaml, dir.path()))).unwrap();
    let from_markdown = block_on(SimplifiedSchema::resolve(bound(&markdown, dir.path()))).unwrap();
    assert_eq!(from_yaml.to_json_schema().unwrap(), from_markdown.to_json_schema().unwrap());
    let from_json = block_on(SimplifiedSchema::resolve(bound(&json, dir.path()))).unwrap();
    assert!(from_json.property("name").is_none(), "raw JSON Schema has no typed view");
    assert!(from_json.validate_strict(&json!({ "name": 3 })).is_err());
}

/// A raw path captures the process directory: relative paths and references
/// resolve from it.
#[test]
#[serial]
fn a_raw_relative_path_resolves_from_the_process_directory() {
    let dir = tempfile::tempdir().unwrap();
    write(dir.path(), "person.yaml", PERSON_YAML);
    let previous = std::env::current_dir().unwrap();
    std::env::set_current_dir(dir.path()).unwrap();
    let by_path = SimplifiedSchema::try_from_yaml(Path::new("person.yaml"));
    let by_reference = SimplifiedSchema::try_from_yaml("./person.yaml");
    std::env::set_current_dir(previous).unwrap();
    let (by_path, by_reference) = (by_path.unwrap(), by_reference.unwrap());
    assert_eq!(by_path.to_json_schema().unwrap(), by_reference.to_json_schema().unwrap());
    assert!(by_path.validate_strict(&json!({ "name": "Ada" })).is_ok());
}

/// Polls a future that never suspends; `SimplifiedSchema::resolve` does all
/// its work in its first poll.
fn block_on<F: std::future::Future>(future: F) -> F::Output {
    let mut future = std::pin::pin!(future);
    let mut context = std::task::Context::from_waker(std::task::Waker::noop());
    match future.as_mut().poll(&mut context) {
        std::task::Poll::Ready(output) => output,
        std::task::Poll::Pending => panic!("schema resolution does not suspend"),
    }
}

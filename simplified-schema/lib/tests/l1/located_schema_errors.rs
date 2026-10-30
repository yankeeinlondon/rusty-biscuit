//! A schema document whose definition the grammar rejects reports the
//! document, the parser's typed error, and the rejected text's byte range in
//! the exact text supplied (LF and CRLF alike), whichever factory read it.

use std::ops::Range;
use std::path::Path;

use simplified_schema::{SchemaError, SimplifiedSchema, SimplifiedSchemaError, parse_standalone_schema_document};

/// The malformed definition every case carries, as authored.
const MALFORMED: &str = "'number(min('";

fn crlf(text: &str) -> String {
    text.replace('\n', "\r\n")
}

/// The located `SchemaDocument` inside a factory error.
fn document_error(result: Result<SimplifiedSchema, SimplifiedSchemaError>) -> SchemaError {
    match result {
        Err(SimplifiedSchemaError::Schema(error)) => error,
        other => panic!("expected a schema error, got {other:?}"),
    }
}

/// Asserts `error` names `path`, keeps the grammar's own `age` failure as its
/// source, and locates [`MALFORMED`] at its only position in `text`.
fn assert_located(error: &SchemaError, path: &Path, text: &str) {
    let SchemaError::SchemaDocument {
        path: actual,
        span: Some(span),
        source: Some(cause),
        ..
    } = error
    else {
        panic!("expected a located schema-document error, got {error:?}");
    };
    assert_eq!(actual, path);
    let SchemaError::Grammar { property, span: grammar_span, .. } = cause.as_ref() else {
        panic!("expected the grammar's own error as the cause, got {cause:?}");
    };
    assert_eq!(property, "age");
    // The grammar's span stays relative to the type string it parsed.
    assert_eq!(*grammar_span, 11..11);
    let start = text.find(MALFORMED).expect("fixture holds the malformed type");
    assert_eq!(text.rfind(MALFORMED), Some(start), "fixture holds it once");
    let expected: Range<usize> = start..start + MALFORMED.len();
    assert_eq!(*span, expected, "{text:?}");
    assert_eq!(&text[span.clone()], MALFORMED);
}

const ROOT_YAML: &str = "$schema:\n  name: string\n  age: 'number(min('\n";
const TYPES_YAML: &str = "$schema:\n  person: Person\ntypes:\n  Person:\n    name: string\n    age: 'number(min('\n";
const MARKDOWN: &str = "---\ntitle: test\n$schema:\n  name: string\n  age: 'number(min('\n---\nbody\n";

#[test]
fn content_yaml_root_payload_failure_is_located_in_the_supplied_text() {
    for text in [ROOT_YAML.to_string(), crlf(ROOT_YAML)] {
        let error = document_error(SimplifiedSchema::try_from(text.as_str()));
        assert_located(&error, Path::new("content"), &text);
    }
}

#[test]
fn content_yaml_types_table_failure_is_located_in_the_supplied_text() {
    for text in [TYPES_YAML.to_string(), crlf(TYPES_YAML)] {
        let error = document_error(SimplifiedSchema::try_from(text.as_str()));
        assert_located(&error, Path::new("content"), &text);
        assert!(error.to_string().contains("invalid named types"), "{error}");
    }
}

#[test]
fn content_markdown_failure_is_located_in_the_whole_document() {
    for text in [MARKDOWN.to_string(), crlf(MARKDOWN)] {
        let error = document_error(SimplifiedSchema::try_from(text.as_str()));
        assert_located(&error, Path::new("content"), &text);
    }
}

/// The low-level standalone parser keeps its message and gains the cause.
#[test]
fn standalone_parser_keeps_its_message_and_adds_the_typed_cause() {
    for text in [ROOT_YAML.to_string(), crlf(ROOT_YAML)] {
        let error = parse_standalone_schema_document(&text, "/w/schema.yaml").expect_err("malformed");
        assert_located(&error, Path::new("/w/schema.yaml"), &text);
        assert_eq!(
            error.to_string(),
            "invalid standalone schema document `/w/schema.yaml`: invalid SimplifiedSchema payload: \
             invalid SimplifiedSchema for property `age`: expected argument, found `Eof`"
        );
    }
}

/// The lower-level parser takes a type string, so its span stays relative
/// to that string: the located errors above wrap it unchanged.
#[test]
fn direct_definition_parser_keeps_its_type_string_span() {
    let value = serde_yaml_ng::Value::String("number(min(".into());
    let error = simplified_schema::parse_property_definition("age", &value).expect_err("malformed");
    assert!(matches!(error, SchemaError::Grammar { ref property, ref span, .. } if property == "age" && *span == (11..11)));
}

#[cfg(feature = "file")]
mod files {
    use std::path::{Path, PathBuf};

    use biscuit_file::FileResolutionContext;
    use simplified_schema::{SchemaError, SchemaSource, SimplifiedSchema};

    use super::{MARKDOWN, ROOT_YAML, TYPES_YAML, assert_located, crlf, document_error};

    fn write(dir: &Path, name: &str, text: &str) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, text).unwrap();
        path
    }

    fn bound(path: &Path, dir: &Path) -> SchemaSource {
        SchemaSource::from(path).bound(FileResolutionContext::new(dir))
    }

    /// The path a factory reports: the file it resolved and read.
    fn reported(error: &SchemaError) -> PathBuf {
        match error {
            SchemaError::SchemaDocument { path, .. } => path.clone(),
            other => panic!("expected a schema-document error, got {other:?}"),
        }
    }

    fn assert_file_located(error: &SchemaError, file: &Path, text: &str) {
        let path = reported(error);
        assert_eq!(
            biscuit_file::canonicalize_simplified(&path).unwrap(),
            biscuit_file::canonicalize_simplified(file).unwrap()
        );
        assert_located(error, &path, text);
    }

    #[test]
    fn yaml_file_factory_locates_root_and_types_failures() {
        let dir = tempfile::tempdir().unwrap();
        for (name, fixture) in [("root", ROOT_YAML), ("types", TYPES_YAML)] {
            for (ending, text) in [("lf", fixture.to_string()), ("crlf", crlf(fixture))] {
                let file = write(dir.path(), &format!("{name}-{ending}.yaml"), &text);
                let error = document_error(SimplifiedSchema::try_from_yaml(bound(&file, dir.path())));
                assert_file_located(&error, &file, &text);
            }
        }
    }

    #[test]
    fn markdown_file_factory_reports_the_file_and_document_span() {
        let dir = tempfile::tempdir().unwrap();
        for (ending, text) in [("lf", MARKDOWN.to_string()), ("crlf", crlf(MARKDOWN))] {
            let file = write(dir.path(), &format!("doc-{ending}.md"), &text);
            let error = document_error(SimplifiedSchema::try_from_markdown(bound(&file, dir.path())));
            assert_file_located(&error, &file, &text);
        }
    }

    #[test]
    fn async_resolve_reports_the_same_located_failure() {
        let dir = tempfile::tempdir().unwrap();
        let text = crlf(ROOT_YAML);
        let file = write(dir.path(), "schema.yaml", &text);
        let error = document_error(block_on(SimplifiedSchema::resolve(bound(&file, dir.path()))));
        assert_file_located(&error, &file, &text);
    }

    #[test]
    fn imported_named_type_failure_is_located_in_the_imported_file() {
        let dir = tempfile::tempdir().unwrap();
        for (ending, text) in [("lf", TYPES_YAML.to_string()), ("crlf", crlf(TYPES_YAML))] {
            let types = write(dir.path(), &format!("types-{ending}.yaml"), &text);
            let main = write(
                dir.path(),
                &format!("main-{ending}.yaml"),
                &format!("$schema:\n  person: Person@./types-{ending}.yaml\n"),
            );
            let error = document_error(SimplifiedSchema::try_from_yaml(bound(&main, dir.path())));
            assert_file_located(&error, &types, &text);
        }
    }

    fn block_on<F: std::future::Future>(future: F) -> F::Output {
        let mut future = std::pin::pin!(future);
        let mut context = std::task::Context::from_waker(std::task::Waker::noop());
        match future.as_mut().poll(&mut context) {
            std::task::Poll::Ready(output) => output,
            std::task::Poll::Pending => panic!("schema resolution does not suspend"),
        }
    }
}

/// A frontmatter that is not YAML: the parser counts lines from the
/// frontmatter's first line, and the span is that position in the document.
#[test]
fn markdown_frontmatter_yaml_failure_points_into_the_whole_document() {
    const NOT_YAML: &str = "---\ntitle: test\n$schema: @bad\n---\nbody\n";
    for text in [NOT_YAML.to_string(), crlf(NOT_YAML)] {
        let error = document_error(SimplifiedSchema::try_from(text.as_str()));
        let SchemaError::SchemaDocument { path, span: Some(span), .. } = &error else {
            panic!("expected a located schema-document error, got {error:?}");
        };
        assert_eq!(path, Path::new("content"));
        let at = text.find("@bad").expect("fixture holds the token");
        assert_eq!(*span, at..at, "{text:?}: {error}");
    }
}

//! The input-shape matrix for the `x-darkmatter-match` JSON Schema keyword,
//! judged through both public validator builders.
//!
//! Every row edits one cell of one base schema: the value of
//! `properties.spec.x-darkmatter-match`. The control `["**/docs/*.md"]`
//! accepts `docs/a.md` and rejects `docs/a.txt` in both modes, so a row that
//! builds and accepts both files has stopped constraining the value.
//!
//! | Shape                       | Request-aware and structural validators      |
//! | --------------------------- | -------------------------------------------- |
//! | control                     | Markdown passes; text fails                  |
//! | absent                      | both pass: the keyword is optional           |
//! | explicit null               | build error: must be an array of strings     |
//! | wrong type, whole field     | build error: must be an array of strings     |
//! | wrong type, one element     | build error; no element is dropped           |
//! | wrong type, every element   | build error: must be an array of strings     |
//! | empty array                 | build error: needs a positive pattern        |
//! | invalid glob string         | build error naming the invalid glob          |
//!
//! Two rows of the general matrix have no cell here. The keyword reader
//! receives a parsed `serde_json::Value`, so a duplicate key has already been
//! collapsed by whichever loader produced it, and source text (a second
//! document, invalid trailing content) belongs to that loader. Those rows are
//! covered where text is read: `schema_roots::path_field_input_matrix`.

use std::path::Path;

use darkmatter::markdown::schemas::{SchemaError, ValidatorCache};
use serde_json::{Value, json};
use tempfile::TempDir;

use crate::request_support::context_at;

#[derive(Debug)]
enum Outcome {
    /// Builds; the Markdown file passes and the text file fails.
    Constrains,
    /// Builds; both files pass.
    Unconstrained,
    /// Validator construction fails with a message containing every fragment.
    BuildError(&'static [&'static str]),
}

const ARRAY_OF_STRINGS: &[&str] = &["x-darkmatter-match must be an array of strings"];

/// `None` removes the keyword; `Some(value)` replaces its value.
fn rows() -> Vec<(&'static str, Option<Value>, Outcome)> {
    vec![
        ("control", Some(json!(["**/docs/*.md"])), Outcome::Constrains),
        ("absent", None, Outcome::Unconstrained),
        ("explicit null", Some(Value::Null), Outcome::BuildError(ARRAY_OF_STRINGS)),
        ("wrong type, whole field", Some(json!(123)), Outcome::BuildError(ARRAY_OF_STRINGS)),
        (
            "wrong type, one element",
            Some(json!(["**/docs/*.md", 123])),
            Outcome::BuildError(ARRAY_OF_STRINGS),
        ),
        ("wrong type, every element", Some(json!([123])), Outcome::BuildError(ARRAY_OF_STRINGS)),
        (
            "empty array",
            Some(json!([])),
            Outcome::BuildError(&["invalid glob", "at least one pattern that is not a `!` exclusion"]),
        ),
        (
            "invalid glob string",
            Some(json!(["**/docs/[x"])),
            Outcome::BuildError(&["invalid glob", "`**/docs/[x`", "not a valid glob"]),
        ),
    ]
}

fn schema_with(cell: Option<Value>) -> Value {
    let mut spec = json!({ "type": "string" });
    if let Some(value) = cell {
        spec["x-darkmatter-match"] = value;
    }
    json!({ "type": "object", "properties": { "spec": spec } })
}

struct Fixture {
    dir: TempDir,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().unwrap();
        std::fs::create_dir_all(dir.path().join("docs")).unwrap();
        std::fs::write(dir.path().join("docs/a.md"), "# A\n").unwrap();
        std::fs::write(dir.path().join("docs/a.txt"), "a\n").unwrap();
        Self { dir }
    }

    fn root(&self) -> &Path {
        self.dir.path()
    }

    fn absolute(&self, relative: &str) -> String {
        biscuit_file::to_portable_string(&self.root().join(relative))
    }
}

/// Walks every row through one builder; `markdown` and `text` spell the two
/// fixture files the way that builder can judge them.
fn walk(
    mode: &str,
    build: impl Fn(&Value) -> Result<std::sync::Arc<jsonschema::Validator>, SchemaError>,
    markdown: &str,
    text: &str,
) {
    for (shape, cell, expected) in rows() {
        let built = build(&schema_with(cell));
        let label = format!("{mode}, {shape}");
        match expected {
            Outcome::Constrains | Outcome::Unconstrained => {
                let validator = built.unwrap_or_else(|error| panic!("{label}: {error}"));
                assert!(validator.is_valid(&json!({ "spec": markdown })), "{label}: Markdown file");
                let text_passes = validator.is_valid(&json!({ "spec": text }));
                match expected {
                    Outcome::Constrains => assert!(!text_passes, "{label}: text file must fail"),
                    _ => assert!(text_passes, "{label}: no constraint, so the text file passes"),
                }
            }
            Outcome::BuildError(fragments) => {
                let error = built.err().unwrap_or_else(|| panic!("{label}: must not build"));
                let SchemaError::BuildValidator { message } = &error else {
                    panic!("{label}: expected a build error, got {error:?}");
                };
                for fragment in fragments {
                    assert!(message.contains(fragment), "{label}: `{fragment}` not in {message:?}");
                }
            }
        }
    }
}

#[test]
fn match_keyword_input_matrix_request_aware() {
    let fixture = Fixture::new();
    let context = context_at(fixture.root());
    walk(
        "request-aware",
        |schema| ValidatorCache::with_capacity(1).validator_for(schema, Some(fixture.root()), &context),
        "./docs/a.md",
        "./docs/a.txt",
    );
}

#[test]
fn match_keyword_input_matrix_structural() {
    let fixture = Fixture::new();
    walk(
        "structural",
        |schema| ValidatorCache::with_capacity(1).structural_validator_for(schema),
        &fixture.absolute("docs/a.md"),
        &fixture.absolute("docs/a.txt"),
    );
}

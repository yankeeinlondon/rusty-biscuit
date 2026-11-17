//! Document loaders through the library's public entry points, one edit of a
//! control text per cell.
//!
//! - Admitted YAML (`numeric::load_admitted_yaml`), reached through the
//!   whole-YAML loader `LoadedYamlDocument::load` and the content factory
//!   `SimplifiedSchema::try_from`. Duplicate keys, malformed text, a second
//!   document, and document markers are covered through the Darkmatter host
//!   (`darkmatter/lib/tests/l1/yaml_document_loading.rs`); the cells here are
//!   the ones no other suite proves.
//! - Raw JSON Schema text, through the JSON file factory
//!   (`try_from_json_schema`, the resolver's raw loader) and `$schema`
//!   resolution of a `.json` target (its classifier). Valid, duplicate-member,
//!   trailing-content, and JSON5 cells are in `schema_files.rs` and
//!   `schema_target_classification.rs`; numeric admission and the loaded
//!   defaults are here.
//!
//! F1: in-memory text, or a temporary directory when a path is needed.

use std::path::Path;

use serde_json::{Value, json};
use simplified_schema::yaml_documents::LoadedYamlDocument;
use simplified_schema::{SchemaError, SimplifiedSchema};

// ── Admitted YAML ─────────────────────────────────────────────────────────

/// A valid whole-YAML document.
const CONTROL: &str = "name: api\nport: 8080\n";

#[track_caller]
fn loaded(text: &str) -> Value {
    LoadedYamlDocument::load(text, Path::new("doc.yaml"))
        .unwrap_or_else(|error| panic!("{text:?}: {error}"))
        .json()
        .clone()
}

/// The load error's message and the original text from its span to the end
/// of that line.
#[track_caller]
fn load_error(text: &str) -> (String, String) {
    match LoadedYamlDocument::load(text, Path::new("doc.yaml")) {
        Err(SchemaError::SchemaDocument { message, span: Some(span), path, .. }) => {
            assert_eq!(path, Path::new("doc.yaml"), "{message}");
            let rest = &text[span.start..];
            (message, rest.split(['\r', '\n']).next().unwrap_or_default().to_string())
        }
        other => panic!("expected a located load error for {text:?}, got {other:?}"),
    }
}

#[test]
fn an_unset_value_loads_as_null_in_every_spelling() {
    let bare = loaded("name: api\nport:\n");
    for text in ["name: api\nport: null\n", "name: api\nport: ~\n", "name: api\nport: Null\n"] {
        assert_eq!(loaded(text), bare, "{text:?}");
    }
    assert_eq!(bare, json!({ "name": "api", "port": null }), "the key is kept, its value is null");
    assert_eq!(loaded(CONTROL), json!({ "name": "api", "port": 8080 }));
}

#[test]
fn trailing_content_is_a_located_load_error() {
    for (text, message, at) in [
        (format!("{CONTROL}]\n"), "did not find expected key", "]"),
        (format!("{CONTROL}...\nport: 1\n"), "more than one document", "port: 1"),
        (format!("{CONTROL}... trailing\n"), "more than one document", "trailing"),
    ] {
        let (found, token) = load_error(&text);
        assert!(found.starts_with("not valid YAML") && found.contains(message), "{text:?}: {found}");
        assert_eq!(token, at, "{text:?}: {found}");
    }
}

/// A load error's span is a byte offset into the caller's text, past a
/// byte-order mark and across CRLF line ends.
#[test]
fn a_load_error_after_a_byte_order_mark_and_crlf_lines_points_into_the_original_text() {
    for (label, prefix, line_end) in [("LF", "", "\n"), ("BOM", "\u{feff}", "\n"), ("CRLF", "", "\r\n"), ("BOM + CRLF", "\u{feff}", "\r\n")] {
        let duplicate = format!("{prefix}name: api{line_end}port: 1{line_end}nested:{line_end}  a: 1{line_end}  a: 2{line_end}");
        let (message, token) = load_error(&duplicate);
        assert!(message.contains("duplicate entry with key \"a\""), "{label}: {message}");
        assert_eq!(token, "a: 1", "{label}: located at the mapping holding the key");

        let malformed = format!("{prefix}name: api{line_end}port: [1{line_end}");
        let (message, _) = load_error(&malformed);
        assert!(message.starts_with("not valid YAML"), "{label}: {message}");
        let number = format!("{prefix}name: api{line_end}port: 9007199254740993{line_end}");
        let (message, token) = load_error(&number);
        assert!(message.contains("quote it"), "{label}: {message}");
        assert_eq!(token, "9007199254740993", "{label}");
    }
}

// ── Raw JSON Schema text ──────────────────────────────────────────────────

/// A raw JSON Schema whose optional `n` defaults to `3`.
const RAW: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": { "n": { "type": "number", "default": 3 } }
}
"#;

/// The content factory reads JSON text as YAML, so its numbers go through
/// the admitted-YAML loader.
#[test]
fn the_content_factory_admits_raw_json_numbers() {
    let schema = SimplifiedSchema::try_from(RAW).expect("control");
    let input = json!({ "n": null });
    let accepted = schema.validate_soft(&input).expect("defaulted");
    assert_eq!(accepted.value.as_ref(), &json!({ "n": 3 }));

    for spelling in ["9007199254740993", "9007199254740993.0"] {
        let text = RAW.replace("\"default\": 3", &format!("\"default\": {spelling}"));
        let refused = SimplifiedSchema::try_from(text.as_str()).expect_err(spelling).to_string();
        assert!(refused.contains("cannot be represented exactly") && refused.contains("properties.n.default"), "{spelling}: {refused}");
    }
}

#[cfg(feature = "file")]
mod json_files {
    use std::sync::Arc;

    use biscuit_file::{FileResolutionContext, canonicalize_simplified};
    use simplified_schema::embedded::ShippedSources;
    use simplified_schema::resolve::resolve_schema_with_roots;
    use simplified_schema::{AppliedDefault, Prepared, SchemaSource, SimplifiedSchemaError};

    use super::*;

    struct Fixture {
        _tmp: tempfile::TempDir,
        dir: std::path::PathBuf,
    }

    impl Fixture {
        fn new() -> Self {
            let tmp = tempfile::tempdir().unwrap();
            let dir = canonicalize_simplified(tmp.path()).unwrap();
            Self { _tmp: tmp, dir }
        }

        fn context(&self) -> FileResolutionContext {
            FileResolutionContext::new(&self.dir)
        }

        /// The JSON file factory over `text`, which it reads instead of the
        /// file.
        fn factory(&self, text: &str) -> Result<SimplifiedSchema, SimplifiedSchemaError> {
            let source = SchemaSource::from(self.dir.join("schema.json").as_path()).bound(self.context()).with_text(text);
            SimplifiedSchema::try_from_json_schema(source)
        }

        /// `$schema: ./target.json` resolved from a document beside it.
        fn resolved(&self, text: &str) -> Result<Value, SchemaError> {
            std::fs::write(self.dir.join("target.json"), text).unwrap();
            resolve_schema_with_roots(&json!("./target.json"), &self.dir, &[], &self.context(), ShippedSources::NONE)
                .map(|resolved| resolved.json_schema)
        }
    }

    fn n_defaulted(value: &Value) -> Vec<AppliedDefault> {
        vec![AppliedDefault {
            path: simplified_schema::PropertyPath::parse("n").unwrap(),
            value: value.clone(),
            pointer: "/properties/n/default".to_string(),
            origin: None,
        }]
    }

    #[test]
    fn both_json_loaders_admit_numbers_from_their_spelling() {
        let fixture = Fixture::new();
        for (spelling, reason) in [
            ("9007199254740993", "cannot be represented exactly"),
            ("9007199254740993.0", "cannot be represented exactly"),
            ("1e400", "out of range"),
        ] {
            let text = RAW.replace("\"default\": 3", &format!("\"default\": {spelling}"));
            let factory = fixture.factory(&text).err().unwrap_or_else(|| panic!("{spelling}: loaded")).to_string();
            assert!(factory.contains(reason), "{spelling}: {factory}");
            let resolved = fixture.resolved(&text).expect_err(spelling).to_string();
            assert!(resolved.contains(reason), "{spelling}: {resolved}");
        }
    }

    /// A loaded raw schema's defaults apply through the factory's schema and
    /// through `Prepared` over the resolved JSON alike, as written (`3.0`
    /// stays `3.0`).
    #[test]
    fn loaded_raw_defaults_fill_missing_and_null_values() {
        let fixture = Fixture::new();
        for (text, default) in [(RAW.to_string(), json!(3)), (RAW.replace("\"default\": 3", "\"default\": 3.0"), json!(3.0))] {
            let schema = fixture.factory(&text).expect("loads");
            let prepared = Prepared::new(Arc::new(fixture.resolved(&text).expect("resolves")), None, fixture.context())
                .expect("prepares");
            for input in [json!({}), json!({ "n": null })] {
                let strict = schema.validate_strict(&input).expect("strict");
                assert_eq!(strict.defaults, n_defaulted(&default), "{input}");
                assert_eq!(strict.value.as_ref(), &input, "strict returns the input as written");
                let soft = prepared.validate_soft(&input).expect("soft");
                assert_eq!(soft.value.as_ref(), &json!({ "n": default }), "{input}");
                assert_eq!(soft.defaults, n_defaulted(&default), "{input}");
            }
        }
    }
}

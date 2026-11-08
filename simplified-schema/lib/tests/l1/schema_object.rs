//! The `SimplifiedSchema` object through its public API: content parsing
//! (`TryFrom<&str>`), validation without a compile step, layering, type
//! queries and views, and export with its parse-export-parse round trip.
//!
//! The file factories (feature `file`) are in `schema_files.rs`; the
//! context-capture and read-count instrumentation is a private test seam,
//! tested inside the crate (`src/schema/tests.rs`).

use std::borrow::Cow;

use serde_json::{Value, json};
use simplified_schema::completion::{AlternativeSite, PathStep};
use simplified_schema::simplified::{PropertyDef, TypeExpr};
use simplified_schema::{SchemaError, SimplifiedSchema, SimplifiedSchemaError, ValidationFailure, ValidationSuccess};

/// The spec's `Person` schema, as a standalone `$schema` document.
const PERSON_YAML: &str = "$schema:\n  name: string(required)\n  age: number(min(0); integer)\n";

fn schema(text: &str) -> SimplifiedSchema {
    SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("schema parses: {error}\n{text}"))
}

#[track_caller]
fn borrowed<'a>(result: &'a Result<ValidationSuccess<'_>, ValidationFailure>, input: &Value) -> &'a Value {
    match result {
        Ok(ValidationSuccess { value: Cow::Borrowed(value), .. }) => {
            assert!(std::ptr::eq(*value, input), "the accepted value is the caller's own input");
            value
        }
        other => panic!("expected the input accepted as written, got {other:?}"),
    }
}

#[track_caller]
fn type_text(schema: &SimplifiedSchema, path: &str) -> String {
    schema
        .query(path)
        .expect("path parses")
        .unwrap_or_else(|| panic!("`{path}` names a type"))
        .type_text()
}

/// `TryFrom<&str>`: content only, classified by schema structure.
mod content {
    use super::*;

    #[test]
    fn yaml_and_markdown_with_the_same_payload_give_the_same_schema() {
        let yaml = schema(PERSON_YAML);
        let markdown = schema(&format!(
            "---\ntitle: People\n{PERSON_YAML}---\n# Body\n\n$schema:\n  nope: boolean\n\nage: number\n"
        ));
        assert_eq!(yaml.to_json_schema().unwrap(), markdown.to_json_schema().unwrap());
        for path in ["name", "age"] {
            assert_eq!(yaml.query(path).unwrap().unwrap().def(), markdown.query(path).unwrap().unwrap().def());
        }
        // The body's `$schema:` text contributes nothing.
        assert!(markdown.property("nope").is_none());
        assert_eq!(markdown.properties().len(), 2);
    }

    #[test]
    fn crlf_content_parses_like_lf() {
        let lf = schema(PERSON_YAML);
        let crlf = schema(&PERSON_YAML.replace('\n', "\r\n"));
        assert_eq!(lf.to_json_schema().unwrap(), crlf.to_json_schema().unwrap());
        let markdown = schema(&format!("---\r\n{}---\r\nbody\r\n", PERSON_YAML.replace('\n', "\r\n")));
        assert_eq!(lf.to_json_schema().unwrap(), markdown.to_json_schema().unwrap());
    }

    #[test]
    fn a_file_looking_string_is_content_and_never_opened() {
        // A real schema file the string names: opening it would succeed.
        let dir = tempfile::tempdir().unwrap();
        let file = dir.path().join("person.yaml");
        std::fs::write(&file, PERSON_YAML).unwrap();
        for text in [file.to_str().unwrap(), "./person.yaml", "person.yaml"] {
            match SimplifiedSchema::try_from(text) {
                Err(SimplifiedSchemaError::MissingSchema { source_name, reason }) => {
                    assert_eq!(source_name, "content");
                    assert!(reason.contains("YAML string is not a schema"), "{reason}");
                }
                other => panic!("`{text}` is content without schema structure, got {other:?}"),
            }
        }
    }

    #[test]
    fn content_without_schema_structure_is_a_typed_missing_schema_error() {
        let cases = [
            ("42", "YAML number is not a schema"),
            ("", "YAML null is not a schema"),
            ("- a\n- b\n", "YAML sequence is not a schema"),
            ("title: plain frontmatter-like YAML\n", "no root `$schema`"),
            ("---\ntitle: no schema\n---\nbody\n", "the frontmatter has no `$schema`"),
            ("---\n---\nbody\n", "the frontmatter has no `$schema`"),
            ("# Just Markdown\n\nNo frontmatter.\n", "YAML string is not a schema"),
        ];
        for (text, expected) in cases {
            match SimplifiedSchema::try_from(text) {
                Err(SimplifiedSchemaError::MissingSchema { reason, .. }) => {
                    assert!(reason.contains(expected), "{text:?}: {reason}");
                }
                other => panic!("{text:?}: expected a missing-schema error, got {other:?}"),
            }
        }
    }

    #[test]
    fn malformed_input_is_a_typed_error() {
        // Malformed definition: the grammar's own error.
        assert!(matches!(
            SimplifiedSchema::try_from("$schema:\n  age: number(min(\n"),
            Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { .. }))
        ));
        // Not YAML at all.
        assert!(matches!(
            SimplifiedSchema::try_from("$schema: [unclosed\n"),
            Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { .. }))
        ));
        // A frontmatter fence near miss.
        assert!(matches!(
            SimplifiedSchema::try_from("----\n$schema:\n  a: string\n----\n"),
            Err(SimplifiedSchemaError::Frontmatter(_))
        ));
        // Markdown frontmatter with a malformed `$schema` definition: the
        // grammar's own error, located in the document.
        assert!(matches!(
            SimplifiedSchema::try_from("---\n$schema:\n  a: 'number(min('\n---\n"),
            Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { source: Some(cause), .. }))
                if matches!(*cause, SchemaError::Grammar { .. })
        ));
        // A misspelled type is an unknown named type, found while expanding.
        assert!(matches!(
            SimplifiedSchema::try_from("---\n$schema:\n  a: strang\n---\n"),
            Err(SimplifiedSchemaError::Schema(SchemaError::UnknownNamedType { name, .. })) if name == "strang"
        ));
    }

    /// Input Robustness Matrix, `$schema` field shapes through the content
    /// loader: absent is missing-schema; the other shapes are the shared
    /// declaration parser's existing outcomes.
    #[test]
    fn schema_field_shapes_through_frontmatter() {
        let frontmatter = |value: &str| SimplifiedSchema::try_from(format!("---\n$schema:{value}\n---\n").as_str());
        let grammar = |result: Result<SimplifiedSchema, SimplifiedSchemaError>| {
            matches!(
                result,
                Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { source: Some(cause), .. }))
                    if matches!(*cause, SchemaError::Grammar { .. })
            )
        };
        // Explicit null is not absence: it reaches the declaration parser.
        assert!(grammar(frontmatter("")));
        assert!(grammar(frontmatter(" null")));
        // Wrong type, whole field.
        assert!(grammar(frontmatter(" 42")));
        // Empty mapping: an object schema that declares nothing.
        let empty = frontmatter(" {}").expect("an empty mapping is an empty object schema");
        assert!(empty.properties().is_empty());
        // Empty root union.
        assert!(frontmatter(" []").is_err());
        // Control.
        assert!(frontmatter("\n  a: string").is_ok());
    }

    #[test]
    fn standalone_envelopes_are_recognized() {
        let tagged = schema("kind: schema\ntypes:\n  Person:\n    name: string(required)\n");
        assert_eq!(type_text(&tagged, "Person.name"), "string(required)");
        let pure_with_types = schema("$schema:\n  owner: Person\ntypes:\n  Person:\n    name: string(required)\n");
        assert_eq!(type_text(&pure_with_types, "owner.name"), "string(required)");
        assert!(pure_with_types.property("Person").is_none(), "a `types` entry is not a document property");
        assert_eq!(pure_with_types.named_type("Person").unwrap().def(), tagged.property("Person").unwrap().def());
    }

    #[test]
    fn raw_json_schema_content_is_recognized_by_its_dialect() {
        let raw = schema(
            r#"{"$schema": "https://json-schema.org/draft/2020-12/schema", "type": "object",
                "properties": {"count": {"type": "number"}}}"#,
        );
        let count = json!({ "count": 3 });
        borrowed(&raw.validate_strict(&count), &count);
        assert!(raw.validate_strict(&json!({ "count": "x" })).is_err());
        assert!(matches!(raw.to_simplified_yaml(), Err(SimplifiedSchemaError::ExportUnsupported { .. })));
        assert!(raw.property("count").is_none(), "raw JSON Schema has no typed view");
    }

    #[test]
    fn external_references_are_kept_as_parsed_with_no_io() {
        let imported = schema("$schema:\n  owner: Person@./people.yaml\n  name: string\n");
        // Inspection works on the parsed reference.
        let owner = imported.property("owner").unwrap();
        let PropertyDef::Single(atom) = owner.def() else { panic!("single atom") };
        assert!(matches!(&atom.ty, TypeExpr::Imported { name, reference } if name == "Person" && reference == "./people.yaml"));
        assert_eq!(type_text(&imported, "name"), "string");
        // Validation cannot prepare a schema whose file it cannot read.
        match imported.validate_strict(&json!({})) {
            Err(ValidationFailure::Unprepared { error }) => {
                assert!(matches!(error.as_ref(), SchemaError::Unresolved { reference, .. } if reference == "Person@./people.yaml"));
            }
            other => panic!("expected an unprepared schema, got {other:?}"),
        }
        assert!(matches!(imported.to_json_schema(), Err(SimplifiedSchemaError::Unprepared(_))));
        // The reference survives export.
        assert!(imported.to_simplified_yaml().unwrap().contains("Person@./people.yaml"));

        for text in ["$schema: ./other.yaml\n", "$schema:\n  - ./a.yaml\n  - b: string\n", "$schema:\n  doc: string(example(./x.md))\n"] {
            let parsed = schema(text);
            assert!(matches!(parsed.validate_soft(&json!({})), Err(ValidationFailure::Unprepared { .. })), "{text}");
        }
    }

    #[test]
    fn remote_references_are_rejected_at_parse() {
        assert!(matches!(
            SimplifiedSchema::try_from("$schema: https://example.com/schema.yaml\n"),
            Err(SimplifiedSchemaError::Schema(SchemaError::SchemaDocument { message, .. })) if message.contains("remote")
        ));
    }
}

/// Validation through the object: no compile call, one object for many
/// values, the strict/soft contract unchanged.
mod validation {
    use super::*;

    #[test]
    fn one_object_validates_many_values_in_both_modes() {
        let person = schema(PERSON_YAML);
        let ada = json!({ "name": "Ada", "age": 36 });
        borrowed(&person.validate_strict(&ada), &ada);
        borrowed(&person.validate_soft(&ada), &ada);

        let quoted = json!({ "name": "Ada", "age": "36" });
        assert!(person.validate_strict(&quoted).is_err());
        let soft = person.validate_soft(&quoted).expect("`\"36\"` converts");
        assert_eq!(soft.value.as_ref(), &json!({ "name": "Ada", "age": 36 }));
        assert_eq!(quoted, json!({ "name": "Ada", "age": "36" }), "the input is unchanged");

        for invalid in [json!({ "age": 3 }), json!({ "name": "Ada", "age": -1 }), json!({ "name": "Ada", "age": "2.5" })] {
            assert!(matches!(person.validate_soft(&invalid), Err(ValidationFailure::InvalidData { .. })), "{invalid}");
        }
        assert_eq!(person.revision(), 0, "validation never changes the definition");
    }

    #[test]
    fn a_selected_named_type_validates_its_own_values() {
        let catalog = schema("kind: schema\ntypes:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n");
        let person = catalog.select("Person").unwrap();
        let ada = json!({ "name": "Ada", "age": 36 });
        borrowed(&person.validate_strict(&ada), &ada);
        assert!(person.validate_strict(&json!({ "age": 1 })).is_err());
        assert!(matches!(catalog.select("Nobody"), Err(SimplifiedSchemaError::Selection { .. })));
        let scalars = schema("kind: schema\ntypes:\n  Id: string\n");
        assert!(matches!(scalars.select("Id"), Err(SimplifiedSchemaError::Selection { .. })));
    }

    #[cfg(feature = "file")]
    #[test]
    fn file_checks_of_content_are_unavailable_not_violated() {
        let content = schema("$schema:\n  doc: file(eager)\n");
        match content.validate_strict(&json!({ "doc": "./notes.md" })) {
            Err(ValidationFailure::UnavailableCheck { problems }) => assert_eq!(problems.len(), 1),
            other => panic!("content has no file context, got {other:?}"),
        }
        // Malformed file syntax is still a violation.
        assert!(matches!(
            content.validate_strict(&json!({ "doc": "~alice/notes.md" })),
            Err(ValidationFailure::InvalidData { .. })
        ));
    }

    #[test]
    fn a_function_typed_property_cannot_be_prepared_and_keeps_its_located_error() {
        let functions = schema("kind: schema\ntypes:\n  greet: 'function(parameters(name: string); returns(string))'\n");
        let error = match functions.validate_strict(&json!({})) {
            Err(ValidationFailure::Unprepared { error }) => error,
            other => panic!("expected the lowering error, got {other:?}"),
        };
        assert!(matches!(error.as_ref(), SchemaError::Convert { property, .. } if property == "greet"), "{error:?}");
        assert!(matches!(functions.to_json_schema(), Err(SimplifiedSchemaError::Unprepared(held)) if std::sync::Arc::ptr_eq(&held, &error)));
    }
}

/// Layering: whole-property replacement, low-to-high incoming order, atomic
/// calls, borrowed sources.
mod layering {
    use super::*;

    fn s1() -> SimplifiedSchema {
        schema("$schema:\n  timeout: number\n")
    }

    fn s2() -> SimplifiedSchema {
        schema("$schema:\n  timeout: string\n")
    }

    /// The spec's precedence table.
    #[test]
    fn the_spec_precedence_table() {
        let cases = [
            ("$schema:\n  name: string\n", "string", "string"),
            ("$schema:\n  name: string\n  timeout: boolean\n", "boolean", "string"),
        ];
        for (receiver, under, over) in cases {
            let (s1, s2) = (s1(), s2());
            let mut layered = schema(receiver);
            layered.layer_under(&[&s1, &s2]).unwrap();
            assert_eq!(type_text(&layered, "timeout"), under, "under: {receiver}");
            assert_eq!(type_text(&layered, "name"), "string");
            let expected = json!({ "name": "x", "timeout": if under == "boolean" { json!(true) } else { json!("t") } });
            assert!(layered.validate_strict(&expected).is_ok(), "under validates the winning declaration");

            let mut layered = schema(receiver);
            layered.layer_over(&[&s1, &s2]).unwrap();
            assert_eq!(type_text(&layered, "timeout"), over, "over: {receiver}");
            assert!(layered.validate_strict(&json!({ "name": "x", "timeout": "t" })).is_ok());
            assert!(layered.validate_strict(&json!({ "name": "x", "timeout": 5 })).is_err());
        }
    }

    #[test]
    fn a_declaration_is_replaced_whole_never_intersected() {
        let base = schema("$schema:\n  level: number(min(5); max(9))\n  tags: string[]\n");
        let mut receiver = schema("$schema:\n  level: number\n");
        receiver.as_base(&base).unwrap();
        // The receiver's `level` replaced the base's whole: no `min(5)`.
        assert!(receiver.validate_strict(&json!({ "level": 1, "tags": ["a"] })).is_ok());
        let mut over = schema("$schema:\n  level: number\n");
        over.override_with(&base).unwrap();
        assert!(over.validate_strict(&json!({ "level": 1 })).is_err());
        assert_eq!(type_text(&over, "level"), "number(min(5);max(9))");
    }

    #[test]
    fn the_returned_reference_is_the_receiver_and_sources_are_unchanged() {
        let (s1, s2) = (s1(), s2());
        let before = (s1.to_json_schema().unwrap(), s2.to_json_schema().unwrap());
        let mut receiver = schema("$schema:\n  name: string\n");
        let receiver_ptr: *const SimplifiedSchema = &receiver;
        let returned: *const SimplifiedSchema = receiver.layer_under(&[&s1, &s2]).unwrap();
        assert!(std::ptr::eq(returned, receiver_ptr));
        assert_eq!((s1.to_json_schema().unwrap(), s2.to_json_schema().unwrap()), before);
        assert_eq!((s1.revision(), s2.revision()), (0, 0));
    }

    #[test]
    fn a_failure_mid_list_leaves_the_receiver_as_it_was() {
        let valid = schema("$schema:\n  extra: number\n");
        // A root union cannot be merged under anything: the existing merge
        // admits only simple object schemas as lower layers.
        let union = schema("$schema:\n  - kind: string\n  - other: number\n");
        let mut receiver = schema("$schema:\n  name: string(required)\n");
        let value = json!({ "name": "n" });
        receiver.validate_strict(&value).unwrap();
        let before = (receiver.revision(), receiver.to_json_schema().unwrap());

        let error = receiver.layer_under(&[&valid, &union]).unwrap_err();
        assert!(matches!(error, SimplifiedSchemaError::Schema(SchemaError::Baseline { .. })), "{error:?}");
        assert_eq!((receiver.revision(), receiver.to_json_schema().unwrap()), before);
        assert!(receiver.property("extra").is_none(), "the valid first layer was not committed");
        // Validation still uses the prior definition.
        assert!(receiver.validate_strict(&json!({ "name": "n", "extra": "not a number" })).is_ok());
        assert!(receiver.validate_strict(&json!({})).is_err());
    }

    #[test]
    fn a_chain_keeps_its_earlier_successes_when_a_later_call_fails() {
        let extra = schema("$schema:\n  extra: number\n");
        let union = schema("$schema:\n  - kind: string\n");
        let mut receiver = schema("$schema:\n  name: string\n");
        let result = receiver.layer_under(&[&extra]).and_then(|schema| schema.layer_under(&[&union]));
        assert!(result.is_err());
        assert_eq!(receiver.revision(), 1);
        assert_eq!(type_text(&receiver, "extra"), "number");
        assert!(receiver.validate_strict(&json!({ "name": "n", "extra": "x" })).is_err());
    }

    #[test]
    fn a_successful_change_is_observed_by_the_next_validation() {
        let mut receiver = schema("$schema:\n  name: string\n");
        let value = json!({ "name": "n", "count": "many" });
        assert!(receiver.validate_strict(&value).is_ok(), "unknown keys are allowed before layering");
        receiver.layer_under(&[&schema("$schema:\n  count: number\n")]).unwrap();
        assert_eq!(receiver.revision(), 1);
        assert!(receiver.validate_strict(&value).is_err(), "the new revision's `count` is judged");
    }

    #[test]
    fn layering_with_nothing_new_keeps_the_revision() {
        let mut receiver = schema("$schema:\n  name: string\n");
        receiver.layer_under(&[&schema("$schema:\n  name: number\n")]).unwrap();
        receiver.layer_under(&[]).unwrap();
        assert_eq!(receiver.revision(), 0, "the receiver kept every property, so nothing changed");
    }

    #[test]
    fn a_layered_schema_exports_its_merged_properties() {
        let mut receiver = schema("$schema:\n  name: string\n");
        receiver.layer_under(&[&s1()]).unwrap();
        let reparsed = schema(&receiver.to_simplified_yaml().unwrap());
        assert_eq!(reparsed.to_json_schema().unwrap(), receiver.to_json_schema().unwrap());
        let mut referring = schema("$schema:\n  name: string\n");
        referring.layer_under(&[&schema("$schema:\n  owner: Person\ntypes:\n  Person:\n    id: string\n")]).unwrap();
        assert!(matches!(referring.to_simplified_yaml(), Err(SimplifiedSchemaError::ExportUnsupported { .. })));
    }
}

/// Passive type queries and borrowed views.
mod queries {
    use super::*;

    fn queried() -> SimplifiedSchema {
        schema(
            "$schema:\n  people: '{name: string(required), tags: string[]}[]'\n  coords: tuple([number, string, ...boolean[]])\n  settings:\n    a.b: boolean\n    a:\n      b: number\n",
        )
    }

    #[test]
    fn dotted_keys_indices_and_quoted_keys() {
        let schema = queried();
        assert_eq!(type_text(&schema, "people[0].name"), "string(required)");
        assert_eq!(type_text(&schema, "people[3].tags"), "string[]");
        assert_eq!(type_text(&schema, "coords[0]"), "number");
        assert_eq!(type_text(&schema, "coords[1]"), "string");
        assert_eq!(type_text(&schema, "coords[7]"), "boolean", "past the elements is the spread's item");
        assert_eq!(type_text(&schema, "settings[\"a.b\"]"), "boolean");
        assert_eq!(type_text(&schema, "settings['a.b']"), "boolean");
        assert_eq!(type_text(&schema, "settings.a.b"), "number");
        // Programmatic segments answer the same without parsing.
        let steps = [PathStep::Key("settings"), PathStep::Key("a.b")];
        assert_eq!(schema.query_steps(&steps).unwrap().type_text(), "boolean");
        assert_eq!(schema.property("settings").unwrap().site(), AlternativeSite::Value);
        assert_eq!(schema.query("people[0]").unwrap().unwrap().site(), AlternativeSite::Item);
    }

    #[test]
    fn empty_missing_and_unsupported_paths_are_none() {
        let schema = queried();
        for path in ["", "nobody", "people[0].nobody", "people[0][1]", "settings.a.b.c", "[0]"] {
            assert!(schema.query(path).unwrap().is_none(), "{path}");
        }
    }

    #[test]
    fn malformed_paths_are_typed_errors() {
        let schema = queried();
        for path in ["a..b", ".a", "a.", "a[", "a[x]", "a[\"b]", "a[0]b", "a[\"b\"x]"] {
            assert!(schema.query(path).is_err(), "{path}");
        }
    }

    #[test]
    fn a_root_union_answers_with_the_union_of_its_arms() {
        let union = schema("$schema:\n  - kind: string\n    size: number\n  - kind: boolean\n");
        let kind = union.property("kind").unwrap();
        assert!(!kind.is_borrowed(), "the union is assembled for the query");
        assert!(matches!(kind.def(), PropertyDef::Union(arms) if arms.len() == 2));
        // The existing root-union lookup assembles every answer, even one
        // arm's: a property-sized copy, never the schema's.
        let size = union.property("size").unwrap();
        assert!(!size.is_borrowed());
        assert_eq!(size.type_text(), "number");
        assert_eq!(union.properties().iter().map(|(name, _)| *name).collect::<Vec<_>>(), ["kind", "size"]);
    }

    #[test]
    fn a_recursive_type_is_queried_lazily_without_expansion() {
        let tree = schema("$schema:\n  root: Node\ntypes:\n  Node:\n    name: string\n    children: Node[]\n");
        let deep = "root.children[0].children[0].children[0].name";
        assert_eq!(type_text(&tree, deep), "string");
        let children = tree.query("root.children").unwrap().unwrap();
        assert!(children.is_borrowed());
        // The recursive reference is a back-reference into the stored
        // definitions, not an expansion.
        assert!(!children.definitions().is_empty());
    }

    #[test]
    fn a_function_view_keeps_the_whole_signature() {
        let catalog = schema(
            "kind: schema\ntypes:\n  greet: 'function(parameters(name: string, title?: string, ...extra: number); returns(string); fallible) -> Greets someone'\n  hello: 'function(parameters(void); returns(string))'\n",
        );
        let greet = catalog.property("greet").unwrap();
        let function = greet.function().expect("a function type");
        let parameters: Vec<_> = function
            .parameters()
            .iter()
            .map(|parameter| (parameter.name.as_str(), parameter.optional, parameter.rest))
            .collect();
        assert_eq!(parameters, [("name", false, false), ("title", true, false), ("extra", false, true)]);
        assert!(matches!(&function.parameters()[2].def, PropertyDef::Single(atom) if atom.ty == TypeExpr::Primitive(simplified_schema::SimplifiedType::Number)));
        assert!(function.fallible());
        assert!(matches!(function.returns(), PropertyDef::Single(atom) if atom.ty == TypeExpr::Primitive(simplified_schema::SimplifiedType::String)));
        assert_eq!(function.description(), Some("Greets someone"));
        let hello = catalog.named_type("hello").unwrap();
        let hello = hello.function().unwrap();
        assert!(hello.parameters().is_empty());
        assert!(!hello.fallible(), "`fallible` defaults to false");
        assert!(catalog.property("greet").unwrap().is_borrowed());
    }
}

/// Export: JSON Schema through the lowerer, and the full-schema YAML
/// document whose re-parse is the same schema (criterion 16).
mod export {
    use super::*;

    /// Parse, export, parse again: the second schema's authored and
    /// lowered forms equal the first's, and exporting again is stable.
    #[track_caller]
    fn round_trips(text: &str) -> String {
        let first = schema(text);
        let exported = first.to_simplified_yaml().unwrap();
        let second = schema(&exported);
        assert_eq!(second.to_simplified_yaml().unwrap(), exported, "export is stable:\n{exported}");
        match (first.to_json_schema(), second.to_json_schema()) {
            (Ok(a), Ok(b)) => assert_eq!(a, b, "lowered forms differ:\n{exported}"),
            (Err(_), Err(_)) => {}
            other => panic!("one side lowers and the other does not: {other:?}"),
        }
        for ((name, a), (other, b)) in first.properties().iter().zip(second.properties().iter()) {
            assert_eq!(name, other);
            assert_eq!(a.def(), b.def(), "`{name}` differs:\n{exported}");
        }
        assert_eq!(first.properties().len(), second.properties().len());
        exported
    }

    #[test]
    fn object_and_metadata() {
        round_trips(
            "$schema:\n  name: string(required) -> The display name\n  age: number(min(0); integer)\n  address:\n    street: string\n    $constraints:\n      min-keys: 1\n  labels:\n    <string>: string\n    <starting::x->: number\n  either:\n    - string -> text\n    - number -> count\n",
        );
    }

    #[test]
    fn root_union_arms() {
        let exported = round_trips("$schema:\n  - kind: enum(a)\n    size: number\n  - kind: enum(b)\n    flag: boolean\n");
        assert!(exported.starts_with("$schema:\n- "), "{exported}");
    }

    #[test]
    fn function_definitions() {
        round_trips(
            "kind: schema\ncategories:\n  - Text -> Text helpers\n  - Math\ntypes:\n  greet: 'function(parameters(name: string, title?: string, ...extra: number); returns(string); fallible) -> Greets someone'\n  hello: 'function(parameters(void); returns(string))'\n",
        );
    }

    /// An informal document keeps its `categories` list, so the restriction
    /// it places on `category(...)` survives the round trip.
    #[test]
    fn informal_categories() {
        let exported = round_trips("categories: [Core]\n$schema:\n  id: Id\ntypes:\n  Id: string(category(Core))\n");
        assert!(!exported.contains("kind:"), "{exported}");
        assert!(exported.starts_with("categories:\n- Core\n"), "{exported}");
    }

    #[test]
    fn finite_recursive_named_definitions() {
        let exported = round_trips(
            "$schema:\n  root: Node\n  pair: Pair\ntypes:\n  Node:\n    name: string\n    children: Node[]\n  Pair:\n    left: Node\n    right: Node\n",
        );
        assert!(exported.contains("children: Node[]"), "a recursive type is written by name:\n{exported}");
    }

    #[test]
    fn json_schema_export_is_the_shared_lowered_form() {
        let person = schema(PERSON_YAML);
        let first = person.to_json_schema().unwrap();
        assert!(std::sync::Arc::ptr_eq(&first, &person.to_json_schema().unwrap()), "not a copy");
        assert_eq!(first["properties"]["age"]["anyOf"][1]["type"], "integer");
        assert_eq!(first["required"], json!(["name"]));
    }

    #[test]
    fn an_inline_object_the_serializer_cannot_write_is_refused() {
        // A list of objects with a pattern key is written inline, which has
        // no pattern-key form: refused rather than dropped.
        let rows = schema("$schema:\n  rows: '{<string>: number}[]'\n");
        assert!(matches!(rows.to_simplified_yaml(), Err(SimplifiedSchemaError::ExportUnsupported { .. })));
        // The postfix form of object constraints is written, and round-trips.
        round_trips("$schema:\n  rows: '{a: number}(min-keys(1))'\n  list: '{a: number}[]'\n");
    }
}

#[test]
fn the_example_schema_text_is_the_spec_person() {
    // Keeps `examples/validate_person.rs` and this file on the same schema.
    assert!(include_str!("../../examples/validate_person.rs").contains("age: number(min(0); integer)"));
}

#[test]
fn the_object_is_send_and_sync() {
    fn check<T: Send + Sync + Clone>() {}
    check::<SimplifiedSchema>();
}

// ── Templates: `{{ … }}` slots admitted passively ─────────────────────────

mod templates {
    use super::*;
    use simplified_schema::simplified::{Constraint, DynamicElement, DynamicKind};

    const OS_SLOT: &str = r#"load_data("./os.yaml", "string[]")"#;

    fn template(text: &str) -> SimplifiedSchema {
        let schema = SimplifiedSchema::try_from(text).unwrap_or_else(|error| panic!("template admitted: {error}"));
        assert!(!schema.is_resolved(), "a template is not resolved");
        schema
    }

    #[track_caller]
    fn assert_unresolved(error: &SimplifiedSchemaError, expressions: &[&str]) {
        let SimplifiedSchemaError::Unresolved(inner) = error else {
            panic!("expected an unresolved-template error, got {error:?}");
        };
        let SchemaError::UnresolvedSlots(unresolved) = inner.as_ref() else {
            panic!("expected UnresolvedSlots, got {inner:?}");
        };
        let found: Vec<&str> = unresolved.slots.iter().map(|slot| slot.expression.as_str()).collect();
        assert_eq!(found, expressions);
    }

    #[test]
    fn the_enum_fixture_is_admitted_and_lists_its_slot() {
        let schema = template(include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml"));
        let expressions: Vec<&str> = schema.pending_slots().iter().map(|slot| slot.expression.as_str()).collect();
        assert_eq!(expressions, vec![OS_SLOT]);
        assert_unresolved(&schema.to_json_schema().unwrap_err(), &[OS_SLOT]);
    }

    #[test]
    fn every_shipped_template_fixture_is_admitted_without_lowering() {
        for (name, text, slots) in [
            ("schema-with-slot", include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml"), 1),
            ("suggest-slot", include_str!("../fixtures/dynamic_slots/suggest-slot.yaml"), 1),
            ("value-positions", include_str!("../fixtures/dynamic_slots/value-positions.yaml"), 5),
        ] {
            let schema = template(text);
            assert_eq!(schema.pending_slots().len(), slots, "{name}");
            assert!(matches!(schema.to_json_schema(), Err(SimplifiedSchemaError::Unresolved(_))), "{name}");
        }
    }

    #[test]
    fn validation_reports_the_template_unprepared_and_judges_nothing() {
        let schema = template(include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml"));
        for result in [
            schema.validate_strict(&json!({ "os_support": ["macOS"] })),
            schema.validate_soft(&json!({ "os_support": ["anything"] })),
        ] {
            let Err(ValidationFailure::Unprepared { error }) = result else {
                panic!("expected an unprepared failure, got {result:?}");
            };
            assert!(matches!(error.as_ref(), SchemaError::UnresolvedSlots(_)), "{error:?}");
        }
    }

    #[test]
    fn layering_with_a_template_on_either_side_fails_and_changes_nothing() {
        let template = template("$schema:\n  os: enum(none, {{ load_data('./os.yaml') }})\n");
        let concrete = schema("$schema:\n  name: string\n");

        let mut receiver = concrete.clone();
        assert_unresolved(&receiver.layer_under(&[&template]).unwrap_err(), &["load_data('./os.yaml')"]);
        assert_eq!(receiver.revision(), 0);
        assert!(receiver.is_resolved());
        assert_unresolved(&receiver.layer_over(&[&template]).unwrap_err(), &["load_data('./os.yaml')"]);

        let mut receiver = template.clone();
        assert_unresolved(&receiver.layer_under(&[&concrete]).unwrap_err(), &["load_data('./os.yaml')"]);
        assert_eq!(receiver.revision(), 0);
        assert_eq!(receiver.pending_slots().len(), 1);
    }

    #[test]
    fn export_keeps_every_slot_as_authored_and_round_trips() {
        for text in [
            include_str!("../fixtures/dynamic_slots/schema-with-slot.yaml"),
            include_str!("../fixtures/dynamic_slots/suggest-slot.yaml"),
            include_str!("../fixtures/dynamic_slots/value-positions.yaml"),
        ] {
            let first = template(text);
            let exported = first.to_simplified_yaml().expect("a template exports");
            for slot in first.pending_slots() {
                assert!(exported.contains(&format!("{{{{ {} }}}}", slot.expression)), "{exported}");
            }
            let second = template(&exported);
            let expressions = |schema: &SimplifiedSchema| -> Vec<String> {
                schema.pending_slots().iter().map(|slot| slot.expression.clone()).collect()
            };
            assert_eq!(expressions(&second), expressions(&first));
            assert_eq!(second.to_simplified_yaml().unwrap(), exported, "export → parse → export is stable");
        }
    }

    #[test]
    fn export_keeps_literal_quoting_beside_a_slot() {
        let schema = template("$schema:\n  os: \"enum('a, b', {{ x }}, c; required)\"\n");
        let exported = schema.to_simplified_yaml().unwrap();
        let again = template(&exported);
        let os = again.property("os").expect("declared");
        let PropertyDef::Single(atom) = os.def() else { panic!("{:?}", os.def()) };
        let Constraint::Dynamic(list) = &atom.constraints[0] else { panic!("{:?}", atom.constraints) };
        assert_eq!(list.kind, DynamicKind::Members);
        assert!(matches!(
            list.elements.as_slice(),
            [DynamicElement::Member(a), DynamicElement::Slot(_), DynamicElement::Member(c)] if a == "a, b" && c == "c"
        ));
        assert_eq!(atom.constraints[1], Constraint::Required);
    }

    #[test]
    fn a_static_schema_is_resolved_and_lowers_its_lists_as_before() {
        let schema = schema("$schema:\n  os: enum(a, b; required)\n  tag: string(suggest(x, y))\n");
        assert!(schema.is_resolved());
        assert!(schema.pending_slots().is_empty());
        let json = schema.to_json_schema().unwrap();
        assert_eq!(json["properties"]["os"]["enum"], json!(["a", "b"]));
        fn suggestions(value: &Value) -> Option<&Value> {
            match value {
                Value::Object(map) => map.get("x-darkmatter-suggest").or_else(|| map.values().find_map(suggestions)),
                Value::Array(items) => items.iter().find_map(suggestions),
                _ => None,
            }
        }
        assert_eq!(suggestions(&json["properties"]["tag"]), Some(&json!(["x", "y"])));
    }
}

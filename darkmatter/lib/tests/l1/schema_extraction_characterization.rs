//! Pre-extraction characterization of the SimplifiedSchema language, written
//! against Darkmatter's public API before any module moves
//! (`2026-10-28-extract-simplified-schema`, Task 1.3). The current behavior is
//! the parity authority: every assertion records what the code does today,
//! including outcomes that look surprising, so the extraction can prove it
//! preserved them. A cell that pins a surprising outcome says so in a comment
//! beginning `PINNED:`.
//!
//! Exact validation is [`coercion::check_exact`] at the shape level and
//! [`EffectiveSchema::validate_raw`] / `validate_for_phase(Completion)` at the
//! frontmatter level; coercing validation is [`coercion::bind`] and
//! [`EffectiveSchema::validate`] / `validate_for_phase(Launch)`, with
//! [`EffectiveSchema::coerce_frontmatter_with_pending`] giving the effective
//! value. The YAML loader is [`parse_standalone_schema_document`] (and
//! [`parse_property_definition`] for one property's type text); the raw JSON
//! Schema loader is reached through [`resolve::resolve_schema`] on a `.json`
//! file reference.

use std::collections::HashSet;
use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::coercion::{self, Bound, ConversionError};
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::expression::call_binding::{NoNamedTypes, shape_from_property_def};
use darkmatter::markdown::schemas::resolve::{ResolvedSchema, resolve_schema};
use darkmatter::markdown::schemas::simplified::{
    DefinitionTable, PropertyDef, SchemaDeclaration, SimplifiedSchema, StandaloneSchemaEnvelope, grammar,
    parse_property_definition, parse_standalone_schema_document, parse_yaml_schema, serialize_property_def, TypeExpr,
};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema, SchemaError, SchemaPhase};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

fn yaml(text: &str) -> YamlValue {
    serde_yaml_ng::from_str(text).expect("test YAML parses")
}

/// One edit applied to a control fixture; every non-control cell changes the
/// fixture in exactly one place (or one document-wide encoding).
#[derive(Clone, Copy, Debug)]
enum Edit {
    Control,
    /// Replace the single occurrence of the first text with the second.
    Replace(&'static str, &'static str),
    Append(&'static str),
    Prepend(&'static str),
    /// Every `\n` becomes `\r\n`.
    Crlf,
    /// The whole document is replaced (scalar-alone and empty cells).
    Whole(&'static str),
}

fn apply(seed: &str, edit: Edit) -> String {
    let edited = match edit {
        Edit::Control => return seed.to_string(),
        Edit::Replace(from, to) => {
            assert_eq!(seed.matches(from).count(), 1, "edit target {from:?} must occur exactly once");
            seed.replacen(from, to, 1)
        }
        Edit::Append(tail) => format!("{seed}{tail}"),
        Edit::Prepend(head) => format!("{head}{seed}"),
        Edit::Crlf => seed.replace('\n', "\r\n"),
        Edit::Whole(text) => text.to_string(),
    };
    assert_ne!(edited, seed, "a non-control edit must change the fixture");
    edited
}

/// The spec's `Person` declaration (spec "R1"), unedited.
const TAGGED_PERSON: &str = "kind: schema
types:
  Person:
    name: string(required)
    age: number(min(0); integer)
";

/// The same `Person` exported through the pure envelope.
const PURE_PERSON: &str = "$schema:
  person: Person
types:
  Person:
    name: string(required)
    age: number(min(0); integer)
";

/// A raw JSON Schema for the same `Person`.
const JSON_PERSON: &str = r#"{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "type": "object",
  "properties": {
    "name": { "type": "string" },
    "age": { "type": "integer", "minimum": 0 }
  },
  "required": ["name"]
}
"#;

/// The first word of an error's `Debug` text: its variant name, for failure
/// messages only (assertions use `matches!`).
fn variant(error: &SchemaError) -> String {
    format!("{error:?}").split([' ', '{', '(']).next().unwrap_or_default().to_string()
}

/// The name of each failure reason in a conversion error, in order.
fn reasons(error: &ConversionError) -> Vec<String> {
    error
        .failures
        .iter()
        .map(|failure| format!("{:?}", failure.reason).split([' ', '{', '(']).next().unwrap_or_default().to_string())
        .collect()
}

/// A [`DarkmatterSchemas`] anchored in a fresh temp directory, never the
/// process directory.
fn schemas_in(dir: &Path) -> DarkmatterSchemas {
    DarkmatterSchemas::new(crate::request_support::context_at(dir))
}

/// The effective schema of a document whose frontmatter is `frontmatter`,
/// authored at `dir/doc.md`.
fn effective_in(dir: &Path, frontmatter: &str) -> Result<Option<EffectiveSchema>, SchemaError> {
    let source: Markdown = format!("---\n{frontmatter}---\nbody\n").into();
    let source = source.with_source(ComposeSource::File(dir.join("doc.md")));
    schemas_in(dir).effective_for(&source)
}

// ---------------------------------------------------------------------------

/// The four rows of the spec's R6 strict/soft table, through every current
/// exact and coercing operation.
mod r6_table {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Bind {
        /// `Bound::Kept`: today's equivalent of "borrow input".
        Kept,
        /// `Bound::Converted`: today's equivalent of "owned value".
        Converted(Value),
        /// Rejected, with the failure reason names.
        Rejected(Vec<String>),
    }

    #[derive(Debug, PartialEq)]
    struct Observed {
        /// `coercion::check_exact`: `Ok` or the failure reason names.
        check_exact: Result<(), Vec<String>>,
        accepts_exactly: bool,
        bind: Bind,
        /// `EffectiveSchema::validate_raw(...).valid`.
        validate_raw: bool,
        /// `validate_for_phase(…, Completion).valid`.
        completion: bool,
        /// `EffectiveSchema::validate(...).valid`.
        validate: bool,
        /// `validate_for_phase(…, Launch).valid`.
        launch: bool,
        /// `coerce_frontmatter_with_pending`: the effective frontmatter.
        coerced: Value,
        /// Whether that frontmatter differs from the input.
        coerced_changed: bool,
    }

    /// Observes `value` against the type text `ty`, alone at the shape level
    /// and as the property `v` of a frontmatter mapping.
    fn observe(ty: &str, value: Value) -> Observed {
        let def = parse_property_definition("v", &YamlValue::String(ty.into())).expect("type parses");
        let shape = shape_from_property_def(&def, &NoNamedTypes).expect("type translates");
        let check_exact = coercion::check_exact(&shape, &value).map_err(|error| reasons(&error));
        let accepts_exactly = coercion::accepts_exactly(&shape, &value);
        let bind = match coercion::bind(&shape, &value) {
            Ok(Bound::Kept(kept)) => {
                assert!(std::ptr::eq(kept, &value), "a kept value is the caller's own value");
                Bind::Kept
            }
            Ok(Bound::Converted(converted)) => Bind::Converted(converted),
            Err(error) => Bind::Rejected(reasons(&error)),
        };

        let workspace = tempfile::tempdir().expect("workspace");
        let effective = effective_in(workspace.path(), &format!("$schema:\n  v: \"{ty}\"\n"))
            .expect("schema resolves")
            .expect("schema is present");
        let frontmatter = json!({ "v": value });
        let phase = |phase| {
            effective
                .validate_for_phase(&frontmatter, phase)
                .expect("phase schema builds")
                .valid
        };
        let coerced = effective.coerce_frontmatter_with_pending(&frontmatter, &HashSet::new());
        Observed {
            check_exact,
            accepts_exactly,
            bind,
            validate_raw: effective.validate_raw(&frontmatter).valid,
            completion: phase(SchemaPhase::Completion),
            validate: effective.validate(&frontmatter).valid,
            launch: phase(SchemaPhase::Launch),
            coerced: coerced.value,
            coerced_changed: coerced.changed,
        }
    }

    #[test]
    fn number_with_number_4_is_accepted_and_kept_by_both() {
        assert_eq!(
            observe("number", json!(4)),
            Observed {
                check_exact: Ok(()),
                accepts_exactly: true,
                bind: Bind::Kept,
                validate_raw: true,
                completion: true,
                validate: true,
                launch: true,
                coerced: json!({ "v": 4 }),
                coerced_changed: false,
            }
        );
    }

    #[test]
    fn number_with_string_4_is_rejected_exactly_and_converted_softly() {
        assert_eq!(
            observe("number", json!("4")),
            Observed {
                check_exact: Err(vec!["ConversionNotAllowed".into()]),
                accepts_exactly: false,
                bind: Bind::Converted(json!(4)),
                validate_raw: false,
                completion: false,
                validate: true,
                launch: true,
                coerced: json!({ "v": 4 }),
                coerced_changed: true,
            }
        );
    }

    #[test]
    fn integer_with_string_2_5_is_rejected_by_both_and_never_repaired() {
        assert_eq!(
            observe("number(integer)", json!("2.5")),
            Observed {
                check_exact: Err(vec!["ConversionNotAllowed".into()]),
                accepts_exactly: false,
                bind: Bind::Rejected(vec!["ConstraintNotMet".into()]),
                validate_raw: false,
                completion: false,
                validate: false,
                launch: false,
                coerced: json!({ "v": "2.5" }),
                coerced_changed: false,
            }
        );
    }

    #[test]
    fn string_or_number_with_string_4_is_kept_as_the_string() {
        assert_eq!(
            observe("string | number", json!("4")),
            Observed {
                check_exact: Ok(()),
                accepts_exactly: true,
                bind: Bind::Kept,
                validate_raw: true,
                completion: true,
                validate: true,
                launch: true,
                coerced: json!({ "v": "4" }),
                coerced_changed: false,
            }
        );
    }
}

// ---------------------------------------------------------------------------

/// No operation changes the value, or the schema, it is handed.
mod immutability {
    use super::*;

    #[test]
    fn every_exact_and_coercing_operation_leaves_its_input_unchanged() {
        let workspace = tempfile::tempdir().expect("workspace");
        let effective = effective_in(
            workspace.path(),
            "$schema:\n  n: number\n  i: number(integer)\n  tags: \"string[]\"\n  inner: \"{ flag: boolean }\"\n",
        )
        .expect("schema resolves")
        .expect("schema is present");
        let input = json!({ "n": "4", "i": "2.5", "tags": [1, "two"], "inner": { "flag": "true" } });
        let before = input.clone();
        let schema_before = effective.json_schema.as_ref().clone();

        let _ = effective.validate_raw(&input);
        let _ = effective.validate(&input);
        let _ = effective.validate_for_phase(&input, SchemaPhase::Launch);
        let _ = effective.validate_for_phase(&input, SchemaPhase::Completion);
        let coerced = effective.coerce_frontmatter_with_pending(&input, &HashSet::new());
        let _ = darkmatter::markdown::schemas::coerce::coerce_frontmatter(&effective.json_schema, &input);
        assert_eq!(input, before, "frontmatter-level operations mutated their input");
        assert!(coerced.changed, "the probe must exercise a conversion, not only a keep");

        let shape = shape_from_property_def(
            &parse_property_definition("n", &YamlValue::String("number".into())).expect("type parses"),
            &NoNamedTypes,
        )
        .expect("type translates");
        let scalar = json!("4");
        let scalar_before = scalar.clone();
        let _ = coercion::check_exact(&shape, &scalar);
        let _ = coercion::accepts_exactly(&shape, &scalar);
        let bound = coercion::bind(&shape, &scalar).expect("converts");
        assert!(matches!(bound, Bound::Converted(_)));
        assert_eq!(scalar, scalar_before, "shape-level operations mutated their input");

        assert_eq!(*effective.json_schema, schema_before, "validation mutated the schema");
    }
}

// ---------------------------------------------------------------------------

/// The YAML schema loader, [`parse_standalone_schema_document`], one cell per
/// field shape and document shape, each one edit from a control fixture.
mod yaml_loader_matrix {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum Expect {
        /// Recognized as a tagged document whose payload is a schema.
        Tagged,
        /// Recognized as a pure document whose payload is a schema.
        Pure,
        /// `Ok(None)`: ordinary YAML, not a schema document.
        NotSchema,
        /// `Err(SchemaError::SchemaDocument)` whose message contains the text.
        Rejects(&'static str),
    }

    fn check(seed: &str, edit: Edit, expect: Expect) {
        let source = apply(seed, edit);
        let outcome = parse_standalone_schema_document(&source, "cell.yaml");
        let ok = match (&outcome, expect) {
            (Ok(Some(document)), Expect::Tagged) => {
                document.envelope == StandaloneSchemaEnvelope::Tagged
                    && matches!(document.declaration, SchemaDeclaration::Schema(_))
            }
            (Ok(Some(document)), Expect::Pure) => {
                document.envelope == StandaloneSchemaEnvelope::Pure
                    && matches!(document.declaration, SchemaDeclaration::Schema(_))
            }
            (Ok(None), Expect::NotSchema) => true,
            (Err(SchemaError::SchemaDocument { message, .. }), Expect::Rejects(fragment)) => message.contains(fragment),
            _ => false,
        };
        assert!(
            ok,
            "edit {edit:?} expected {expect:?}, got {}\n--- source ---\n{source}",
            match &outcome {
                Ok(Some(document)) => format!("Ok(Some({:?}, {:?}))", document.envelope, document.declaration),
                Ok(None) => "Ok(None)".into(),
                Err(error) => format!("Err({}: {error})", variant(error)),
            }
        );
    }

    macro_rules! cells {
        ($seed:ident { $($name:ident: $edit:expr => $expect:expr;)* }) => {
            $(
                #[test]
                fn $name() {
                    check($seed, $edit, $expect);
                }
            )*
        };
    }

    use Edit::*;
    use Expect::*;

    #[test]
    fn control_person_types_parse_with_both_fields() {
        let document = parse_standalone_schema_document(TAGGED_PERSON, "person.yaml")
            .expect("parses")
            .expect("recognized");
        let types = document.named_types().expect("named types");
        assert_eq!(types.properties.keys().collect::<Vec<_>>(), ["Person"]);
        let person = match &types.properties["Person"] {
            PropertyDef::Single(atom) => match &atom.ty {
                TypeExpr::InlineObject(shape) => shape,
                other => panic!("Person is not an inline object: {other:?}"),
            },
            other => panic!("Person is a union: {other:?}"),
        };
        assert_eq!(person.properties.keys().collect::<Vec<_>>(), ["name", "age"]);
        assert_eq!(serialize_property_def(&person.properties["age"]), "number(min(0);integer)");
    }

    // `kind` on the tagged control.
    cells!(TAGGED_PERSON {
        tagged_control: Control => Tagged;
        kind_absent: Replace("kind: schema\n", "") => NotSchema;
        kind_null_bare: Replace("kind: schema", "kind:") => NotSchema;
        kind_null_explicit: Replace("kind: schema", "kind: null") => NotSchema;
        kind_wrong_type: Replace("kind: schema", "kind: 5") => NotSchema;
        kind_empty_string: Replace("kind: schema", "kind: \"\"") => NotSchema;
        kind_empty_mapping: Replace("kind: schema", "kind: {}") => NotSchema;
        kind_other_word: Replace("kind: schema", "kind: catalog") => NotSchema;
    });

    // `types` on the tagged control.
    cells!(TAGGED_PERSON {
        // PINNED: the "missing `types`" message is unreachable; the key-count
        // check fires first and names an empty list of unsupported keys.
        types_absent: Whole("kind: schema\n") => Rejects("found unsupported keys: ");
        types_null_bare: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types:\n") => Rejects("`types` must be a mapping");
        types_null_explicit: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types: null\n") => Rejects("`types` must be a mapping");
        types_wrong_type: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types: [Person]\n") => Rejects("`types` must be a mapping");
        types_empty_string: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types: \"\"\n") => Rejects("`types` must be a mapping");
        types_empty_mapping: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types: {}\n") => Tagged;
        types_entry_non_string: Replace("  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "  Person: 5\n") => Rejects("invalid SimplifiedSchema payload");
    });

    // Property type text, through the tagged control.
    cells!(TAGGED_PERSON {
        property_type_null_bare: Replace("age: number(min(0); integer)", "age:") => Tagged;
        property_type_null_explicit: Replace("age: number(min(0); integer)", "age: null") => Tagged;
        property_type_wrong_type: Replace("age: number(min(0); integer)", "age: 5") => Rejects("invalid SimplifiedSchema payload");
        property_type_empty_string: Replace("age: number(min(0); integer)", "age: \"\"") => Rejects("invalid SimplifiedSchema payload");
        property_type_empty_mapping: Replace("age: number(min(0); integer)", "age: {}") => Tagged;
        // PINNED: an unknown word parses as an unresolved local reference.
        property_type_unknown_word: Replace("age: number(min(0); integer)", "age: numbr") => Tagged;
    });

    // Document shapes, through the tagged control.
    cells!(TAGGED_PERSON {
        duplicate_key: Append("kind: schema\n") => Rejects("not valid YAML");
        trailing_invalid_content: Append("]\n") => Rejects("not valid YAML");
        second_yaml_document: Append("---\nkind: schema\ntypes: {}\n") => Rejects("not valid YAML");
        crlf_line_endings: Crlf => Tagged;
        // PINNED: a leading BOM is read as a second YAML document and rejected.
        byte_order_mark: Prepend("\u{feff}") => Rejects("more than one document");
        yaml_scalar_alone: Whole("42\n") => NotSchema;
        yaml_string_alone: Whole("./person.yaml\n") => NotSchema;
        empty_document: Whole("") => NotSchema;
        not_yaml_at_all: Whole("{ unclosed\n") => Rejects("not valid YAML");
        unsupported_tagged_key: Append("extra: 1\n") => Rejects("found unsupported keys: extra");
    });

    // `$schema` on the pure control.
    cells!(PURE_PERSON {
        pure_control: Control => Pure;
        schema_absent: Replace("$schema:\n  person: Person\n", "") => NotSchema;
        schema_null_bare: Replace("$schema:\n  person: Person\n", "$schema:\n") => Rejects("invalid SimplifiedSchema payload");
        schema_null_explicit: Replace("$schema:\n  person: Person\n", "$schema: null\n") => Rejects("invalid SimplifiedSchema payload");
        schema_wrong_type: Replace("$schema:\n  person: Person\n", "$schema: 5\n") => Rejects("invalid SimplifiedSchema payload");
        schema_empty_string: Replace("$schema:\n  person: Person\n", "$schema: \"\"\n") => Rejects("invalid SimplifiedSchema payload");
        schema_empty_mapping: Replace("$schema:\n  person: Person\n", "$schema: {}\n") => Pure;
        schema_remote_reference: Replace("$schema:\n  person: Person\n", "$schema: https://example.com/person.yaml\n") => Rejects("invalid SimplifiedSchema payload");
        pure_types_wrong_type: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types: [Person]\n") => Rejects("`types` must be a mapping");
        pure_types_null: Replace("types:\n  Person:\n    name: string(required)\n    age: number(min(0); integer)\n", "types:\n") => Rejects("`types` must be a mapping");
    });

    /// A `$schema` file reference is classified by the loader, not opened:
    /// the payload is a `Reference` declaration.
    #[test]
    fn pure_schema_file_reference_is_a_reference_declaration() {
        let source = apply(PURE_PERSON, Replace("$schema:\n  person: Person\n", "$schema: ./person.yaml\n"));
        // PINNED: `types` beside a whole-file reference export is accepted.
        let document = parse_standalone_schema_document(&source, "cell.yaml")
            .expect("parses")
            .expect("recognized");
        assert!(matches!(document.declaration, SchemaDeclaration::Reference(_)));
    }

    /// Property type text through [`parse_property_definition`], which keeps
    /// the typed error the document loader wraps.
    #[test]
    fn property_type_text_shapes_through_the_property_parser() {
        let parse = |value: YamlValue| parse_property_definition("age", &value);
        assert!(parse(YamlValue::String("number(min(0); integer)".into())).is_ok(), "control");
        // PINNED: a null type text parses (it is the documented `null` type
        // spelled by omission), rather than being a missing-type error.
        let null = parse(YamlValue::Null).expect("null type text parses");
        assert_eq!(serialize_property_def(&null), "null");
        let number = parse(yaml("5"));
        assert!(matches!(number, Err(SchemaError::Grammar { .. })), "wrong type: {number:?}");
        let empty = parse(YamlValue::String(String::new()));
        assert!(matches!(empty, Err(SchemaError::Grammar { .. })), "empty string: {empty:?}");
        let mapping = parse(yaml("{}")).expect("empty mapping parses");
        assert_eq!(serialize_property_def(&mapping), "{}");
        // PINNED: an unknown word is a local named-type reference
        // (`numbr@this`), not a grammar error; it fails only at resolution.
        let unknown = parse(YamlValue::String("numbr".into())).expect("unknown word parses");
        assert!(
            matches!(&unknown, PropertyDef::Single(atom) if matches!(&atom.ty, TypeExpr::Imported { name, .. } if name == "numbr")),
            "unknown word: {unknown:?}"
        );
    }

    /// The `$schema` body parser alone: a YAML scalar is not a schema.
    #[test]
    fn schema_body_scalar_is_a_grammar_error() {
        assert!(matches!(parse_yaml_schema(&yaml("42")), Err(SchemaError::Grammar { .. })));
        assert!(matches!(parse_yaml_schema(&yaml("string")), Err(SchemaError::Grammar { .. })));
        assert!(matches!(parse_yaml_schema(&yaml("{}")), Ok(SimplifiedSchema::Single(_))));
    }

    /// The loader as the file resolver uses it: a `.yaml` reference in a
    /// document's `$schema`.
    #[test]
    fn file_wiring_control_loads_and_one_invalid_shape_is_located() {
        let workspace = tempfile::tempdir().expect("workspace");
        let dir = workspace.path();
        std::fs::write(dir.join("person.yaml"), TAGGED_PERSON).expect("write control");
        std::fs::write(dir.join("broken.yaml"), apply(TAGGED_PERSON, Append("]\n"))).expect("write cell");
        let context = crate::request_support::context_at(dir);

        let control = resolve_schema(&json!("./person.yaml"), dir, &context).expect("control resolves");
        assert!(control.simplified.is_some(), "a tagged file is a SimplifiedSchema");
        assert_eq!(control.referenced_files.len(), 1);

        let broken = resolve_schema(&json!("./broken.yaml"), dir, &context);
        assert!(
            matches!(&broken, Err(SchemaError::SchemaDocument { path, .. }) if path.ends_with("broken.yaml")),
            "got {broken:?}"
        );

        let remote = resolve_schema(&json!("https://example.com/person.yaml"), dir, &context);
        assert!(matches!(remote, Err(SchemaError::RemoteUnsupported { .. })), "got {remote:?}");
    }
}

// ---------------------------------------------------------------------------

/// The raw JSON Schema loader: a `.json` file a `$schema` reference names,
/// read once and kept as raw JSON Schema.
mod json_schema_loader_matrix {
    use super::*;

    #[derive(Clone, Copy, Debug)]
    enum Expect {
        /// Loaded as raw JSON Schema (`simplified: None`).
        Loads,
        /// `Err(SchemaError::SchemaDocument)` whose message contains the text.
        Rejects(&'static str),
        /// `Err(SchemaError::AmbiguousReferenced)`.
        Ambiguous,
    }

    fn load(source: &str) -> Result<ResolvedSchema, SchemaError> {
        let workspace = tempfile::tempdir().expect("workspace");
        let dir = workspace.path();
        std::fs::write(dir.join("schema.json"), source).expect("write cell");
        resolve_schema(&json!("./schema.json"), dir, &crate::request_support::context_at(dir))
    }

    fn check(edit: Edit, expect: Expect) {
        let source = apply(JSON_PERSON, edit);
        let outcome = load(&source);
        let ok = match (&outcome, expect) {
            (Ok(resolved), Expect::Loads) => resolved.simplified.is_none() && resolved.json_schema.is_object(),
            (Err(SchemaError::SchemaDocument { message, .. }), Expect::Rejects(fragment)) => message.contains(fragment),
            (Err(SchemaError::AmbiguousReferenced { .. }), Expect::Ambiguous) => true,
            _ => false,
        };
        assert!(
            ok,
            "edit {edit:?} expected {expect:?}, got {}\n--- source ---\n{source}",
            match &outcome {
                Ok(resolved) => format!("Ok(simplified: {}, json: {})", resolved.simplified.is_some(), resolved.json_schema),
                Err(error) => format!("Err({}: {error})", variant(error)),
            }
        );
    }

    macro_rules! cells {
        ($($name:ident: $edit:expr => $expect:expr;)*) => {
            $(
                #[test]
                fn $name() {
                    check($edit, $expect);
                }
            )*
        };
    }

    use Edit::*;
    use Expect::*;

    const DIALECT: &str = "\"$schema\": \"https://json-schema.org/draft/2020-12/schema\",\n";

    cells! {
        json_control: Control => Loads;
        // `$schema` dialect field: the loader does not read it.
        schema_absent: Replace(DIALECT, "") => Loads;
        schema_null: Replace(DIALECT, "\"$schema\": null,\n") => Loads;
        schema_wrong_type: Replace(DIALECT, "\"$schema\": 5,\n") => Loads;
        schema_empty_string: Replace(DIALECT, "\"$schema\": \"\",\n") => Loads;
        schema_empty_object: Replace(DIALECT, "\"$schema\": {},\n") => Loads;
        schema_draft_07_dialect: Replace("draft/2020-12/schema", "draft-07/schema#") => Loads;
        // PINNED: `kind: schema` in a JSON file does not make it SimplifiedSchema.
        kind_schema_in_json: Replace(DIALECT, "\"kind\": \"schema\",\n") => Loads;
        // PINNED: a remote `$ref` is loaded without complaint; only a remote
        // `$schema` *reference* is `RemoteUnsupported`.
        remote_ref: Replace("{ \"type\": \"string\" }", "{ \"$ref\": \"https://example.com/name.json\" }") => Loads;
        // Document shapes.
        duplicate_key: Replace("\"type\": \"object\",\n", "\"type\": \"object\",\n  \"type\": \"object\",\n") => Rejects("duplicate");
        trailing_garbage: Append("}\n") => Rejects("trailing");
        second_value: Append("{}\n") => Rejects("trailing");
        crlf_line_endings: Crlf => Loads;
        // PINNED: a leading BOM is invalid JSON here.
        byte_order_mark: Prepend("\u{feff}") => Rejects("invalid JSON: expected value");
        json5_comment: Prepend("// comment\n") => Rejects("invalid JSON: expected value");
        empty_file: Whole("") => Rejects("invalid JSON: EOF");
        top_level_array: Whole("[]\n") => Ambiguous;
        top_level_scalar: Whole("42\n") => Ambiguous;
        empty_object: Whole("{}\n") => Loads;
    }

    #[test]
    fn remote_schema_reference_is_remote_unsupported_without_io() {
        let workspace = tempfile::tempdir().expect("workspace");
        let dir = workspace.path();
        let outcome = resolve_schema(
            &json!("https://example.com/person.json"),
            dir,
            &crate::request_support::context_at(dir),
        );
        assert!(matches!(outcome, Err(SchemaError::RemoteUnsupported { .. })), "got {outcome:?}");
    }

    #[test]
    fn missing_json_file_is_no_match() {
        let workspace = tempfile::tempdir().expect("workspace");
        let dir = workspace.path();
        let outcome = resolve_schema(&json!("./absent.json"), dir, &crate::request_support::context_at(dir));
        assert!(matches!(outcome, Err(SchemaError::NoMatch { .. })), "got {outcome:?}");
    }
}

// ---------------------------------------------------------------------------

/// The frontmatter `$schema` field, read through a document's effective
/// schema (the only current entry point that reads it from Markdown).
mod frontmatter_schema_field {
    use super::*;

    fn outcome(frontmatter: &str) -> Result<Option<EffectiveSchema>, SchemaError> {
        let workspace = tempfile::tempdir().expect("workspace");
        effective_in(workspace.path(), frontmatter)
    }

    #[test]
    fn control_inline_person_is_present() {
        let effective = outcome("$schema:\n  name: string(required)\n  age: number(min(0); integer)\n")
            .expect("resolves")
            .expect("present");
        assert!(effective.validate(&json!({ "name": "Ada", "age": 36 })).valid);
        assert!(!effective.validate(&json!({ "age": 36 })).valid);
    }

    #[test]
    fn absent_is_no_schema() {
        assert!(matches!(outcome("title: hello\n"), Ok(None)));
    }

    #[test]
    fn null_bare_is_a_frontmatter_shape_error() {
        // A present null is not absence: it is rejected, unlike a missing key.
        let result = outcome("$schema:\n");
        assert!(
            matches!(result, Err(SchemaError::FrontmatterShape { .. })),
            "got {:?}",
            result.map(|schema| schema.is_some())
        );
    }

    #[test]
    fn null_explicit_is_a_frontmatter_shape_error() {
        let result = outcome("$schema: null\n");
        assert!(
            matches!(result, Err(SchemaError::FrontmatterShape { .. })),
            "got {:?}",
            result.map(|schema| schema.is_some())
        );
    }

    #[test]
    fn wrong_type_is_a_frontmatter_shape_error() {
        let result = outcome("$schema: 5\n");
        assert!(
            matches!(result, Err(SchemaError::FrontmatterShape { .. })),
            "got {:?}",
            result.map(|schema| schema.is_some())
        );
    }

    #[test]
    fn empty_string_is_unresolved() {
        let result = outcome("$schema: \"\"\n");
        assert!(
            matches!(result, Err(SchemaError::Unresolved { .. })),
            "got {:?}",
            result.map(|schema| schema.is_some())
        );
    }

    #[test]
    fn empty_mapping_is_a_present_empty_schema() {
        let result = outcome("$schema: {}\n");
        assert!(matches!(result, Ok(Some(_))), "got {:?}", result.map(|schema| schema.is_some()));
    }

    #[test]
    fn remote_reference_is_remote_unsupported() {
        let result = outcome("$schema: https://example.com/person.yaml\n");
        assert!(
            matches!(result, Err(SchemaError::RemoteUnsupported { .. })),
            "got {:?}",
            result.map(|schema| schema.is_some())
        );
    }
}

// ---------------------------------------------------------------------------

/// Query edge cases through the three current path helpers (inventory §9).
mod query_edge_cases {
    use super::*;
    use darkmatter::markdown::schemas::completion::{AlternativeSite, PathStep, def_at_path};
    use darkmatter::markdown::schemas::property_def_at_path;
    use darkmatter::markdown::schemas::simplified::suggestions_for_path;
    use std::borrow::Cow;

    const SCHEMA: &str = r#"a: "{ b: number(max(0); suggest(1)) }"
a.b: number(max(1); suggest(1))
items: "{ name: string(suggest(x)) }[]"
pick: "boolean | string(suggest(s))"
"#;

    fn schema() -> SimplifiedSchema {
        parse_yaml_schema(&yaml(SCHEMA)).expect("query schema parses")
    }

    fn root_shape(schema: &SimplifiedSchema) -> &darkmatter::markdown::schemas::simplified::SchemaShape {
        match schema {
            SimplifiedSchema::Single(shape) => shape,
            SimplifiedSchema::Union(_) => panic!("query schema is a single shape"),
        }
    }

    fn at(schema: &SimplifiedSchema, path: &[PathStep<'_>]) -> Option<(String, AlternativeSite)> {
        def_at_path(schema, &DefinitionTable::default(), &Value::Null, path)
            .map(|(def, site)| (serialize_property_def(&def), site))
    }

    #[test]
    fn dotted_literal_key_and_nested_path_are_distinct() {
        let schema = schema();
        assert_eq!(suggestions_for_path(&schema, &["a.b"]).expect("literal key").items[0].label, "1");
        assert!(suggestions_for_path(&schema, &["a", "b"]).expect("nested path").items.is_empty());

        assert_eq!(
            at(&schema, &[PathStep::Key("a.b")]),
            Some(("number(max(1);suggest(1))".into(), AlternativeSite::Value))
        );
        assert_eq!(
            at(&schema, &[PathStep::Key("a"), PathStep::Key("b")]),
            Some(("number(max(0);suggest(1))".into(), AlternativeSite::Value))
        );

        let shape = root_shape(&schema);
        let literal = property_def_at_path(shape, &Value::Null, &["a.b"]).expect("literal key");
        assert!(matches!(literal, Cow::Borrowed(_)), "a top-level leaf is borrowed");
        assert_eq!(serialize_property_def(&literal), "number(max(1);suggest(1))");
        let nested = property_def_at_path(shape, &Value::Null, &["a", "b"]).expect("nested path");
        assert!(matches!(nested, Cow::Owned(_)), "a nested leaf is owned");
        assert_eq!(serialize_property_def(&nested), "number(max(0);suggest(1))");
    }

    #[test]
    fn index_step_selects_the_array_item_site() {
        let schema = schema();
        assert_eq!(
            at(&schema, &[PathStep::Key("items"), PathStep::Index(0)]),
            Some(("{name: string(suggest(x))}[]".into(), AlternativeSite::Item))
        );
        assert_eq!(
            at(&schema, &[PathStep::Key("items"), PathStep::Index(0), PathStep::Key("name")]),
            Some(("string(suggest(x))".into(), AlternativeSite::Value))
        );
        assert_eq!(at(&schema, &[PathStep::Index(0)]), None, "a leading index");
        assert_eq!(
            at(&schema, &[PathStep::Key("items"), PathStep::Index(0), PathStep::Index(1)]),
            None,
            "an index after an index"
        );
        // The segment helpers have no index step; they descend through `[]`.
        let shape = root_shape(&schema);
        let name = property_def_at_path(shape, &Value::Null, &["items", "name"]).expect("descends through []");
        assert_eq!(serialize_property_def(&name), "string(suggest(x))");
        assert_eq!(
            suggestions_for_path(&schema, &["items", "name"]).map(|query| query.items.len()),
            Some(1),
            "suggestions descend through `[]` too"
        );
    }

    #[test]
    fn empty_path_is_none_everywhere() {
        let schema = schema();
        assert!(suggestions_for_path(&schema, &[]).is_none());
        assert_eq!(at(&schema, &[]), None);
        assert!(property_def_at_path(root_shape(&schema), &Value::Null, &[]).is_none());
    }

    #[test]
    fn missing_match_is_none_everywhere() {
        let schema = schema();
        assert!(suggestions_for_path(&schema, &["nope"]).is_none());
        assert!(suggestions_for_path(&schema, &["a", "nope"]).is_none());
        assert_eq!(at(&schema, &[PathStep::Key("nope")]), None);
        assert_eq!(at(&schema, &[PathStep::Key("a"), PathStep::Key("nope")]), None);
        assert!(property_def_at_path(root_shape(&schema), &Value::Null, &["nope"]).is_none());
        assert!(property_def_at_path(root_shape(&schema), &Value::Null, &["a", "nope"]).is_none());
    }

    #[test]
    fn property_union_suggestions_come_from_the_first_suggesting_alternative() {
        let schema = schema();
        let query = suggestions_for_path(&schema, &["pick"]).expect("union property");
        let labels: Vec<_> = query.items.iter().map(|item| item.label.as_str()).collect();
        assert_eq!(labels, ["s"]);
        assert_eq!(
            at(&schema, &[PathStep::Key("pick")]),
            Some(("boolean | string(suggest(s))".into(), AlternativeSite::Value))
        );
    }

    #[test]
    fn root_union_key_resolves_to_the_union_of_declaring_arms() {
        let schema = parse_yaml_schema(&yaml("- { k: string, x: boolean }\n- { k: number }\n")).expect("root union");
        assert!(matches!(schema, SimplifiedSchema::Union(_)));
        let definitions = DefinitionTable::default();
        let (def, site) = def_at_path(&schema, &definitions, &Value::Null, &[PathStep::Key("k")]).expect("k");
        assert!(matches!(def, Cow::Owned(PropertyDef::Union(_))), "got {def:?}");
        assert_eq!(serialize_property_def(&def), "string | number");
        assert_eq!(site, AlternativeSite::Value);
        let (only, _) = def_at_path(&schema, &definitions, &Value::Null, &[PathStep::Key("x")]).expect("x");
        assert!(matches!(only, Cow::Owned(PropertyDef::Single(_))), "one declaring arm: {only:?}");
    }
}

// ---------------------------------------------------------------------------

/// Function-schema parse and serialize round trip.
mod function_schema {
    use super::*;
    use darkmatter::markdown::schemas::simplified::{
        Constraint, FunctionParameters, FunctionType, PropertyAtom, to_json_schema,
    };

    const SIGNATURE: &str =
        "function(parameters(label: string, count?: number, ...rest: boolean); returns(string); category(Text)) -> Formats a label.";

    fn function(def: &PropertyDef) -> (&PropertyAtom, &FunctionType) {
        let PropertyDef::Single(atom) = def else {
            panic!("a function type is one atom: {def:?}");
        };
        let TypeExpr::Function(function) = &atom.ty else {
            panic!("not a function type: {:?}", atom.ty);
        };
        (atom, function)
    }

    #[test]
    fn parameters_markers_returns_and_metadata_parse() {
        let def = grammar::parse_type_definition("format", SIGNATURE).expect("signature parses");
        let (atom, function) = function(&def);
        let FunctionParameters::List(parameters) = &function.parameters else {
            panic!("expected a parameter list");
        };
        let summary: Vec<_> = parameters
            .iter()
            .map(|parameter| (parameter.name.as_str(), serialize_property_def(&parameter.def), parameter.optional, parameter.rest))
            .collect();
        assert_eq!(
            summary,
            [
                ("label", "string".to_string(), false, false),
                ("count", "number".to_string(), true, false),
                ("rest", "boolean".to_string(), false, true),
            ]
        );
        assert_eq!(serialize_property_def(&function.returns), "string");
        assert!(!function.fallible, "`fallible` defaults to false");
        assert!(function.narrows.is_none());
        assert!(function.examples.is_empty());
        assert_eq!(atom.constraints, [Constraint::Category("Text".into())]);
        assert_eq!(atom.description.as_deref(), Some("Formats a label."));
    }

    #[test]
    fn round_trip_preserves_the_default_fallible_and_metadata() {
        let def = grammar::parse_type_definition("format", SIGNATURE).expect("signature parses");
        let written = serialize_property_def(&def);
        let reparsed = grammar::parse_type_definition("format", &written).expect("serialized form parses");
        assert_eq!(reparsed, def, "round trip changed the definition; written as {written:?}");
        let (atom, function) = function(&reparsed);
        assert!(!function.fallible);
        assert_eq!(atom.description.as_deref(), Some("Formats a label."));
        assert_eq!(atom.constraints, [Constraint::Category("Text".into())]);
    }

    #[test]
    fn round_trip_preserves_an_authored_fallible_and_void_parameters() {
        let text = "function(parameters(void); returns(number | null); fallible) -> May fail.";
        let def = grammar::parse_type_definition("probe", text).expect("signature parses");
        let (_, parsed) = function(&def);
        assert!(parsed.fallible);
        assert_eq!(parsed.parameters, FunctionParameters::Void);
        let written = serialize_property_def(&def);
        let reparsed = grammar::parse_type_definition("probe", &written).expect("serialized form parses");
        assert_eq!(reparsed, def, "written as {written:?}");
        assert!(function(&reparsed).1.fallible);
    }

    #[test]
    fn lowering_a_function_type_is_a_convert_error() {
        let schema = parse_yaml_schema(&yaml(&format!("format: {SIGNATURE:?}\n"))).expect("schema parses");
        let lowered = to_json_schema(&schema);
        assert!(matches!(lowered, Err(SchemaError::Convert { ref property, .. }) if property == "format"), "got {lowered:?}");
    }
}

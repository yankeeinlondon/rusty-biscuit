//! Whole-YAML-document validation by top-level `kind`
//! (`DarkmatterSchemas::validate_kind_document`).
//!
//! Every cell starts from a real control document and applies one edit (two
//! where a cell needs a sibling key to isolate the field under test): the
//! shipped `darkmatter/schemas/schema.yaml` for `kind: schema`, and
//! Claudine's shipped `claudine/schemas/sequence.yaml` trigger for
//! `kind: schema-trigger` (`schema-trigger.yaml` is itself a `kind: schema`
//! document, so it cannot be the trigger kind's control).
//!
//! The agreement matrix uses the Rust envelope parsers as the oracle: a
//! document the YAML definition rejects is always rejected by Rust too. A cell
//! the YAML accepts and Rust rejects is a recorded gap, and each gap names the
//! Rust rule the definition cannot express.

use std::path::Path;

use darkmatter::markdown::schemas::{
    DarkmatterSchemas, parse_trigger_envelope_from_str,
    kind_documents::{KindDocuments, KindValidation, SHIPPED_KINDS},
};
use simplified_schema::{SchemaError, ValidationProblem, embedded::EmbeddedDocument, parse_standalone_schema_document};

const SCHEMA_CONTROL: &str = include_str!("../../../schemas/schema.yaml");
const TRIGGER_DEFINITION: &str = include_str!("../../../schemas/schema-trigger.yaml");
const TRIGGER_CONTROL: &str = include_str!("../../../../claudine/schemas/sequence.yaml");

const DOCUMENT: &str = "document.yaml";

fn api() -> (tempfile::TempDir, DarkmatterSchemas) {
    let dir = tempfile::tempdir().expect("temp dir");
    let api = DarkmatterSchemas::new(crate::request_support::context_at(dir.path()));
    (dir, api)
}

/// `control` with the top-level `key` (its line and every indented line after
/// it) replaced by `key: value`, or removed for `None`; appended when absent.
fn edit(control: &str, key: &str, value: Option<&str>) -> String {
    let lines: Vec<&str> = control.lines().collect();
    let prefix = format!("{key}:");
    let start = lines.iter().position(|line| {
        line.strip_prefix(prefix.as_str()).is_some_and(|rest| rest.is_empty() || rest.starts_with(' '))
    });
    let replacement = value.map(|value| if value.is_empty() { prefix.clone() } else { format!("{prefix} {value}") });
    let mut out: Vec<String> = Vec::new();
    match start {
        Some(start) => {
            let end = lines[start + 1..]
                .iter()
                .position(|line| !(line.is_empty() || line.starts_with(' ')))
                .map_or(lines.len(), |offset| start + 1 + offset);
            out.extend(lines[..start].iter().map(|line| (*line).to_owned()));
            out.extend(replacement);
            out.extend(lines[end..].iter().map(|line| (*line).to_owned()));
        }
        None => {
            out.extend(lines.iter().map(|line| (*line).to_owned()));
            out.extend(replacement);
        }
    }
    out.join("\n") + "\n"
}

fn edits(control: &str, changes: &[(&str, Option<&str>)]) -> String {
    changes.iter().fold(control.to_owned(), |text, (key, value)| edit(&text, key, *value))
}

/// The 1-based line of the top-level `key` in `text`.
fn line_of(text: &str, key: &str) -> u32 {
    let prefix = format!("{key}:");
    let index = text.lines().position(|line| line.starts_with(&prefix)).expect("key present");
    u32::try_from(index + 1).expect("small fixture")
}

fn validate(api: &DarkmatterSchemas, text: &str) -> Option<KindValidation> {
    api.validate_kind_document(text, Path::new(DOCUMENT)).expect("the document loads")
}

fn problems(api: &DarkmatterSchemas, text: &str) -> Vec<ValidationProblem> {
    let validation = validate(api, text).expect("a claimed kind document");
    assert_eq!(validation.report.valid, validation.report.problems.is_empty());
    validation.report.problems
}

fn problem_at<'a>(problems: &'a [ValidationProblem], path: &str) -> &'a ValidationProblem {
    problems
        .iter()
        .find(|problem| problem.path == path)
        .unwrap_or_else(|| panic!("no problem at `{path}`: {problems:#?}"))
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Yaml {
    Valid,
    /// Invalid, with a problem at this instance path.
    Invalid(&'static str),
}

struct Cell {
    name: &'static str,
    kind: &'static str,
    text: String,
    yaml: Yaml,
    rust_valid: bool,
    /// The Rust rule the YAML definition cannot express; set exactly when
    /// the YAML accepts what Rust rejects.
    gap: Option<&'static str>,
}

fn schema_cell(
    name: &'static str,
    changes: &[(&str, Option<&str>)],
    yaml: Yaml,
    rust_valid: bool,
    gap: Option<&'static str>,
) -> Cell {
    Cell { name, kind: "schema", text: edits(SCHEMA_CONTROL, changes), yaml, rust_valid, gap }
}

fn trigger_cell(
    name: &'static str,
    changes: &[(&str, Option<&str>)],
    yaml: Yaml,
    rust_valid: bool,
    gap: Option<&'static str>,
) -> Cell {
    Cell { name, kind: "schema-trigger", text: edits(TRIGGER_CONTROL, changes), yaml, rust_valid, gap }
}

const NULL_ADMITTED: &str = "an optional property admits null; Rust rejects an explicit null";
const OPEN_ROOT: &str = "a root schema always allows extra keys; Rust rejects unknown top-level keys";

/// The agreement matrix: one row per field shape of each validator, plus
/// every recorded gap.
fn cells() -> Vec<Cell> {
    use Yaml::{Invalid, Valid};
    vec![
        // kind: schema; the `schema` meta-type walked on `$schema`.
        schema_cell("schema control", &[], Valid, true, None),
        schema_cell("$schema wrong type", &[("$schema", Some("5"))], Invalid("/$schema"), false, None),
        schema_cell("$schema empty mapping", &[("$schema", Some("{}"))], Valid, true, None),
        schema_cell("$schema empty union", &[("$schema", Some("[]"))], Invalid("/$schema"), false, None),
        schema_cell("$schema invalid type string", &[("$schema", Some("{ a: \"nosuchtype(\" }"))], Invalid("/$schema"), false, None),
        schema_cell("$schema absent and no types", &[("$schema", None)], Invalid(""), false, None),
        schema_cell("$schema null and no types", &[("$schema", Some(""))], Invalid(""), false, None),
        schema_cell("$schema absent with types", &[("$schema", None), ("types", Some("{}"))], Valid, true, None),
        schema_cell("$schema null with types", &[("$schema", Some("")), ("types", Some("{}"))], Valid, false, Some(NULL_ADMITTED)),
        // `types`: control plus one invalid shape (same meta-type).
        schema_cell("types mapping", &[("types", Some("{ item: string }"))], Valid, true, None),
        schema_cell("types wrong type", &[("types", Some("5"))], Invalid("/types"), false, None),
        schema_cell(
            "types sequence",
            &[("types", Some("[{ a: string }]"))],
            Valid,
            false,
            Some("the `schema` meta-type accepts a sequence; Rust requires `types` to be a mapping"),
        ),
        schema_cell("types null", &[("$schema", None), ("types", Some(""))], Invalid(""), false, None),
        // `string` on `description`.
        schema_cell("description wrong type", &[("description", Some("5"))], Invalid("/description"), false, None),
        schema_cell("description empty", &[("description", Some("\"\""))], Valid, true, None),
        schema_cell("description absent", &[("description", None)], Valid, true, None),
        schema_cell("description null", &[("description", Some(""))], Valid, false, Some(NULL_ADMITTED)),
        // `string[]` on `categories`.
        schema_cell("categories list", &[("categories", Some("[Tools, Text -> text helpers]"))], Valid, true, None),
        schema_cell("categories whole field wrong", &[("categories", Some("foo"))], Invalid("/categories"), false, None),
        schema_cell("categories one element wrong", &[("categories", Some("[a, 1]"))], Invalid("/categories/1"), false, None),
        schema_cell("categories every element wrong", &[("categories", Some("[1]"))], Invalid("/categories/0"), false, None),
        schema_cell("categories empty", &[("categories", Some("[]"))], Valid, true, None),
        schema_cell("categories null", &[("categories", Some(""))], Valid, false, Some(NULL_ADMITTED)),
        // Open root.
        schema_cell("schema name key", &[("name", Some("schema"))], Valid, false, Some(OPEN_ROOT)),
        // kind: schema-trigger; `match-payload` walked on `match`.
        trigger_cell("trigger control", &[], Valid, true, None),
        trigger_cell("match absent", &[("match", None)], Invalid(""), false, None),
        trigger_cell("match null", &[("match", Some(""))], Invalid("/match"), false, None),
        trigger_cell("match wrong type", &[("match", Some("5"))], Invalid("/match"), false, None),
        trigger_cell("match element wrong type", &[("match", Some("[1]"))], Invalid("/match/0"), false, None),
        trigger_cell("match list", &[("match", Some("[{ sequence: string(required) }]"))], Valid, true, None),
        trigger_cell(
            "match empty list",
            &[("match", Some("[]"))],
            Valid,
            false,
            Some("an empty `match` list is a Rust lint: it would match every document"),
        ),
        trigger_cell(
            "match empty mapping",
            &[("match", Some("{}"))],
            Valid,
            false,
            Some("an empty condition mapping is a Rust lint: it would match every document"),
        ),
        trigger_cell("match group wrong type", &[("match", Some("{ group: 5 }"))], Invalid("/match/group"), false, None),
        trigger_cell("match $path wrong type", &[("match", Some("{ $path: 5 }"))], Invalid("/match/$path"), false, None),
        trigger_cell("match $path glob", &[("match", Some("{ $path: \"**/*.md\" }"))], Valid, true, None),
        trigger_cell(
            "match retired any",
            &[("match", Some("{ any: [{ sequence: string(required) }] }"))],
            Valid,
            false,
            Some("`any` is retired in Rust; in YAML it is an ordinary property condition"),
        ),
        trigger_cell(
            "match min-match count too large",
            &[("match", Some("{ min-match: { count: 9, of: [{ sequence: string(required) }] } }"))],
            Valid,
            false,
            Some("`min-match.count` bounds depend on the number of children, a Rust rule"),
        ),
        // `type-definition` on a property condition.
        trigger_cell("condition type string", &[("match", Some("{ sequence: string(required) }"))], Valid, true, None),
        trigger_cell("condition wrong type", &[("match", Some("{ sequence: 5 }"))], Invalid("/match/sequence"), false, None),
        // The trigger's `$schema` (same meta-type as the schema kind's).
        trigger_cell("trigger $schema absent", &[("$schema", None)], Valid, true, None),
        trigger_cell("trigger $schema wrong type", &[("$schema", Some("5"))], Invalid("/$schema"), false, None),
        trigger_cell("trigger $schema null", &[("$schema", Some(""))], Valid, false, Some(NULL_ADMITTED)),
        trigger_cell("trigger extra key", &[("extra", Some("1"))], Valid, false, Some(OPEN_ROOT)),
        trigger_cell("trigger types key", &[("types", Some("{}"))], Valid, false, Some(OPEN_ROOT)),
    ]
}

fn rust_valid(cell: &Cell) -> bool {
    match cell.kind {
        "schema" => matches!(parse_standalone_schema_document(&cell.text, Path::new(DOCUMENT)), Ok(Some(_))),
        _ => matches!(parse_trigger_envelope_from_str(&cell.text), Ok(Some(_))),
    }
}

#[test]
fn the_agreement_matrix_holds_and_every_gap_is_explained() {
    let (_dir, api) = api();
    for cell in cells() {
        let name = cell.name;
        let validation = validate(&api, &cell.text).unwrap_or_else(|| panic!("{name}: unclaimed\n{}", cell.text));
        assert_eq!(validation.kind, cell.kind, "{name}");
        let observed = if validation.report.valid {
            Yaml::Valid
        } else {
            let problem = problem_at(&validation.report.problems, match cell.yaml {
                Yaml::Invalid(path) => path,
                Yaml::Valid => panic!("{name}: expected valid, got {:#?}\n{}", validation.report.problems, cell.text),
            });
            assert!(problem.line.is_some() && problem.column.is_some(), "{name}: unlocated {problem:?}");
            cell.yaml
        };
        assert_eq!(observed, cell.yaml, "{name}\n{}", cell.text);
        assert_eq!(rust_valid(&cell), cell.rust_valid, "{name}: Rust oracle\n{}", cell.text);
        if observed != Yaml::Valid {
            assert!(!cell.rust_valid, "{name}: the definition rejects a document Rust accepts; fix the YAML");
        }
        assert_eq!(
            cell.gap.is_some(),
            observed == Yaml::Valid && !cell.rust_valid,
            "{name}: a gap is recorded exactly where YAML accepts what Rust rejects",
        );
    }
}

#[test]
fn a_valid_control_document_reports_its_kind_and_no_problems() {
    let (_dir, api) = api();
    for (text, kind) in [(SCHEMA_CONTROL, "schema"), (TRIGGER_CONTROL, "schema-trigger")] {
        let validation = validate(&api, text).expect("claimed");
        assert_eq!(validation.kind, kind);
        assert!(validation.report.valid, "{kind}: {:#?}", validation.report.problems);
        assert!(validation.report.pending.is_empty() && validation.report.deferred.is_empty());
    }
}

#[test]
fn only_a_string_kind_naming_a_shipped_kind_is_claimed() {
    let (_dir, api) = api();
    for (name, text) in [
        ("kind absent", edit(SCHEMA_CONTROL, "kind", None)),
        ("kind null", edit(SCHEMA_CONTROL, "kind", Some(""))),
        ("kind number", edit(SCHEMA_CONTROL, "kind", Some("5"))),
        ("kind quoted number", edit(SCHEMA_CONTROL, "kind", Some("\"5\""))),
        ("kind list", edit(SCHEMA_CONTROL, "kind", Some("[schema]"))),
        ("kind other", edit(SCHEMA_CONTROL, "kind", Some("task"))),
        ("kind case", edit(SCHEMA_CONTROL, "kind", Some("Schema"))),
        ("retired trigger kind", edit(TRIGGER_CONTROL, "kind", Some("trigger-schema"))),
        ("top-level sequence", "- kind: schema\n".to_owned()),
        ("top-level scalar", "schema\n".to_owned()),
        ("empty document", String::new()),
    ] {
        assert!(validate(&api, &text).is_none(), "{name} must be unclaimed");
    }
    // Quoted and native YAML strings claim alike.
    let quoted = edit(SCHEMA_CONTROL, "kind", Some("\"schema\""));
    assert_eq!(validate(&api, &quoted).expect("claimed").kind, "schema");
}

/// The loader's document shapes: one loader, tested once.
#[test]
fn a_document_that_does_not_load_is_a_located_schema_document_error() {
    let (_dir, api) = api();
    let duplicate = SCHEMA_CONTROL.replacen("description:", "description: first\ndescription:", 1);
    let trailing = format!("{SCHEMA_CONTROL}  garbage: [\n");
    let second = format!("{SCHEMA_CONTROL}---\nkind: schema\ntypes: {{}}\n");
    let integer_key = edit(SCHEMA_CONTROL, "1", Some("one"));
    for (name, text, expected) in [
        ("duplicate key", duplicate, "not valid YAML: duplicate entry with key \"description\""),
        ("trailing invalid content", trailing, "not valid YAML: "),
        ("second document", second, "more than one document"),
        ("non-string key", integer_key, "cannot be read as a kind document"),
    ] {
        let error = api
            .validate_kind_document(&text, Path::new(DOCUMENT))
            .expect_err(name);
        let SchemaError::SchemaDocument { path, message, .. } = &error else {
            panic!("{name}: expected SchemaDocument, got {error:?}");
        };
        assert_eq!(path, Path::new(DOCUMENT), "{name}");
        assert!(message.contains(expected), "{name}: {message}");
        // Rust rejects the same text.
        assert!(parse_standalone_schema_document(&text, Path::new(DOCUMENT)).is_err(), "{name}");
    }
}

/// One cell per validator names the line and column of the offending key or
/// value; a missing key or failed union arm names the root mapping.
#[test]
fn problems_are_located_at_the_offending_value_or_the_parent_mapping() {
    let (_dir, api) = api();
    let located = |text: &str, path: &str| {
        let problems = problems(&api, text);
        let problem = problem_at(&problems, path);
        (problem.line.expect("line"), problem.column.expect("column"))
    };

    let text = edit(SCHEMA_CONTROL, "$schema", Some("5"));
    assert_eq!(located(&text, "/$schema"), (line_of(&text, "$schema"), 10), "schema meta-type");

    let text = edit(SCHEMA_CONTROL, "description", Some("5"));
    assert_eq!(located(&text, "/description"), (line_of(&text, "description"), 14), "string");

    let text = edit(SCHEMA_CONTROL, "categories", Some("[a, 1]"));
    assert_eq!(located(&text, "/categories/1"), (line_of(&text, "categories"), 17), "string[] element");

    let text = edit(SCHEMA_CONTROL, "$schema", None);
    assert_eq!(located(&text, ""), (line_of(&text, "kind"), 1), "required arm at the root mapping");

    let text = edit(TRIGGER_CONTROL, "match", None);
    assert_eq!(located(&text, ""), (line_of(&text, "kind"), 1), "missing match at the root mapping");

    let text = edit(TRIGGER_CONTROL, "match", Some("5"));
    assert_eq!(located(&text, "/match"), (line_of(&text, "match"), 8), "match-payload");

    let text = edit(TRIGGER_CONTROL, "match", Some("\n    sequence: 5"));
    assert_eq!(located(&text, "/match/sequence"), (line_of(&text, "match") + 1, 15), "type-definition");
}

#[test]
fn the_problem_names_the_missing_property() {
    let (_dir, api) = api();
    let problems = problems(&api, &edit(TRIGGER_CONTROL, "match", None));
    let problem = problem_at(&problems, "");
    assert!(problem.message.contains("\"match\" is a required property"), "{problem:?}");
}

/// Both shipped definitions are `kind: schema` documents that validate
/// against the shipped registry, `schema.yaml` against itself included.
#[test]
fn the_shipped_definitions_validate_against_the_shipped_registry() {
    let (_dir, api) = api();
    for (path, text) in [
        ("darkmatter/schemas/schema.yaml", SCHEMA_CONTROL),
        ("darkmatter/schemas/schema-trigger.yaml", TRIGGER_DEFINITION),
    ] {
        let validation = api.validate_kind_document(text, Path::new(path)).expect("loads").expect("claimed");
        assert_eq!(validation.kind, "schema", "{path}");
        assert!(validation.report.valid, "{path}: {:#?}", validation.report.problems);
    }
    let paths: Vec<&str> = SHIPPED_KINDS.iter().map(|(_, path)| *path).collect();
    assert_eq!(paths, ["darkmatter/schemas/schema.yaml", "darkmatter/schemas/schema-trigger.yaml"]);
    assert!(api.effective_for_kind("schema").expect("compiles").is_some());
    assert!(api.effective_for_kind("schema-trigger").expect("compiles").is_some());
    assert!(api.effective_for_kind("task").expect("no kind").is_none());
}

/// Editing a definition changes validation with no Rust change.
#[test]
fn a_modified_definition_changes_the_verdict_without_a_rust_change() {
    let (_dir, api) = api();
    let modified = SCHEMA_CONTROL.replace("      description: string ->", "      description: string(required) ->");
    assert_eq!(modified.matches("description: string(required)").count(), 2, "both arms edited");
    let documents = [EmbeddedDocument { path: "darkmatter/schemas/schema.yaml", text: &modified }];
    let registry = KindDocuments::from_documents(&[("schema", "darkmatter/schemas/schema.yaml")], &documents)
        .expect("the modified definition prepares");
    let modified_api = api.clone().with_kind_documents(registry);

    let document = edit(SCHEMA_CONTROL, "description", None);
    assert!(validate(&api, &document).expect("claimed").report.valid, "shipped: description is optional");

    let validation = validate(&modified_api, &document).expect("claimed");
    assert!(!validation.report.valid);
    let problem = problem_at(&validation.report.problems, "");
    assert!(problem.message.contains("\"description\" is a required property"), "{problem:?}");
    assert_eq!((problem.line, problem.column), (Some(line_of(&document, "kind")), Some(1)));

    // The modified registry holds only what it was given.
    assert!(validate(&modified_api, TRIGGER_CONTROL).is_none(), "schema-trigger is not registered there");
    assert!(validate(&modified_api, SCHEMA_CONTROL).expect("claimed").report.valid);
}

#[test]
fn a_registry_entry_that_does_not_resolve_is_an_error() {
    let error = KindDocuments::from_documents(&[("schema", "missing.yaml")], &[]).expect_err("unregistered path");
    assert!(error.to_string().contains("not a registered embedded schema document"), "{error}");
}

/// Validation reads nothing from the document path.
#[test]
fn validation_never_reads_the_named_path() {
    let (dir, api) = api();
    let path = dir.path().join("absent.yaml");
    let validation = api.validate_kind_document(SCHEMA_CONTROL, &path).expect("loads").expect("claimed");
    assert!(validation.report.valid);
    assert!(!path.exists());
}

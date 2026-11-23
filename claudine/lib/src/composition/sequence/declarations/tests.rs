//! Placement, shape, and preparation of sequence declarations, observed
//! through the public resolver (`resolve_sequence_plan`) on direct YAML,
//! direct Markdown, and referenced documents.

use std::fs;
use std::path::{Path, PathBuf};

use serde_json::json;
use tempfile::TempDir;

use crate::composition::error::CompositionError;
use crate::composition::sequence::model::SequencePlan;
use crate::composition::sequence::{reserved, resolve_sequence_plan};
use crate::composition::types::ResolvedCompositionSource;

/// Resolve `path` as the document Claudine was asked to run.
fn resolve_file(path: &Path) -> Result<Option<SequencePlan>, CompositionError> {
    let source = if path.extension().is_some_and(|extension| extension == "md") {
        crate::composition::resolve_fixture_source(path.to_str().unwrap())?
    } else {
        ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path.to_path_buf(),
            original_text: fs::read_to_string(path).unwrap(),
            markdown: crate::composition::resolve::load_yaml_document(path)?,
        }
    };
    resolve_sequence_plan(&source, &crate::test_support::context_for(path))
}

fn write(dir: &TempDir, name: &str, text: &str) -> PathBuf {
    let path = dir.path().join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&path, text).unwrap();
    path
}

/// A direct YAML sequence document holding `extra` beside `sequence`.
fn direct(dir: &TempDir, extra: &str, sequence: &str) -> Result<Option<SequencePlan>, CompositionError> {
    let path = write(dir, "s.yaml", &format!("kind: sequence\n{extra}sequence: {sequence}\n"));
    resolve_file(&path)
}

fn misplaced(error: &CompositionError) -> (&str, &str) {
    match error {
        CompositionError::SequenceDeclarationMisplaced { property, hint, .. } => (property, hint),
        other => panic!("expected SequenceDeclarationMisplaced, got {other:?}"),
    }
}

fn shape(error: &CompositionError) -> (&str, &str) {
    match error {
        CompositionError::SequenceDeclarationShape { property, message, .. } => (property, message),
        other => panic!("expected SequenceDeclarationShape, got {other:?}"),
    }
}

// -- Root placement ---------------------------------------------------------

/// `$state` and `template` belong to an inline list. Beside a reference or an
/// expression source (even as `null`, which is still present) they are
/// typed placement errors, never silently ignored or applied to the target.
#[test]
fn root_declarations_beside_a_non_list_source_are_misplaced() {
    let dir = TempDir::new().unwrap();
    write(&dir, "child.yaml", "kind: sequence\nsequence: [alpha]\n");
    for (extra, sequence, key) in [
        ("$state:\n  color: string\n", "child.yaml", "$state"),
        ("$state: null\n", "child.yaml", "$state"),
        ("template:\n  desc: x\n", "child.yaml", "template"),
        ("template: {}\n", "\"{{ items }}\"", "template"),
    ] {
        let error = direct(&dir, extra, sequence).unwrap_err();
        let (property, hint) = misplaced(&error);
        assert_eq!(property, key, "{extra}");
        assert!(hint.contains("referenced whole document"), "{hint}");
    }
    // The same rule in Markdown frontmatter.
    let markdown = write(&dir, "s.md", "---\nsequence: child.yaml\n$state: {}\n---\nBody\n");
    assert_eq!(misplaced(&resolve_file(&markdown).unwrap_err()).0, "$state");
}

/// Root `$schema` is the document schema, not a declaration: it is allowed
/// beside any source and never judges a step's state.
#[test]
fn root_schema_is_not_a_state_declaration() {
    let dir = TempDir::new().unwrap();
    write(&dir, "child.yaml", "kind: sequence\nsequence: [alpha]\n");
    let plan = direct(&dir, "$schema:\n  spec: string(required)\n", "child.yaml").unwrap().unwrap();
    assert_eq!(plan.steps.len(), 1);
    // Steps lacking `spec` are not judged by the document's schema.
    let plan = direct(&dir, "$schema:\n  spec: string(required)\n", "[alpha, beta]").unwrap().unwrap();
    assert_eq!(plan.steps.len(), 2);
}

// -- Shapes -----------------------------------------------------------------

/// Absent means no contract; `{}` is an open schema and an explicit no-op
/// template; `null`, empty, scalar, and invalid-arm shapes are located
/// errors, judged even when the list is empty.
#[test]
fn declaration_shapes_are_judged_by_presence_on_an_empty_list() {
    let dir = TempDir::new().unwrap();
    // Control: no declaration, an open schema, a no-op template.
    for extra in ["", "$state: {}\n", "template: {}\n", "$state: {}\ntemplate: {}\n"] {
        assert!(direct(&dir, extra, "[]").unwrap().unwrap().steps.is_empty(), "{extra}");
        assert_eq!(direct(&dir, extra, "[{name: a, any: 1}]").unwrap().unwrap().steps.len(), 1, "{extra}");
    }
    for (extra, property, needle) in [
        ("$state: null\n", "$state", "must not be null"),
        ("$state: []\n", "$state", "at least one"),
        ("$state: \"\"\n", "$state", "must not be empty"),
        ("$state: 42\n", "$state", "got number"),
        ("$state: true\n", "$state", "got boolean"),
        ("$state:\n  - {}\n  - 3\n", "$state[1]", "got number"),
        ("$state:\n  - \"\"\n", "$state[0]", "an empty string"),
        ("template: null\n", "template", "must not be null"),
        ("template: [a]\n", "template", "must be an object, got array"),
        ("template: text\n", "template", "must be an object, got string"),
    ] {
        let error = direct(&dir, extra, "[]").unwrap_err();
        let (found, message) = shape(&error);
        assert_eq!(found, property, "{extra}");
        assert!(message.contains(needle), "{extra}: {message}");
    }
}

/// A well-shaped `$state` that fails to parse or resolve is a load error
/// carrying the schema layer's typed cause, on an empty list too.
#[test]
fn an_unloadable_state_schema_fails_an_empty_list() {
    let dir = TempDir::new().unwrap();
    for extra in ["$state: ./missing.schema.yaml\n", "$state:\n  rank: \"number(\"\n"] {
        let error = direct(&dir, extra, "[]").unwrap_err();
        match &error {
            CompositionError::SequenceDeclarationLoad { property, source_path, .. } => {
                assert_eq!(property, "$state");
                assert!(source_path.ends_with("s.yaml"), "{}", source_path.display());
            }
            other => panic!("{extra}: expected SequenceDeclarationLoad, got {other:?}"),
        }
        // The cause stays downcastable to the schema layer's own error.
        let cause = std::error::Error::source(&error).expect("typed cause");
        let schema_error = cause
            .downcast_ref::<simplified_schema::SchemaError>()
            .or_else(|| cause.downcast_ref::<Box<simplified_schema::SchemaError>>().map(AsRef::as_ref));
        assert!(schema_error.is_some(), "{extra}: {cause}");
    }
}

/// A template default may write neither generated/overlay state nor a
/// declaration, whether or not the list has items.
#[test]
fn template_defaults_cannot_write_declarations() {
    let dir = TempDir::new().unwrap();
    for key in ["$state", "template", "$schema", "id", "seq"] {
        let extra = format!("template:\n  \"{key}\": {{}}\n");
        for sequence in ["[]", "[alpha]"] {
            let error = direct(&dir, &extra, sequence).unwrap_err();
            assert!(
                matches!(&error, CompositionError::SequenceReservedTemplateKey(found) if found == key),
                "{key} on {sequence}: {error:?}"
            );
        }
    }
}

// -- Task placement ---------------------------------------------------------

/// An inline nested sequence (directly, as a group leg, or in a task file)
/// owns `$schema`/`$state`/`template` as options; the parent never reads them
/// as its step's state.
#[test]
fn inline_containers_accept_their_own_declarations() {
    let dir = TempDir::new().unwrap();
    let declarations = "{ \"$schema\": { spec: string }, \"$state\": { color: string }, template: { color: red } }";
    let nested = format!("[{{ name: outer, note: kept, sequence: [inner], {} }}]", &declarations[2..declarations.len() - 2]);
    let plan = direct(&dir, "", &nested).unwrap().unwrap();
    let step = &plan.steps[0];
    assert_eq!(step.state.extra.get("note"), Some(&json!("kept")));
    for key in ["$schema", "$state", "template"] {
        assert!(!step.state.extra.contains_key(key), "{key} is not step state");
        assert!(step.executable.as_ref().unwrap().options.contains_key(key), "{key} is a container option");
    }
    // A group leg's inline sequence.
    let leg = "[{ name: g, group: { tasks: [{ name: leg, sequence: [a], \"$state\": {} }] } }]";
    assert_eq!(direct(&dir, "", leg).unwrap().unwrap().steps.len(), 1);
}

/// Beside every other executable, and on a step that runs the body, a task
/// declaration is a located placement error naming where it belongs.
#[test]
fn task_declarations_beside_a_non_list_are_misplaced() {
    let dir = TempDir::new().unwrap();
    write(&dir, "child.yaml", "kind: sequence\nsequence: [alpha]\n");
    write(&dir, "p.md", "---\ntitle: p\n---\nBody\n");
    for (task, key, needle) in [
        ("{ name: a, prompt: p.md, \"$state\": {} }", "$state", "wrap it in an inline"),
        ("{ name: a, shell: \"echo hi\", template: {} }", "template", "wrap it in an inline"),
        ("{ name: a, side_effect: { say: hi }, \"$schema\": {} }", "$schema", "wrap it in an inline"),
        ("{ name: a, \"$state\": null }", "$state", "runs the document body"),
        ("{ name: a, sequence: child.yaml, \"$state\": {} }", "$state", "at that document's root"),
        ("{ name: a, group: { tasks: [{ name: l, shell: x }] }, template: {} }", "template", "inline sequence leg"),
    ] {
        let error = direct(&dir, "", &format!("[{task}]")).unwrap_err();
        let (property, hint) = misplaced(&error);
        assert_eq!(property, format!("sequence[0].{key}"), "{task}");
        assert!(hint.contains(needle), "{task}: {hint}");
    }
    // A group object's grammar stays closed to declarations.
    let error = direct(&dir, "", "[{ name: g, group: { \"$state\": {}, tasks: [{ name: l, shell: x }] } }]").unwrap_err();
    assert_eq!(misplaced(&error).0, "sequence[0].group.$state");
}

/// Task declarations take the same shapes, located at the task.
#[test]
fn task_declaration_shapes_are_located_at_the_task() {
    let dir = TempDir::new().unwrap();
    for (key, value, property) in [
        ("$schema", "null", "sequence[0].$schema"),
        ("$state", "[]", "sequence[0].$state"),
        ("template", "3", "sequence[0].template"),
    ] {
        let error = direct(&dir, "", &format!("[{{ name: c, sequence: [], \"{key}\": {value} }}]")).unwrap_err();
        assert_eq!(shape(&error).0, property, "{key}: {value}");
    }
    // In a nested list, the path is the full container path.
    let error = direct(&dir, "", "[{ name: o, sequence: [{ name: i, sequence: [], \"$state\": 1 }] }]").unwrap_err();
    assert_eq!(shape(&error).0, "sequence[0].sequence[0].$state");
}

// -- `$state` judging -------------------------------------------------------

/// `$state` judges the immediate steps' normalized state, generated fields
/// included, and the violation names its owner.
#[test]
fn state_violations_name_the_step_and_the_declaring_owner() {
    let dir = TempDir::new().unwrap();
    let error = direct(&dir, "$state:\n  color: string(required)\n", "[{ name: blue, color: blue }, red]").unwrap_err();
    match &error {
        CompositionError::SequenceStateSchemaViolation { index, id, property, source_path, container, .. } => {
            assert_eq!((*index, id.as_str(), property.as_str()), (1, "red", "color"));
            assert_eq!(container, "sequence");
            assert!(source_path.ends_with("s.yaml"));
        }
        other => panic!("expected SequenceStateSchemaViolation, got {other:?}"),
    }
    let text = error.to_string();
    assert!(text.starts_with("sequence step 1 (`red`) failed schema validation at `color`:"), "{text}");
    assert!(text.contains("(`$state` of `sequence` in "), "{text}");

    // A referenced document's violation names that document, not the referrer.
    write(&dir, "sub/child.yaml", "kind: sequence\n$state:\n  rank: number(required)\nsequence: [alpha]\n");
    let referrer = write(&dir, "ref.md", "---\nsequence: sub/child.yaml\n---\nBody\n");
    match resolve_file(&referrer).unwrap_err() {
        CompositionError::SequenceStateSchemaViolation { source_path, .. } => {
            assert!(source_path.ends_with("sub/child.yaml"), "{}", source_path.display());
        }
        other => panic!("expected SequenceStateSchemaViolation, got {other:?}"),
    }
}

/// A relative `$state` reference resolves beside the declaring document,
/// not the referrer, so a wrong base cannot pass by accident.
#[test]
fn a_state_reference_resolves_in_the_declaring_documents_context() {
    let dir = TempDir::new().unwrap();
    write(&dir, "sub/state.schema.yaml", "kind: schema\n$schema:\n  rank: number(required)\n");
    write(&dir, "sub/child.yaml", "kind: sequence\n$state: ./state.schema.yaml\nsequence: [{ name: a, rank: 3 }]\n");
    let referrer = write(&dir, "ref.md", "---\nsequence: sub/child.yaml\n---\nBody\n");
    assert_eq!(resolve_file(&referrer).unwrap().unwrap().steps.len(), 1);

    write(&dir, "sub/bad.yaml", "kind: sequence\n$state: ./state.schema.yaml\nsequence: [{ name: a, rank: x }]\n");
    let referrer = write(&dir, "bad.md", "---\nsequence: sub/bad.yaml\n---\nBody\n");
    assert!(matches!(
        resolve_file(&referrer).unwrap_err(),
        CompositionError::SequenceStateSchemaViolation { ref property, .. } if property == "/rank"
    ));
}

// -- Value routes -----------------------------------------------------------

/// The one predicate every value route uses names the declaration a key
/// would replace, through a dotted or indexed path too.
#[test]
fn declaration_writes_are_detected_by_their_root() {
    for (key, expected) in [
        ("template", Some("template")),
        ("template.desc", Some("template")),
        ("$state", Some("$state")),
        ("$state[0]", Some("$state")),
        ("templates", None),
        ("$schema", None),
        ("state", None),
    ] {
        assert_eq!(reserved::declaration_written_by(key), expected, "{key}");
        assert_eq!(reserved::is_declaration_write(key), expected.is_some(), "{key}");
    }
    for key in reserved::SEQUENCE_DECLARATION_KEYS {
        assert!(reserved::is_reserved_state_key(key), "{key} cannot be authored as state");
    }
    let error = super::reject_declaration_write("template.desc", Path::new("s.yaml"), "a setter").unwrap_err();
    let (property, hint) = misplaced(&error);
    assert_eq!(property, "template.desc");
    assert!(hint.contains("a setter supplies values"), "{hint}");
    assert!(super::reject_declaration_write("spec", Path::new("s.yaml"), "a setter").is_ok());
}

/// A step may not author a declaration name as state.
#[test]
fn step_state_cannot_author_a_declaration_key() {
    let dir = TempDir::new().unwrap();
    let error = direct(&dir, "", "[{ name: a, shell: x, \"$state\": {} }]").unwrap_err();
    assert_eq!(misplaced(&error).0, "sequence[0].$state");
}

// -- The `$state` declaration matrix ----------------------------------------

/// The caller, the declaring document, its schema files, and an eager file
/// in four directories, so no reference can resolve from the wrong base by
/// accident: `../schemas/…` from the caller's directory names a folder that
/// does not exist.
struct Homes {
    _dir: TempDir,
    caller: PathBuf,
    declaring: PathBuf,
}

fn homes() -> Homes {
    let dir = TempDir::new().unwrap();
    let caller = dir.path().join("caller/deep");
    fs::create_dir_all(&caller).unwrap();
    let declaring = dir.path().join("decl");
    fs::create_dir_all(&declaring).unwrap();
    let schemas = dir.path().join("schemas");
    fs::create_dir_all(&schemas).unwrap();
    fs::write(schemas.join("state.schema.yaml"), "kind: schema\n$schema:\n  rank: number(required)\n").unwrap();
    fs::write(schemas.join("types.yaml"), "kind: schema\ntypes:\n  shade: \"enum(red, blue)\"\n").unwrap();
    fs::create_dir_all(dir.path().join("eager")).unwrap();
    fs::write(dir.path().join("eager/brief.md"), "# brief\n").unwrap();
    Homes { _dir: dir, caller, declaring }
}

impl Homes {
    /// Resolve `decl/s.yaml` (`$state: <state>` over `rows`) for a request
    /// launched in the caller's directory.
    fn resolve(&self, state: &str, rows: &str) -> Result<Option<SequencePlan>, CompositionError> {
        let path = self.declaring.join("s.yaml");
        fs::write(&path, format!("kind: sequence\n$state: {state}\nsequence: {rows}\n")).unwrap();
        let source = ResolvedCompositionSource {
            original_ref: path.display().to_string(),
            resolved_path: path.clone(),
            original_text: fs::read_to_string(&path).unwrap(),
            markdown: crate::composition::resolve::load_yaml_document(&path)?,
        };
        resolve_sequence_plan(&source, &crate::test_support::context_at(&self.caller))
    }
}

/// The schema layer's error behind a declaration load failure.
fn schema_cause(error: &CompositionError) -> &simplified_schema::SchemaError {
    let cause = std::error::Error::source(error).expect("a typed cause");
    cause
        .downcast_ref::<simplified_schema::SchemaError>()
        .or_else(|| cause.downcast_ref::<Box<simplified_schema::SchemaError>>().map(AsRef::as_ref))
        .unwrap_or_else(|| panic!("not a schema error: {cause}"))
}

/// Every accepted `$state` form resolves from the declaring document's
/// directory and judges the rows; each rejected form fails at its owner with
/// the schema layer's own cause or the failing arm's location. One edit from
/// a passing row turns each control into its negative.
#[test]
fn every_state_form_resolves_from_the_declaring_document() {
    let homes = homes();
    let good = "[{ name: a, rank: 1, color: red, brief: ../eager/brief.md }]";
    for state in [
        "{}",
        "{ rank: number(required) }",
        "../schemas/state.schema.yaml",
        "{ color: shade(required)@../schemas/types.yaml }",
        "{ brief: file(required;eager) }",
        "[{ rank: number(required) }, ../schemas/state.schema.yaml]",
    ] {
        let plan = homes.resolve(state, good).unwrap_or_else(|error| panic!("{state}: {error:?}")).unwrap();
        assert_eq!(plan.steps.len(), 1, "{state}");
        // Accepted forms constrain: a row breaking the declared type fails.
        if state != "{}" && !state.starts_with('[') {
            let bad = good.replace("rank: 1", "rank: x").replace("color: red", "color: green").replace("../eager/brief.md", "./brief.md");
            assert!(
                matches!(homes.resolve(state, &bad), Err(CompositionError::SequenceStateSchemaViolation { .. })),
                "{state} judged the row"
            );
        }
    }

    // Unloadable forms keep their owner and the schema layer's cause.
    for state in [
        "./state.schema.yaml",
        "{ color: shade@../schemas/missing.yaml }",
        "{ color: tint@../schemas/types.yaml }",
        "[{ rank: number }, ../schemas/missing.yaml]",
    ] {
        let error = homes.resolve(state, "[]").unwrap_err();
        match &error {
            CompositionError::SequenceDeclarationLoad { property, source_path, .. } => {
                assert_eq!(property, "$state", "{state}");
                assert!(source_path.ends_with("decl/s.yaml"), "{state}: {}", source_path.display());
            }
            other => panic!("{state}: expected SequenceDeclarationLoad, got {other:?}"),
        }
        let cause = schema_cause(&error).to_string();
        let named = if state.contains("tint") { "tint" } else if state.contains("missing") { "missing.yaml" } else { "state.schema.yaml" };
        assert!(cause.contains(named), "{state}: {cause}");
    }

    // A union whose arms are all malformed names the first failing arm.
    for (state, property) in [("[\"\", 3]", "$state[0]"), ("[3, {}]", "$state[0]"), ("[{}, ../schemas/state.schema.yaml, null]", "$state[2]")] {
        let error = homes.resolve(state, "[]").unwrap_err();
        assert_eq!(shape(&error).0, property, "{state}");
    }
}

/// A referenced document's `$state` resolves beside that document, never
/// beside its referrer: a schema file only the referrer's folder holds is a
/// load error owned by the referenced document.
#[test]
fn a_referenced_documents_unloadable_state_is_owned_by_that_document() {
    let dir = TempDir::new().unwrap();
    write(&dir, "state.schema.yaml", "kind: schema\n$schema:\n  rank: number\n");
    write(&dir, "sub/child.yaml", "kind: sequence\n$state: ./state.schema.yaml\nsequence: [{ name: a, rank: 3 }]\n");
    let referrer = write(&dir, "ref.md", "---\nsequence: sub/child.yaml\n---\nBody\n");
    let error = resolve_file(&referrer).unwrap_err();
    let CompositionError::SequenceDeclarationLoad { property, source_path, .. } = &error else {
        panic!("expected SequenceDeclarationLoad, got {error:?}");
    };
    assert_eq!(property, "$state");
    assert!(source_path.ends_with("sub/child.yaml"), "{}", source_path.display());
    assert!(schema_cause(&error).to_string().contains("state.schema.yaml"), "{error}");
}

/// The one step-state validator, over each shape a field can take. The
/// control row passes; each case is that row with one field edited.
/// Conversion the declared type permits is accepted, and the plan keeps the
/// authored value.
#[test]
fn the_state_validator_judges_every_field_shape_passively() {
    let dir = TempDir::new().unwrap();
    let state = "$state:\n  color: string(required)\n  rank: number(required)\n  tags: string[]\n  meta: \"{ note: string }\"\n";
    let control = json!({ "name": "a", "color": "red", "rank": 1, "tags": ["x"], "meta": { "note": "n" } });
    let resolve = |row: &serde_json::Value| direct(&dir, state, &format!("[{row}]"));
    resolve(&control).unwrap().unwrap();

    let edited = |field: &str, value: Option<serde_json::Value>| {
        let mut row = control.clone();
        match value {
            Some(value) => row[field] = value,
            None => {
                row.as_object_mut().unwrap().remove(field);
            }
        }
        row
    };
    // Allowed empties and permitted conversion.
    for (field, value) in [
        ("color", json!("")),
        ("tags", json!([])),
        ("meta", json!({})),
        ("rank", json!("3")),
    ] {
        let plan = resolve(&edited(field, Some(value.clone()))).unwrap_or_else(|error| panic!("{field}={value}: {error:?}")).unwrap();
        assert_eq!(plan.steps[0].state.extra[field], value, "{field}: the plan keeps the authored value");
    }
    // Absent, explicit null, whole-field wrong type, mixed and entirely
    // invalid elements.
    for (field, value, property) in [
        ("color", None, "color"),
        ("rank", None, "rank"),
        ("color", Some(json!(null)), "/color"),
        ("rank", Some(json!(null)), "/rank"),
        ("color", Some(json!(["red"])), "/color"),
        ("rank", Some(json!("high")), "/rank"),
        ("tags", Some(json!("x")), "/tags"),
        ("meta", Some(json!([])), "/meta"),
        ("tags", Some(json!(["x", {}])), "/tags/1"),
        ("tags", Some(json!([{}, []])), "/tags/0"),
    ] {
        let row = edited(field, value.clone());
        match resolve(&row).unwrap_err() {
            CompositionError::SequenceStateSchemaViolation { index, property: found, .. } => {
                assert_eq!((index, found.as_str()), (0, property), "{field}={value:?}");
            }
            other => panic!("{field}={value:?}: expected a state violation, got {other:?}"),
        }
    }
}

// -- The formal kind --------------------------------------------------------

/// A YAML file declaring `kind: sequence` with an inline list is judged by
/// Claudine's prepared formal kind once the runtime grammar has accepted it;
/// a Markdown document with the same frontmatter, a YAML file claiming no
/// kind, and a reference source are not.
#[test]
fn a_formal_yaml_sequence_is_judged_by_the_prepared_kind() {
    let dir = TempDir::new().unwrap();
    assert!(direct(&dir, "description: a summary\n", "[alpha, { name: beta }]").unwrap().is_some());
    assert!(direct(&dir, "", "[]").unwrap().is_some(), "an empty list is a valid formal sequence");

    let error = direct(&dir, "description: [not, text]\n", "[alpha]").unwrap_err();
    match &error {
        CompositionError::SchemaValidation { source_path, message, problems, .. } => {
            assert_eq!(source_path, &dir.path().join("s.yaml"));
            assert!(message.contains("`kind: sequence`") && message.contains("/description"), "{message}");
            assert_eq!(problems, &["/description".to_string()]);
        }
        other => panic!("expected SchemaValidation, got {other:?}"),
    }

    let unclaimed = write(&dir, "plain.yaml", "description: [not, text]\nsequence: [alpha]\n");
    assert!(resolve_file(&unclaimed).unwrap().is_some(), "a YAML file claiming no kind is not judged by it");
    let markdown = write(&dir, "doc.md", "---\nkind: sequence\ndescription: [not, text]\nsequence: [alpha]\n---\n");
    assert!(resolve_file(&markdown).unwrap().is_some(), "a Markdown document is not a formal YAML file");
    write(&dir, "child.yaml", "kind: sequence\nsequence: [alpha]\n");
    assert!(direct(&dir, "description: [not, text]\n", "child.yaml").unwrap().is_some(), "a reference keeps the runtime grammar");
}

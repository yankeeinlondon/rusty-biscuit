//! Input-robustness matrix for the YAML schema surfaces a union-array
//! declaration reads: the imported file's `kind`, `types`, and
//! `types.policy`, and the document's `$schema` and
//! `$schema.content_policy`.
//!
//! Each test starts from the Content Policy acceptance fixture, makes one edit
//! per cell, and asserts the public outcome of loading and validating the
//! document from disk. The control row proves the unedited fixture validates,
//! so each edit is known to be load-bearing. Neither file is modified.

use std::path::Path;

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaError};

use crate::request_support::context_at;
use crate::union_fixtures::{CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, content_policy_union_schema};

/// The control document's data: a mixed scalar/object list.
const CONTROL_DATA: &str =
    "content_policy: [\"ValidFor(3mo)\", {rule: \"ValidUntil(2027-01-01)\", action: archive}]\n";

/// The public outcome of loading and validating one document.
#[derive(Debug)]
enum Outcome {
    /// `Markdown::try_from` rejected the frontmatter.
    LoadFails,
    /// Schema preparation failed.
    PrepareFails(SchemaError),
    /// Validation ran; the problem paths (empty when valid).
    Validated(Vec<String>),
}

/// What a cell expects.
#[derive(Debug, Clone, Copy)]
enum Expect {
    LoadFails,
    PrepareFails,
    /// Preparation fails because the referenced name is not defined.
    UnknownName,
    Valid,
    /// Validation reports problems, all at or below `/content_policy`.
    Invalid,
}

fn temp_root() -> (tempfile::TempDir, std::path::PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// The control document's frontmatter with its `$schema` entry replaced by
/// `schema_entry` (no entry when empty).
fn document(schema_entry: &str) -> String {
    format!("---\n{schema_entry}{CONTROL_DATA}---\nBody with a [link](./x.md).\n")
}

fn control_schema_entry() -> String {
    format!("$schema:\n  content_policy: \"{CONTENT_POLICY_LIST}\"\n")
}

fn run(dir: &Path, schema_file: &str, document_text: &str) -> Outcome {
    let schema_path = dir.join(CONTENT_POLICY_FILE);
    let document_path = dir.join("doc.md");
    std::fs::write(&schema_path, schema_file).expect("write schema");
    std::fs::write(&document_path, document_text).expect("write document");
    let outcome = match Markdown::try_from(document_path.as_path()) {
        Err(_) => Outcome::LoadFails,
        Ok(markdown) => {
            let markdown = markdown.with_source(ComposeSource::File(document_path.clone()));
            match DarkmatterSchemas::new(context_at(dir)).validate(&markdown) {
                Err(error) => Outcome::PrepareFails(error),
                Ok(report) => {
                    assert_eq!(report.valid, report.problems.is_empty());
                    Outcome::Validated(report.problems.into_iter().map(|p| p.path).collect())
                }
            }
        }
    };
    assert_eq!(std::fs::read_to_string(&schema_path).expect("read schema"), schema_file);
    assert_eq!(std::fs::read_to_string(&document_path).expect("read document"), document_text);
    outcome
}

fn unknown_name(error: &SchemaError) -> bool {
    match error {
        SchemaError::UnknownNamedType { name, .. } => name == "policy",
        SchemaError::ReferencedSchema { source, .. } => unknown_name(source),
        _ => false,
    }
}

fn check(cell: &str, outcome: Outcome, expect: Expect) {
    let ok = match (&outcome, expect) {
        (Outcome::LoadFails, Expect::LoadFails) => true,
        (Outcome::PrepareFails(_), Expect::PrepareFails) => true,
        (Outcome::PrepareFails(error), Expect::UnknownName) => unknown_name(error),
        (Outcome::Validated(problems), Expect::Valid) => problems.is_empty(),
        (Outcome::Validated(problems), Expect::Invalid) => {
            !problems.is_empty() && problems.iter().all(|path| path.starts_with("/content_policy"))
        }
        _ => false,
    };
    assert!(ok, "{cell}: expected {expect:?}, got {outcome:?}");
}

/// Replaces `from` (which must occur exactly once) with `to`.
fn edit(text: &str, from: &str, to: &str) -> String {
    assert_eq!(text.matches(from).count(), 1, "`{from}` occurs exactly once");
    text.replacen(from, to, 1)
}

#[test]
fn imported_kind_cells() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let schema = content_policy_union_schema(shipped);
    let (_dir, root) = temp_root();
    let doc = document(&control_schema_entry());
    let kind = |to: &str| edit(&schema, "\nkind: schema\n", to);
    let cells: Vec<(&str, String, Expect)> = vec![
        ("control", schema.clone(), Expect::Valid),
        ("absent", kind("\n"), Expect::PrepareFails),
        ("null", kind("\nkind: null\n"), Expect::PrepareFails),
        ("empty-value null", kind("\nkind:\n"), Expect::PrepareFails),
        ("wrong type", kind("\nkind: 1\n"), Expect::PrepareFails),
        ("one wrong element", kind("\nkind: [schema, 1]\n"), Expect::PrepareFails),
        ("every element wrong", kind("\nkind: [1, 2]\n"), Expect::PrepareFails),
        ("empty", kind("\nkind: ''\n"), Expect::PrepareFails),
        ("duplicate key", kind("\nkind: schema\nkind: schema\n"), Expect::PrepareFails),
        ("trailing document", format!("{schema}---\nkind: schema\n"), Expect::PrepareFails),
        ("invalid YAML", format!("{schema}  : [\n"), Expect::PrepareFails),
    ];
    for (cell, text, expect) in cells {
        check(cell, run(&root, &text, &doc), expect);
    }
}

#[test]
fn imported_types_cells() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let schema = content_policy_union_schema(shipped);
    let (_dir, root) = temp_root();
    let doc = document(&control_schema_entry());
    let head = &schema[..schema.find("\ntypes:\n").expect("types table") + 1];
    let cells: Vec<(&str, String, Expect)> = vec![
        ("control", schema.clone(), Expect::Valid),
        ("absent", head.to_string(), Expect::PrepareFails),
        ("null", format!("{head}types: null\n"), Expect::PrepareFails),
        ("wrong type", format!("{head}types: 1\n"), Expect::PrepareFails),
        ("one wrong element", format!("{head}types: [{{policy: string}}, 1]\n"), Expect::PrepareFails),
        ("every element wrong", format!("{head}types: [1, 2]\n"), Expect::PrepareFails),
        ("empty", format!("{head}types: {{}}\n"), Expect::UnknownName),
        ("duplicate key", format!("{schema}types:\n  other: string\n"), Expect::PrepareFails),
        ("trailing document", format!("{schema}---\nextra: 1\n"), Expect::PrepareFails),
        ("invalid YAML", format!("{schema}  : [\n"), Expect::PrepareFails),
    ];
    for (cell, text, expect) in cells {
        check(cell, run(&root, &text, &doc), expect);
    }
}

#[test]
fn imported_policy_definition_cells() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let schema = content_policy_union_schema(shipped);
    let (_dir, root) = temp_root();
    let doc = document(&control_schema_entry());
    let policy = "  policy:\n    - short_form\n    - long_form\n";
    let with = |to: &str| edit(&schema, policy, to);
    let cells: Vec<(&str, String, Expect)> = vec![
        ("control", schema.clone(), Expect::Valid),
        ("absent", with(""), Expect::UnknownName),
        // A native null definition is the `null` type: the control list's
        // non-null items fail validation.
        ("null", with("  policy: null\n"), Expect::Invalid),
        ("wrong type", with("  policy: 1\n"), Expect::PrepareFails),
        ("one wrong element", with("  policy:\n    - short_form\n    - 1\n"), Expect::PrepareFails),
        ("every element wrong", with("  policy: [1, 2]\n"), Expect::PrepareFails),
        ("empty sequence", with("  policy: []\n"), Expect::PrepareFails),
        // An explicit empty mapping is a legal object definition: the target
        // changes, so the control list's scalar item fails validation.
        ("empty mapping", with("  policy: {}\n"), Expect::Invalid),
        ("duplicate key", format!("{schema}  policy: [short_form]\n"), Expect::PrepareFails),
        ("trailing document", format!("{schema}---\nextra: 1\n"), Expect::PrepareFails),
        ("invalid YAML", format!("{schema}  : [\n"), Expect::PrepareFails),
    ];
    for (cell, text, expect) in cells {
        check(cell, run(&root, &text, &doc), expect);
    }
}

#[test]
fn document_schema_cells() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let schema = content_policy_union_schema(shipped);
    let (_dir, root) = temp_root();
    let declaration = format!("{{content_policy: \"{CONTENT_POLICY_LIST}\"}}");
    let cells: Vec<(&str, String, Expect)> = vec![
        ("control", document(&control_schema_entry()), Expect::Valid),
        // No explicit schema: baseline-only behavior, never an empty explicit
        // schema that rejects the data.
        ("absent", document(""), Expect::Valid),
        ("null", document("$schema: null\n"), Expect::PrepareFails),
        ("empty-value null", document("$schema:\n"), Expect::PrepareFails),
        ("wrong type", document("$schema: 1\n"), Expect::PrepareFails),
        ("one wrong element", document(&format!("$schema: [{declaration}, 1]\n")), Expect::PrepareFails),
        ("every element wrong", document("$schema: [1, 2]\n"), Expect::PrepareFails),
        // An explicit empty shape declares nothing and allows extra keys.
        ("empty mapping", document("$schema: {}\n"), Expect::Valid),
        ("empty sequence", document("$schema: []\n"), Expect::PrepareFails),
        (
            "duplicate key",
            document(&format!("{}$schema: {{}}\n", control_schema_entry())),
            Expect::LoadFails,
        ),
        (
            "invalid frontmatter",
            document(&format!("{}broken: [\n", control_schema_entry())),
            Expect::LoadFails,
        ),
    ];
    for (cell, text, expect) in cells {
        check(cell, run(&root, &schema, &text), expect);
    }
}

#[test]
fn document_content_policy_declaration_cells() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let schema = content_policy_union_schema(shipped);
    let (_dir, root) = temp_root();
    let declared = |definition: &str| document(&format!("$schema:\n  content_policy: {definition}\n"));
    let list = format!("\"{CONTENT_POLICY_LIST}\"");
    let cells: Vec<(&str, String, Expect)> = vec![
        ("control", declared(&list), Expect::Valid),
        // No list declaration: no union validation of the data, and never an
        // empty union that rejects it.
        ("absent", document("$schema:\n  note: string\n"), Expect::Valid),
        // A native null declaration is the `null` type, which the control
        // list fails.
        ("null", declared("null"), Expect::Invalid),
        ("empty-value null", document("$schema:\n  content_policy:\n"), Expect::Invalid),
        ("wrong type", declared("1"), Expect::PrepareFails),
        ("one wrong element", declared(&format!("[{list}, 1]")), Expect::PrepareFails),
        ("every element wrong", declared("[1, 2]"), Expect::PrepareFails),
        ("empty string", declared("''"), Expect::PrepareFails),
        ("empty sequence", declared("[]"), Expect::PrepareFails),
        // An explicit object definition: the control list fails validation.
        ("empty mapping", declared("{}"), Expect::Invalid),
        (
            "duplicate key",
            document(&format!("$schema:\n  content_policy: {list}\n  content_policy: {list}\n")),
            Expect::LoadFails,
        ),
        (
            "invalid frontmatter",
            declared(&format!("\"{CONTENT_POLICY_LIST}")),
            Expect::LoadFails,
        ),
    ];
    for (cell, text, expect) in cells {
        check(cell, run(&root, &schema, &text), expect);
    }
}

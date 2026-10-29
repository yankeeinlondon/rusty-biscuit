//! Reader (a): the YAML SimplifiedSchema standalone envelope, walked from two
//! shipped schemas rather than hand-authored seeds. The seeds in
//! `schema_yaml_spellings` already pin these shapes against the classifier;
//! these cells add the real writers' presentation (four-space indentation,
//! quoted descriptions, interleaved comments) and assert only what a caller
//! sees: a schema error, or a schema that validates a sample document.
//!
//! Cells whose outcome waits on a pending ruling are absent: a null named
//! type (R5, today the documented `null` type) and an absent `kind` (R5).
//!
//! The `categories:` cells and the `function` type controls walk the
//! documented categories example, a types file whose one named type is a
//! function signature.

use super::validate_document;
use crate::schema_yaml_spellings::{Edit, Edit::*, apply};
use darkmatter::markdown::schemas::{SchemaError, parse_standalone_schema_document};

/// The public outcome of one cell.
#[derive(Clone, Copy)]
enum Outcome {
    /// The schema loads; each frontmatter instance validates as stated.
    Validates(&'static [(&'static str, bool)]),
    /// A recognized envelope with a malformed payload: `SchemaDocument`.
    Rejects,
    /// Loading fails and nothing usable is produced, with no recognized
    /// envelope to name.
    Fails,
}

use Outcome::*;

struct Cell {
    label: &'static str,
    edit: Edit,
    outcome: Outcome,
}

const fn cell(label: &'static str, edit: Edit, outcome: Outcome) -> Cell {
    Cell { label, edit, outcome }
}

/// Walks `cells` over `seed`, one schema file per cell so a path-keyed
/// validator cache never serves an earlier cell.
fn walk(seed_name: &str, seed: &str, cells: &[Cell]) {
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    let mut failures = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let source = apply(seed, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(source, seed, "{seed_name}/{}: the edit must change the seed", cell.label);
        }
        let file_name = format!("{seed_name}-{index}.yaml");
        std::fs::write(dir.join(&file_name), &source).expect("write cell schema");
        let reference = format!("$schema: ./{file_name}");
        let problem = match cell.outcome {
            Validates(instances) => instances.iter().find_map(|(instance, expected)| {
                match validate_document(dir, &format!("{reference}\n{instance}")) {
                    Ok(report) if report.valid == *expected => None,
                    Ok(report) => Some(format!(
                        "{instance:?}: expected valid={expected}, problems {:?}",
                        report.problems
                    )),
                    Err(error) => Some(format!("{instance:?}: schema failed to load: {error}")),
                }
            }),
            Rejects => match validate_document(dir, &reference) {
                Err(SchemaError::SchemaDocument { .. }) => None,
                other => Some(format!("expected SchemaDocument, got {other:?}")),
            },
            Fails => validate_document(dir, &reference)
                .ok()
                .map(|report| format!("expected a load error, got a report {report:?}")),
        };
        if let Some(problem) = problem {
            failures.push(format!("{seed_name}/{}: {problem}\n--- source ---\n{source}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

const TIMING_CONTROL: &[(&str, bool)] = &[
    ("timing:\n  document_ms: 5", true),
    ("timing:\n  document_ms: -1", false),
    ("timing:\n  document_ms: word", false),
];

/// `darkmatter/schemas/partials/timing.yaml`: the tagged envelope's `types`
/// map and its one named type `timing`.
const TAGGED_CELLS: &[Cell] = &[
    cell("control", Control, Validates(TIMING_CONTROL)),
    // `types`
    cell("types absent", Remove("types"), Rejects),
    cell("types null", Set("types", " null"), Rejects),
    cell("types number", Set("types", " 5"), Rejects),
    cell("types list", Set("types", " [timing]"), Rejects),
    cell("types one wrong element", Replace("    timing:\n", "    bad: 5\n    timing:\n"), Rejects),
    cell("types every element wrong", Set("types", "\n    first: 5\n    second: true"), Rejects),
    cell(
        "types empty mapping",
        Set("types", " {}"),
        Validates(&[("timing:\n  document_ms: -1", true)]),
    ),
    cell("types duplicate", Duplicate("types"), Rejects),
    cell("second document", Append("---\nkind: schema\ntypes: {}\n"), Rejects),
    cell("trailing garbage", Append("]\n"), Rejects),
    // named type `timing`
    cell("timing number", Set("timing", " 5"), Rejects),
    cell("timing bool", Set("timing", " true"), Rejects),
    cell("timing one wrong union arm", Set("timing", " [object, 5]"), Rejects),
    cell("timing every union arm wrong", Set("timing", " [5]"), Rejects),
    cell("timing empty union", Set("timing", " []"), Rejects),
    cell("timing empty string", Set("timing", " \"\""), Rejects),
    cell(
        "timing invalid member constraint",
        Replace("number(integer; min(0); generated)", "number(integer; bogus(0); generated)"),
        Rejects,
    ),
    cell("timing duplicate", Duplicate("timing"), Rejects),
];

const SLASH_VALID: &str = "last_updated: 2026-10-21\nagent: a\nmodel: m\nsupport: partial\nrequires_claudine_update: false";
const SLASH_BAD_SUPPORT: &str = "last_updated: 2026-10-21\nagent: a\nmodel: m\nsupport: bogus\nrequires_claudine_update: false";

/// `claudine/docs/research/slash-commands/_schema.yaml`: the pure envelope's
/// root `$schema` mapping.
const PURE_CELLS: &[Cell] = &[
    cell("control", Control, Validates(&[(SLASH_VALID, true), (SLASH_BAD_SUPPORT, false)])),
    // Removing the sole key leaves only comments: the requested schema has no
    // usable reading.
    cell("$schema absent", Remove("$schema"), Fails),
    cell("$schema null", Set("$schema", " null"), Rejects),
    cell("$schema number", Set("$schema", " 5"), Rejects),
    cell("$schema bool", Set("$schema", " true"), Rejects),
    cell("$schema one wrong element", Set("$schema", " [./a.yaml, 5]"), Rejects),
    cell("$schema every element wrong", Set("$schema", " [5]"), Rejects),
    cell("$schema empty mapping", Set("$schema", " {}"), Validates(&[(SLASH_BAD_SUPPORT, true)])),
    cell("$schema empty list", Set("$schema", " []"), Rejects),
    cell("$schema empty reference", Set("$schema", " \"\""), Rejects),
    cell("$schema duplicate", Duplicate("$schema"), Rejects),
    cell("$schema second document", Append("---\n$schema: {}\n"), Rejects),
    cell("$schema trailing garbage", Append("]\n"), Rejects),
    cell("member invalid constraint", Replace("date(required)", "date(bogus)"), Rejects),
];

#[test]
fn shipped_tagged_envelope_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/timing.shipped.yaml");
    walk("timing", seed, TAGGED_CELLS);
}

#[test]
fn shipped_pure_envelope_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/slash-commands.shipped.yaml");
    walk("slash-commands", seed, PURE_CELLS);
}

/// The public outcome of one types-file cell.
#[derive(Clone, Copy)]
enum TypesOutcome {
    /// The types file loads; used as a document schema it fails because its
    /// named type is a description-only `function`.
    Loads,
    /// The types file is rejected with a `SchemaDocument` error containing
    /// the text, and so is a document schema that references it.
    RejectsWith(&'static str),
}

use TypesOutcome::{Loads, RejectsWith};

struct TypesCell {
    label: &'static str,
    edit: Edit,
    outcome: TypesOutcome,
}

const fn types_cell(label: &'static str, edit: Edit, outcome: TypesOutcome) -> TypesCell {
    TypesCell { label, edit, outcome }
}

/// Walks `cells` over a types-file seed, asserting the standalone load result
/// and the result of using the file as a document schema.
fn walk_types(seed_name: &str, seed: &str, cells: &[TypesCell]) {
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    let mut failures = Vec::new();
    for (index, cell) in cells.iter().enumerate() {
        let source = apply(seed, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(source, seed, "{seed_name}/{}: the edit must change the seed", cell.label);
        }
        let file_name = format!("{seed_name}-{index}.yaml");
        let path = dir.join(&file_name);
        std::fs::write(&path, &source).expect("write cell schema");
        let loaded = parse_standalone_schema_document(&source, &path);
        let as_document = validate_document(dir, &format!("$schema: ./{file_name}"));
        let problem = match cell.outcome {
            Loads => match (&loaded, &as_document) {
                (Ok(Some(_)), Err(SchemaError::ReferencedSchema { source, .. }))
                    if matches!(source.as_ref(), SchemaError::Convert { property, message }
                        if property == "min" && message.contains("description-only")) => None,
                (Ok(Some(_)), other) => Some(format!(
                    "as a document schema the function must be a load error, got {other:?}"
                )),
                (other, _) => Some(format!("expected the types file to load, got {other:?}")),
            },
            RejectsWith(text) => match (&loaded, &as_document) {
                (Err(SchemaError::SchemaDocument { message, .. }), Err(SchemaError::SchemaDocument { .. }))
                    if message.contains(text) => None,
                (loaded, as_document) => Some(format!(
                    "expected SchemaDocument containing `{text}`, got {loaded:?}; as a document: {as_document:?}"
                )),
            },
        };
        if let Some(problem) = problem {
            failures.push(format!("{seed_name}/{}: {problem}\n--- source ---\n{source}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

/// The `categories:` list and the `category(...)` it governs.
const CATEGORY_CELLS: &[TypesCell] = &[
    types_cell("control", Control, Loads),
    types_cell("categories absent", Remove("categories"), Loads),
    types_cell("categories null", Set("categories", " null"), RejectsWith("`categories` must be a list")),
    types_cell("categories number", Set("categories", " 5"), RejectsWith("`categories` must be a list")),
    types_cell(
        "categories mapping",
        Set("categories", " {Math: numbers}"),
        RejectsWith("`categories` must be a list"),
    ),
    types_cell(
        "categories one wrong element",
        Replace("    - Devops\n", "    - Devops\n    - 5\n"),
        RejectsWith("`categories[2]`: a category entry must be a string"),
    ),
    types_cell(
        "categories every element wrong",
        Set("categories", " [5, true]"),
        RejectsWith("`categories[0]`: a category entry must be a string"),
    ),
    types_cell(
        "categories empty list",
        Set("categories", " []"),
        RejectsWith("category `Math` is not declared in `categories:`"),
    ),
    types_cell(
        "categories blank name",
        Replace("    - Devops\n", "    - \" -> unnamed\"\n"),
        RejectsWith("`categories[1]`: a category name cannot be blank"),
    ),
    types_cell(
        "categories duplicate name",
        Replace("    - Devops\n", "    - Math\n"),
        RejectsWith("`categories[1]`: duplicate category `Math` (first declared at `categories[0]`)"),
    ),
    types_cell("categories duplicate key", Duplicate("categories"), RejectsWith("not valid YAML")),
    types_cell(
        "category undeclared",
        Replace("category(Math)", "category(Text)"),
        RejectsWith("`types.min`: category `Text` is not declared"),
    ),
    types_cell(
        "category case differs",
        Replace("category(Math)", "category(math)"),
        RejectsWith("category `math` is not declared"),
    ),
    types_cell("category multi-line entry", Replace("category(Math)", "category(Filesystem)"), Loads),
    types_cell("category without description", Replace("category(Math)", "category(Devops)"), Loads),
];

/// The `function(...)` named type: accepted and rejected signature parts.
const FUNCTION_CELLS: &[TypesCell] = &[
    types_cell("control", Control, Loads),
    types_cell("void parameters", Replace("a: number, b: number", "void"), Loads),
    types_cell("optional and rest parameters", Replace("b: number", "b?: number, ...rest: number"), Loads),
    types_cell("fallible", Replace("category(Math);", "category(Math);\n        fallible;"), Loads),
    types_cell(
        "inline examples",
        Replace(
            "category(Math);",
            "category(Math);\n        example(min(3, 5) => 3);\n        example(min(\"a\", 5) => error);",
        ),
        Loads,
    ),
    types_cell("union return", Replace("returns(number)", "returns(number | null)"), Loads),
    types_cell(
        "parameters missing",
        RemoveLine("parameters(a: number, b: number);"),
        RejectsWith("requires `parameters(...)`"),
    ),
    types_cell("returns missing", RemoveLine("returns(number);"), RejectsWith("requires `returns(...)`")),
    types_cell(
        "empty parameters",
        Replace("(a: number, b: number)", "()"),
        RejectsWith("`parameters()` is not valid"),
    ),
    types_cell(
        "draft optional spelling",
        Replace("b: number)", "b: number(optional))"),
        RejectsWith("unknown constraint `optional"),
    ),
    types_cell(
        "unknown constraint",
        Replace("returns(number);", "returns(number); pure;"),
        RejectsWith("unknown constraint `pure`"),
    ),
    types_cell(
        "duplicate returns",
        Replace("returns(number);", "returns(number); returns(string);"),
        RejectsWith("duplicate `returns`"),
    ),
    types_cell(
        "duplicate parameter name",
        Replace("b: number)", "a: number)"),
        RejectsWith("duplicate parameter name `a`"),
    ),
    types_cell(
        "void mixed",
        Replace("a: number, b: number", "void, b: number"),
        RejectsWith("`void` cannot be mixed"),
    ),
    types_cell(
        "example result not a literal",
        Replace("category(Math);", "category(Math);\n        example(min(3, 5) => three);"),
        RejectsWith("an example result must be a literal"),
    ),
    types_cell(
        "duplicate overload",
        Replace(
            "    min: |-\n",
            "    min:\n    - 'function(parameters(x: number, y: number); returns(string))'\n    - |-\n",
        ),
        RejectsWith("the same binding contract"),
    ),
];

#[test]
fn documented_categories_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/categories.documented.yaml");
    walk_types("categories", seed, CATEGORY_CELLS);
}

#[test]
fn documented_function_type_matrix() {
    let seed = include_str!("../../fixtures/schema_enhancements_matrix/categories.documented.yaml");
    walk_types("function", seed, FUNCTION_CELLS);
}

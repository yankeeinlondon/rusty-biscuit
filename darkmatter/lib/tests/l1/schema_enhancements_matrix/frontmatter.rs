//! Reader (d): frontmatter YAML, composed as a document file through the
//! public compose path. The control is `md get --yaml` output for every
//! property, under a schema declaring each archetype; each cell edits that
//! output once.
//!
//! Absent here, because a later task decides them: numeric exactness,
//! overflow and underflow (Wave 2 Numeric), `numberlike`/`boolish`
//! preservation through containers and numeric-text conversion of list or
//! object members (Wave 3/4), tuple slots and union-arm values (Grammar,
//! then Schema adapter), and every JSON value-loading surface's duplicate
//! cell (R3).

use std::path::Path;

use darkmatter::markdown::{
    Markdown, MarkdownError,
    compose::{ComposeContext, ComposeOptions},
};
use serde_json::{Value, json};

use crate::schema_yaml_spellings::{Edit, Edit::*, apply};

const BODY: &str = "{{ n }}|{{ maybe }}|{{ zip }}|{{ flag }}|{{ is_null(maybe) }}\n";

#[derive(Clone, Copy)]
enum Outcome {
    /// Composes: the rendered body, and each listed property's composed value
    /// (`None`: the property is absent from the composed frontmatter).
    Composes(&'static str, fn() -> Vec<(&'static str, Option<Value>)>),
    /// The document file fails to load: a frontmatter parse error.
    ParseFails,
    /// Loads, then composition stops at schema validation.
    ValidationFails,
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

fn control_values() -> Vec<(&'static str, Option<Value>)> {
    vec![
        ("n", Some(json!(7))),
        ("maybe", Some(json!(2))),
        // Real producer output: `md get --yaml` quotes the numberlike text and
        // leaves `On` plain; both stay strings.
        ("zip", Some(json!("02134"))),
        ("flag", Some(json!("On"))),
        ("values", Some(json!([1, 2]))),
        ("meta", Some(json!({"level": 3}))),
    ]
}

const CELLS: &[Cell] = &[
    cell("control", Control, Composes("7|2|02134|On|false", control_values)),
    // required `n`
    cell("n absent", Remove("n"), ValidationFails),
    cell("n null", Set("n", " null"), ValidationFails),
    cell("n duplicate", Duplicate("n"), ParseFails),
    // optional `maybe`: absence and an explicit null are both valid.
    // Composition materializes an absent declared property as null, so the
    // two are distinct only in the (unchanged) source; no number is invented.
    cell(
        "maybe absent",
        Remove("maybe"),
        Composes("7||02134|On|true", || vec![("maybe", Some(Value::Null))]),
    ),
    cell(
        "maybe null",
        Set("maybe", " null"),
        Composes("7||02134|On|true", || vec![("maybe", Some(Value::Null))]),
    ),
    cell("maybe duplicate", Duplicate("maybe"), ParseFails),
    // `zip` and `flag` preservation and presence
    cell(
        "zip absent",
        Remove("zip"),
        Composes("7|2||On|false", || vec![("zip", Some(Value::Null)), ("flag", Some(json!("On")))]),
    ),
    cell(
        "flag null",
        Set("flag", " null"),
        Composes("7|2|02134||false", || vec![("flag", Some(Value::Null)), ("zip", Some(json!("02134")))]),
    ),
    cell("zip duplicate", Duplicate("zip"), ParseFails),
    // typed list `values`: absence never becomes `[]`
    cell(
        "values empty",
        Set("values", " []"),
        Composes("7|2|02134|On|false", || vec![("values", Some(json!([])))]),
    ),
    cell(
        "values absent",
        Remove("values"),
        Composes("7|2|02134|On|false", || vec![("values", Some(Value::Null))]),
    ),
    cell("values scalar", Set("values", " 5"), ValidationFails),
    cell("values duplicate", Duplicate("values"), ParseFails),
    // typed object `meta`: absence never becomes `{}`
    cell(
        "meta empty",
        Set("meta", " {}"),
        Composes("7|2|02134|On|false", || vec![("meta", Some(json!({})))]),
    ),
    cell(
        "meta absent",
        Remove("meta"),
        Composes("7|2|02134|On|false", || vec![("meta", Some(Value::Null))]),
    ),
    cell("meta nested duplicate", DuplicateLine("level: 3"), ParseFails),
    // whole frontmatter
    cell("trailing garbage", Append("]\n"), ParseFails),
    cell("document end then content", Append("...\nother: 1\n"), ParseFails),
];

/// Composes the cell's document file; the error cells must leave it
/// byte-for-byte unchanged.
fn check_cell(dir: &Path, index: usize, frontmatter: &str, outcome: Outcome) -> Result<(), String> {
    let path = dir.join(format!("doc-{index}.md"));
    let document = format!("---\n{frontmatter}---\n{BODY}");
    std::fs::write(&path, &document).expect("write cell document");
    let loaded = Markdown::try_from(path.as_path());
    let composed = loaded.as_ref().map_err(ToString::to_string).and_then(|markdown| {
        let options = ComposeOptions::new_with_context(ComposeContext::capture_for_content(dir, ""))
            .with_source_file(path.clone());
        markdown.compose_with(&crate::request_support::request(options)).map_err(|error| match error {
            MarkdownError::SchemaValidationFailed { .. } => "schema validation failed".to_string(),
            other => other.to_string(),
        })
    });
    let unchanged = std::fs::read_to_string(&path).expect("read back") == document;
    let result = match (outcome, loaded.is_ok(), composed) {
        (Composes(body, values), _, Ok((markdown, _))) => {
            let map = markdown.frontmatter().as_map();
            let mut wrong = Vec::new();
            if markdown.content().trim_end() != body {
                wrong.push(format!("body {:?}, expected {body:?}", markdown.content().trim_end()));
            }
            for (key, expected) in values() {
                if map.get(key) != expected.as_ref() {
                    wrong.push(format!("{key}: {:?}, expected {expected:?}", map.get(key)));
                }
            }
            if wrong.is_empty() { Ok(()) } else { Err(wrong.join("; ")) }
        }
        (ParseFails, false, Err(_)) => Ok(()),
        (ValidationFails, true, Err(error)) if error == "schema validation failed" => Ok(()),
        (_, loaded, result) => Err(format!(
            "unexpected result (loaded={loaded}): {:?}",
            result.map(|(markdown, _)| markdown.content().to_string())
        )),
    };
    if !unchanged {
        return Err("the source document was modified".into());
    }
    result
}

#[test]
fn frontmatter_value_matrix() {
    let control = include_str!("../../fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml");
    let schema = include_str!("../../fixtures/schema_enhancements_matrix/frontmatter.schema.yaml");
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    std::fs::write(dir.join("frontmatter.schema.yaml"), schema).expect("write schema");
    let mut failures = Vec::new();
    for (index, cell) in CELLS.iter().enumerate() {
        let frontmatter = apply(control, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(frontmatter, control, "{}: the edit must change the control", cell.label);
        }
        if let Err(problem) = check_cell(dir, index, &frontmatter, cell.outcome) {
            failures.push(format!("{}: {problem}\n--- frontmatter ---\n{frontmatter}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

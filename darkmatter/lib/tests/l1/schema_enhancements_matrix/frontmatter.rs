//! Reader (d): frontmatter YAML, composed as a document file through the
//! public compose path. The control is `md get --yaml` output for every
//! property, under a schema declaring each archetype; each cell edits that
//! output once.
//!
//! The conversion cells point the same control at a schema with one more
//! property per conversion archetype (`frontmatter.conversion.schema.yaml`):
//! numeric text in list and object members, `numberlike`/`boolish`
//! preservation through containers, tuple slots, pattern-key values, a
//! whole-value union, and a `json` content target, including duplicate keys
//! in its text (R3).

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
    // numeric admission: a native number no `f64` holds exactly, or that is
    // not finite, fails to load; quoted, it stays text and fails the type.
    cell("n 2^53 + 1", Set("n", " 9007199254740993"), ParseFails),
    cell("n -(2^53 + 1)", Set("n", " -9007199254740993"), ParseFails),
    cell("n 2^53 + 1 decimal", Set("n", " 9007199254740993.0"), ParseFails),
    cell("n 2^53 + 1 exponent", Set("n", " 9.007199254740993e15"), ParseFails),
    cell("n beyond u64", Set("n", " 99999999999999999999"), ParseFails),
    cell("n underflow", Set("n", " 1e-400"), ParseFails),
    cell("n inf", Set("n", " .inf"), ParseFails),
    cell("n nan", Set("n", " .nan"), ParseFails),
    cell("values element underflow", Replace("- 2", "- 1e-400"), ParseFails),
    // YAML reads `1e400` as text, not a number.
    cell("n overflow", Set("n", " 1e400"), ValidationFails),
    cell("n quoted 2^53 + 1", Set("n", " \"9007199254740993\""), ValidationFails),
    cell(
        "n 2^53",
        Set("n", " 9007199254740992"),
        Composes("9007199254740992|2|02134|On|false", || vec![("n", Some(json!(9_007_199_254_740_992_u64)))]),
    ),
    cell(
        "n -2^53",
        Set("n", " -9007199254740992"),
        Composes("-9007199254740992|2|02134|On|false", || vec![("n", Some(json!(-9_007_199_254_740_992_i64)))]),
    ),
    cell(
        "n quoted 2^53",
        Set("n", " \"9007199254740992\""),
        Composes("9007199254740992|2|02134|On|false", || vec![("n", Some(json!(9_007_199_254_740_992_u64)))]),
    ),
    cell("n decimal", Set("n", " 0.1"), Composes("0.1|2|02134|On|false", || vec![("n", Some(json!(0.1)))])),
    cell("n exponent", Set("n", " 1e3"), Composes("1000|2|02134|On|false", || vec![("n", Some(json!(1000.0)))])),
    cell("maybe whole float", Set("maybe", " 4.0"), Composes("7|4|02134|On|false", || vec![("maybe", Some(json!(4.0)))])),
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

const CONVERSION_CELLS: &[Cell] = &[
    cell("control", Control, Composes("7|2|02134|On|false", control_values)),
    // numeric text in a typed list and a typed object converts
    cell(
        "values numeric text",
        Set("values", " ['1', '2.5']"),
        Composes("7|2|02134|On|false", || vec![("values", Some(json!([1, 2.5])))]),
    ),
    cell("values one wrong element", Set("values", " ['1', pear]"), ValidationFails),
    cell("values every element wrong", Set("values", " [pear, plum]"), ValidationFails),
    cell(
        "meta numeric text",
        Replace("level: 3", "level: '3'"),
        Composes("7|2|02134|On|false", || vec![("meta", Some(json!({"level": 3})))]),
    ),
    cell("meta wrong member", Replace("level: 3", "level: pear"), ValidationFails),
    // `numberlike`/`boolish` through containers: validated, kept as written
    cell(
        "codes kept",
        Append("codes: ['02134', 5, ' 4 ']\n"),
        Composes("7|2|02134|On|false", || vec![("codes", Some(json!(["02134", 5, " 4 "])))]),
    ),
    cell("codes one wrong element", Append("codes: ['02134', pear]\n"), ValidationFails),
    cell(
        "toggles kept",
        Append("toggles: ['On', 'no', true]\n"),
        Composes("7|2|02134|On|false", || vec![("toggles", Some(json!(["On", "no", true])))]),
    ),
    cell("toggles native number", Append("toggles: [1]\n"), ValidationFails),
    cell(
        "nested kept",
        Append("nested:\n  zip: '007'\n  on: 'Yes'\n"),
        Composes("7|2|02134|On|false", || vec![("nested", Some(json!({"zip": "007", "on": "Yes"})))]),
    ),
    // tuple slots
    cell(
        "pair every slot",
        Append("pair: [a, '4', 'yes']\n"),
        Composes("7|2|02134|On|false", || vec![("pair", Some(json!(["a", 4, true])))]),
    ),
    cell(
        "pair optional slot absent",
        Append("pair: [5, '4']\n"),
        Composes("7|2|02134|On|false", || vec![("pair", Some(json!(["5", 4])))]),
    ),
    cell("pair required slot absent", Append("pair: [a]\n"), ValidationFails),
    cell("pair optional slot null", Append("pair: [a, 4, null]\n"), ValidationFails),
    cell("pair extra element", Append("pair: [a, 4, true, x]\n"), ValidationFails),
    cell("pair empty", Append("pair: []\n"), ValidationFails),
    cell("pair scalar", Append("pair: a\n"), ValidationFails),
    // pattern-key values
    cell(
        "dict values",
        Append("dict:\n  a: '3'\n  b: 4\n"),
        Composes("7|2|02134|On|false", || vec![("dict", Some(json!({"a": 3, "b": 4})))]),
    ),
    cell("dict one wrong value", Append("dict: {a: '3', b: pear}\n"), ValidationFails),
    cell(
        "dict empty",
        Append("dict: {}\n"),
        Composes("7|2|02134|On|false", || vec![("dict", Some(json!({})))]),
    ),
    cell("dict nested duplicate", Append("dict:\n  a: 1\n  a: 2\n"), ParseFails),
    // whole-value union: number wins over boolean for `"1"`
    cell(
        "either number text",
        Append("either: '1'\n"),
        Composes("7|2|02134|On|false", || vec![("either", Some(json!(1)))]),
    ),
    cell(
        "either boolean word",
        Append("either: 'yes'\n"),
        Composes("7|2|02134|On|false", || vec![("either", Some(json!(true)))]),
    ),
    cell("either neither", Append("either: pear\n"), ValidationFails),
    // the `json` content target
    cell(
        "config native mapping",
        Append("config:\n  port: 8080\n  debug: true\n"),
        Composes("7|2|02134|On|false", || vec![("config", Some(json!("{\"port\":8080,\"debug\":true}")))]),
    ),
    cell(
        "config text",
        Append("config: '{\"a\": 1}'\n"),
        Composes("7|2|02134|On|false", || vec![("config", Some(json!("{\"a\": 1}")))]),
    ),
    cell(
        "config null",
        Append("config: null\n"),
        Composes("7|2|02134|On|false", || vec![("config", Some(Value::Null))]),
    ),
    cell("config malformed text", Append("config: '{a'\n"), ValidationFails),
    cell("config empty text", Append("config: ''\n"), ValidationFails),
    cell("config trailing text", Append("config: '{\"a\": 1} x'\n"), ValidationFails),
    cell("config duplicate key in text", Append("config: '{\"a\": 1, \"a\": 2}'\n"), ValidationFails),
    cell("config overflow in text", Append("config: '[1e400]'\n"), ValidationFails),
    // a failure anywhere commits nothing, and the report names only it
    cell("convertible beside a failure", Append("either: '1'\ndict: {a: pear}\n"), ValidationFails),
];

#[test]
fn frontmatter_conversion_matrix() {
    let control = include_str!("../../fixtures/schema_enhancements_matrix/frontmatter.md-get.yaml")
        .replacen("./frontmatter.schema.yaml", "./frontmatter.conversion.schema.yaml", 1);
    let schema = include_str!("../../fixtures/schema_enhancements_matrix/frontmatter.conversion.schema.yaml");
    let workspace = tempfile::tempdir().expect("matrix workspace");
    let dir = workspace.path();
    std::fs::write(dir.join("frontmatter.conversion.schema.yaml"), schema).expect("write schema");
    let mut failures = Vec::new();
    for (index, cell) in CONVERSION_CELLS.iter().enumerate() {
        let frontmatter = apply(&control, cell.edit);
        if !matches!(cell.edit, Control) {
            assert_ne!(frontmatter, control, "{}: the edit must change the control", cell.label);
        }
        if let Err(problem) = check_cell(dir, index, &frontmatter, cell.outcome) {
            failures.push(format!("{}: {problem}\n--- frontmatter ---\n{frontmatter}", cell.label));
        }
    }
    assert!(failures.is_empty(), "{} cell(s) failed:\n\n{}", failures.len(), failures.join("\n\n"));
}

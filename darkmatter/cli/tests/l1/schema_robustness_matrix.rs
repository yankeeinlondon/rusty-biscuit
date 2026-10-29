//! The input robustness matrix of `2026-10-16-core-schema-types`, through the
//! public `md schema validate` invocation path.
//!
//! Declaration columns (root `$schema`, a `kind: schema` catalog's `types`, a
//! named definition `types.T`, a property definition, a property union) start
//! from a copy of a real shipped artifact. Each column has a positive control
//! that adds one declaration to that copy, and every other cell is one edit
//! from its control. Instance columns judge values against the core types.
//!
//! Each test validates all its documents in one `md` process and matches the
//! NDJSON records back to their documents by file name. Load errors are
//! asserted per record (`"error": "schema"` is the exit-`2` class,
//! `"error": "frontmatter_parse"` the exit-`3` class) and by the process exit
//! code, which is the most specific class present.
//!
//! `md schema validate` judges the authored document, which is final
//! validation for a declaration without `generated` or `default`; every
//! declaration here has neither.

use crate::common;

use common::CliProcessFixture;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

/// One of the nine rows every matrix column covers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    Control,
    Absent,
    ExplicitNull,
    WrongWhole,
    WrongOneElement,
    WrongEveryElement,
    Empty,
    DuplicateKey,
    TrailingContent,
}

const SHAPES: [Shape; 9] = [
    Shape::Control,
    Shape::Absent,
    Shape::ExplicitNull,
    Shape::WrongWhole,
    Shape::WrongOneElement,
    Shape::WrongEveryElement,
    Shape::Empty,
    Shape::DuplicateKey,
    Shape::TrailingContent,
];

/// The public outcome of one document.
#[derive(Clone, Debug, PartialEq)]
enum Expect {
    Valid,
    /// Valid, and the record carries a warning with this code.
    ValidWarning(&'static str),
    /// Validation fails with a problem located at this property (a root
    /// `required` problem names it; a value problem's path starts with it).
    Invalid(&'static str),
    /// The schema does not load (exit `2`); the message contains each of
    /// these texts.
    SchemaLoad(Vec<String>),
    /// The frontmatter does not parse (exit `3`); the message contains this
    /// text.
    Parse(String),
}

impl Expect {
    fn exit_class(&self) -> i32 {
        match self {
            Self::Valid | Self::ValidWarning(_) => 0,
            Self::Invalid(_) => 1,
            Self::SchemaLoad(_) => 2,
            Self::Parse(_) => 3,
        }
    }
}

fn load(text: &str) -> Expect {
    Expect::SchemaLoad(vec![text.to_string()])
}

/// A load error whose message names the broken schema file and contains
/// `text`.
fn load_in_file(text: &str) -> Expect {
    Expect::SchemaLoad(vec![FILE.to_string(), text.to_string()])
}

/// Placeholder for the name of the schema file a cell's document
/// references; [`naming`] substitutes it.
const FILE: &str = "{file}";

/// `expect` with [`FILE`] replaced by `file`: a load error that names the
/// broken schema file.
fn naming(expect: Expect, file: &str) -> Expect {
    match expect {
        Expect::SchemaLoad(texts) => {
            Expect::SchemaLoad(texts.into_iter().map(|text| text.replace(FILE, file)).collect())
        }
        other => other,
    }
}

fn parse(text: &str) -> Expect {
    Expect::Parse(text.to_string())
}

/// One document of a matrix column.
struct Cell {
    column: String,
    shape: Shape,
    name: String,
    frontmatter: String,
    expect: Expect,
    /// Whether the cell belongs to a matrix column, which must cover all
    /// nine shapes; single-purpose controls do not.
    covered: bool,
}

/// A batch of documents and the auxiliary schema files they reference,
/// validated in one `md` process.
#[derive(Default)]
struct Matrix {
    files: Vec<(String, String)>,
    cells: Vec<Cell>,
}

impl Matrix {
    fn file(&mut self, name: impl Into<String>, content: impl Into<String>) {
        self.files.push((name.into(), content.into()));
    }

    fn cell(&mut self, column: &str, shape: Shape, label: &str, frontmatter: impl Into<String>, expect: Expect) {
        let name = format!("{}__{}.md", slug(column), slug(label));
        assert!(
            self.cells.iter().all(|cell| cell.name != name),
            "duplicate cell name {name}"
        );
        self.cells.push(Cell {
            column: column.to_string(),
            shape,
            name,
            frontmatter: frontmatter.into(),
            expect,
            covered: true,
        });
    }

    /// A single-purpose control outside the nine-shape matrix.
    fn extra(&mut self, column: &str, label: &str, frontmatter: impl Into<String>, expect: Expect) {
        self.cell(column, Shape::Control, label, frontmatter, expect);
        self.cells.last_mut().expect("just pushed").covered = false;
    }

    /// Asserts every column covers all nine shapes and opens with a valid
    /// positive control, then runs the batch and asserts every record.
    fn assert(self) -> BTreeMap<String, Value> {
        let mut columns: BTreeMap<&str, Vec<&Cell>> = BTreeMap::new();
        for cell in self.cells.iter().filter(|cell| cell.covered) {
            columns.entry(&cell.column).or_default().push(cell);
        }
        for (column, cells) in &columns {
            let covered: BTreeSet<Shape> = cells.iter().map(|cell| cell.shape).collect();
            let missing: Vec<_> = SHAPES.iter().filter(|shape| !covered.contains(shape)).collect();
            assert!(missing.is_empty(), "column {column} lacks rows {missing:?}");
            assert!(
                cells[0].shape == Shape::Control && cells[0].expect == Expect::Valid,
                "column {column} must open with a valid positive control"
            );
        }
        run(&self.files, &self.cells, &BTreeSet::new())
    }
}

fn slug(text: &str) -> String {
    text.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '_' })
        .collect()
}

/// Writes the batch, validates every document in one `md` run, and asserts
/// each record and the process exit code. Cells named in `compared` are only
/// required to validate (valid or invalid, never a load error); the caller
/// compares their outcomes.
fn run(files: &[(String, String)], cells: &[Cell], compared: &BTreeSet<String>) -> BTreeMap<String, Value> {
    let process = CliProcessFixture::new();
    let dir = process.cwd();
    for (name, content) in files {
        std::fs::write(dir.join(name), content).expect("write schema file");
    }
    for cell in cells {
        std::fs::write(dir.join(&cell.name), format!("---\n{}---\nBody\n", cell.frontmatter))
            .expect("write document");
    }
    // These columns judge document schemas, as they did before the built-in
    // baseline became the default. With it, every distinct document schema
    // compiles the whole merged baseline (about 12 ms each in a debug build),
    // which pushes a column past the L1 time budget; baseline selection and
    // merging have their own tests in `schema_validate_baseline_selection`.
    let output = process
        .command()
        .args(["schema", "validate", "--format", "json", "--disable", "baseline-schema"])
        .args(cells.iter().map(|cell| cell.name.as_str()))
        .output()
        .expect("md runs");
    let stderr = String::from_utf8_lossy(&output.stderr);
    let records: BTreeMap<String, Value> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| {
            let record: Value = serde_json::from_str(line).expect("NDJSON record");
            (record["file"].as_str().expect("file").to_string(), record)
        })
        .collect();

    let mut mismatches = Vec::new();
    for cell in cells {
        let record = records
            .get(&cell.name)
            .unwrap_or_else(|| panic!("no record for {}; stderr: {stderr}", cell.name));
        let checked = if compared.contains(&cell.name) {
            match record.get("error") {
                None => Ok(()),
                Some(_) => Err(format!("must validate, got {record}")),
            }
        } else {
            check(record, &cell.expect)
        };
        if let Err(actual) = checked {
            mismatches.push(format!(
                "[{} / {:?}] {}: expected {:?}, {actual}\n{}",
                cell.column, cell.shape, cell.name, cell.expect, cell.frontmatter
            ));
        }
    }
    assert!(mismatches.is_empty(), "{} mismatches:\n\n{}", mismatches.len(), mismatches.join("\n\n"));
    let expected_exit = cells.iter().map(|cell| cell.expect.exit_class()).max().unwrap_or(0);
    assert_eq!(output.status.code(), Some(expected_exit), "exit code; stderr: {stderr}");
    records
}

fn check(record: &Value, expect: &Expect) -> Result<(), String> {
    let error = record["error"].as_str();
    let message = record["message"].as_str().unwrap_or_default();
    let valid = record["valid"].as_bool();
    let warnings: Vec<&str> = record["warnings"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|warning| warning["code"].as_str())
        .collect();
    let ok = match expect {
        Expect::Valid => error.is_none() && valid == Some(true) && warnings.is_empty(),
        Expect::ValidWarning(code) => error.is_none() && valid == Some(true) && warnings.contains(code),
        Expect::Invalid(property) => {
            let pointer = format!("/{property}");
            error.is_none()
                && valid == Some(false)
                && record["problems"].as_array().into_iter().flatten().any(|problem| {
                    problem["property"].as_str() == Some(property)
                        || problem["path"]
                            .as_str()
                            .is_some_and(|path| path == pointer || path.starts_with(&format!("{pointer}/")))
                })
        }
        Expect::SchemaLoad(texts) => error == Some("schema") && texts.iter().all(|text| message.contains(text.as_str())),
        Expect::Parse(text) => error == Some("frontmatter_parse") && message.contains(text.as_str()),
    };
    if ok { Ok(()) } else { Err(format!("got {record}")) }
}

// ── declaration side ─────────────────────────────────────────────────────

/// A core type at a declaration position, with a witness value it accepts
/// and, unless it accepts everything, one it rejects. Values are YAML flow
/// text.
struct CoreType {
    name: &'static str,
    definition: &'static str,
    accepted: &'static str,
    rejected: Option<&'static str>,
}

const CORE_TYPES: [CoreType; 5] = [
    CoreType { name: "unknown", definition: "unknown", accepted: "{a: [1, null, x]}", rejected: None },
    CoreType { name: "any", definition: "any", accepted: "[1, x, null]", rejected: None },
    CoreType { name: "null", definition: "'null'", accepted: "null", rejected: Some("x") },
    CoreType {
        name: "alphanumeric",
        definition: "alphanumeric(allow-dashes)",
        accepted: "a-b",
        rejected: Some("a_b"),
    },
    CoreType { name: "shell", definition: "shell", accepted: "echo ok", rejected: Some("'  '") },
];

#[test]
fn root_schema_declaration_column() {
    let shipped = include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.yaml");
    let mut matrix = Matrix::default();
    matrix.file("spec.md", "spec\n");
    matrix.file("review.md", "review\n");
    let col = "root $schema";
    let arms = "$schema:\n  - schema: file(required)\n  - review: file(required)\n";
    let with = |schema: &str, values: &str| format!("{schema}{values}");
    use Shape::*;

    matrix.cell(col, Control, "control schema file", with(arms, "schema: spec.md\n"), Expect::Valid);
    matrix.cell(col, Control, "control review file", with(arms, "review: review.md\n"), Expect::Valid);
    matrix.cell(col, Control, "control both files", with(arms, "schema: spec.md\nreview: review.md\n"), Expect::Valid);
    matrix.cell(col, Control, "control neither", with(arms, ""), Expect::Invalid("schema"));

    // The document has no inline declaration: no synthetic schema, and with
    // no baseline nothing is constrained.
    matrix.cell(col, Absent, "absent", "schema: 5\n", Expect::Valid);
    matrix.cell(col, ExplicitNull, "null", "$schema: null\nschema: spec.md\n", load("$schema must be a mapping, sequence, or string; got null"));
    matrix.cell(col, ExplicitNull, "empty value", "$schema:\nschema: spec.md\n", load("$schema must be a mapping, sequence, or string; got null"));
    matrix.cell(col, WrongWhole, "number", "$schema: 5\n", load("$schema must be a mapping, sequence, or string; got number"));
    matrix.cell(col, WrongWhole, "boolean", "$schema: true\n", load("$schema must be a mapping, sequence, or string; got boolean"));
    matrix.cell(
        col,
        WrongOneElement,
        "null arm",
        "$schema:\n  - schema: file(required)\n  - null\nschema: spec.md\n",
        load("root union arms must be a mapping or a file reference, got null"),
    );
    matrix.cell(
        col,
        WrongOneElement,
        "number arm",
        "$schema:\n  - schema: file(required)\n  - 5\nschema: spec.md\n",
        load("root union arms must be a mapping or a file reference, got number"),
    );
    matrix.cell(
        col,
        WrongEveryElement,
        "all invalid arms",
        "$schema:\n  - 5\n  - true\n",
        load("root union arms must be a mapping or a file reference"),
    );
    // `{}` is an explicit open shape; `[]` is not a union.
    matrix.cell(col, Empty, "empty mapping", "$schema: {}\nanything: [1, x]\n", Expect::Valid);
    matrix.cell(col, Empty, "empty sequence", "$schema: []\n", load("$schema root union must have at least one arm"));
    matrix.cell(
        col,
        DuplicateKey,
        "duplicate $schema",
        format!("{arms}$schema:\n  - review: file(required)\nschema: spec.md\n"),
        parse("duplicate entry with key \"$schema\""),
    );
    matrix.cell(
        col,
        DuplicateKey,
        "duplicate shape key",
        "$schema:\n  - schema: file(required)\n    schema: string\n  - review: file(required)\nschema: spec.md\n",
        parse("$schema[0]: duplicate entry with key \"schema\""),
    );
    matrix.cell(
        col,
        TrailingContent,
        "invalid yaml",
        "$schema:\n  - schema: file(required)\n  - review: [file\nschema: spec.md\n",
        parse("did not find expected ',' or ']'"),
    );
    matrix.cell(
        col,
        TrailingContent,
        "extra document",
        format!("{arms}schema: spec.md\n...\nreview: review.md\n"),
        parse("more than one document"),
    );

    // The same column through an explicitly requested pure-schema artifact:
    // the shipped `sequence.yaml`.
    let pure = "pure-schema artifact";
    matrix.file("sequence.yaml", shipped);
    matrix.cell(pure, Control, "control", "$schema: ./sequence.yaml\nsequence_name: s\n", Expect::Valid);
    matrix.cell(pure, Control, "control witness", "$schema: ./sequence.yaml\n", Expect::Invalid("sequence_name"));
    // Removing the declaration leaves nothing to classify.
    matrix.file("sequence_absent.yaml", "");
    matrix.cell(pure, Absent, "absent", "$schema: ./sequence_absent.yaml\n", load("sequence_absent.yaml"));
    // Plan-table imprecision: removing only the `$schema:` key line leaves a
    // mapping that the established raw-JSON-Schema fallback reads as a schema
    // that constrains nothing. It is not a load error; it is reported with
    // the missing-envelope warning instead of passing silently.
    let dedented = shipped.replacen("$schema:\n", "", 1);
    assert_ne!(dedented, shipped, "the shipped artifact declares a root `$schema:` mapping");
    matrix.file("sequence_key_removed.yaml", dedented);
    matrix.cell(
        pure,
        Absent,
        "root key removed",
        "$schema: ./sequence_key_removed.yaml\n",
        Expect::ValidWarning("dm.schema.missing_simplified_envelope"),
    );
    for (shape, label, content) in [
        (ExplicitNull, "null", "$schema: null\n".to_string()),
        (ExplicitNull, "empty value", "$schema:\n".to_string()),
        (WrongWhole, "number", "$schema: 5\n".to_string()),
        (WrongOneElement, "number arm", format!("{}  - 5\n", shipped.replacen("$schema:\n", "$schema:\n  -\n", 1))),
        (WrongEveryElement, "all invalid arms", "$schema: [5, true]\n".to_string()),
        (Empty, "empty sequence", "$schema: []\n".to_string()),
        (DuplicateKey, "duplicate $schema", format!("{shipped}$schema:\n  other: string\n")),
        (TrailingContent, "extra document", format!("{shipped}---\nx: 1\n")),
    ] {
        let file = format!("sequence_{}.yaml", slug(label));
        matrix.file(&file, content);
        matrix.cell(pure, shape, label, format!("$schema: ./{file}\nsequence_name: s\n"), load(&file));
    }
    matrix.file("sequence_empty_mapping.yaml", "$schema: {}\n");
    matrix.cell(pure, Empty, "empty mapping", "$schema: ./sequence_empty_mapping.yaml\nanything: 1\n", Expect::Valid);

    let records = matrix.assert();
    assert!(
        records["root__schema__absent.md"]["schema"].is_null(),
        "a document without `$schema` gets no synthetic schema"
    );
}

#[test]
fn types_catalog_declaration_column() {
    let shipped = include_str!("../../../docs/schemas/env.yaml");
    let (head, types) = shipped.split_once("types:\n").expect("env.yaml declares `types:`");
    let mut matrix = Matrix::default();
    let col = "types catalog";
    use Shape::*;
    let catalog = |matrix: &mut Matrix, shape: Shape, label: &str, content: String, values: &str, expect: Expect| {
        let file = format!("catalog_{}.yaml", slug(label));
        let expect = naming(expect, &file);
        matrix.file(&file, content);
        matrix.cell(
            col,
            shape,
            label,
            format!("$schema:\n  agent: AGENT@./{file}\n  model: MODEL@./{file}\n{values}"),
            expect,
        );
    };

    catalog(&mut matrix, Control, "control", shipped.to_string(), "agent: claude\nmodel: opus\n", Expect::Valid);
    catalog(&mut matrix, Control, "control witness", shipped.to_string(), "agent: {name: x}\n", Expect::Invalid("agent"));
    catalog(&mut matrix, Absent, "absent", head.to_string(), "agent: claude\n", load(FILE));
    catalog(&mut matrix, ExplicitNull, "null", format!("{head}types: null\n"), "", load(FILE));
    catalog(&mut matrix, ExplicitNull, "empty value", format!("{head}types:\n"), "", load("`types` must be a mapping"));
    catalog(&mut matrix, WrongWhole, "number", format!("{head}types: 5\n"), "", load("`types` must be a mapping"));
    catalog(&mut matrix, WrongOneElement, "sequence", format!("{head}types:\n  - AGENT: string\n"), "", load("`types` must be a mapping"));
    catalog(&mut matrix, WrongEveryElement, "invalid sequence", format!("{head}types: [5, true]\n"), "", load("`types` must be a mapping"));
    // A declared empty catalog loads; the retained reference names a type it
    // does not hold.
    catalog(&mut matrix, Empty, "empty mapping", format!("{head}types: {{}}\n"), "", load("unknown named type `AGENT`"));
    catalog(&mut matrix, DuplicateKey, "duplicate types", format!("{shipped}types:\n  OTHER: string\n"), "", load(FILE));
    catalog(&mut matrix, DuplicateKey, "duplicate name", format!("{head}types:\n{types}    AGENT: number\n"), "", load(FILE));
    catalog(&mut matrix, TrailingContent, "extra document", format!("{shipped}---\nx: 1\n"), "", load(FILE));
    catalog(&mut matrix, TrailingContent, "invalid yaml", format!("{shipped}    BAD: [string\n"), "", load(FILE));
    matrix.assert();
}

#[test]
fn named_type_definition_column() {
    let shipped = include_str!("../../../docs/schemas/env.yaml");
    let shipped = shipped.trim_end();
    let mut matrix = Matrix::default();
    use Shape::*;
    for core in &CORE_TYPES {
        let col = format!("types.T {}", core.name);
        let col = col.as_str();
        // `T` is added to the shipped catalog, and `L` refers to it locally;
        // the document imports both.
        let catalog = |definition: &str| format!("{shipped}\n    {definition}\n    L: T\n");
        let defined = catalog(&format!("T: {}", core.definition));
        let add = |matrix: &mut Matrix, shape: Shape, label: &str, content: String, cells: Vec<(&str, String, Expect)>| {
            let file = format!("named_{}_{}.yaml", slug(core.name), slug(label));
            matrix.file(&file, content);
            for (witness, values, expect) in cells {
                let expect = naming(expect, &file);
                matrix.cell(
                    col,
                    shape,
                    &format!("{label} {witness}"),
                    format!("$schema:\n  p: T@./{file}\n  q: L@./{file}\n{values}"),
                    expect,
                );
            }
        };
        let mut control = vec![
            ("imported", format!("p: {}\n", core.accepted), Expect::Valid),
            ("local", format!("q: {}\n", core.accepted), Expect::Valid),
            ("omitted", String::new(), Expect::Valid),
        ];
        if let Some(rejected) = core.rejected {
            control.push(("imported rejects", format!("p: {rejected}\n"), Expect::Invalid("p")));
            control.push(("local rejects", format!("q: {rejected}\n"), Expect::Invalid("q")));
        }
        add(&mut matrix, Control, "control", defined.clone(), control);
        add(
            &mut matrix,
            Absent,
            "absent",
            format!("{shipped}\n    L: T\n"),
            vec![("ref", String::new(), load("unknown named type `T`"))],
        );
        for (label, definition) in [("null", "T: null"), ("tilde", "T: ~"), ("empty value", "T:")] {
            add(
                &mut matrix,
                ExplicitNull,
                label,
                catalog(definition),
                vec![
                    ("null", "p: null\nq: null\n".to_string(), Expect::Valid),
                    ("omitted", String::new(), Expect::Valid),
                    ("imported text", "p: x\n".to_string(), Expect::Invalid("p")),
                    ("local text", "q: x\n".to_string(), Expect::Invalid("q")),
                ],
            );
        }
        let wrong = |text: &str| vec![("ref", String::new(), load_in_file(text))];
        add(&mut matrix, WrongWhole, "number", catalog("T: 5"), wrong("property `T`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got number"));
        add(&mut matrix, WrongWhole, "boolean", catalog("T: true"), wrong("property `T`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got boolean"));
        add(
            &mut matrix,
            WrongOneElement,
            "number arm",
            catalog(&format!("T: [{}, 5]", core.definition)),
            wrong("property `T`: property-level union arm[1] must be a string type expression"),
        );
        add(&mut matrix, WrongEveryElement, "all invalid arms", catalog("T: [5, true]"), wrong("property `T`: property-level union arm[0]"));
        add(
            &mut matrix,
            Empty,
            "empty mapping",
            catalog("T: {}"),
            vec![
                ("object", "p: {}\nq: {}\n".to_string(), Expect::Valid),
                ("text", "p: x\n".to_string(), Expect::Invalid("p")),
                ("key", "q: {a: 1}\n".to_string(), Expect::Invalid("q")),
            ],
        );
        add(&mut matrix, Empty, "empty sequence", catalog("T: []"), wrong("property `T`: property-level union must have at least one arm"));
        add(&mut matrix, Empty, "empty expression", catalog("T: ''"), wrong("property `T`: expected a type name"));
        add(
            &mut matrix,
            DuplicateKey,
            "duplicate",
            format!("{defined}    T: string\n"),
            vec![("ref", String::new(), load(FILE))],
        );
        add(
            &mut matrix,
            TrailingContent,
            "extra document",
            format!("{defined}---\nx: 1\n"),
            vec![("ref", String::new(), load(FILE))],
        );
        add(
            &mut matrix,
            TrailingContent,
            "invalid yaml",
            catalog(&format!("T: [{}", core.definition)),
            vec![("ref", String::new(), load(FILE))],
        );
    }
    matrix.assert();
}

#[test]
fn property_definition_column() {
    let shipped = include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.yaml");
    let shipped = shipped.trim_end();
    let mut matrix = Matrix::default();
    use Shape::*;
    for core in &CORE_TYPES {
        // The same property at the root and at a nested position. The nested
        // mapping keeps a sibling so removing `p` does not turn `o` itself
        // into the `null` type.
        for (position, indent, key, wrap) in [
            ("root", "  ", "p", (|value: &str| format!("p: {value}\n")) as fn(&str) -> String),
            ("nested", "    ", "o", |value: &str| format!("o: {{p: {value}}}\n")),
        ] {
            let col = format!("$schema.p {} {position}", core.name);
            let col = col.as_str();
            let schema = |definition: Option<&str>| {
                let line = definition.map(|definition| format!("{indent}{definition}\n")).unwrap_or_default();
                match position {
                    "root" => format!("{shipped}\n{line}"),
                    _ => format!("{shipped}\n  o:\n    x: string\n{line}"),
                }
            };
            let add = |matrix: &mut Matrix, shape: Shape, label: &str, content: String, cells: Vec<(&str, String, Expect)>| {
                let file = format!("property_{}_{position}_{}.yaml", slug(core.name), slug(label));
                matrix.file(&file, content);
                for (witness, values, expect) in cells {
                    let expect = naming(expect, &file);
                    matrix.cell(
                        col,
                        shape,
                        &format!("{label} {witness}"),
                        format!("$schema: ./{file}\nsequence_name: s\n{values}"),
                        expect,
                    );
                }
            };
            let defined = schema(Some(&format!("p: {}", core.definition)));
            let mut control = vec![("accepted", wrap(core.accepted), Expect::Valid)];
            if let Some(rejected) = core.rejected {
                control.push(("rejected", wrap(rejected), Expect::Invalid(key)));
            }
            add(&mut matrix, Control, "control", defined.clone(), control);
            // Removing `p` makes it undeclared: the open root admits it, while
            // a nested mapping is a closed inline shape that rejects it as an
            // unexpected key.
            let absent = if position == "root" { Expect::Valid } else { Expect::Invalid("o") };
            add(&mut matrix, Absent, "absent", schema(None), vec![("witness", wrap("{a: [1, x]}"), absent)]);
            for (label, definition) in [("null", "p: null"), ("empty value", "p:")] {
                add(
                    &mut matrix,
                    ExplicitNull,
                    label,
                    schema(Some(definition)),
                    vec![
                        ("null", wrap("null"), Expect::Valid),
                        ("text", wrap("x"), Expect::Invalid(key)),
                    ],
                );
            }
            let wrong = |text: &str| vec![("ref", String::new(), load_in_file(text))];
            add(&mut matrix, WrongWhole, "number", schema(Some("p: 5")), wrong("property `p`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got number"));
            add(&mut matrix, WrongWhole, "boolean", schema(Some("p: false")), wrong("property `p`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got boolean"));
            add(
                &mut matrix,
                WrongOneElement,
                "number arm",
                schema(Some(&format!("p: [{}, 5]", core.definition))),
                wrong("property `p`: property-level union arm[1] must be a string type expression"),
            );
            add(&mut matrix, WrongEveryElement, "all invalid arms", schema(Some("p: [5, true]")), wrong("property `p`: property-level union arm[0]"));
            add(
                &mut matrix,
                Empty,
                "empty mapping",
                schema(Some("p: {}")),
                vec![
                    ("object", wrap("{}"), Expect::Valid),
                    ("key", wrap("{a: 1}"), Expect::Invalid(key)),
                    ("text", wrap("x"), Expect::Invalid(key)),
                ],
            );
            add(&mut matrix, Empty, "empty sequence", schema(Some("p: []")), wrong("property `p`: property-level union must have at least one arm"));
            add(&mut matrix, Empty, "empty expression", schema(Some("p: ''")), wrong("property `p`: expected a type name"));
            add(
                &mut matrix,
                DuplicateKey,
                "duplicate",
                format!("{defined}{indent}p: string\n"),
                vec![("ref", String::new(), load(FILE))],
            );
            add(
                &mut matrix,
                TrailingContent,
                "extra document",
                format!("{defined}---\nx: 1\n"),
                vec![("ref", String::new(), load(FILE))],
            );
            add(
                &mut matrix,
                TrailingContent,
                "invalid yaml",
                schema(Some(&format!("p: [{}", core.definition))),
                vec![("ref", String::new(), load(FILE))],
            );
        }
    }
    matrix.assert();
}

#[test]
fn union_definition_column() {
    let shipped = include_str!("../../../tests/fixtures/schema-triggers/schemas/sequence.yaml");
    let shipped = shipped.trim_end();
    let catalog = include_str!("../../../docs/schemas/env.yaml");
    let catalog = catalog.trim_end();
    let mut matrix = Matrix::default();
    use Shape::*;
    for core in &CORE_TYPES {
        let col = format!("$schema.p union {}", core.name);
        let col = col.as_str();
        let schema = |definition: Option<&str>| {
            let line = definition.map(|definition| format!("  {definition}\n")).unwrap_or_default();
            format!("{shipped}\n{line}")
        };
        let add = |matrix: &mut Matrix, shape: Shape, label: &str, content: String, cells: Vec<(&str, String, Expect)>| {
            let file = format!("union_{}_{}.yaml", slug(core.name), slug(label));
            matrix.file(&file, content);
            for (witness, values, expect) in cells {
                let expect = naming(expect, &file);
                matrix.cell(
                    col,
                    shape,
                    &format!("{label} {witness}"),
                    format!("$schema: ./{file}\nsequence_name: s\n{values}"),
                    expect,
                );
            }
        };
        let both = |rejected: bool| {
            let mut cells = vec![
                ("accepted", format!("p: {}\n", core.accepted), Expect::Valid),
                ("null", "p: null\n".to_string(), Expect::Valid),
                ("omitted", String::new(), Expect::Valid),
            ];
            if let (true, Some(value)) = (rejected, core.rejected) {
                cells.push(("rejected", format!("p: {value}\n"), Expect::Invalid("p")));
            }
            cells
        };
        let defined = schema(Some(&format!("p: [{}, null]", core.definition)));
        add(&mut matrix, Control, "control", defined.clone(), both(true));
        // Native null arms in every YAML spelling.
        add(&mut matrix, Control, "tilde arm", schema(Some(&format!("p: [{}, ~]", core.definition))), both(true));
        add(
            &mut matrix,
            Control,
            "empty item arm",
            schema(Some(&format!("p:\n    - {}\n    -", core.definition))),
            both(true),
        );
        add(&mut matrix, Absent, "absent", schema(None), vec![("witness", "p: {a: [1, x]}\n".to_string(), Expect::Valid)]);
        // A whole null is the scalar `null` type.
        add(
            &mut matrix,
            ExplicitNull,
            "whole null",
            schema(Some("p: null")),
            vec![
                ("null", "p: null\n".to_string(), Expect::Valid),
                ("non-null", "p: x\n".to_string(), Expect::Invalid("p")),
            ],
        );
        let wrong = |text: &str| vec![("ref", String::new(), load_in_file(text))];
        add(&mut matrix, WrongWhole, "number", schema(Some("p: 5")), wrong("property `p`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got number"));
        add(&mut matrix, WrongWhole, "boolean", schema(Some("p: true")), wrong("property `p`: property value must be a string type expression, a nested mapping object, null, or a sequence of expressions, got boolean"));
        add(
            &mut matrix,
            WrongOneElement,
            "number arm",
            schema(Some(&format!("p: [{}, 5]", core.definition))),
            wrong("property `p`: property-level union arm[1] must be a string type expression"),
        );
        add(
            &mut matrix,
            WrongOneElement,
            "boolean arm beside object",
            schema(Some("p: [{a: string}, true]")),
            wrong("property `p`: property-level union arm[1] must be a string type expression"),
        );
        add(&mut matrix, WrongEveryElement, "all invalid arms", schema(Some("p: [5, true]")), wrong("property `p`: property-level union arm[0]"));
        add(&mut matrix, Empty, "empty sequence", schema(Some("p: []")), wrong("property `p`: property-level union must have at least one arm"));
        // `{}` in place of the union is a closed empty object shape.
        add(
            &mut matrix,
            Empty,
            "empty mapping",
            schema(Some("p: {}")),
            vec![
                ("object", "p: {}\n".to_string(), Expect::Valid),
                ("key", "p: {a: 1}\n".to_string(), Expect::Invalid("p")),
            ],
        );
        add(
            &mut matrix,
            DuplicateKey,
            "duplicate in object arm",
            schema(Some(&format!("p:\n    - a: {}\n      a: string\n    - null", core.definition))),
            vec![("ref", String::new(), load(FILE))],
        );
        add(
            &mut matrix,
            DuplicateKey,
            "duplicate property",
            format!("{defined}  p: string\n"),
            vec![("ref", String::new(), load(FILE))],
        );
        add(
            &mut matrix,
            TrailingContent,
            "extra document",
            format!("{defined}---\nx: 1\n"),
            vec![("ref", String::new(), load(FILE))],
        );

        // A postfixed named union: `required` on a null-admitting union has
        // no effect, through an import from the shipped catalog.
        let file = format!("named_union_{}.yaml", slug(core.name));
        matrix.file(&file, format!("{catalog}\n    maybe: [{}, null]\n", core.definition));
        let named = format!("$schema:\n  p: maybe(required)@./{file}\n");
        let named_col = format!("postfixed named union {}", core.name);
        matrix.cell(&named_col, Control, "control", format!("{named}p: {}\n", core.accepted), Expect::Valid);
        matrix.cell(&named_col, Absent, "absent", named.clone(), Expect::Valid);
        matrix.cell(&named_col, ExplicitNull, "null", format!("{named}p: null\n"), Expect::Valid);
        let rejected = core.rejected.map_or(Expect::Valid, |_| Expect::Invalid("p"));
        matrix.cell(&named_col, WrongWhole, "wrong", format!("{named}p: {}\n", core.rejected.unwrap_or("5")), rejected);
        let sequence = if matches!(core.name, "unknown" | "any") { Expect::Valid } else { Expect::Invalid("p") };
        matrix.cell(&named_col, WrongOneElement, "one element", format!("{named}p: [{}, 5]\n", core.accepted), sequence.clone());
        matrix.cell(&named_col, WrongEveryElement, "every element", format!("{named}p: [5, 6]\n"), sequence);
        let empty = match core.name {
            "unknown" | "any" => Expect::Valid,
            // The empty string is a valid base `alphanumeric` value.
            "alphanumeric" => Expect::Valid,
            _ => Expect::Invalid("p"),
        };
        matrix.cell(&named_col, Empty, "empty", format!("{named}p: ''\n"), empty);
        matrix.cell(&named_col, DuplicateKey, "duplicate", format!("{named}p: null\np: null\n"), parse("duplicate entry with key \"p\""));
        matrix.cell(&named_col, TrailingContent, "trailing", format!("{named}p: null\n...\nq: 1\n"), parse("more than one document"));
    }
    matrix.assert();
}

// ── instance side ────────────────────────────────────────────────────────

/// The rows every instance column shares for duplicate keys and trailing or
/// invalid content.
fn yaml_rows(value: &str) -> Vec<(Shape, &'static str, String, Expect)> {
    vec![
        (Shape::DuplicateKey, "duplicate key", format!("p: {value}\np: {value}\n"), parse("duplicate entry with key \"p\"")),
        (Shape::TrailingContent, "extra document", format!("p: {value}\n...\nq: 1\n"), parse("more than one document")),
        (Shape::TrailingContent, "invalid yaml", "p: [abc\n".to_string(), parse("did not find expected ',' or ']'")),
    ]
}

#[test]
fn instance_side_matrix() {
    let mut matrix = Matrix::default();
    use Expect::{Invalid, Valid};
    use Shape::*;
    let p = Invalid("p");
    let column = |matrix: &mut Matrix, name: &str, declaration: &str, rows: Vec<(Shape, &str, String, Expect)>| {
        for (shape, label, values, expect) in rows {
            matrix.cell(name, shape, label, format!("$schema:\n  p: {declaration}\n{values}"), expect);
        }
    };
    let v = |text: &str| format!("p: {text}\n");

    // p: A
    let mut rows = vec![
        (Control, "control", v("abc1"), Valid),
        (Absent, "absent", String::new(), p.clone()),
        (ExplicitNull, "null", v("null"), p.clone()),
        (WrongWhole, "number", v("5"), p.clone()),
        (WrongWhole, "boolean", v("true"), p.clone()),
        (WrongOneElement, "mixed sequence", v("[abc1, 5]"), p.clone()),
        (WrongEveryElement, "invalid sequence", v("[5, 6]"), p.clone()),
        (Empty, "empty string", v("''"), Valid),
    ];
    rows.extend(yaml_rows("abc1"));
    column(&mut matrix, "A", "alphanumeric(required)", rows);
    for variant in ["min(1)", "not-empty"] {
        let rows = vec![
            (Control, "control", v("a"), Valid),
            (Absent, "absent", String::new(), p.clone()),
            (ExplicitNull, "null", v("null"), p.clone()),
            (WrongWhole, "number", v("5"), p.clone()),
            (WrongOneElement, "mixed sequence", v("[a, 5]"), p.clone()),
            (WrongEveryElement, "invalid sequence", v("[5]"), p.clone()),
            (Empty, "empty string", v("''"), p.clone()),
            (DuplicateKey, "duplicate key", "p: a\np: b\n".to_string(), parse("duplicate entry with key \"p\"")),
            (TrailingContent, "invalid yaml", "p: 'a\n".to_string(), parse("while scanning a quoted scalar")),
        ];
        column(&mut matrix, &format!("A {variant}"), &format!("alphanumeric({variant}; required)"), rows);
    }

    // p: S
    let mut rows = vec![
        (Control, "control", v("echo ok"), Valid),
        (Absent, "absent", String::new(), p.clone()),
        (ExplicitNull, "null", v("null"), p.clone()),
        (WrongWhole, "number coerces", v("5"), Valid),
        (WrongWhole, "boolean coerces", v("true"), Valid),
        (WrongWhole, "object", v("{a: 1}"), p.clone()),
        (WrongOneElement, "mixed sequence", v("[echo, 5]"), p.clone()),
        (WrongEveryElement, "invalid sequence", v("[5]"), p.clone()),
        (Empty, "empty string", v("''"), p.clone()),
        (Empty, "whitespace only", v("'  '"), p.clone()),
    ];
    rows.extend(yaml_rows("echo ok"));
    column(&mut matrix, "S", "shell(required)", rows);

    // p: N
    let mut rows = vec![
        (Control, "control", v("null"), Valid),
        (Absent, "absent", String::new(), Valid),
        (ExplicitNull, "tilde", v("~"), Valid),
        (WrongWhole, "text", v("x"), p.clone()),
        (WrongWhole, "number", v("0"), p.clone()),
        (WrongWhole, "boolean", v("false"), p.clone()),
        (WrongOneElement, "mixed sequence", v("[null, x]"), p.clone()),
        (WrongEveryElement, "invalid sequence", v("[x, 1]"), p.clone()),
        (Empty, "empty string", v("''"), p.clone()),
        (Empty, "empty array", v("[]"), p.clone()),
        (Empty, "empty object", v("{}"), p.clone()),
    ];
    rows.extend(yaml_rows("null"));
    column(&mut matrix, "N", "'null(required)'", rows);

    // p: X, in both spellings. Values pass unchanged; preservation of native
    // types is asserted through composition by the library L1
    // `schemas_core_type_values::unknown_values_keep_their_native_types`.
    for (name, declaration) in [("X unknown", "'unknown(required)'"), ("X any", "'any(required)'")] {
        let mut rows = vec![
            (Control, "control", v("{a: {b: [1, null, x]}}"), Valid),
            (Absent, "absent", String::new(), Valid),
            (ExplicitNull, "null", v("null"), Valid),
            (WrongWhole, "number", v("5"), Valid),
            (WrongWhole, "boolean", v("true"), Valid),
            (WrongWhole, "text", v("x"), Valid),
            (WrongOneElement, "mixed array", v("[1, x, null, {a: 1}]"), Valid),
            (WrongEveryElement, "array", v("[1, 2]"), Valid),
            (Empty, "empty string", v("''"), Valid),
            (Empty, "empty array", v("[]"), Valid),
            (Empty, "empty object", v("{}"), Valid),
            (DuplicateKey, "nested duplicate", "p:\n  a: 1\n  a: 2\n".to_string(), parse("duplicate entry with key \"a\"")),
        ];
        rows.extend(yaml_rows("x"));
        column(&mut matrix, name, declaration, rows);
    }

    // p: U
    let mut rows = vec![
        (Control, "control", v("abc"), Valid),
        (Absent, "absent", String::new(), Valid),
        (ExplicitNull, "null", v("null"), Valid),
        (WrongWhole, "number coerces", v("1234"), Valid),
        (WrongWhole, "boolean coerces", v("true"), Valid),
        (WrongWhole, "object", v("{a: 1}"), p.clone()),
        (WrongOneElement, "mixed sequence", v("[abc, 1]"), p.clone()),
        (WrongEveryElement, "invalid sequence", v("[1]"), p.clone()),
        (Empty, "empty string", v("''"), p.clone()),
        (Empty, "short", v("ab"), p.clone()),
    ];
    rows.extend(yaml_rows("abc"));
    column(&mut matrix, "U", "['string(required; min(3))', null]", rows.clone());
    // `U` resolved through a named type that carries `required`.
    for (shape, label, values, expect) in rows {
        matrix.cell(
            "U named",
            shape,
            label,
            format!("$schema:\n  code: 'string(min(3))'\n  word: 'string(required; min(3))'\n  p: [word, null]\nword: abcd\n{values}"),
            expect,
        );
    }

    // p: L, for three item types. Item nullability never changes container
    // presence.
    for (name, item, valid_item, one_bad, every_bad, one_ok, every_ok) in [
        ("L alphanumeric", "alphanumeric", "abc", "[abc, 123]", "[123]", false, false),
        ("L unknown", "unknown", "abc", "[abc, 123]", "[123]", true, true),
        ("L null", "null", "null", "[null, x]", "[x]", false, false),
    ] {
        let judged = |ok: bool| if ok { Valid } else { p.clone() };
        let mut rows = vec![
            (Control, "control", v(&format!("[{valid_item}]")), Valid),
            (Absent, "absent", String::new(), p.clone()),
            (ExplicitNull, "null", v("null"), p.clone()),
            (WrongWhole, "scalar", v("abc"), p.clone()),
            (WrongWhole, "object", v("{a: 1}"), p.clone()),
            (WrongOneElement, "one invalid item", v(one_bad), judged(one_ok)),
            (WrongEveryElement, "every item invalid", v(every_bad), judged(every_ok)),
            (Empty, "empty array", v("[]"), Valid),
            (DuplicateKey, "object item duplicate", "p:\n  - {a: 1, a: 2}\n".to_string(), parse("duplicate entry with key \"a\"")),
        ];
        rows.extend(yaml_rows(&format!("[{valid_item}]")));
        column(&mut matrix, name, &format!("'{item}[](required)'"), rows);
        // The item-count variant. The plan's `min-items(1)` is spelled
        // `[](min(1))`: an array-level `min`.
        let rows = vec![
            (Control, "control", v(&format!("[{valid_item}]")), Valid),
            (Absent, "absent", String::new(), p.clone()),
            (ExplicitNull, "null", v("null"), p.clone()),
            (WrongWhole, "scalar", v("abc"), p.clone()),
            (WrongOneElement, "one invalid item", v(one_bad), judged(one_ok)),
            (WrongEveryElement, "every item invalid", v(every_bad), judged(every_ok)),
            (Empty, "empty array", v("[]"), p.clone()),
            (DuplicateKey, "duplicate key", format!("p: [{valid_item}]\np: []\n"), parse("duplicate entry with key \"p\"")),
            (TrailingContent, "invalid yaml", "p: [abc\n".to_string(), parse("did not find expected ',' or ']'")),
        ];
        column(&mut matrix, &format!("{name} min(1)"), &format!("'{item}[](min(1); required)'"), rows);
    }

    // p: O
    let mut rows = vec![
        (Control, "control", v("{shell: echo}"), Valid),
        (Absent, "absent", String::new(), p.clone()),
        (ExplicitNull, "null", v("null"), p.clone()),
        (WrongWhole, "scalar", v("echo"), p.clone()),
        (WrongWhole, "array", v("[{shell: echo}]"), p.clone()),
        (WrongOneElement, "extra key", v("{shell: echo, say: hi}"), p.clone()),
        (WrongOneElement, "malformed shell", v("{shell: {a: 1}}"), p.clone()),
        (WrongOneElement, "blank shell", v("{shell: ' '}"), p.clone()),
        (WrongEveryElement, "all unexpected keys", v("{say: hi, set: 1}"), p.clone()),
        (Empty, "empty object", v("{}"), p.clone()),
        (DuplicateKey, "nested duplicate", "p:\n  shell: a\n  shell: b\n".to_string(), parse("duplicate entry with key \"shell\"")),
    ];
    rows.extend(yaml_rows("{shell: echo}"));
    column(&mut matrix, "O", "'{ shell: shell(required) }(required)'", rows);

    // Ordinary optional and required concrete types keep their presence.
    let controls = [
        ("string optional", "string", vec![("absent", String::new(), Valid), ("null", v("null"), Valid), ("number", v("{a: 1}"), p.clone())]),
        ("string required", "'string(required)'", vec![("absent", String::new(), p.clone()), ("null", v("null"), p.clone())]),
        ("number required", "'number(required)'", vec![("absent", String::new(), p.clone()), ("null", v("null"), p.clone())]),
        ("object required", "'object(required)'", vec![("absent", String::new(), p.clone()), ("empty", v("{}"), Valid)]),
    ];
    for (name, declaration, rows) in controls {
        for (label, values, expect) in rows {
            matrix.extra(&format!("concrete {name}"), label, format!("$schema:\n  p: {declaration}\n{values}"), expect);
        }
    }
    // Raw JSON Schema keeps its authored `required` list for nullable
    // properties.
    matrix.file(
        "raw.json",
        r#"{"type":"object","required":["p","q"],"properties":{"p":{"type":"null"},"q":{}}}"#,
    );
    for (label, values, expect) in [
        ("absent", "", Invalid("p")),
        ("q absent", "p: null\n", Invalid("q")),
        ("both null", "p: null\nq: null\n", Valid),
        ("p non-null", "p: 1\nq: null\n", Invalid("p")),
    ] {
        matrix.extra("raw json required nullable", label, format!("$schema: ./raw.json\n{values}"), expect);
    }
    matrix.assert();
}


// ── alphanumeric options ─────────────────────────────────────────────────

#[test]
fn alphanumeric_options_through_md_schema_validate() {
    let mut matrix = Matrix::default();
    use Expect::{Invalid, Valid};
    let p = Invalid("p");
    let add = |matrix: &mut Matrix, column: &str, label: &str, declaration: &str, values: &str, expect: Expect| {
        matrix.extra(column, label, format!("$schema:\n  p: \"{declaration}\"\n{values}"), expect);
    };

    // Each option with a positive and a negative value. A narrowing option is
    // paired with the allow-option that makes it meaningful.
    for (options, accepted, rejected) in [
        ("allow-dashes", "a-b", "a_b"),
        ("allow-underscores", "a_b", "a-b"),
        ("allow-slashes", "a/b", "'a\\b'"),
        ("allow-periods", "a.b", "a/b"),
        ("allow-whitespace", "a b", "\"a\\tb\""),
        ("allow-periods, block-double-periods", "a.b", "a..b"),
        ("allow-dashes, block-leading-dash", "a-", "-a"),
        ("allow-slashes, block-leading-slash", "a/", "/a"),
        ("leading-alpha", "abc", "1abc"),
    ] {
        let declaration = format!("alphanumeric({options}; required)");
        add(&mut matrix, "option", &format!("{options} accepts"), &declaration, &format!("p: {accepted}\n"), Valid);
        add(&mut matrix, "option", &format!("{options} rejects"), &declaration, &format!("p: {rejected}\n"), p.clone());
    }
    // The base set and its edges.
    for (label, value, expect) in [
        ("letters and digits", "abc1", Valid),
        ("empty string", "''", Valid),
        ("non-ASCII letter", "é", p.clone()),
        ("trailing newline", "\"abc\\n\"", p.clone()),
        ("leading newline", "\"\\nabc\"", p.clone()),
        ("space", "a b", p.clone()),
        ("native number", "5", p.clone()),
        ("native float", "1.5", p.clone()),
        ("native boolean", "true", p.clone()),
    ] {
        add(&mut matrix, "base", label, "alphanumeric(required)", &format!("p: {value}\n"), expect);
    }
    for constraint in ["min(1)", "not-empty"] {
        add(&mut matrix, "base", &format!("{constraint} empty"), &format!("alphanumeric({constraint}; required)"), "p: ''\n", p.clone());
    }
    // Native scalars are never coerced, as items or beside other arms.
    add(&mut matrix, "native", "array item", "alphanumeric[](required)", "p: [a, 12]\n", p.clone());
    add(&mut matrix, "native", "boolean item", "alphanumeric[](required)", "p: [a, false]\n", p.clone());
    matrix.extra("native", "number arm", "$schema:\n  p: [alphanumeric, number]\np: 7\n", Valid);
    matrix.extra("native", "boolean against number arm", "$schema:\n  p: [alphanumeric, number]\np: true\n", p.clone());
    // Overlapping patterns: the option pattern, a user pattern, and
    // `not-empty` all apply.
    let overlapping = "alphanumeric(allow-dashes; pattern('^a'); not-empty; required)";
    for (label, value, expect) in [
        ("all hold", "a-b", Valid),
        ("empty", "''", p.clone()),
        ("user pattern", "b-a", p.clone()),
        ("option pattern", "a_b", p.clone()),
        ("edge newline", "\"a-b\\n\"", p.clone()),
    ] {
        add(&mut matrix, "overlapping", label, overlapping, &format!("p: {value}\n"), expect);
    }
    // Defaults and suggestions must be valid values.
    add(&mut matrix, "default", "valid default", "alphanumeric(allow-dashes; default(a-b))", "", Valid);
    add(&mut matrix, "default", "valid suggestion", "alphanumeric(min(2); suggest(ab, abc))", "", Valid);
    for (label, declaration, value) in [
        ("default outside the set", "alphanumeric(default(a-b))", "default(\"a-b\")"),
        ("default below min", "alphanumeric(min(3); default(ab))", "default(\"ab\")"),
        // A native number is not coerced to text, even as a default.
        ("numeric default", "alphanumeric(default(12))", "default(12)"),
        ("suggestion outside the set", "alphanumeric(suggest(ab, a-b))", "suggest(\"a-b\")"),
        ("suggestion against leading-alpha", "alphanumeric(leading-alpha; suggest(1a))", "suggest(\"1a\")"),
    ] {
        add(
            &mut matrix,
            "default",
            label,
            declaration,
            "",
            load(&format!("`{value}` is not a valid `alphanumeric` value for this type")),
        );
    }
    // Option grammar errors.
    for (label, declaration, message) in [
        ("unknown option", "alphanumeric(bogus)", "unknown `alphanumeric` option `bogus`"),
        ("repeated option", "alphanumeric(allow-dashes, allow-dashes)", "repeated `alphanumeric` option `allow-dashes`"),
        ("option after separator", "alphanumeric(min(1); allow-dashes)", "option `allow-dashes` must be written before the first `;`"),
        ("mixed without separator", "alphanumeric(allow-dashes, min(1))", "separate `alphanumeric` options from constraints with `;`"),
    ] {
        add(&mut matrix, "grammar", label, declaration, "", load(message));
    }

    // Redundant options are semantically equivalent to their canonical form:
    // the same accept/reject results for every witness.
    let witnesses = ["''", "a", "abc1", "-a", "a-", "/a", "a/b", "a..b", "a.b", "1a", "a_b"];
    let pairs = [
        ("alphanumeric(block-leading-dash)", "alphanumeric"),
        ("alphanumeric(block-leading-slash)", "alphanumeric"),
        ("alphanumeric(block-double-periods)", "alphanumeric"),
        ("alphanumeric(allow-dashes, leading-alpha, block-leading-dash)", "alphanumeric(allow-dashes, leading-alpha)"),
        ("alphanumeric(allow-slashes, leading-alpha, block-leading-slash)", "alphanumeric(allow-slashes, leading-alpha)"),
        ("alphanumeric(leading-alpha, allow-periods)", "alphanumeric(allow-periods, leading-alpha)"),
    ];
    let mut equivalence = Vec::new();
    let mut compared = BTreeSet::new();
    for (index, (redundant, canonical)) in pairs.iter().enumerate() {
        for (witness_index, witness) in witnesses.iter().enumerate() {
            let names = [redundant, canonical].map(|declaration| {
                let column = format!("equivalence {index} {witness_index}");
                let frontmatter = format!("$schema:\n  p: '{declaration}'\np: {witness}\n");
                // The expectation is a placeholder: `run` only requires these
                // to validate, and the pairs are compared below.
                matrix.extra(&column, declaration, frontmatter, Invalid("p"));
                let name = matrix.cells.last().expect("just added").name.clone();
                compared.insert(name.clone());
                name
            });
            equivalence.push((redundant, canonical, witness, names));
        }
    }
    // Canonical identity at a use site: a named type may be repeated with an
    // equivalent spelling of its option set in any order, but a different
    // set is an error.
    matrix.file(
        "slugs.yaml",
        "kind: schema\ntypes:\n  \
         slug: alphanumeric(block-leading-slash, allow-dashes, leading-alpha, block-leading-dash)\n  \
         path: alphanumeric(allow-slashes, block-double-periods)\n",
    );
    for (label, declaration, values, expect) in [
        ("canonical spelling", "slug(allow-dashes, leading-alpha; max(3))@./slugs.yaml", "p: a-b\n", Valid),
        ("reordered spelling", "slug(leading-alpha, allow-dashes; max(3))@./slugs.yaml", "p: a-bc\n", p.clone()),
        ("redundant dropped", "path(allow-slashes)@./slugs.yaml", "p: a/b\n", Valid),
        (
            "different set",
            "slug(allow-dashes; max(3))@./slugs.yaml",
            "p: a-b\n",
            load("contradictory `alphanumeric` options on named type `slug`"),
        ),
        (
            "added narrowing option",
            "path(allow-slashes, block-leading-slash)@./slugs.yaml",
            "p: a/b\n",
            load("contradictory `alphanumeric` options on named type `path`"),
        ),
    ] {
        add(&mut matrix, "use site", label, declaration, values, expect);
    }

    let records = run(&matrix.files, &matrix.cells, &compared);
    let mut differences = Vec::new();
    for (redundant, canonical, witness, [left, right]) in equivalence {
        let outcome = |name: &str| records[name]["valid"].as_bool();
        if outcome(&left) != outcome(&right) {
            differences.push(format!("{redundant} vs {canonical} on {witness}: {} vs {}", records[&left], records[&right]));
        }
    }
    assert!(differences.is_empty(), "{differences:#?}");
}


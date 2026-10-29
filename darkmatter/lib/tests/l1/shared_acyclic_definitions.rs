//! Behavior of a repeated, structured, acyclic named type at each of its uses.
//!
//! `entry` is imported at two document properties, as a list item, and twice
//! inside `bundle`; `slug` is a primitive alias used twice. These outcomes
//! were recorded while every acyclic use was still inlined, so they hold
//! however the resolver stores the type: the verdict, the problem code, the
//! full instance path, the source line and column, the declared description,
//! and the property's declaration origin at each use.

use std::path::{Path, PathBuf};

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::validate::build_position_map;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, SchemaOriginKind, ValidationProblemCode, ValidationReport,
};
use serde_json::json;
use tempfile::TempDir;

const TYPES: &str = "kind: schema
types:
  entry: \"{ name: string(required) -> the entry name, size: number(min(0)) -> the entry size }\"
  bundle: \"{ entries: entry[](required) -> the bundled entries, primary: entry -> the primary entry }\"
  slug: \"alphanumeric(allow-dashes)\"
";

const SCHEMA: &str = "$schema:
  first: \"entry@./types.yaml -> the first entry\"
  second: \"entry(required)@./types.yaml -> the second entry\"
  items: \"entry[]@./types.yaml -> listed entries\"
  nest: \"bundle@./types.yaml\"
  label: \"slug(min(3))@./types.yaml\"
  tag: \"slug@./types.yaml\"
";

struct Fixture {
    _dir: TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = TempDir::new().expect("tempdir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical tempdir");
        std::fs::write(root.join("types.yaml"), TYPES).expect("write types");
        std::fs::write(root.join("schema.yaml"), SCHEMA).expect("write schema");
        Self { _dir: dir, root }
    }

    fn schemas(&self) -> DarkmatterSchemas {
        DarkmatterSchemas::new(FileResolutionContext::new(&self.root))
    }

    /// A document in the fixture directory whose frontmatter is
    /// `$schema: ./schema.yaml` followed by `body`.
    fn document(&self, body: &str) -> (Markdown, String) {
        let yaml = format!("$schema: ./schema.yaml\n{body}");
        let path = self.root.join("doc.md");
        let text = format!("---\n{yaml}---\nbody\n");
        std::fs::write(&path, &text).expect("write document");
        let markdown = Markdown::try_from(path.as_path()).expect("document loads").with_source(ComposeSource::File(path));
        (markdown, yaml)
    }

    fn effective(&self, markdown: &Markdown) -> EffectiveSchema {
        self.schemas().effective_for(markdown).expect("schema resolves").expect("an effective schema")
    }

    fn report(&self, body: &str) -> (ValidationReport, String, EffectiveSchema) {
        let (markdown, yaml) = self.document(body);
        let effective = self.effective(&markdown);
        let frontmatter = serde_json::Value::Object(
            markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect(),
        );
        let report = effective.validate_with_positions(&frontmatter, &build_position_map(&yaml));
        (report, yaml, effective)
    }
}

/// One problem as `(code, instance path, line, column, description)`.
type Tuple = (ValidationProblemCode, String, Option<u32>, Option<u32>, Option<String>);

fn tuples(report: &ValidationReport) -> Vec<Tuple> {
    let mut found: Vec<Tuple> = report
        .problems
        .iter()
        .map(|problem| {
            (problem.code, problem.path.clone(), problem.line, problem.column, problem.description.clone())
        })
        .collect();
    found.sort();
    found
}

/// The 1-based line (within the frontmatter YAML) and column of `needle`.
fn position_of(yaml: &str, needle: &str) -> (Option<u32>, Option<u32>) {
    yaml.lines()
        .enumerate()
        .find_map(|(index, line)| line.find(needle).map(|column| (Some(index as u32 + 1), Some(column as u32 + 1))))
        .unwrap_or_else(|| panic!("{needle:?} is in the frontmatter"))
}

fn assert_declared_by_schema_file(effective: &EffectiveSchema, root: &Path, property: &str) {
    let origin = effective.origins.get(property).unwrap_or_else(|| panic!("`{property}` has an origin"));
    assert_eq!(origin.kind, SchemaOriginKind::ReferencedFile, "{property}");
    assert_eq!(origin.file(), Some(root.join("schema.yaml").as_path()), "{property}");
}

const VALID: &str = "first:
  name: a
  size: 1
second:
  name: b
items:
  - name: c
  - name: d
    size: 0
nest:
  entries:
    - name: e
  primary:
    name: f
label: abc
tag: x
";

#[test]
fn every_use_of_a_repeated_type_accepts_a_valid_value() {
    let fixture = Fixture::new();
    let (report, _, effective) = fixture.report(VALID);
    assert!(report.valid, "{:#?}", report.problems);
    for property in ["first", "second", "items", "nest", "label", "tag"] {
        assert_declared_by_schema_file(&effective, &fixture.root, property);
    }
}

#[test]
fn an_invalid_value_is_located_at_each_of_two_uses() {
    let fixture = Fixture::new();
    let body = "first:\n  name: a\n  size: -1\nsecond:\n  name: { n: 7 }\n";
    let (report, yaml, effective) = fixture.report(body);
    let (size_line, size_column) = position_of(&yaml, "-1");
    let (name_line, name_column) = position_of(&yaml, "{ n: 7 }");
    assert_eq!(
        tuples(&report),
        vec![
            (
                ValidationProblemCode::TypeMismatch,
                "/second/name".to_string(),
                name_line,
                name_column,
                Some("the entry name".to_string())
            ),
            (
                ValidationProblemCode::ConstraintViolation,
                "/first/size".to_string(),
                size_line,
                size_column,
                Some("the entry size".to_string())
            ),
        ]
    );
    assert_declared_by_schema_file(&effective, &fixture.root, "first");
    assert_declared_by_schema_file(&effective, &fixture.root, "second");
}

#[test]
fn a_nested_list_item_and_a_bundled_use_are_located_at_full_paths() {
    let fixture = Fixture::new();
    let body = "second:\n  name: b\nitems:\n  - name: c\n  - name: d\n    size: big\nnest:\n  entries:\n    - name: e\n    - size: 2\n  primary:\n    name: [x]\n";
    let (report, yaml, _) = fixture.report(body);
    let (big_line, big_column) = position_of(&yaml, "big");
    let (x_line, x_column) = position_of(&yaml, "[x]");
    let (missing_line, missing_column) = position_of(&yaml, "- size: 2");
    assert_eq!(
        tuples(&report),
        vec![
            (
                ValidationProblemCode::MissingRequired,
                "/nest/entries/1".to_string(),
                missing_line,
                missing_column.map(|column| column + 2),
                Some("the entry name".to_string())
            ),
            (
                ValidationProblemCode::TypeMismatch,
                "/items/1/size".to_string(),
                big_line,
                big_column,
                Some("the entry size".to_string())
            ),
            (
                ValidationProblemCode::TypeMismatch,
                "/nest/primary/name".to_string(),
                x_line,
                x_column,
                Some("the entry name".to_string())
            ),
        ]
    );
}

#[test]
fn presence_and_null_belong_to_each_use() {
    let fixture = Fixture::new();

    // `first` is optional at its use, `second` required at its use; the body
    // requires `name` wherever the type appears.
    let (report, _, _) = fixture.report("first:\n  size: 1\n");
    let found = tuples(&report);
    assert_eq!(
        found.iter().map(|(code, path, ..)| (*code, path.as_str())).collect::<Vec<_>>(),
        vec![(ValidationProblemCode::MissingRequired, ""), (ValidationProblemCode::MissingRequired, "/first")],
        "{found:#?}"
    );
    let missing: Vec<_> = report.problems.iter().filter_map(|problem| problem.property.as_deref()).collect();
    assert_eq!(missing.len(), 2);
    assert!(missing.contains(&"second") && missing.contains(&"name"), "{missing:?}");

    // An optional use accepts null; the required use rejects it.
    let (report, _, _) = fixture.report("first: null\nsecond:\n  name: b\nnest: null\n");
    assert!(report.valid, "{:#?}", report.problems);
    let (report, _, _) = fixture.report("second: null\n");
    assert!(!report.valid);
    assert!(report.problems.iter().all(|problem| problem.path == "/second"), "{:#?}", report.problems);

    // An optional list does not make its items nullable.
    let (report, _, _) = fixture.report("second:\n  name: b\nitems: [null]\n");
    assert_eq!(
        tuples(&report).iter().map(|(code, path, ..)| (*code, path.clone())).collect::<Vec<_>>(),
        vec![(ValidationProblemCode::TypeMismatch, "/items/0".to_string())]
    );
}

#[test]
fn a_narrowed_primitive_alias_keeps_its_own_bound() {
    let fixture = Fixture::new();
    let (report, _, _) = fixture.report("second:\n  name: b\nlabel: ab\ntag: ab\n");
    assert_eq!(
        tuples(&report).iter().map(|(code, path, ..)| (*code, path.clone())).collect::<Vec<_>>(),
        vec![(ValidationProblemCode::ConstraintViolation, "/label".to_string())]
    );
}

#[test]
fn conversion_reaches_the_value_at_every_use() {
    let fixture = Fixture::new();
    let (markdown, _) = fixture.document("second:\n  name: b\n");
    let effective = fixture.effective(&markdown);
    let outcome = effective.coerce_frontmatter_with_pending(
        &json!({
            "first": { "name": "a", "size": "3" },
            "second": { "name": "b", "size": "4" },
            "items": [{ "name": "c", "size": "5" }],
            "nest": { "entries": [{ "name": "d", "size": "6" }], "primary": { "name": "e", "size": "7" } },
        }),
        &Default::default(),
    );
    assert_eq!(
        outcome.value,
        json!({
            "first": { "name": "a", "size": 3 },
            "second": { "name": "b", "size": 4 },
            "items": [{ "name": "c", "size": 5 }],
            "nest": { "entries": [{ "name": "d", "size": 6 }], "primary": { "name": "e", "size": 7 } },
        })
    );
}

//! The schema-trigger input robustness matrix
//! (`2026-10-27-darkmatter-lifecycle-ownership`, "Trigger envelope and
//! condition fields"): one test per cell, each a single source edit of one
//! control trigger, judged only through the public scan.
//!
//! The control is the dialect fixture `claudine.trigger.yaml` with its
//! `match:` rewritten as a top-level list that holds one node of every
//! column; its envelope, its `kind` condition, its `none` node, and its
//! payload file are the fixture's own. Every cell writes the edited trigger
//! into a temporary repository's `schemas/` folder and either
//!
//! - loads, and the documents [`evaluate_registry`] selects and the payload
//!   [`DarkmatterSchemas::effective_for`] layers are asserted;
//! - is not a trigger, so discovery succeeds without it; or
//! - fails discovery with a [`SchemaError::TriggerLoad`] for the trigger file
//!   whose message names the field (or, for YAML failures, the line).
//!
//! Duplicate keys and trailing garbage are edits of the source bytes, so the
//! YAML parser sees them before any mapping could merge them.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{ComposeSource, RequestSnapshot, build_resolution_context};
use darkmatter::markdown::schemas::triggers::evaluate_registry;
use darkmatter::markdown::schemas::{DarkmatterSchemas, SchemaError};
use serde_json::Value;

/// The fixture trigger the control is built from.
const FIXTURE_TRIGGER: &str = include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.trigger.yaml");

/// The fixture payload `claudine.yaml`, which declares `provider`.
const FIXTURE_PAYLOAD: &str = include_str!("../../../tests/fixtures/schema-triggers/schemas/claudine.yaml");

/// The fixture's `match:` mapping, replaced by [`MATCH_LIST`] in the control.
const FIXTURE_MATCH: &str = "match:\n  all:\n    - kind: enum(claudine; required)\n    - none:\n        - prompt: string(required)\n        - steps: unknown[](required)\n";

/// The control's `match:`: `match[0]` property, `[1]` none, `[2]` $path,
/// `[3]` all, `[4]` group, `[5]` min-match.
const MATCH_LIST: &str = "\
match:
  - kind: enum(claudine; required)
  - none:
      - prompt: string(required)
      - steps: unknown[](required)
  - $path: \"^docs/**\"
  - all:
      - owner: string(required)
      - team: string(required)
  - group:
      - mode: string(required)
      - level: number(required)
  - min-match:
      count: 2
      of:
        - tier: string(required)
        - rank: number(required)
        - seat: string(required)
";

fn control() -> String {
    assert_eq!(FIXTURE_TRIGGER.matches(FIXTURE_MATCH).count(), 1, "the fixture's `match:` changed");
    FIXTURE_TRIGGER.replace(FIXTURE_MATCH, MATCH_LIST)
}

/// The frontmatter every selected document shares.
const BASE: &[(&str, &str)] =
    &[("kind", "claudine"), ("owner", "o"), ("team", "t"), ("mode", "m"), ("tier", "t"), ("rank", "1")];

/// Frontmatter changes to [`BASE`]: `None` removes the key.
type Changes = &'static [(&'static str, Option<&'static str>)];

/// `(name, folder, frontmatter changes)`.
const DOCUMENTS: &[(&str, &str, Changes)] = &[
    ("selected", "docs", &[]),
    ("wrong-kind", "docs", &[("kind", Some("other"))]),
    ("outside-path", "notes", &[]),
    ("with-prompt", "docs", &[("prompt", Some("p"))]),
    ("no-team", "docs", &[("team", None)]),
    ("level-only", "docs", &[("mode", None), ("level", Some("2"))]),
    ("no-group", "docs", &[("mode", None)]),
    ("tier-only", "docs", &[("rank", None)]),
    ("tier-seat", "docs", &[("rank", None), ("seat", Some("s"))]),
];

/// The documents the unedited control selects.
const CONTROL_SELECTED: &[&str] = &["level-only", "selected", "tier-seat"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Column {
    Kind,
    Match,
    Payload,
    Group,
    All,
    None,
    MinMatch,
    Count,
    Of,
    Path,
    Property,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Shape {
    Control,
    Absent,
    Null,
    WrongType,
    OneWrong,
    EveryWrong,
    Empty,
    Duplicate,
    TrailingYaml,
    /// A cell outside the table's shapes (replacement guidance, an empty
    /// collection where the table's empty value is a string).
    Extra,
}

/// One source edit of the control.
enum Edit {
    None,
    Replace(&'static str, &'static str),
    /// Inserts a line after the line `anchor`; YAML failures are expected
    /// on the inserted line.
    InsertAfter(&'static str, &'static str),
}

#[derive(Clone, Copy)]
enum Payload {
    /// `provider` is layered onto the selected documents.
    Applied,
    /// Nothing is layered: a matcher-only trigger or an empty schema.
    Nothing,
}

enum Expect {
    Loads(&'static [&'static str], Payload),
    NotTrigger,
    Fails(&'static [&'static str]),
    /// A YAML failure reported on the inserted line.
    FailsOnInsertedLine,
}

/// A temporary repository holding the fixture payload, one trigger, and
/// [`DOCUMENTS`].
struct Repository {
    _dir: tempfile::TempDir,
    root: PathBuf,
    home: PathBuf,
}

impl Repository {
    fn new(trigger: &str) -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path().join("repo");
        let git = root.join(".git");
        std::fs::create_dir_all(git.join("objects")).expect("git objects");
        std::fs::create_dir_all(git.join("refs/heads")).expect("git refs");
        std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("git HEAD");
        std::fs::write(git.join("config"), "[core]\nrepositoryformatversion = 0\nbare = false\n").expect("git config");
        std::fs::create_dir_all(root.join("schemas")).expect("schemas dir");
        std::fs::write(root.join("schemas/claudine.yaml"), FIXTURE_PAYLOAD).expect("payload");
        std::fs::write(root.join("schemas/matrix.trigger.yaml"), trigger).expect("trigger");
        for (name, folder, changes) in DOCUMENTS {
            let mut frontmatter: Vec<(&str, &str)> = BASE.to_vec();
            for (key, value) in *changes {
                frontmatter.retain(|(existing, _)| existing != key);
                if let Some(value) = value {
                    frontmatter.push((key, value));
                }
            }
            let text: String = frontmatter.iter().map(|(key, value)| format!("{key}: {value}\n")).collect();
            let path = root.join(folder).join(format!("{name}.md"));
            std::fs::create_dir_all(path.parent().expect("folder")).expect("document folder");
            std::fs::write(&path, format!("---\n{text}---\nBody.\n")).expect("document");
        }
        let home = dir.path().join("home");
        std::fs::create_dir_all(&home).expect("home dir");
        Self { _dir: dir, root, home }
    }

    fn document(&self, name: &str) -> PathBuf {
        let (_, folder, _) = DOCUMENTS.iter().find(|(document, ..)| *document == name).expect("known document");
        self.root.join(folder).join(format!("{name}.md"))
    }

    fn schemas_for(&self, document: &Path) -> Result<DarkmatterSchemas, SchemaError> {
        let snapshot =
            RequestSnapshot::new(document.parent().expect("folder")).with_home(Some(self.home.clone()));
        let context = build_resolution_context(&snapshot).expect("fixture context");
        DarkmatterSchemas::new(context).with_trigger_discovery()
    }

    /// The documents the matrix trigger selects, or `None` when discovery
    /// loaded no trigger from it.
    fn selected(&self) -> Option<BTreeSet<&'static str>> {
        let mut selected = BTreeSet::new();
        let mut loaded = false;
        for (name, ..) in DOCUMENTS {
            let path = self.document(name);
            let schemas = self.schemas_for(&path).unwrap_or_else(|error| panic!("{name}: discovery failed: {error}"));
            let registry = schemas.trigger_registry().expect("registry");
            for evaluation in evaluate_registry(registry, &frontmatter(&path), Some(&path)) {
                if evaluation.trigger.source.file_name() == Some("matrix.trigger.yaml") {
                    loaded = true;
                    if evaluation.matched {
                        selected.insert(*name);
                    }
                }
            }
        }
        loaded.then_some(selected)
    }

    /// The effective schema's properties for document `name`.
    fn properties(&self, name: &str) -> BTreeSet<String> {
        let path = self.document(name);
        let markdown =
            Markdown::try_from(path.as_path()).expect("document").with_source(ComposeSource::File(path.clone()));
        let schemas = self.schemas_for(&path).expect("discovery");
        schemas
            .effective_for(&markdown)
            .expect("effective schema")
            .and_then(|effective| effective.json_schema["properties"].as_object().map(|map| map.keys().cloned().collect()))
            .unwrap_or_default()
    }

    /// The trigger-load error discovery reports, rendered.
    fn failure(&self) -> String {
        match self.schemas_for(&self.document("selected")) {
            Err(SchemaError::TriggerLoad { path, source }) => {
                assert!(path.ends_with("matrix.trigger.yaml"), "{}", path.display());
                source.to_string()
            }
            Err(other) => panic!("expected a trigger load error, got {other:?}"),
            Ok(_) => panic!("discovery loaded the edited trigger"),
        }
    }
}

fn frontmatter(path: &Path) -> Value {
    let markdown = Markdown::try_from(path).expect("document");
    Value::Object(markdown.frontmatter().as_map().iter().map(|(key, value)| (key.clone(), value.clone())).collect())
}

/// Applies `edit` to the control; returns the source and, for an insertion,
/// the one-based inserted line.
fn edited(edit: &Edit) -> (String, Option<usize>) {
    let control = control();
    match edit {
        Edit::None => (control, None),
        Edit::Replace(from, to) => {
            assert_eq!(control.matches(from).count(), 1, "edit anchor {from:?} must occur once");
            (control.replacen(from, to, 1), None)
        }
        Edit::InsertAfter(anchor, line) => {
            let lines: Vec<&str> = control.lines().collect();
            let at = lines.iter().position(|candidate| candidate == anchor).expect("anchor line");
            assert_eq!(lines.iter().filter(|candidate| *candidate == anchor).count(), 1, "anchor {anchor:?}");
            let mut out: Vec<&str> = lines[..=at].to_vec();
            out.push(line);
            out.extend(&lines[at + 1..]);
            (format!("{}\n", out.join("\n")), Some(at + 2))
        }
    }
}

fn run_cell(edit: Edit, expect: Expect) {
    let (source, inserted) = edited(&edit);
    let repository = Repository::new(&source);
    match expect {
        Expect::Loads(selected, payload) => {
            let actual = repository.selected().expect("the edited trigger loads");
            assert_eq!(actual, selected.iter().copied().collect::<BTreeSet<_>>(), "selection\n{source}");
            let applied = repository.properties("selected").contains("provider");
            match payload {
                Payload::Applied => assert!(applied, "payload not layered\n{source}"),
                Payload::Nothing => assert!(!applied, "a payload was layered\n{source}"),
            }
        }
        Expect::NotTrigger => {
            assert_eq!(repository.selected(), None, "the file loaded as a trigger\n{source}");
            assert!(!repository.properties("selected").contains("provider"));
        }
        Expect::Fails(fragments) => {
            let message = repository.failure();
            for fragment in fragments {
                assert!(message.contains(fragment), "`{fragment}` not in {message:?}\n{source}");
            }
        }
        Expect::FailsOnInsertedLine => {
            let message = repository.failure();
            let line = inserted.expect("an insertion");
            assert!(
                message.contains("not valid YAML") && message.contains(&format!("line {line},")),
                "expected a YAML failure on line {line}: {message:?}\n{source}"
            );
        }
    }
}

macro_rules! matrix {
    ($($test:ident: $column:ident / $shape:ident => $edit:expr, $expect:expr;)*) => {
        $(
            #[test]
            fn $test() {
                run_cell($edit, $expect);
            }
        )*

        /// Every generated cell, with whether it expects a load failure.
        const CELLS: &[(Column, Shape, bool)] = &[$((Column::$column, Shape::$shape, matches!($expect, Expect::Fails(_) | Expect::FailsOnInsertedLine))),*];
    };
}

const DUPLICATE: &[&str] = &["not valid YAML", "duplicate entry with key"];

matrix! {
    // ── kind ─────────────────────────────────────────────────────────────
    kind_control: Kind / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    kind_absent: Kind / Absent => Edit::Replace("kind: schema-trigger\n", ""), Expect::NotTrigger;
    kind_null: Kind / Null => Edit::Replace("kind: schema-trigger\n", "kind: null\n"), Expect::Fails(&["`kind`", "got null"]);
    kind_wrong_type: Kind / WrongType => Edit::Replace("kind: schema-trigger\n", "kind: 42\n"), Expect::Fails(&["`kind`", "got number"]);
    kind_one_wrong: Kind / OneWrong => Edit::Replace("kind: schema-trigger\n", "kind: [schema-trigger]\n"), Expect::Fails(&["`kind`", "got sequence"]);
    kind_every_wrong: Kind / EveryWrong => Edit::Replace("kind: schema-trigger\n", "kind: {schema-trigger: 1}\n"), Expect::Fails(&["`kind`", "got mapping"]);
    kind_empty: Kind / Empty => Edit::Replace("kind: schema-trigger\n", "kind: \"\"\n"), Expect::NotTrigger;
    kind_duplicate: Kind / Duplicate => Edit::InsertAfter("kind: schema-trigger", "kind: schema-trigger"), Expect::Fails(DUPLICATE);
    kind_trailing_yaml: Kind / TrailingYaml => Edit::InsertAfter("kind: schema-trigger", "notes: a: b"), Expect::FailsOnInsertedLine;
    kind_empty_collection: Kind / Extra => Edit::Replace("kind: schema-trigger\n", "kind: []\n"), Expect::Fails(&["`kind`", "got sequence"]);
    kind_renamed: Kind / Extra => Edit::Replace("kind: schema-trigger\n", "kind: trigger-schema\n"), Expect::Fails(&["`kind`", "`trigger-schema` was renamed", "use `schema-trigger`"]);
    kind_renamed_unparseable: Kind / Extra => Edit::Replace("kind: schema-trigger\n", "kind: trigger-schema\nnotes: a: b\n"), Expect::Fails(&["`kind`", "use `schema-trigger`"]);

    // ── match ────────────────────────────────────────────────────────────
    match_control: Match / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    match_absent: Match / Absent => Edit::Replace(MATCH_LIST, ""), Expect::Fails(&["`match`", "requires a `match:` key"]);
    match_null: Match / Null => Edit::Replace(MATCH_LIST, "match: null\n"), Expect::Fails(&["`match`", "got null"]);
    match_wrong_type: Match / WrongType => Edit::Replace(MATCH_LIST, "match: kind\n"), Expect::Fails(&["`match`", "got string"]);
    match_one_wrong: Match / OneWrong => Edit::Replace("  - kind: enum(claudine; required)\n", "  - 42\n"), Expect::Fails(&["`match[0]`", "got number"]);
    match_every_wrong: Match / EveryWrong => Edit::Replace(MATCH_LIST, "match: [1, true]\n"), Expect::Fails(&["`match[0]`", "got number"]);
    match_empty: Match / Empty => Edit::Replace(MATCH_LIST, "match: []\n"), Expect::Fails(&["`match`", "at least one condition"]);
    match_duplicate: Match / Duplicate => Edit::InsertAfter("$schema: claudine.yaml", "match: {kind: enum(claudine; required)}"), Expect::Fails(DUPLICATE);
    match_trailing_yaml: Match / TrailingYaml => Edit::InsertAfter("  - kind: enum(claudine; required)", "    notes: a: b"), Expect::FailsOnInsertedLine;
    match_empty_mapping: Match / Extra => Edit::Replace(MATCH_LIST, "match: {}\n"), Expect::Fails(&["`match`", "at least one key"]);

    // ── $schema payload ──────────────────────────────────────────────────
    payload_control: Payload / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    payload_absent: Payload / Absent => Edit::Replace("$schema: claudine.yaml\n", ""), Expect::Loads(CONTROL_SELECTED, Payload::Nothing);
    payload_null: Payload / Null => Edit::Replace("$schema: claudine.yaml\n", "$schema: null\n"), Expect::Fails(&["`$schema`", "explicit null"]);
    payload_wrong_type: Payload / WrongType => Edit::Replace("$schema: claudine.yaml\n", "$schema: 42\n"), Expect::Fails(&["`$schema`", "got number"]);
    payload_one_wrong: Payload / OneWrong => Edit::Replace("$schema: claudine.yaml\n", "$schema:\n  provider: string(required)\n  region: 42\n"), Expect::Fails(&["region"]);
    payload_every_wrong: Payload / EveryWrong => Edit::Replace("$schema: claudine.yaml\n", "$schema:\n  provider: 42\n  region: true\n"), Expect::Fails(&["provider"]);
    payload_empty: Payload / Empty => Edit::Replace("$schema: claudine.yaml\n", "$schema: {}\n"), Expect::Loads(CONTROL_SELECTED, Payload::Nothing);
    payload_duplicate: Payload / Duplicate => Edit::InsertAfter("$schema: claudine.yaml", "$schema: claudine.yaml"), Expect::Fails(DUPLICATE);
    payload_trailing_yaml: Payload / TrailingYaml => Edit::InsertAfter("$schema: claudine.yaml", "notes: a: b"), Expect::FailsOnInsertedLine;
    payload_reference_list: Payload / Extra => Edit::Replace("$schema: claudine.yaml\n", "$schema: [claudine.yaml, 42]\n"), Expect::Fails(&[]);

    // ── group ────────────────────────────────────────────────────────────
    group_control: Group / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    group_absent: Group / Absent => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", ""), Expect::Loads(&["level-only", "no-group", "selected", "tier-seat"], Payload::Applied);
    group_null: Group / Null => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - group: null\n"), Expect::Fails(&["`match[4].group`", "got null"]);
    group_wrong_type: Group / WrongType => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - group: mode\n"), Expect::Fails(&["`match[4].group`", "got string"]);
    group_one_wrong: Group / OneWrong => Edit::Replace("      - level: number(required)\n", "      - 42\n"), Expect::Fails(&["`match[4].group[1]`", "got number"]);
    group_every_wrong: Group / EveryWrong => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - group: [1, 2]\n"), Expect::Fails(&["`match[4].group[0]`", "got number"]);
    group_empty: Group / Empty => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - group: []\n"), Expect::Fails(&["`match[4].group`", "at least one child"]);
    group_duplicate: Group / Duplicate => Edit::InsertAfter("      - level: number(required)", "    group: [{mode: string(required)}]"), Expect::Fails(DUPLICATE);
    group_trailing_yaml: Group / TrailingYaml => Edit::InsertAfter("      - level: number(required)", "    notes: a: b"), Expect::FailsOnInsertedLine;
    group_any_renamed: Group / Extra => Edit::Replace("  - group:\n", "  - any:\n"), Expect::Fails(&["`match[4].any`", "use `group:`"]);
    group_anonymous_list: Group / Extra => Edit::Replace("  - group:\n      - mode: string(required)\n      - level: number(required)\n", "  - - mode: string(required)\n    - level: number(required)\n"), Expect::Fails(&["`match[4]`", "`group:`"]);

    // ── all ──────────────────────────────────────────────────────────────
    all_control: All / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    all_absent: All / Absent => Edit::Replace("  - all:\n      - owner: string(required)\n      - team: string(required)\n", ""), Expect::Loads(&["level-only", "no-team", "selected", "tier-seat"], Payload::Applied);
    all_null: All / Null => Edit::Replace("  - all:\n      - owner: string(required)\n      - team: string(required)\n", "  - all: null\n"), Expect::Fails(&["`match[3].all`", "got null"]);
    all_wrong_type: All / WrongType => Edit::Replace("  - all:\n      - owner: string(required)\n      - team: string(required)\n", "  - all: team\n"), Expect::Fails(&["`match[3].all`", "got string"]);
    all_one_wrong: All / OneWrong => Edit::Replace("      - team: string(required)\n", "      - 42\n"), Expect::Fails(&["`match[3].all[1]`", "got number"]);
    all_every_wrong: All / EveryWrong => Edit::Replace("  - all:\n      - owner: string(required)\n      - team: string(required)\n", "  - all: [1, 2]\n"), Expect::Fails(&["`match[3].all[0]`", "got number"]);
    all_empty: All / Empty => Edit::Replace("  - all:\n      - owner: string(required)\n      - team: string(required)\n", "  - all: []\n"), Expect::Fails(&["`match[3].all`", "at least one child"]);
    all_duplicate: All / Duplicate => Edit::InsertAfter("      - team: string(required)", "    all: [{owner: string(required)}]"), Expect::Fails(DUPLICATE);
    all_trailing_yaml: All / TrailingYaml => Edit::InsertAfter("      - team: string(required)", "    notes: a: b"), Expect::FailsOnInsertedLine;

    // ── none ─────────────────────────────────────────────────────────────
    none_control: None / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    none_absent: None / Absent => Edit::Replace("  - none:\n      - prompt: string(required)\n      - steps: unknown[](required)\n", ""), Expect::Loads(&["level-only", "selected", "tier-seat", "with-prompt"], Payload::Applied);
    none_null: None / Null => Edit::Replace("  - none:\n      - prompt: string(required)\n      - steps: unknown[](required)\n", "  - none: null\n"), Expect::Fails(&["`match[1].none`", "got null"]);
    none_wrong_type: None / WrongType => Edit::Replace("  - none:\n      - prompt: string(required)\n      - steps: unknown[](required)\n", "  - none: prompt\n"), Expect::Fails(&["`match[1].none`", "got string"]);
    none_one_wrong: None / OneWrong => Edit::Replace("      - steps: unknown[](required)\n", "      - 42\n"), Expect::Fails(&["`match[1].none[1]`", "got number"]);
    none_every_wrong: None / EveryWrong => Edit::Replace("  - none:\n      - prompt: string(required)\n      - steps: unknown[](required)\n", "  - none: [1, 2]\n"), Expect::Fails(&["`match[1].none[0]`", "got number"]);
    none_empty: None / Empty => Edit::Replace("  - none:\n      - prompt: string(required)\n      - steps: unknown[](required)\n", "  - none: []\n"), Expect::Fails(&["`match[1].none`", "at least one child"]);
    none_duplicate: None / Duplicate => Edit::InsertAfter("      - steps: unknown[](required)", "    none: [{prompt: string(required)}]"), Expect::Fails(DUPLICATE);
    none_trailing_yaml: None / TrailingYaml => Edit::InsertAfter("      - steps: unknown[](required)", "    notes: a: b"), Expect::FailsOnInsertedLine;

    // ── min-match ────────────────────────────────────────────────────────
    min_match_control: MinMatch / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    min_match_absent: MinMatch / Absent => Edit::Replace("  - min-match:\n      count: 2\n      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", ""), Expect::Loads(&["level-only", "selected", "tier-only", "tier-seat"], Payload::Applied);
    min_match_null: MinMatch / Null => Edit::Replace("  - min-match:\n      count: 2\n      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "  - min-match: null\n"), Expect::Fails(&["`match[5].min-match`", "got null"]);
    min_match_wrong_type: MinMatch / WrongType => Edit::Replace("  - min-match:\n      count: 2\n      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "  - min-match: [2]\n"), Expect::Fails(&["`match[5].min-match`", "got sequence"]);
    min_match_one_wrong: MinMatch / OneWrong => Edit::Replace("      count: 2\n", "      count: 2\n      quota: 3\n"), Expect::Fails(&["`match[5].min-match`", "found `quota`"]);
    min_match_every_wrong: MinMatch / EveryWrong => Edit::Replace("  - min-match:\n      count: 2\n      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "  - min-match: {count: two, of: tier}\n"), Expect::Fails(&["`match[5].min-match.count`"]);
    min_match_empty: MinMatch / Empty => Edit::Replace("  - min-match:\n      count: 2\n      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "  - min-match: {}\n"), Expect::Fails(&["`match[5].min-match.count`", "requires a `count`"]);
    min_match_duplicate: MinMatch / Duplicate => Edit::InsertAfter("        - seat: string(required)", "    min-match: {count: 1, of: [{tier: string(required)}]}"), Expect::Fails(DUPLICATE);
    min_match_trailing_yaml: MinMatch / TrailingYaml => Edit::InsertAfter("        - seat: string(required)", "    notes: a: b"), Expect::FailsOnInsertedLine;

    // ── min-match.count ──────────────────────────────────────────────────
    count_control: Count / Control => Edit::Replace("      count: 2\n", "      count: 1\n"), Expect::Loads(&["level-only", "selected", "tier-only", "tier-seat"], Payload::Applied);
    count_absent: Count / Absent => Edit::Replace("      count: 2\n", ""), Expect::Fails(&["`match[5].min-match.count`", "requires a `count`"]);
    count_null: Count / Null => Edit::Replace("      count: 2\n", "      count: null\n"), Expect::Fails(&["`match[5].min-match.count`", "got null"]);
    count_wrong_type: Count / WrongType => Edit::Replace("      count: 2\n", "      count: two\n"), Expect::Fails(&["`match[5].min-match.count`", "got string"]);
    count_one_wrong: Count / OneWrong => Edit::Replace("      count: 2\n", "      count: [2]\n"), Expect::Fails(&["`match[5].min-match.count`", "got sequence"]);
    count_every_wrong: Count / EveryWrong => Edit::Replace("      count: 2\n", "      count: {n: 2}\n"), Expect::Fails(&["`match[5].min-match.count`", "got mapping"]);
    count_empty: Count / Empty => Edit::Replace("      count: 2\n", "      count: \"\"\n"), Expect::Fails(&["`match[5].min-match.count`", "got string"]);
    count_duplicate: Count / Duplicate => Edit::InsertAfter("      count: 2", "      count: 1"), Expect::Fails(DUPLICATE);
    count_trailing_yaml: Count / TrailingYaml => Edit::InsertAfter("      count: 2", "      notes: a: b"), Expect::FailsOnInsertedLine;
    count_zero: Count / Extra => Edit::Replace("      count: 2\n", "      count: 0\n"), Expect::Fails(&["`match[5].min-match.count`", "at least 1"]);
    count_above_candidates: Count / Extra => Edit::Replace("      count: 2\n", "      count: 4\n"), Expect::Fails(&["`match[5].min-match.count`", "cannot exceed"]);
    count_negative: Count / Extra => Edit::Replace("      count: 2\n", "      count: -1\n"), Expect::Fails(&["`match[5].min-match.count`", "positive integer"]);

    // ── min-match.of ─────────────────────────────────────────────────────
    of_control: Of / Control => Edit::Replace("        - seat: string(required)\n", "        - desk: string(required)\n"), Expect::Loads(&["level-only", "selected"], Payload::Applied);
    of_absent: Of / Absent => Edit::Replace("      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", ""), Expect::Fails(&["`match[5].min-match.of`", "requires an `of` list"]);
    of_null: Of / Null => Edit::Replace("      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "      of: null\n"), Expect::Fails(&["`match[5].min-match.of`", "got null"]);
    of_wrong_type: Of / WrongType => Edit::Replace("      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "      of: tier\n"), Expect::Fails(&["`match[5].min-match.of`", "got string"]);
    of_one_wrong: Of / OneWrong => Edit::Replace("        - seat: string(required)\n", "        - 42\n"), Expect::Fails(&["`match[5].min-match.of[2]`", "got number"]);
    of_every_wrong: Of / EveryWrong => Edit::Replace("      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "      of: [1, 2, 3]\n"), Expect::Fails(&["`match[5].min-match.of[0]`", "got number"]);
    of_empty: Of / Empty => Edit::Replace("      of:\n        - tier: string(required)\n        - rank: number(required)\n        - seat: string(required)\n", "      of: []\n"), Expect::Fails(&["`match[5].min-match.of`", "at least one child"]);
    of_duplicate: Of / Duplicate => Edit::InsertAfter("        - seat: string(required)", "      of: [{tier: string(required)}]"), Expect::Fails(DUPLICATE);
    of_trailing_yaml: Of / TrailingYaml => Edit::InsertAfter("        - seat: string(required)", "      notes: a: b"), Expect::FailsOnInsertedLine;

    // ── $path ────────────────────────────────────────────────────────────
    path_control: Path / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    path_absent: Path / Absent => Edit::Replace("  - $path: \"^docs/**\"\n", ""), Expect::Loads(&["level-only", "outside-path", "selected", "tier-seat"], Payload::Applied);
    path_null: Path / Null => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: null\n"), Expect::Fails(&["`match[2].$path`", "got null"]);
    path_wrong_type: Path / WrongType => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: 42\n"), Expect::Fails(&["`match[2].$path`", "got number"]);
    path_one_wrong: Path / OneWrong => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: [\"^docs/**\", 42]\n"), Expect::Fails(&["`match[2].$path[1]`", "got number"]);
    path_every_wrong: Path / EveryWrong => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: [1, 2]\n"), Expect::Fails(&["`match[2].$path[0]`", "got number"]);
    path_empty: Path / Empty => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: \"\"\n"), Expect::Fails(&["`match[2].$path`"]);
    path_duplicate: Path / Duplicate => Edit::InsertAfter("  - $path: \"^docs/**\"", "    $path: \"^notes/**\""), Expect::Fails(DUPLICATE);
    path_trailing_yaml: Path / TrailingYaml => Edit::InsertAfter("  - $path: \"^docs/**\"", "    notes: a: b"), Expect::FailsOnInsertedLine;
    path_empty_list: Path / Extra => Edit::Replace("  - $path: \"^docs/**\"\n", "  - $path: []\n"), Expect::Fails(&["`match[2].$path`", "at least one glob pattern"]);

    // ── property condition ───────────────────────────────────────────────
    property_control: Property / Control => Edit::None, Expect::Loads(CONTROL_SELECTED, Payload::Applied);
    property_absent: Property / Absent => Edit::Replace("  - kind: enum(claudine; required)\n", ""), Expect::Loads(&["level-only", "selected", "tier-seat", "wrong-kind"], Payload::Applied);
    property_null: Property / Null => Edit::Replace("  - kind: enum(claudine; required)\n", "  - kind: null\n"), Expect::Fails(&["`match[0].kind`", "got null"]);
    property_wrong_type: Property / WrongType => Edit::Replace("  - kind: enum(claudine; required)\n", "  - kind: 42\n"), Expect::Fails(&["`match[0].kind`", "got number"]);
    property_one_wrong: Property / OneWrong => Edit::Replace("  - kind: enum(claudine; required)\n", "  - kind:\n      name: string(required)\n      size: 42\n"), Expect::Fails(&["size"]);
    property_every_wrong: Property / EveryWrong => Edit::Replace("  - kind: enum(claudine; required)\n", "  - kind:\n      name: 1\n      size: 42\n"), Expect::Fails(&["name"]);
    property_empty: Property / Empty => Edit::Replace("  - kind: enum(claudine; required)\n", "  - kind: \"\"\n"), Expect::Fails(&["kind"]);
    property_duplicate: Property / Duplicate => Edit::InsertAfter("  - kind: enum(claudine; required)", "    kind: string(required)"), Expect::Fails(DUPLICATE);
    property_trailing_yaml: Property / TrailingYaml => Edit::InsertAfter("  - kind: enum(claudine; required)", "    notes: a: b"), Expect::FailsOnInsertedLine;
}

/// The plan's table, one row per shape and one column per field, verbatim
/// up to the first `:`: `E` and `N/A` cells must fail, every other cell
/// must not.
const TABLE: &[(Shape, [&str; 11])] = &[
    (Shape::Control, ["schema-trigger discovered", "selects expected docs", "applies expected payload", "OR selection", "AND selection", "exclusion selection", "threshold selection", "threshold changes selection", "candidates change selection", "path selects docs", "type/presence selects docs"]),
    (Shape::Absent, ["non-trigger classification", "E", "matcher-only trigger, no payload", "no group node", "no all node", "no none node", "no threshold node", "E", "E", "no path predicate", "no property predicate"]),
    (Shape::Null, ["E", "E", "E", "E", "E", "E", "E", "E", "E", "E", "E"]),
    (Shape::WrongType, ["E", "E", "E", "E", "E", "E", "E", "E", "E", "E", "E"]),
    (Shape::OneWrong, ["N/A", "E", "E", "E", "E", "E", "E", "N/A", "E", "E", "E"]),
    (Shape::EveryWrong, ["N/A", "E", "E", "E", "E", "E", "E", "N/A", "E", "E", "E"]),
    (Shape::Empty, ["non-trigger empty string", "E", "preserve explicit empty schema", "E", "E", "E", "E", "E", "E", "E", "E"]),
    (Shape::Duplicate, ["E", "E", "E", "E", "E", "E", "E", "E", "E", "E", "E"]),
    (Shape::TrailingYaml, ["E", "E", "E", "E", "E", "E", "E", "E", "E", "E", "E"]),
];

const COLUMNS: [Column; 11] = [
    Column::Kind,
    Column::Match,
    Column::Payload,
    Column::Group,
    Column::All,
    Column::None,
    Column::MinMatch,
    Column::Count,
    Column::Of,
    Column::Path,
    Column::Property,
];

#[test]
fn the_generated_cells_are_exactly_the_table() {
    let mut expected = Vec::new();
    for (shape, row) in TABLE {
        for (column, outcome) in COLUMNS.iter().zip(row) {
            expected.push((*column, *shape, *outcome == "E" || *outcome == "N/A"));
        }
    }
    expected.sort();
    let mut generated: Vec<_> = CELLS.iter().copied().filter(|(_, shape, _)| *shape != Shape::Extra).collect();
    generated.sort();
    assert_eq!(generated, expected);
    // The replacement-guidance cells the plan requires beside the table.
    assert!(CELLS.contains(&(Column::Kind, Shape::Extra, true)), "old `trigger-schema`");
    assert!(CELLS.contains(&(Column::Group, Shape::Extra, true)), "old `any`");
}

//! Cleanup through `U[]` item unions: an item that fails every arm is
//! reported and left byte-for-byte unchanged (no quoting, no key deletion),
//! while a repairable non-union sibling is still quoted.
//!
//! Auto-apply stays gated on the document having exactly one raw problem, so
//! the sibling repair is proven beside a *valid* union list, and the mixed
//! document only proves the sibling is reported.

use std::path::{Path, PathBuf};

use biscuit_file::{YamlRepair, apply_edit_set};
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, SchemaCleanAnalysis, analyze_frontmatter,
};
use serde_json::Value;

use crate::request_support::context_at;
use crate::union_fixtures::{OVERLAPPING_TYPES, REPAIRABLE_SIBLING, TAGGED_TYPES, write_files};

/// `steps` is a literal-tagged union of closed object arms; `entries` is an
/// untagged union of closed object arms.
const SCHEMA: &str =
    "$schema:\n  steps: \"step[]@./tagged.yaml\"\n  entries: \"entry[]@./overlapping.yaml\"\n";

/// An untagged item with an extra key: it fails both closed `entry` arms.
const UNTAGGED_EXTRA_KEY: &str = "entries:\n  - name: x\n    extra: 1\n";

/// A `shell`-tagged item with an extra key: the tag names one arm, and the
/// item still fails it (and the `say` arm).
const TAGGED_EXTRA_KEY: &str = "steps:\n  - kind: shell\n    run: ls\n    extra: 1\n";

/// Valid items for both lists.
const VALID_ITEMS: &str = "entries:\n  - name: x\nsteps:\n  - kind: say\n    text: hi\n";

struct Fixture {
    _dir: tempfile::TempDir,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
        write_files(
            &root,
            &[
                ("tagged.yaml", TAGGED_TYPES),
                ("overlapping.yaml", OVERLAPPING_TYPES),
            ],
        );
        Self { _dir: dir, root }
    }

    /// Writes `body` as the document's frontmatter, loads it from disk, and
    /// resolves its effective schema against the document's directory.
    fn effective(&self, body: &str) -> EffectiveSchema {
        let path = self.root.join("doc.md");
        std::fs::write(&path, format!("---\n{body}---\nBody\n")).expect("write document");
        let markdown = Markdown::try_from(path.as_path()).expect("document loads");
        DarkmatterSchemas::new(context_at(Path::new(&self.root)))
            .effective_for(&markdown)
            .expect("schema resolves")
            .expect("the document declares a schema")
    }
}

/// Applies every auto-apply-eligible repair, as `md clean` does.
fn auto_apply(analysis: &SchemaCleanAnalysis, body: &str) -> String {
    let repairs: Vec<YamlRepair> = analysis
        .diagnostics()
        .iter()
        .filter(|diagnostic| diagnostic.classification.is_auto_apply_eligible())
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect();
    apply_edit_set(body, &repairs).source
}

/// Applies every repair any diagnostic carries, eligible or not: a careless
/// caller must still not be able to edit a union item.
fn apply_all(analysis: &SchemaCleanAnalysis, body: &str) -> String {
    let repairs: Vec<YamlRepair> = analysis
        .diagnostics()
        .iter()
        .flat_map(|diagnostic| diagnostic.repairs.iter().cloned())
        .collect();
    apply_edit_set(body, &repairs).source
}

/// The frontmatter body as the JSON instance validators see.
fn instance(body: &str) -> Value {
    let mut value: Value = serde_yaml_ng::from_str(body).expect("frontmatter parses");
    value.as_object_mut().expect("mapping").remove("$schema");
    value
}

/// The compose-parity (coerced) problem paths for `body`.
fn coerced_problem_paths(effective: &EffectiveSchema, body: &str) -> Vec<String> {
    effective
        .validate(&instance(body))
        .problems
        .into_iter()
        .map(|problem| problem.path)
        .collect()
}

/// Asserts cleanup leaves `body` byte-identical whichever repairs a caller
/// applies, and that the analysis reports one diagnostic per coerced problem.
fn assert_unrepaired_and_reported(fixture: &Fixture, body: &str) -> Vec<String> {
    let effective = fixture.effective(body);
    let analysis = analyze_frontmatter(&effective, body);
    assert_eq!(
        auto_apply(&analysis, body),
        body,
        "auto-apply must not touch a union item"
    );
    assert_eq!(
        apply_all(&analysis, body),
        body,
        "no diagnostic may carry a union-item repair"
    );
    let paths = coerced_problem_paths(&effective, body);
    assert_eq!(
        analysis.diagnostics().len(),
        paths.len(),
        "every problem is reported: {:?}",
        analysis.diagnostics()
    );
    paths
}

#[test]
fn union_items_with_extra_keys_are_reported_and_left_unchanged() {
    let fixture = Fixture::new();
    let body = format!("{SCHEMA}{UNTAGGED_EXTRA_KEY}{TAGGED_EXTRA_KEY}");

    let paths = assert_unrepaired_and_reported(&fixture, &body);

    assert!(
        paths.iter().any(|path| path.starts_with("/entries/0")),
        "the untagged item is reported: {paths:?}"
    );
    assert!(
        paths.iter().any(|path| path.starts_with("/steps/0")),
        "the tagged item is reported: {paths:?}"
    );
}

#[test]
fn union_item_with_extra_key_alone_keeps_its_key() {
    let fixture = Fixture::new();
    for (item, prefix) in [
        (UNTAGGED_EXTRA_KEY, "/entries/0"),
        (TAGGED_EXTRA_KEY, "/steps/0"),
    ] {
        let body = format!("{SCHEMA}{item}");
        let raw = fixture
            .effective(&body)
            .validate_raw(&instance(&body))
            .problems;
        assert_eq!(
            raw.len(),
            1,
            "a single raw problem opens the auto-apply gate: {raw:?}"
        );

        let paths = assert_unrepaired_and_reported(&fixture, &body);

        assert!(
            paths.iter().all(|path| path.starts_with(prefix)),
            "{item}: {paths:?}"
        );
    }
}

/// The sharp case for the schema walk: `name` is a string in *every* `entry`
/// arm and the mistyped scalar is the document's only raw problem, so a walk
/// that read `items.anyOf` as one object arm would prove and auto-apply the
/// quote. Both arms read `1.20` as the same text `"1.2"`, so the conversion
/// engine converts it and composition reports nothing; cleanup still leaves
/// the item untouched. A tagged item whose tag names the arm is likewise
/// never quoted.
#[test]
fn mistyped_scalar_inside_union_item_is_not_quoted() {
    let fixture = Fixture::new();

    let untagged = format!("{SCHEMA}entries:\n  - name: 1.20\n");
    let raw = fixture
        .effective(&untagged)
        .validate_raw(&instance(&untagged))
        .problems;
    assert_eq!(
        raw.len(),
        1,
        "a single raw problem opens the auto-apply gate: {raw:?}"
    );
    let paths = assert_unrepaired_and_reported(&fixture, &untagged);
    assert!(paths.is_empty(), "{paths:?}");

    let tagged = format!("{SCHEMA}steps:\n  - kind: shell\n    run: 1.20\n");
    let effective = fixture.effective(&tagged);
    let analysis = analyze_frontmatter(&effective, &tagged);
    assert_eq!(apply_all(&analysis, &tagged), tagged);
}

#[test]
fn repairable_sibling_beside_valid_union_items_is_quoted() {
    let fixture = Fixture::new();
    let (sibling_schema, sibling_value) = REPAIRABLE_SIBLING;
    let body = format!("{SCHEMA}  {sibling_schema}\n{VALID_ITEMS}{sibling_value}");
    let effective = fixture.effective(&body);

    let applied = auto_apply(&analyze_frontmatter(&effective, &body), &body);

    assert_eq!(applied, body.replace("label: 1.20", "label: \"1.20\""));
    assert!(effective.validate_raw(&instance(&applied)).valid);
}

/// Documents the one-problem gate: beside an invalid union item the sibling
/// is reported, with nothing auto-applied.
#[test]
fn repairable_sibling_beside_invalid_union_item_is_reported_not_applied() {
    let fixture = Fixture::new();
    let (sibling_schema, sibling_value) = REPAIRABLE_SIBLING;
    let body = format!("{SCHEMA}  {sibling_schema}\n{UNTAGGED_EXTRA_KEY}{sibling_value}");
    let effective = fixture.effective(&body);

    let analysis = analyze_frontmatter(&effective, &body);

    assert_eq!(apply_all(&analysis, &body), body);
    let label = body.rfind("1.20").expect("sibling value");
    assert!(
        analysis
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.span == (label..label + 4)),
        "the sibling is reported: {:?}",
        analysis.diagnostics()
    );
    assert!(
        coerced_problem_paths(&effective, &body)
            .iter()
            .any(|path| path == "/entries/0"),
        "the union item is reported"
    );
    assert_eq!(
        analysis.diagnostics().len(),
        2,
        "{:?}",
        analysis.diagnostics()
    );
}

//! Union-arm validators retained by one eager-file rewrite.
//!
//! The rewrite takes each eligible arm's validator from the shared
//! `ValidatorCache`, keyed by the context it judges in, and keeps the handle
//! for the rest of the invocation. Compilation is therefore per distinct arm
//! and context, never per list item, while every value and every file's
//! existence is still judged on each call.
//!
//! Compile counts are asserted only with the `work-counters` feature; the
//! verdict assertions run either way.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::{CallerInputRecord, CallerInputRecords, ComposeSource};
use darkmatter::markdown::schemas::{DarkmatterSchemas, EffectiveSchema};
use serde_json::{Value, json};

use crate::request_support::context_at;

/// Two eager-file object arms told apart by a literal tag.
const TAGGED: &str = "kind: schema\ntypes:\n  \
    tagged:\n    \
    - \"{ kind: literal(doc; required), path: file(eager; required) }\"\n    \
    - \"{ kind: literal(spec; required), spec: file(eager; required) }\"\n";

/// [`TAGGED`] with the `spec` arm's body changed to require a `note`.
const TAGGED_SPEC_NEEDS_NOTE: &str = "kind: schema\ntypes:\n  \
    tagged:\n    \
    - \"{ kind: literal(doc; required), path: file(eager; required) }\"\n    \
    - \"{ kind: literal(spec; required), spec: file(eager; required), note: string(required) }\"\n";

/// `pick`'s `tree` arm accepts a value only through the recursive `node`
/// definition, which lowers to a shared root `$defs` entry; `overlap`'s two
/// arms both accept `{ path }`.
const SHARED: &str = "kind: schema\ntypes:\n  \
    node: \"{ doc: file(eager), children: node[](required) }\"\n  \
    pick:\n    \
    - \"{ kind: literal(leaf; required), doc: file(eager; required) }\"\n    \
    - \"{ kind: literal(tree; required), tree: node(required) }\"\n  \
    overlap:\n    \
    - \"{ path: file(eager; required) }\"\n    \
    - \"{ path: file(eager; required), size: number }\"\n";

/// A canonical temp root holding `types` as `types.yaml` and the targets
/// `a.md` and `b.md`.
fn fixture(types: &str) -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    std::fs::write(root.join("types.yaml"), types).expect("write types");
    std::fs::write(root.join("a.md"), "# A\n").expect("write a.md");
    std::fs::write(root.join("b.md"), "# B\n").expect("write b.md");
    (dir, root)
}

/// Writes `doc.md` declaring `schema` (lines indented under `$schema:`) and
/// prepares its effective schema through `schemas`.
fn effective_with(schemas: &DarkmatterSchemas, root: &Path, schema: &str) -> EffectiveSchema {
    let path = root.join("doc.md");
    std::fs::write(&path, format!("---\n$schema:\n{schema}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    schemas
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("declared schema")
}

fn prepare(root: &Path, schema: &str) -> EffectiveSchema {
    effective_with(&DarkmatterSchemas::new(context_at(root)), root, schema)
}

fn normalize(effective: &EffectiveSchema, instance: &Value) -> Value {
    effective
        .normalize_frontmatter(instance, &HashSet::new())
        .value
}

/// `count` alternating `doc`/`spec` items with `./`-relative paths.
fn tagged_items(count: usize) -> Value {
    Value::Array(
        (0..count)
            .map(|index| {
                if index % 2 == 0 {
                    json!({ "kind": "doc", "path": "./a.md" })
                } else {
                    json!({ "kind": "spec", "spec": "./b.md" })
                }
            })
            .collect(),
    )
}

const FILES: &str = "  files: \"tagged[]@./types.yaml\"\n";

/// The counters a compile count is measured from; nothing without the
/// `work-counters` feature, where [`assert_compiles`] checks nothing.
#[cfg(feature = "work-counters")]
type Compiles = darkmatter::work_counters::WorkSnapshot;
#[cfg(not(feature = "work-counters"))]
struct Compiles;

fn compiles_now() -> Compiles {
    #[cfg(feature = "work-counters")]
    return darkmatter::work_counters::snapshot();
    #[cfg(not(feature = "work-counters"))]
    Compiles
}

/// Asserts that `expected` validators were compiled since `before`, every one
/// of them a rewrite arm.
#[cfg_attr(not(feature = "work-counters"), allow(unused_variables))]
fn assert_compiles(label: &str, before: &Compiles, expected: u64) {
    #[cfg(feature = "work-counters")]
    {
        use darkmatter::work_counters::{Work, snapshot};
        let work = snapshot().since(before);
        assert_eq!(
            work.get(Work::ValidatorCompile),
            expected,
            "{label}: validators compiled"
        );
        assert_eq!(
            work.get(Work::RewriteArmCompile),
            expected,
            "{label}: rewrite arms compiled"
        );
    }
}

#[test]
fn a_cold_rewrite_compiles_each_arm_once_whatever_the_batch_size() {
    for count in [10, 100, 1_000] {
        // A fresh root is a fresh context, so each batch starts cold.
        let (_dir, root) = fixture(TAGGED);
        let effective = prepare(&root, FILES);
        let instance = json!({ "files": tagged_items(count) });

        let cold = compiles_now();
        let normalized = normalize(&effective, &instance);
        assert_compiles(&format!("{count} items, cold"), &cold, 2);
        let files = normalized["files"].as_array().expect("files stay a list");
        assert_eq!(files.len(), count);
        for (index, item) in files.iter().enumerate() {
            let expected = if index % 2 == 0 {
                json!({ "kind": "doc", "path": "a.md" })
            } else {
                json!({ "kind": "spec", "spec": "b.md" })
            };
            assert_eq!(item, &expected, "{count} items: item {index}");
        }

        let warm = compiles_now();
        let again = normalize(&effective, &instance);
        assert_compiles(&format!("{count} items, warm"), &warm, 0);
        assert_eq!(again, normalized, "{count} items: the warm rewrite agrees");

        // A second request prepares its own effective schema in the same
        // context and reuses the arms the first one compiled.
        let second_request = prepare(&root, FILES);
        let warm_request = compiles_now();
        assert_eq!(normalize(&second_request, &instance), normalized);
        assert_compiles(&format!("{count} items, second request"), &warm_request, 0);
    }
}

#[test]
fn a_changed_arm_body_is_recompiled_and_judges_with_its_new_body() {
    let (_dir, root) = fixture(TAGGED);
    let instance = json!({ "files": tagged_items(4) });
    let before = normalize(&prepare(&root, FILES), &instance);
    assert_eq!(
        before["files"][1],
        json!({ "kind": "spec", "spec": "b.md" })
    );

    std::fs::write(root.join("types.yaml"), TAGGED_SPEC_NEEDS_NOTE).expect("edit types");
    let edited = prepare(&root, FILES);
    let changed = compiles_now();
    let after = normalize(&edited, &instance);

    assert_compiles("only the edited arm", &changed, 1);
    assert_eq!(
        after["files"][0],
        json!({ "kind": "doc", "path": "a.md" }),
        "the unchanged arm still rewrites"
    );
    assert_eq!(
        after["files"][1],
        json!({ "kind": "spec", "spec": "./b.md" }),
        "the edited arm now requires `note`, so no arm accepts the item"
    );
}

#[test]
fn an_arm_accepting_only_through_a_shared_definition_is_selected() {
    let (_dir, root) = fixture(SHARED);
    let effective = prepare(&root, "  items: \"pick[]@./types.yaml\"\n");
    let tree = json!({
        "kind": "tree",
        "tree": { "doc": "./a.md", "children": [{ "doc": "./b.md", "children": [] }] },
    });
    // `children` is required by the definition, so this tree is rejected.
    let broken = json!({ "kind": "tree", "tree": { "doc": "./a.md" } });
    let leaf = json!({ "kind": "leaf", "doc": "./b.md" });

    let normalized = normalize(
        &effective,
        &json!({ "items": [tree, broken.clone(), leaf] }),
    );

    assert_eq!(
        normalized["items"],
        json!([
            { "kind": "tree", "tree": { "doc": "a.md", "children": [{ "doc": "b.md", "children": [] }] } },
            broken,
            { "kind": "leaf", "doc": "b.md" },
        ])
    );
}

#[test]
fn zero_or_several_accepting_arms_leave_each_item_unchanged() {
    let (_dir, root) = fixture(SHARED);
    std::fs::write(root.join("tagged.yaml"), TAGGED).expect("write tagged types");
    let effective = prepare(
        &root,
        "  files: \"tagged[]@./tagged.yaml\"\n  paths: \"overlap[]@./types.yaml\"\n",
    );
    let unknown_tag = json!({ "kind": "other", "path": "./a.md" });
    let instance = json!({
        "files": [{ "kind": "doc", "path": "./a.md" }, unknown_tag.clone(), { "kind": "spec", "spec": "./b.md" }],
        "paths": [{ "path": "./a.md" }, { "path": "./b.md", "size": 2 }],
    });

    let normalized = normalize(&effective, &instance);

    assert_eq!(
        normalized["files"],
        json!([{ "kind": "doc", "path": "a.md" }, unknown_tag, { "kind": "spec", "spec": "b.md" }]),
        "an item no arm accepts stays as written beside items that are rewritten"
    );
    assert_eq!(
        normalized["paths"],
        json!([{ "path": "./a.md" }, { "path": "b.md", "size": 2 }]),
        "an item both arms accept is never guessed; one only the sized arm accepts is rewritten"
    );
}

#[test]
fn a_file_created_or_removed_between_uses_changes_the_verdict() {
    let (_dir, root) = fixture(TAGGED);
    let effective = prepare(&root, FILES);
    let instance = json!({ "files": [{ "kind": "doc", "path": "./later.md" }] });
    let unchanged = instance["files"].clone();

    assert_eq!(
        normalize(&effective, &instance)["files"],
        unchanged,
        "missing: no arm accepts"
    );

    std::fs::write(root.join("later.md"), "# Later\n").expect("create later.md");
    let created = compiles_now();
    assert_eq!(
        normalize(&effective, &instance)["files"],
        json!([{ "kind": "doc", "path": "later.md" }]),
        "created: the retained validator judges the file afresh"
    );
    assert_compiles("after creating the file", &created, 0);

    std::fs::remove_file(root.join("later.md")).expect("remove later.md");
    let removed = compiles_now();
    assert_eq!(
        normalize(&effective, &instance)["files"],
        unchanged,
        "removed: no arm accepts again"
    );
    assert_compiles("after removing the file", &removed, 0);
}

/// A root union's arms see the whole document, so a caller-supplied
/// property's arm validators are compiled for the caller's origin and kept
/// apart from the document's.
#[test]
fn document_and_caller_origins_use_separate_root_arm_validators() {
    let (_dir, root) = fixture(TAGGED);
    let launch = root.join("launch");
    std::fs::create_dir_all(&launch).expect("create launch");
    let schema = "  - kind: literal(doc; required)\n    path: file(eager; required)\n  \
        - kind: literal(spec; required)\n    spec: file(eager; required)\n";
    let records: CallerInputRecords = [(
        "path".to_string(),
        CallerInputRecord::new(json!("./a.md"), context_at(&launch)),
    )]
    .into_iter()
    .collect();
    let schemas = DarkmatterSchemas::new(context_at(&root));
    let document = effective_with(&schemas, &root, schema);
    let entries_for_document = schemas.cache().len();
    let caller = effective_with(
        &schemas.clone().with_caller_input_records(&records),
        &root,
        schema,
    );
    assert!(
        schemas.cache().len() >= entries_for_document + 2,
        "the caller's origin compiles its own two root arms"
    );

    let instance = json!({ "kind": "doc", "path": "./a.md" });
    let rewrite = compiles_now();
    let from_document = normalize(&document, &instance);
    let from_caller = normalize(&caller, &instance);

    assert_compiles("each origin reuses its own root arms", &rewrite, 0);
    assert_eq!(from_document, json!({ "kind": "doc", "path": "a.md" }));
    assert_eq!(from_caller, from_document);
}

/// The `$schema` block of R1-F2's root union: arm 0 declares the recursive
/// `node` and types `spec` with `spelling`; arm 1 accepts only a number, so a
/// file value selects arm 0.
fn origin_schema(spelling: &str) -> String {
    format!(
        "  - node: \"file(eager; match(./*.md)) | {{ children: node[] }}\"\n    \
         spec: {spelling}\n  - spec: number(required)\n"
    )
}

/// [`fixture`] plus `launch/a.md`, the file a caller captured at `launch/`
/// names.
fn origin_fixture() -> (tempfile::TempDir, PathBuf) {
    let (dir, root) = fixture(TAGGED);
    std::fs::create_dir_all(root.join("launch")).expect("create launch");
    std::fs::write(root.join("launch/a.md"), "# launch A\n").expect("write launch/a.md");
    (dir, root)
}

/// `launch/a.md`'s absolute path, as a caller captured at `launch/` supplies
/// it.
fn launch_file(root: &Path) -> Value {
    json!(root.join("launch").join("a.md").to_string_lossy())
}

/// The effective schema for `schema`, with each `(property, raw)` of
/// `callers` recorded as supplied by a caller captured at `launch/`.
fn prepare_with_callers(root: &Path, schema: &str, callers: &[(&str, &Value)]) -> EffectiveSchema {
    let launch = context_at(&root.join("launch"));
    let records: CallerInputRecords = callers
        .iter()
        .map(|(property, raw)| {
            (property.to_string(), CallerInputRecord::new((*raw).clone(), launch.clone()))
        })
        .collect();
    effective_with(
        &DarkmatterSchemas::new(context_at(root)).with_caller_input_records(&records),
        root,
        schema,
    )
}

/// Validates `instance` (asserting it is accepted) and returns its
/// normalized form.
fn validated_normalized(row: &str, effective: &EffectiveSchema, instance: &Value) -> Value {
    let report = effective.validate(instance);
    assert!(report.valid, "{row}: validation accepts {instance}: {:?}", report.problems);
    normalize(effective, instance)
}

/// `wrap(value)` places a file value in the row's container shape.
fn origin_row(row: &str, spelling: &str, wrap: fn(Value) -> Value, caller: bool) {
    let (_dir, root) = origin_fixture();
    let schema = origin_schema(spelling);
    let (raw, expected) = if caller {
        (launch_file(&root), json!("launch/a.md"))
    } else {
        (json!("./a.md"), json!("a.md"))
    };
    let spec = wrap(raw);
    let effective = if caller {
        prepare_with_callers(&root, &schema, &[("spec", &spec)])
    } else {
        prepare(&root, &schema)
    };

    let normalized = validated_normalized(row, &effective, &json!({ "spec": spec }));

    assert_eq!(normalized["spec"], wrap(expected), "{row}: `spec: {spelling}`");
}

fn bare(value: Value) -> Value {
    value
}

fn listed(value: Value) -> Value {
    json!([value])
}

fn under_path(value: Value) -> Value {
    json!({ "path": value })
}

/// R1-F2: a value inside a root-union arm is judged and rewritten from the
/// origin of the top-level property that owns it, whether the file arm is
/// reached directly, through a reference, a property union, a list, or an
/// inline object. Each row validates, then normalizes to the path its own
/// origin names.
macro_rules! origin_rows {
    ($($name:ident: $row:literal, $spelling:literal, $wrap:ident, $caller:literal;)+) => {
        $(
            #[test]
            fn $name() {
                origin_row($row, $spelling, $wrap, $caller);
            }
        )+
    };
}

origin_rows! {
    caller_origin_direct_root_arm_file: "R1-F2.1", "file(eager; required; match(./*.md))", bare, true;
    caller_origin_reference_to_recursive_union: "R1-F2.2", "node(required)", bare, true;
    caller_origin_property_union_with_reference: "R1-F2.3", "\"node | number\"", bare, true;
    caller_origin_list_of_recursive_union: "R1-F2.4", "node[]", listed, true;
    caller_origin_inline_object_of_recursive_union: "R1-F2.5", "\"{ path: node }\"", under_path, true;
    document_origin_direct_root_arm_file: "R1-F2.6", "file(eager; required; match(./*.md))", bare, false;
    document_origin_reference_to_recursive_union: "R1-F2.7", "node(required)", bare, false;
    document_origin_property_union_with_reference: "R1-F2.8", "\"node | number\"", bare, false;
    document_origin_list_of_recursive_union: "R1-F2.9", "node[]", listed, false;
    document_origin_inline_object_of_recursive_union: "R1-F2.10", "\"{ path: node }\"", under_path, false;
}

/// R1-F2.11: two top-level properties typed by the same recursive union in
/// one document, one supplied by a caller from `launch/` and one authored in
/// the document, are each judged and rewritten from their own origin. The
/// retained arm validators are therefore kept per origin, and a list under
/// one property still compiles each distinct arm once.
#[test]
fn caller_origin_and_document_origin_properties_keep_their_own_arm_validators() {
    let (_dir, root) = origin_fixture();
    let schema = "  - node: \"file(eager; match(./*.md)) | { children: node[] }\"\n    \
        spec: node[]\n    other: node[]\n  - spec: number(required)\n";
    let items = |raw: Value, count: usize| Value::Array(vec![raw; count]);
    let spec = items(launch_file(&root), 20);
    let other = items(json!("./a.md"), 20);
    let effective = prepare_with_callers(&root, schema, &[("spec", &spec)]);
    let instance = json!({ "spec": spec, "other": other });

    let cold = compiles_now();
    let normalized = validated_normalized("R1-F2.11", &effective, &instance);
    // `validate` compiles nothing new: `effective_for` already compiled the
    // root arms. Each property's union has two arms, one per origin and one
    // validator each, however many items the list holds.
    assert_compiles("R1-F2.11, cold", &cold, 4);

    assert_eq!(normalized["spec"], items(json!("launch/a.md"), 20), "caller-supplied `spec`");
    assert_eq!(normalized["other"], items(json!("a.md"), 20), "document-authored `other`");

    let warm = compiles_now();
    assert_eq!(normalize(&effective, &instance), normalized);
    assert_compiles("R1-F2.11, warm", &warm, 0);
}

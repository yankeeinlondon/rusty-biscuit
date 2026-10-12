//! Typed-tree consumers of a resolved union type (`U[]`, `U(required)`):
//! the `md schema about` catalog and the model it describes, frontmatter
//! shape resolution, launch/completion phase projection, trigger matching,
//! detection, and the `ctx.*` descriptor catalog.
//!
//! Every consumer here is passive. The phase cases also check that
//! validation creates no file and that a phase never probes one.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::compose::context::context_variable_descriptors;
use darkmatter::markdown::schemas::resolve::resolve_yaml_schema;
use darkmatter::markdown::schemas::triggers::{MatchExpr, PathSubject, matches};
use darkmatter::markdown::schemas::{
    Constraint, DarkmatterSchemas, DetectOptions, EffectiveSchema, PropertyAtom, PropertyDef,
    SchemaPhase, SimplifiedSchema, TypeExpr, darkmatter_base_schema,
    detect_schema, effective_property_shape, expression_atom, frontmatter_expression_values,
    nested_property_shape, parse_yaml_schema, property_def_at_path, schema_shape_descriptors,
    schema_to_yaml,
};
use serde_json::{Value, json};
use serde_yaml_ng::Value as YamlValue;

use crate::request_support::context_at;
use crate::union_fixtures::{
    CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, FILE_TYPES, content_policy_union_schema, write_files,
};

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// The resolved shape of an inline `$schema` mapping body read from `dir`.
fn resolved_shape(dir: &Path, schema: &str) -> SimplifiedSchema {
    let value: YamlValue = serde_yaml_ng::from_str(schema).expect("fixture YAML parses");
    resolve_yaml_schema(&value, dir, &context_at(dir))
        .expect("fixture schema resolves")
        .simplified
        .expect("a SimplifiedSchema")
}

fn single_atom<'a>(schema: &'a SimplifiedSchema, name: &str) -> &'a PropertyAtom {
    let SimplifiedSchema::Single(shape) = schema else {
        panic!("expected a single-shape schema");
    };
    match shape.properties.get(name) {
        Some(PropertyDef::Single(atom)) => atom,
        other => panic!("{name}: expected one atom, got {other:?}"),
    }
}

/// The Content Policy acceptance fixture written into `dir`, resolved as
/// `content_policy: policy[](min(1))@./content-policy.yaml`.
fn content_policy_schema(dir: &Path, shipped: &str) -> SimplifiedSchema {
    write_files(dir, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    resolved_shape(dir, &format!("content_policy: \"{CONTENT_POLICY_LIST}\"\n"))
}

/// The arm descriptions of the shipped `short_form` union, in order.
fn short_form_descriptions(shipped: &str) -> Vec<String> {
    shipped
        .lines()
        .filter(|line| line.starts_with("    - 'string("))
        .map(|line| {
            let (_, description) = line.rsplit_once(" -> ").expect("every short_form arm is described");
            description.trim_end_matches('\'').to_string()
        })
        .collect()
}

fn dir_listing(dir: &Path) -> BTreeSet<String> {
    std::fs::read_dir(dir)
        .expect("read fixture dir")
        .map(|entry| entry.expect("dir entry").file_name().to_string_lossy().into_owned())
        .collect()
}

// ── md schema about: the catalog and the model it describes ───────────────

#[test]
fn about_model_exposes_every_content_policy_alternative_and_the_container() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    let schema = content_policy_schema(&root, shipped);
    let atom = single_atom(&schema, "content_policy");

    assert!(atom.is_array, "content_policy is a list");
    assert_eq!(atom.array_constraints, vec![Constraint::MinItems(1)], "container constraints");
    assert!(atom.constraints.is_empty(), "no marker reaches the container");
    let arms = atom.union_arms().expect("the list's items are union alternatives");
    assert_eq!(arms.len(), 9, "short_form's eight arms flatten beside long_form");
    assert!(arms.iter().all(|arm| !arm.is_array), "alternatives describe one item each");

    let described: Vec<Option<&str>> =
        arms[..8].iter().map(|arm| arm.description.as_deref()).collect();
    let expected = short_form_descriptions(shipped);
    assert_eq!(expected.len(), 8, "the shipped short_form still has eight arms");
    assert_eq!(described, expected.iter().map(|text| Some(text.as_str())).collect::<Vec<_>>());

    let TypeExpr::InlineObject(long_form) = &arms[8].ty else {
        panic!("long_form is an object alternative: {:?}", arms[8].ty);
    };
    let Some(PropertyDef::Single(rule)) = long_form.properties.get("rule") else {
        panic!("long_form.rule is one union atom");
    };
    assert_eq!(rule.constraints, vec![Constraint::Required], "rule keeps its use-site marker");
    assert_eq!(rule.union_arms().map(<[PropertyAtom]>::len), Some(8));
}

#[test]
fn about_catalog_describes_named_union_references_without_anonymous_syntax() {
    let shapes = schema_shape_descriptors();
    let array = shapes
        .iter()
        .find(|shape| shape.name == "Array of a named union")
        .expect("the catalog describes an array of a union");
    for phrase in ["alternatives", "own constraints and description", "apply to the list", "rejected"] {
        assert!(array.description.contains(phrase), "{phrase:?} missing: {}", array.description);
    }
    let marked = shapes
        .iter()
        .find(|shape| shape.name == "Marked union reference")
        .expect("the catalog describes a marked union reference");
    assert!(marked.description.contains("alternatives keep their own descriptions"));

    for shape in shapes {
        for text in [shape.form, shape.example, shape.description] {
            assert!(!text.contains(")[]"), "{}: anonymous union array syntax in {text:?}", shape.name);
            assert!(!text.contains(" | "), "{}: anonymous union syntax in {text:?}", shape.name);
        }
    }
}

#[test]
fn about_catalog_union_examples_resolve_to_what_they_describe() {
    let (_dir, root) = temp_root();
    let example = |name: &str| {
        schema_shape_descriptors()
            .iter()
            .find(|shape| shape.name == name)
            .expect("descriptor present")
            .example
    };

    let schema = resolved_shape(&root, example("Array of a named union"));
    let steps = single_atom(&schema, "steps");
    assert!(steps.is_array);
    assert_eq!(steps.array_constraints, vec![Constraint::MinItems(1), Constraint::Unique]);
    assert_eq!(steps.description.as_deref(), Some("The steps to run"));
    let arms = steps.union_arms().expect("item alternatives");
    assert_eq!(arms.len(), 2);
    assert_eq!(arms[0].description.as_deref(), Some("A shell command"), "arm description kept");
    assert!(matches!(arms[1].ty, TypeExpr::InlineObject(_)));

    let schema = resolved_shape(&root, example("Marked union reference"));
    let answer = single_atom(&schema, "answer");
    assert!(!answer.is_array);
    assert_eq!(answer.constraints, vec![Constraint::Required]);
    assert_eq!(answer.description.as_deref(), Some("The chosen value"));
    assert_eq!(answer.union_arms().map(<[PropertyAtom]>::len), Some(2));
}

// ── frontmatter shape resolution ───────────────────────────────────────────

const TAGGED_STEPS: &str = concat!(
    "step:\n",
    "  - \"{ kind: literal(check; required), run: expression }\"\n",
    "  - \"{ kind: literal(note; required), run: string, text: string }\"\n",
    "steps: step[]\n",
    "one: step(required)\n",
    "gate_choice: [expression, number]\n",
    "gate: gate_choice(required)\n",
);

#[test]
fn frontmatter_shape_narrows_a_marked_union_by_its_literal_tag() {
    let (_dir, root) = temp_root();
    let schema = resolved_shape(&root, TAGGED_STEPS);
    let check = json!({ "one": { "kind": "check", "run": "a && b" } });
    let note = json!({ "one": { "kind": "note", "run": "a && b" } });
    let shape = effective_property_shape(&[], Some(&schema), &check);

    let run = property_def_at_path(&shape, &check, &["one", "run"]).expect("check arm's run");
    assert!(expression_atom(&run).is_some(), "the check arm types run as an expression");
    let run = property_def_at_path(&shape, &note, &["one", "run"]).expect("note arm's run");
    assert!(expression_atom(&run).is_none(), "the note arm types run as a string");
    let paths = |frontmatter: &Value| -> Vec<Vec<String>> {
        frontmatter_expression_values(&shape, frontmatter).into_iter().map(|value| value.path).collect()
    };
    assert_eq!(paths(&check), vec![vec!["one".to_string(), "run".to_string()]]);
    assert!(paths(&note).is_empty());

    // An unknown tag selects no arm, so the alternatives merge.
    let unknown = json!({ "one": { "kind": "other" } });
    let merged = nested_property_shape(&shape, &unknown, &["one"]).expect("merged arms");
    assert_eq!(merged.properties.keys().collect::<Vec<_>>(), ["kind", "run", "text"]);
}

#[test]
fn frontmatter_shape_descends_into_the_item_alternatives_of_an_array_of_a_union() {
    let (_dir, root) = temp_root();
    let schema = resolved_shape(&root, TAGGED_STEPS);
    let frontmatter = json!({ "steps": [{ "kind": "check", "run": "a" }, { "kind": "note", "text": "b" }] });
    let shape = effective_property_shape(&[], Some(&schema), &frontmatter);

    // A key path has no sequence index, so a list's items cannot be told
    // apart by tag: every object alternative contributes its keys, and a key
    // two alternatives type differently resolves to both types.
    let items = nested_property_shape(&shape, &frontmatter, &["steps"]).expect("item alternatives");
    assert_eq!(items.properties.keys().collect::<Vec<_>>(), ["kind", "run", "text"]);
    let run = property_def_at_path(&shape, &frontmatter, &["steps", "run"]).expect("run");
    let PropertyDef::Union(run_types) = run.as_ref() else {
        panic!("run is typed by both alternatives: {run:?}");
    };
    assert_eq!(run_types.len(), 2);
    assert!(expression_atom(&run).is_some());
    assert!(frontmatter_expression_values(&shape, &frontmatter).is_empty(), "sequence values are never expressions");
}

#[test]
fn frontmatter_shape_finds_an_expression_alternative_of_a_marked_union() {
    let (_dir, root) = temp_root();
    let schema = resolved_shape(&root, TAGGED_STEPS);
    let frontmatter = json!({ "gate": "ready && !blocked" });
    let shape = effective_property_shape(&[], Some(&schema), &frontmatter);

    let values = frontmatter_expression_values(&shape, &frontmatter);
    assert_eq!(values.len(), 1);
    assert_eq!(values[0].path, ["gate"]);
    assert_eq!(values[0].expression, "ready && !blocked");
}

// ── launch / completion phase projection ───────────────────────────────────

/// The effective schema of a document in `dir` whose `$schema` mapping body
/// is `schema`, loaded from disk.
fn effective_in(dir: &Path, schema: &str) -> EffectiveSchema {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n$schema:\n{schema}---\nBody\n")).expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    DarkmatterSchemas::new(context_at(dir))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("effective schema")
}

fn phase_valid(schema: &EffectiveSchema, instance: Value, phase: SchemaPhase) -> bool {
    schema.validate_for_phase(&instance, phase).expect("phase schema builds").valid
}

#[test]
fn inherited_required_on_a_list_of_a_union_waits_for_completion_unless_the_list_is_eager() {
    let (_dir, root) = temp_root();
    let schema = effective_in(
        &root,
        concat!(
            "  choice: [\"string(required)\", number]\n",
            "  list: choice[]\n",
            "  eager_list: choice[](eager)\n",
            "  marked: choice(eager)\n",
        ),
    );
    let supplied = json!({ "choice": "c", "list": ["a", 1], "eager_list": ["b"], "marked": 2 });

    // The arm's `required` hoists to each property, so all four are required
    // once complete; at launch only the eager ones are.
    assert!(phase_valid(&schema, supplied.clone(), SchemaPhase::Completion));
    assert!(phase_valid(&schema, supplied.clone(), SchemaPhase::Launch));
    let without = |key: &str| {
        let mut instance = supplied.clone();
        instance.as_object_mut().expect("object").remove(key);
        instance
    };
    for key in ["choice", "list", "eager_list", "marked"] {
        assert!(!phase_valid(&schema, without(key), SchemaPhase::Completion), "{key} at completion");
    }
    assert!(phase_valid(&schema, without("list"), SchemaPhase::Launch), "a lazy list may wait");
    assert!(phase_valid(&schema, without("choice"), SchemaPhase::Launch), "a lazy union may wait");
    assert!(!phase_valid(&schema, without("eager_list"), SchemaPhase::Launch), "eager on the list");
    assert!(!phase_valid(&schema, without("marked"), SchemaPhase::Launch), "eager on the union");
    assert!(!phase_valid(&schema, json!({ "eager_list": [true] }), SchemaPhase::Launch), "items still type-check");
}

#[test]
fn an_eager_alternative_times_a_marked_union_but_only_the_items_of_a_list() {
    let (_dir, root) = temp_root();
    let schema = effective_in(
        &root,
        concat!(
            "  choice: [\"string(required; eager)\", number]\n",
            "  described: choice -> Either kind of value\n",
            "  listed: choice[]\n",
        ),
    );
    let supplied = json!({ "choice": "c", "described": 1, "listed": [2] });
    assert!(phase_valid(&schema, supplied.clone(), SchemaPhase::Launch));
    let without = |key: &str| {
        let mut instance = supplied.clone();
        instance.as_object_mut().expect("object").remove(key);
        instance
    };
    // As in a property-level union, an eager arm times the property.
    assert!(!phase_valid(&schema, without("described"), SchemaPhase::Launch));
    // As with `string(eager)[]`, an item's `eager` times the items only.
    assert!(phase_valid(&schema, without("listed"), SchemaPhase::Launch));
    assert!(!phase_valid(&schema, without("listed"), SchemaPhase::Completion));
}

#[test]
fn nested_object_fields_in_union_items_keep_their_own_presence_and_timing() {
    let (_dir, root) = temp_root();
    let schema = effective_in(
        &root,
        concat!(
            "  job: [\"{ source: string(required), input: string(required; eager) }\", number]\n",
            "  jobs: job[](eager)\n",
        ),
    );
    let partial = json!({ "jobs": [{ "input": "in" }] });
    assert!(phase_valid(&schema, partial.clone(), SchemaPhase::Launch), "source is not eager");
    assert!(!phase_valid(&schema, partial, SchemaPhase::Completion), "source is required once complete");
    assert!(
        !phase_valid(&schema, json!({ "jobs": [{ "source": "s" }] }), SchemaPhase::Launch),
        "an eager nested field is required at launch",
    );
    assert!(phase_valid(&schema, json!({ "jobs": [{ "source": "s", "input": "in" }] }), SchemaPhase::Completion));
}

#[test]
fn eager_on_a_list_of_file_alternatives_is_timing_and_lazy_arms_stay_lazy() {
    let (_dir, root) = temp_root();
    write_files(&root, &[("types.yaml", FILE_TYPES)]);
    write_files(&root, &[("present.md", "present\n")]);
    let schema = effective_in(
        &root,
        concat!(
            "  lazy: \"lazy_asset[](required; eager)@./types.yaml\"\n",
            "  checked: \"asset[](required; eager)@./types.yaml\"\n",
            "  later: \"asset[](required)@./types.yaml\"\n",
        ),
    );
    let before = dir_listing(&root);

    let missing = json!({ "lazy": ["missing.md"], "checked": ["present.md", 3], "later": [1] });
    assert!(schema.validate(&missing).valid, "a lazy file arm never checks existence: {:?}", schema.validate(&missing).problems);
    let probed = json!({ "lazy": [1], "checked": ["missing.md"], "later": [1] });
    assert!(!schema.validate(&probed).valid, "the inherited file(eager) arm owns its check");
    assert!(phase_valid(&schema, probed, SchemaPhase::Launch), "phase validation never probes a file");

    assert!(phase_valid(&schema, json!({ "lazy": [1], "checked": [2] }), SchemaPhase::Launch), "later may wait");
    assert!(!phase_valid(&schema, json!({ "lazy": [1], "later": [2] }), SchemaPhase::Launch), "checked is eager");
    assert!(!phase_valid(&schema, json!({ "lazy": [1], "checked": [2] }), SchemaPhase::Completion));
    assert_eq!(dir_listing(&root), before, "validation creates no file");
}

// ── trigger matching ───────────────────────────────────────────────────────

fn property_condition(name: &str, atom: &PropertyAtom) -> MatchExpr {
    MatchExpr::Property { name: name.to_string(), atom: atom.clone() }
}

#[test]
fn a_list_of_a_union_matches_when_every_item_matches_some_alternative() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_dir, root) = temp_root();
    let schema = content_policy_schema(&root, shipped);
    let condition = property_condition("content_policy", single_atom(&schema, "content_policy"));
    let subject = PathSubject::detached();
    let matched = |value: Value| matches(&condition, &json!({ "content_policy": value }), &subject);

    assert!(matched(json!(["ValidFor(3mo)", { "rule": "ValidUntil(2027-01-01)", "action": "archive" }])));
    assert!(matched(json!(["Evergreen", "TimeSensitive"])), "items may choose different alternatives");
    assert!(!matched(json!([null])), "a null item matches no alternative");
    assert!(!matched(json!("ValidFor(3mo)")), "a scalar is not a list");
    assert!(!matched(json!([])), "min(1) on the list");
    assert!(!matched(json!(["Duration(3mo)"])), "no alternative matches");
    assert!(!matched(json!(["Evergreen", "Duration(3mo)"])), "every item must match");
    assert!(!matched(json!([{ "action": "archive" }])), "the object alternative requires rule");
    assert!(!matched(json!([{ "rule": "ValidFor(3mo)", "action": "delete" }])), "action members");
    assert!(matches(&condition, &json!({}), &subject), "an optional list is a guard");
}

#[test]
fn an_inherited_required_alternative_makes_the_condition_a_gate() {
    let (_dir, root) = temp_root();
    let schema = resolved_shape(&root, "choice: [\"string(required)\", number]\nlist: choice[]\n");
    let condition = property_condition("list", single_atom(&schema, "list"));
    let subject = PathSubject::detached();

    assert!(!matches(&condition, &json!({}), &subject), "absent list defeats the gate");
    assert!(!matches(&condition, &json!({ "list": null }), &subject));
    assert!(matches(&condition, &json!({ "list": ["a", 2] }), &subject));
}

// ── detection and the ctx.* descriptor catalog ─────────────────────────────

#[test]
fn detection_never_produces_a_union_type_and_its_yaml_parses_back() {
    let mixed: Markdown = "---\nitems: [1, a, {x: 1}]\nvalue: 1\n---\n".into();
    let other: Markdown = "---\nitems: [b]\nvalue: text\n---\n".into();
    let (_dir, root) = temp_root();
    let detected = detect_schema(&[&mixed, &other], DetectOptions { merge: true }, &context_at(&root));
    let SimplifiedSchema::Single(shape) = &detected else {
        panic!("detection yields one shape");
    };
    let atoms: Vec<&PropertyAtom> = shape
        .properties
        .values()
        .flat_map(|def| match def {
            PropertyDef::Single(atom) => std::slice::from_ref(atom),
            PropertyDef::Union(atoms) => atoms.as_slice(),
        })
        .collect();
    assert!(atoms.len() >= 3, "value widens to a property union: {atoms:?}");
    assert!(atoms.iter().all(|atom| atom.union_arms().is_none()), "{atoms:?}");

    let yaml: YamlValue = serde_yaml_ng::from_str(&schema_to_yaml(&detected)).expect("YAML");
    assert_eq!(parse_yaml_schema(&yaml["$schema"]).expect("parses back"), detected);
}

#[test]
fn the_ctx_catalog_projects_no_union_type() {
    let SimplifiedSchema::Single(root) = darkmatter_base_schema() else {
        panic!("the base schema is one shape");
    };
    let Some(PropertyDef::Single(ctx)) = root.properties.get("ctx") else {
        panic!("ctx is one atom");
    };
    let TypeExpr::InlineObject(ctx) = &ctx.ty else {
        panic!("ctx is an object");
    };
    let descriptors = context_variable_descriptors();
    assert_eq!(descriptors.len(), ctx.properties.len());
    for (descriptor, (name, def)) in descriptors.iter().zip(&ctx.properties) {
        assert_eq!(descriptor.name, name.as_str());
        assert!(def.alternatives().is_none(), "ctx.{name} is not a union");
        let PropertyDef::Single(atom) = def else {
            panic!("ctx.{name} is one atom");
        };
        assert!(atom.union_arms().is_none(), "ctx.{name} is not a union type");
        let TypeExpr::Primitive(primitive) = atom.ty else {
            panic!("ctx.{name} is a primitive");
        };
        assert_eq!(descriptor.display_type.base, primitive, "ctx.{name} is not an object fallback");
    }
}

//! Completion shapes below a recursive reference.
//!
//! An editor asks [`for_path`] for the keys and values at the cursor; below a
//! recursive named type each level is the referenced definition, opened one
//! reference at a time along the cursor path. Every test would offer nothing
//! (or fail to resolve the path) if completion did not follow `Ref`.

use std::cell::RefCell;

use biscuit_file::FileResolutionContext;
use darkmatter::markdown::Markdown;
use darkmatter::markdown::schemas::completion::{
    AlternativeCompletion, AlternativeSite, DefinitionLookup, PathStep, alternative_accepts,
    alternative_accepts_in, def_at_path, for_list_item, for_path, for_path_in, for_property_value,
};
use darkmatter::markdown::schemas::simplified::query::resolved_alternatives;
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, DefKey, Definition, DefinitionTable, EffectiveSchema, PropertyDef, SimplifiedSchema,
    TypeExpr,
};
use serde_json::{Value, json};
use tempfile::TempDir;

/// `item` recurses through `children` and, with `meta`, through `meta.owner`;
/// `tag` recurses on its own. A cursor path through `children` reaches only
/// `item`, so `meta` and `tag` must never be opened.
const TREE_SCHEMA: &str = concat!(
    "$schema:\n",
    "  item: \"{ children: item[](required), meta: meta, ",
    "value: string(required; suggest(alpha, beta)) -> The node text }\"\n",
    "  meta: \"{ owner: item, tags: tag[] }\"\n",
    "  tag: \"{ label: string, sub: tag[] }\"\n",
    "  tree: item(required)\n",
    "  forest: \"item[]\"\n",
);

/// A recursive union discriminated by `kind`.
const NODE_SCHEMA: &str = concat!(
    "$schema:\n",
    "  node:\n",
    "    - \"{ kind: literal(leaf; required), text: string }\"\n",
    "    - \"{ kind: literal(branch; required), kids: node[](required) }\"\n",
    "  root: node(required)\n",
);

fn effective(dir: &TempDir, schema: &str) -> EffectiveSchema {
    let md: Markdown = format!("---\n{schema}---\nbody\n").into();
    DarkmatterSchemas::new(FileResolutionContext::new(dir.path()))
        .effective_for(&md)
        .expect("the schema resolves")
        .expect("the document declares a schema")
}

fn simplified(effective: &EffectiveSchema) -> &SimplifiedSchema {
    effective.simplified.as_ref().expect("a SimplifiedSchema projection")
}

/// `tree`, then `children[0]` repeated `depth` times.
fn tree_path(depth: usize) -> Vec<PathStep<'static>> {
    let mut path = vec![PathStep::Key("tree")];
    for _ in 0..depth {
        path.extend([PathStep::Key("children"), PathStep::Index(0)]);
    }
    path
}

fn keys(completion: &AlternativeCompletion) -> Vec<&str> {
    completion.keys.iter().map(|key| key.name.as_str()).collect()
}

fn labels(completion: &AlternativeCompletion) -> Vec<&str> {
    completion.values.iter().map(|value| value.label.as_str()).collect()
}

/// Records each definition lookup by name before delegating.
struct Recording<'a> {
    inner: &'a EffectiveSchema,
    looked_up: RefCell<Vec<String>>,
}

impl DefinitionLookup for Recording<'_> {
    fn lookup_definition(&self, key: &DefKey) -> Option<&Definition> {
        self.looked_up.borrow_mut().push(key.name.clone());
        self.inner.lookup_definition(key)
    }
}

/// Acceptance 3: each recursion level offers the referenced definition's
/// keys with their required markers and descriptions, from the top-level
/// property down to level 3, and the top-level entry points open the
/// reference too.
#[test]
fn item_keys_are_offered_at_recursion_levels_zero_through_three() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, TREE_SCHEMA);
    let frontmatter = json!({
        "tree": { "value": "a", "children": [
            { "value": "b", "children": [
                { "value": "c", "children": [ { "value": "d", "children": [] } ] }
            ] }
        ] }
    });

    for depth in 0..=3 {
        let completion = for_path(&effective, &frontmatter, &tree_path(depth))
            .unwrap_or_else(|| panic!("level {depth} resolves"));
        assert_eq!(completion.is_array, depth > 0, "level {depth}");
        assert_eq!(keys(&completion), ["children", "meta", "value"], "level {depth}");
        let required: Vec<bool> = completion.keys.iter().map(|key| key.required).collect();
        assert_eq!(required, [true, false, true], "level {depth}");
        assert_eq!(completion.keys[2].description.as_deref(), Some("The node text"), "level {depth}");
    }

    let value = for_property_value(&effective, "tree", &Value::Null).expect("tree's value");
    assert_eq!(keys(&value), ["children", "meta", "value"]);
    let element = for_list_item(&effective, "forest", &Value::Null).expect("a forest element");
    assert_eq!(keys(&element), ["children", "meta", "value"]);
}

/// A scalar below recursion keeps its `suggest(...)` candidates and its
/// description, at levels 1 through 3.
#[test]
fn a_leaf_below_recursion_keeps_its_suggestions_and_description() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, TREE_SCHEMA);

    for depth in 1..=3 {
        let mut path = tree_path(depth);
        path.push(PathStep::Key("value"));
        let completion = for_path(&effective, &Value::Null, &path).unwrap_or_else(|| panic!("level {depth}"));
        assert_eq!(labels(&completion), ["alpha", "beta"], "level {depth}");
        assert_eq!(completion.values[0].insert_text, "\"alpha\"");

        let (def, site) = def_at_path(simplified(&effective), &effective, &Value::Null, &path).expect("value");
        assert_eq!(site, AlternativeSite::Value);
        let PropertyDef::Single(atom) = def.as_ref() else { panic!("a single atom") };
        assert_eq!(atom.description.as_deref(), Some("The node text"));
    }
}

/// Spec design 8: an element that is still empty, missing, or missing
/// required keys offers the keys for the next insertion.
#[test]
fn incomplete_items_and_objects_offer_the_next_keys() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, TREE_SCHEMA);
    let frontmatter = json!({
        "tree": { "value": "a", "children": [
            { "value": "b", "children": [ null, { "value": "c" } ] }
        ] }
    });
    let level_two = |index: usize| {
        let mut path = tree_path(1);
        path.extend([PathStep::Key("children"), PathStep::Index(index)]);
        path
    };

    for (index, case) in [(0, "an empty element"), (1, "an object missing `children`"), (5, "an element not authored yet")]
    {
        let completion = for_path(&effective, &frontmatter, &level_two(index)).unwrap_or_else(|| panic!("{case}"));
        assert_eq!(keys(&completion), ["children", "meta", "value"], "{case}");
        assert!(completion.keys[0].required, "{case}: `children` is required");
    }

    // An incomplete object's own children list is reachable below it.
    let mut path = level_two(1);
    path.extend([PathStep::Key("children"), PathStep::Index(0)]);
    let completion = for_path(&effective, &frontmatter, &path).expect("level 3 below an incomplete object");
    assert_eq!(keys(&completion), ["children", "meta", "value"]);
}

/// Acceptance 3 laziness: a level-`n` lookup opens `item` once per reference
/// it stands on (`n + 1` times) and never `meta` or `tag`, which the table
/// holds but the cursor path does not reach.
#[test]
fn descent_opens_only_the_definitions_on_the_cursor_path() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, TREE_SCHEMA);
    let mut in_table: Vec<&str> = effective.definitions().iter().map(|(key, _)| key.name.as_str()).collect();
    in_table.sort_unstable();
    assert_eq!(in_table, ["item", "meta", "tag"], "the unused definitions are in scope");

    for depth in 0..=3 {
        let recording = Recording { inner: &effective, looked_up: RefCell::new(Vec::new()) };
        let completion = for_path_in(simplified(&effective), &recording, &Value::Null, &tree_path(depth))
            .unwrap_or_else(|| panic!("level {depth}"));
        assert_eq!(keys(&completion), ["children", "meta", "value"], "level {depth}");
        assert_eq!(recording.looked_up.into_inner(), vec!["item"; depth + 1], "level {depth}");
    }
}

/// A reference whose definition is out of scope offers nothing: no keys, no
/// values, no deeper path, and no accepted value.
#[test]
fn a_missing_definition_offers_nothing() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, TREE_SCHEMA);
    let empty = DefinitionTable::default();
    let schema = simplified(&effective);

    let top = for_path_in(schema, &empty, &Value::Null, &tree_path(0));
    assert!(
        top.as_ref().is_none_or(|completion| completion.keys.is_empty() && completion.values.is_empty()),
        "{top:?}"
    );
    for depth in 1..=3 {
        assert_eq!(for_path_in(schema, &empty, &Value::Null, &tree_path(depth)), None, "level {depth}");
    }

    let SimplifiedSchema::Single(shape) = schema else { panic!("a single shape") };
    let PropertyDef::Single(tree) = &shape.properties["tree"] else { panic!("a single atom") };
    assert!(matches!(tree.ty, TypeExpr::Ref(_)), "{tree:?}");
    let node = json!({ "value": "a", "children": [] });
    assert!(alternative_accepts_in(tree, &node, &effective), "the definition in scope accepts it");
    assert!(!alternative_accepts_in(tree, &node, &empty));
    assert!(!alternative_accepts(tree, &node));
}

/// A discriminated recursive union narrows to the tagged arm at every level,
/// both for the completed value and for each ancestor the path crosses, and
/// an alternative containing the reference is judged against the definition.
#[test]
fn a_recursive_discriminated_union_narrows_at_every_level() {
    let dir = TempDir::new().expect("tempdir");
    let effective = effective(&dir, NODE_SCHEMA);
    let frontmatter = json!({
        "root": { "kind": "branch", "kids": [
            { "kind": "branch", "kids": [ { "kind": "leaf" }, null ] }
        ] }
    });
    let at = |path: &[PathStep<'_>]| for_path(&effective, &frontmatter, path);
    let root = PathStep::Key("root");
    let kids = PathStep::Key("kids");

    let top = at(&[root]).expect("root");
    assert_eq!((top.selected_arm, keys(&top)), (Some(1), vec!["kind", "kids"]));
    let level_one = at(&[root, kids, PathStep::Index(0)]).expect("level 1");
    assert_eq!((level_one.selected_arm, keys(&level_one)), (Some(1), vec!["kind", "kids"]));
    let leaf = at(&[root, kids, PathStep::Index(0), kids, PathStep::Index(0)]).expect("level 2 leaf");
    assert_eq!((leaf.selected_arm, keys(&leaf)), (Some(0), vec!["kind", "text"]));
    let empty = at(&[root, kids, PathStep::Index(0), kids, PathStep::Index(1)]).expect("level 2 empty");
    assert_eq!((empty.selected_arm, keys(&empty)), (None, vec!["kind", "text", "kids"]));
    assert_eq!(labels(&empty), Vec::<&str>::new());

    // The leaf ancestor narrows away `kids`; the branch ancestor keeps it.
    let below_leaf = [root, kids, PathStep::Index(0), kids, PathStep::Index(0), kids, PathStep::Index(0)];
    assert_eq!(at(&below_leaf), None);
    let below_empty = [root, kids, PathStep::Index(0), kids, PathStep::Index(1), kids, PathStep::Index(0)];
    assert_eq!(keys(&at(&below_empty).expect("an unnarrowed element")), ["kind", "text", "kids"]);

    let SimplifiedSchema::Single(shape) = simplified(&effective) else { panic!("a single shape") };
    let alternatives = resolved_alternatives(&shape.properties["root"], &effective);
    assert_eq!(alternatives.len(), 2, "the reference opens to both arms");
    let branch = alternatives[1].atom;
    let good = json!({ "kind": "branch", "kids": [ { "kind": "branch", "kids": [ { "kind": "leaf" } ] } ] });
    let bad = json!({ "kind": "branch", "kids": [ { "kind": "branch", "kids": [ { "kind": "bogus" } ] } ] });
    assert!(alternative_accepts_in(branch, &good, &effective));
    assert!(!alternative_accepts_in(branch, &bad, &effective), "the depth-3 element has no arm");
    assert!(!alternative_accepts(branch, &good), "without the table the reference is unresolved");
}

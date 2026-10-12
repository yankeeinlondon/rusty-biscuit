//! Completion inside a union value: an element of `U[]` offers the keys of its
//! object alternatives and the values of its scalar alternatives, combined,
//! unless an unambiguous literal tag selects one alternative. The same union
//! used as a property completes the same way.
//!
//! Every test loads a hermetic document from a temp dir and goes through the
//! public `schemas::completion` and suggestion APIs.

use std::path::{Path, PathBuf};

use darkmatter::markdown::Markdown;
use darkmatter::markdown::compose::ComposeSource;
use darkmatter::markdown::schemas::completion::{
    AlternativeCompletion, CompletionKind, alternative_accepts, for_list_item, for_property,
    for_property_value,
};
use darkmatter::markdown::schemas::{
    DarkmatterSchemas, EffectiveSchema, PropertyDef, SimplifiedSchema, flattened_alternatives,
    suggestions_for_def, suggestions_for_path,
};
use serde_json::{Value, json};

use crate::request_support::context_at;
use crate::union_fixtures::{
    CONTENT_POLICY_FILE, CONTENT_POLICY_LIST, FILE_TYPES, OVERLAPPING_TYPES, TAGGED_TYPES,
    content_policy_union_schema, write_files,
};

/// Object, enum, suggest, and literal alternatives in one union, plus a union
/// whose two object arms share the same tag value.
const MIXED_TYPES: &str = concat!(
    "kind: schema\n",
    "types:\n",
    "  mode:\n",
    "    - \"{ kind: literal(shell; required), run: string(required) -> Command line }\"\n",
    "    - \"{ kind: literal(say; required), text: string(required) }\"\n",
    "    - enum(fast, slow)\n",
    "    - string(suggest(alpha, \"two words\"))\n",
    "    - literal(7)\n",
    "  dup:\n",
    "    - \"{ kind: literal(shell; required), run: string(required) }\"\n",
    "    - \"{ kind: literal(shell; required), cmd: string }\"\n",
    "  choice: [\"enum(a, b)\", url]\n",
);

fn temp_root() -> (tempfile::TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("temp dir");
    let root = biscuit_file::canonicalize_simplified(dir.path()).expect("canonical temp dir");
    (dir, root)
}

/// Writes `doc.md` with the `$schema` mapping body `schema` and `frontmatter`,
/// loads it from disk, and returns its effective schema with the document.
fn load(dir: &Path, schema: &str, frontmatter: &str) -> (EffectiveSchema, Markdown) {
    let path = dir.join("doc.md");
    std::fs::write(&path, format!("---\n$schema:\n{schema}{frontmatter}---\nBody\n"))
        .expect("write document");
    let markdown = Markdown::try_from(path.as_path())
        .expect("document loads")
        .with_source(ComposeSource::File(path.clone()));
    let effective = DarkmatterSchemas::new(context_at(dir))
        .effective_for(&markdown)
        .expect("schema resolves")
        .expect("document declares a schema");
    (effective, markdown)
}

/// The authored elements of the array property `name`.
fn items(markdown: &Markdown, name: &str) -> Vec<Value> {
    match markdown.frontmatter().get::<Value>(name).expect("frontmatter reads") {
        Some(Value::Array(items)) => items,
        other => panic!("`{name}` is not a list: {other:?}"),
    }
}

fn keys(completion: &AlternativeCompletion) -> Vec<&str> {
    completion.keys.iter().map(|key| key.name.as_str()).collect()
}

fn values(completion: &AlternativeCompletion) -> Vec<&str> {
    completion.values.iter().map(|value| value.label.as_str()).collect()
}

fn item(effective: &EffectiveSchema, property: &str, value: &Value) -> AlternativeCompletion {
    for_list_item(effective, property, value)
        .unwrap_or_else(|| panic!("`{property}` has item alternatives"))
}

fn property_def<'a>(effective: &'a EffectiveSchema, name: &str) -> &'a PropertyDef {
    let Some(SimplifiedSchema::Single(shape)) = &effective.simplified else {
        panic!("expected a single-shape schema");
    };
    shape.properties.get(name).expect("property is declared")
}

/// Acceptance criterion 5 on the Content Policy list: an empty element offers
/// `rule`/`action` from the long form and the short-form suggestion.
#[test]
fn content_policy_item_offers_object_keys_and_scalar_values() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_guard, dir) = temp_root();
    write_files(&dir, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    let (effective, _) = load(
        &dir,
        &format!("  content_policy: \"{CONTENT_POLICY_LIST}\"\n"),
        "content_policy: [null]\n",
    );

    let completion = item(&effective, "content_policy", &Value::Null);

    assert!(completion.is_array);
    assert_eq!(completion.selected_arm, None);
    assert_eq!(keys(&completion), ["rule", "action"]);
    assert!(completion.keys.iter().all(|key| key.required), "{:?}", completion.keys);
    // Only `Evergreen` survives: `suggest(...)` is linted against its own arm,
    // whose pattern is `^Evergreen$`, not against the whole union.
    assert_eq!(values(&completion), ["Evergreen"]);
    let evergreen = completion
        .values
        .iter()
        .find(|value| value.label == "Evergreen")
        .expect("Evergreen offered");
    assert_eq!(evergreen.insert_text, "\"Evergreen\"");
    assert_eq!(evergreen.arm, 0);
}

/// An invalid scalar element keeps only the scalar alternatives, a partial
/// object element only the object ones, and a missing or null element both.
#[test]
fn missing_invalid_and_partial_items_offer_their_applicable_alternatives() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_guard, dir) = temp_root();
    write_files(&dir, &[(CONTENT_POLICY_FILE, &content_policy_union_schema(shipped))]);
    let (effective, markdown) = load(
        &dir,
        &format!("  content_policy: \"{CONTENT_POLICY_LIST}\"\n"),
        "content_policy: [\"Duration(3mo)\", {action: archive}, null]\n",
    );
    let authored = items(&markdown, "content_policy");

    let invalid = item(&effective, "content_policy", &authored[0]);
    let partial = item(&effective, "content_policy", &authored[1]);
    let null = item(&effective, "content_policy", &authored[2]);
    let missing = item(&effective, "content_policy", &Value::Null);

    assert!(invalid.keys.is_empty(), "{invalid:?}");
    assert!(values(&invalid).contains(&"Evergreen"), "{invalid:?}");
    assert_eq!(keys(&partial), ["rule", "action"]);
    assert!(partial.values.is_empty(), "{partial:?}");
    assert_eq!(keys(&null), ["rule", "action"]);
    assert_eq!(values(&null), values(&invalid));
    assert_eq!(missing, null);
    assert!(
        for_list_item(&effective, "content_policy", &json!([["nested"]])).is_some_and(|completion| {
            completion.keys.is_empty() && completion.values.is_empty()
        }),
        "an array element of a list of non-array alternatives offers nothing",
    );
}

/// Neighboring elements each select their own arm from their own tag.
#[test]
fn a_literal_tag_narrows_each_item_independently() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", TAGGED_TYPES)]);
    let (effective, markdown) = load(
        &dir,
        "  steps: \"step[]@./types.yaml\"\n",
        "steps: [{kind: shell}, {kind: say}, {}]\n",
    );
    let authored = items(&markdown, "steps");

    let shell = item(&effective, "steps", &authored[0]);
    let say = item(&effective, "steps", &authored[1]);
    let untagged = item(&effective, "steps", &authored[2]);

    assert_eq!(shell.selected_arm, Some(0));
    assert_eq!(keys(&shell), ["kind", "run"]);
    assert_eq!(say.selected_arm, Some(1));
    assert_eq!(keys(&say), ["kind", "text"]);
    assert_eq!(untagged.selected_arm, None);
    assert_eq!(keys(&untagged), ["kind", "run", "text"]);
}

/// A tag naming no arm, or a tag value two arms share, never narrows.
#[test]
fn unknown_and_duplicate_tags_keep_the_combined_choices() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", MIXED_TYPES)]);
    let (effective, markdown) = load(
        &dir,
        "  modes: \"mode[]@./types.yaml\"\n  dups: \"dup[]@./types.yaml\"\n",
        "modes: [{kind: other}, {kind: 7}]\ndups: [{kind: shell}]\n",
    );
    let modes = items(&markdown, "modes");
    let dups = items(&markdown, "dups");

    let unknown = item(&effective, "modes", &modes[0]);
    let wrong_type = item(&effective, "modes", &modes[1]);
    let duplicate = item(&effective, "dups", &dups[0]);

    for combined in [&unknown, &wrong_type] {
        assert_eq!(combined.selected_arm, None);
        assert_eq!(keys(combined), ["kind", "run", "text"]);
        assert!(combined.values.is_empty(), "an object element offers no scalars");
    }
    assert_eq!(duplicate.selected_arm, None);
    assert_eq!(keys(&duplicate), ["kind", "run", "cmd"]);
}

/// Overlapping object arms both contribute; a shared key is required only
/// when every contributing arm requires it.
#[test]
fn overlapping_arms_combine_without_choosing() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", OVERLAPPING_TYPES)]);
    let (effective, markdown) =
        load(&dir, "  entries: \"entry[]@./types.yaml\"\n", "entries: [{name: x}]\n");

    let completion = item(&effective, "entries", &items(&markdown, "entries")[0]);

    assert_eq!(completion.selected_arm, None);
    let name = &completion.keys[0];
    assert_eq!((name.name.as_str(), name.required, name.arms.clone()), ("name", true, vec![0, 1]));
    let size = &completion.keys[1];
    assert_eq!((size.name.as_str(), size.required, size.arms.clone()), ("size", false, vec![1]));
    assert_eq!(completion.keys.len(), 2);
}

/// Scalar alternatives offer `enum` members, `suggest(...)` candidates, and
/// `literal(...)` values, YAML-quoted where they are strings; object keys
/// carry their descriptions.
#[test]
fn scalar_arms_offer_enum_suggest_and_literal_values() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", MIXED_TYPES)]);
    let (effective, _) = load(&dir, "  modes: \"mode[]@./types.yaml\"\n", "");

    let typed = item(&effective, "modes", &json!("a"));
    let empty = item(&effective, "modes", &Value::Null);

    assert!(typed.keys.is_empty());
    assert_eq!(values(&typed), ["fast", "slow", "alpha", "two words", "7"]);
    let inserts: Vec<&str> = typed.values.iter().map(|value| value.insert_text.as_str()).collect();
    assert_eq!(inserts, ["\"fast\"", "\"slow\"", "\"alpha\"", "\"two words\"", "7"]);
    let arms: Vec<usize> = typed.values.iter().map(|value| value.arm).collect();
    assert_eq!(arms, [2, 2, 3, 3, 4]);
    assert_eq!(values(&empty), values(&typed));
    assert_eq!(empty.keys[1].description.as_deref(), Some("Command line"));
}

/// `is_array` follows the site: an element of `U[]` is an array item for the
/// item query, the suggestion queries, and shell completion alike.
#[test]
fn is_array_is_reported_for_items_suggestions_and_shell_completion() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", MIXED_TYPES)]);
    let (effective, _) = load(
        &dir,
        concat!(
            "  modes: \"mode[]@./types.yaml\"\n",
            "  mode: \"mode(required)@./types.yaml\"\n",
            "  choices: \"choice[]@./types.yaml\"\n",
            "  choice: \"choice(required)@./types.yaml\"\n",
        ),
        "mode: fast\nchoice: a\n",
    );

    assert!(item(&effective, "modes", &Value::Null).is_array);
    assert!(for_list_item(&effective, "mode", &Value::Null).is_none(), "not an array");
    let value = for_property_value(&effective, "mode", &Value::Null).expect("value alternatives");
    assert!(!value.is_array);
    assert!(for_property_value(&effective, "modes", &Value::Null).is_none(), "an array");

    let def_query = suggestions_for_def("modes", property_def(&effective, "modes")).expect("suggest");
    assert!(def_query.is_array);
    let path_query = suggestions_for_path(effective.simplified.as_ref().unwrap(), &["modes"])
        .expect("suggestions through the item union");
    assert!(path_query.is_array);
    let labels: Vec<&str> = path_query.items.iter().map(|item| item.label.as_str()).collect();
    assert_eq!(labels, ["alpha", "two words"]);
    assert!(!suggestions_for_def("mode", property_def(&effective, "mode")).unwrap().is_array);

    let choices = for_property(&effective, "choices").expect("shell completion for choice[]");
    assert!(choices.is_array);
    assert_eq!(choices.kind, CompletionKind::Enum { members: vec!["a".into(), "b".into()] });
    let choice = for_property(&effective, "choice").expect("shell completion for choice(required)");
    assert!(!choice.is_array);
    assert_eq!(choice.kind, CompletionKind::Enum { members: vec!["a".into(), "b".into()] });
}

/// The same union completes identically as a property value (bare and with a
/// postfix) and as an array element, apart from `is_array`.
#[test]
fn a_union_property_completes_like_a_union_item() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", MIXED_TYPES)]);
    let (effective, _) = load(
        &dir,
        concat!(
            "  as_item: \"mode[]@./types.yaml\"\n",
            "  as_value: \"mode@./types.yaml\"\n",
            "  as_required: \"mode(required)@./types.yaml\"\n",
        ),
        "as_required: fast\n",
    );
    assert!(
        matches!(property_def(&effective, "as_value"), PropertyDef::Union(_)),
        "a bare reference expands to a property-level union",
    );

    for authored in [Value::Null, json!({"kind": "say"}), json!({"kind": "nope"}), json!("f"), json!([])] {
        let mut as_item = item(&effective, "as_item", &authored);
        as_item.is_array = false;
        for property in ["as_value", "as_required"] {
            let as_value = for_property_value(&effective, property, &authored).expect("value alternatives");
            assert_eq!(as_value, as_item, "{property} for {authored}");
        }
    }
}

/// Completion is passive: an eager-file arm is neither read, created, nor
/// normalized, and the fixture tree is unchanged.
#[test]
fn completion_reads_and_writes_no_files() {
    let (_guard, dir) = temp_root();
    write_files(&dir, &[("types.yaml", FILE_TYPES)]);
    let (effective, markdown) = load(
        &dir,
        "  assets: \"asset[]@./types.yaml\"\n",
        "assets: [missing.png, 3]\n",
    );
    let snapshot = |dir: &Path| {
        let mut entries: Vec<(String, String)> = std::fs::read_dir(dir)
            .expect("list fixture")
            .map(|entry| {
                let path = entry.expect("entry").path();
                let name = path.file_name().unwrap().to_string_lossy().into_owned();
                (name, std::fs::read_to_string(&path).expect("read fixture file"))
            })
            .collect();
        entries.sort();
        entries
    };
    let before = snapshot(&dir);

    for authored in items(&markdown, "assets") {
        let completion = item(&effective, "assets", &authored);
        assert!(completion.keys.is_empty() && completion.values.is_empty(), "{completion:?}");
    }

    assert_eq!(snapshot(&dir), before);
    assert!(!dir.join("missing.png").exists());
}

/// The library names the alternatives an item matches, so an editor can show
/// one only when it is unique: each item is judged as a present value (`null`
/// matches no alternative), and an overlapping value matches several.
#[test]
fn alternative_accepts_judges_each_item_against_every_flattened_alternative() {
    let shipped = include_str!("../../../../content-policy/schemas/content-policy.yaml");
    let (_guard, dir) = temp_root();
    write_files(
        &dir,
        &[
            (CONTENT_POLICY_FILE, &content_policy_union_schema(shipped)),
            ("types.yaml", OVERLAPPING_TYPES),
        ],
    );
    let (effective, markdown) = load(
        &dir,
        &format!(
            "  content_policy: \"{CONTENT_POLICY_LIST}\"\n  entries: \"entry[]@./types.yaml\"\n"
        ),
        concat!(
            "entries: [{ name: a }]\n",
            "content_policy:\n",
            "  - Evergreen\n",
            "  - { rule: Evergreen, action: archive }\n",
            "  - Duration(3mo)\n",
            "  - null\n",
        ),
    );
    let def = property_def(&effective, "content_policy");
    let alternatives = flattened_alternatives(def);
    assert_eq!(alternatives.len(), 9, "Content Policy flattens to nine alternatives");
    assert!(alternatives.iter().all(|alternative| alternative.is_array));

    let accepting = |value: &Value| -> Vec<usize> {
        alternatives
            .iter()
            .enumerate()
            .filter(|(_, alternative)| alternative_accepts(alternative.atom, value))
            .map(|(index, _)| index)
            .collect()
    };
    let list = items(&markdown, "content_policy");
    assert_eq!(accepting(&list[0]), [0], "the Evergreen pattern arm alone");
    assert_eq!(accepting(&list[1]), [8], "the long-form object arm alone");
    assert!(accepting(&list[2]).is_empty(), "no arm accepts `Duration(3mo)`");
    assert!(accepting(&list[3]).is_empty(), "items get no null wrapper");

    let entries = flattened_alternatives(property_def(&effective, "entries"));
    let overlapping: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, alternative)| alternative_accepts(alternative.atom, &json!({ "name": "a" })))
        .map(|(index, _)| index)
        .collect();
    assert_eq!(overlapping, [0, 1], "an overlapping item matches both arms");
}

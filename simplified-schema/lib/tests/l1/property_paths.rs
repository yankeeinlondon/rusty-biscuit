//! `PropertyPath` through its public API: the grammar and its round trip, the
//! walk that gives segments their meaning, and the pure edits.

use std::collections::HashSet;

use indexmap::IndexMap;
use proptest::prelude::*;
use serde_json::{Value, json};
use simplified_schema::coercion::ValueKind;
use simplified_schema::completion::PathStep;
use simplified_schema::{
    PathEditError, PropertyPath, PropertyPathError, RemovalPlan, ResolveStop, remove_at, set_at,
};

fn path(text: &str) -> PropertyPath {
    PropertyPath::parse(text).unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn top_level(value: Value) -> IndexMap<String, Value> {
    let Value::Object(map) = value else {
        panic!("a fixture root is an object");
    };
    map.into_iter().collect()
}

fn as_value(root: &IndexMap<String, Value>) -> Value {
    Value::Object(root.clone().into_iter().collect())
}

fn keys(value: &Value) -> Vec<&str> {
    value.as_object().expect("an object").keys().map(String::as_str).collect()
}

// ── grammar ─────────────────────────────────────────────────────────────────

#[test]
fn dots_separate_and_the_two_escapes_decode() {
    for (written, expected) in [
        ("style.page.max-width", vec!["style", "page", "max-width"]),
        ("title", vec!["title"]),
        ("_private.key", vec!["_private", "key"]),
        (r"style\.page\.max-width", vec!["style.page.max-width"]),
        (r"C:\\.name", vec![r"C:\", "name"]),
        (r"a\\b", vec![r"a\b"]),
    ] {
        assert_eq!(path(written).segments(), expected, "{written}");
    }
}

#[test]
fn escape_pairs_match_left_to_right() {
    // `\\` then `.`: a literal backslash, then a separator.
    assert_eq!(path(r"a\\.b").segments(), [r"a\", "b"]);
    // `\\` then `\.`: a literal backslash and a literal dot.
    assert_eq!(path(r"a\\\.b").segments(), [r"a\.b"]);
}

#[test]
fn a_backslash_before_anything_else_is_itself() {
    assert_eq!(path(r"C:\dir").segments(), [r"C:\dir"]);
    assert_eq!(path(r"a\").segments(), [r"a\"]);
    assert_eq!(path(r"\").segments(), [r"\"]);
    assert_eq!(path(r"a\b.c").segments(), [r"a\b", "c"]);
}

#[test]
fn the_whole_empty_argument_is_one_empty_key() {
    assert_eq!(path("").segments(), [""]);
}

#[test]
fn an_empty_segment_in_a_longer_path_is_a_syntax_error() {
    for (written, position) in [("a.", 1), (".a", 0), ("a..b", 1), (".", 0), ("a.b.", 2)] {
        assert_eq!(
            PropertyPath::parse(written),
            Err(PropertyPathError::EmptySegment {
                argument: written.to_string(),
                position,
            }),
            "{written}"
        );
    }
}

#[test]
fn an_escaped_dot_does_not_empty_a_segment() {
    assert_eq!(path(r"\.").segments(), ["."]);
    assert_eq!(path(r"a.\.").segments(), ["a", "."]);
}

#[test]
fn unicode_and_whitespace_are_kept_unnormalized() {
    assert_eq!(path(" a . b ").segments(), [" a ", " b "]);
    assert_eq!(path("café.日本語").segments(), ["café", "日本語"]);
    // Precomposed and decomposed spellings stay different keys.
    assert_ne!(path("caf\u{e9}"), path("cafe\u{301}"));
}

#[test]
fn a_special_first_character_is_an_ordinary_key() {
    assert_eq!(path("$schema").segments(), ["$schema"]);
}

#[test]
fn the_argument_is_kept_as_written_and_printing_is_canonical() {
    let written = path(r"C:\dir.x");
    assert_eq!(written.as_written(), r"C:\dir.x");
    assert_eq!(written.to_string(), r"C:\\dir.x");
}

#[test]
fn equality_and_hashing_compare_decoded_segments() {
    assert_eq!(path(r"C:\dir"), path(r"C:\\dir"));
    assert_ne!(path(r"a\.b"), path("a.b"));
    let distinct: HashSet<PropertyPath> = [path(r"C:\dir"), path(r"C:\\dir"), path("a.b"), path(r"a\.b")].into();
    assert_eq!(distinct.len(), 3);
}

#[test]
fn from_segments_refuses_what_no_path_can_spell() {
    assert_eq!(PropertyPath::from_segments(["a.b", "c"]).unwrap().as_written(), r"a\.b.c");
    assert_eq!(PropertyPath::from_segments([""]).unwrap().segments(), [""]);
    assert!(PropertyPath::from_segments(["a", ""]).is_err());
    assert!(PropertyPath::from_segments(Vec::<String>::new()).is_err());
}

#[test]
fn round_trip_holds_for_the_hard_spellings() {
    for segments in [
        vec![r"trailing\"],
        vec![r"trailing\", "child"],
        vec!["literal.dot"],
        vec![r"\."],
        vec![r"\\.", r"\\"],
        vec![r"\", r"\"],
        vec![""],
    ] {
        let printed = PropertyPath::from_segments(segments.clone()).unwrap().to_string();
        assert_eq!(path(&printed).segments(), segments, "{printed}");
    }
}

proptest! {
    #[test]
    fn parse_of_print_is_the_segments(segments in prop::collection::vec("[a.\\\\é ]{1,6}", 1..5)) {
        let printed = PropertyPath::from_segments(segments.clone()).unwrap().to_string();
        let parsed = PropertyPath::parse(&printed).unwrap();
        prop_assert_eq!(parsed.segments(), segments.as_slice());
    }
}

// ── the walk ────────────────────────────────────────────────────────────────

#[test]
fn a_number_is_an_index_on_an_array_and_a_key_on_a_mapping() {
    let root = json!({ "items": ["a", "b"], "map": { "0": "zero", "00": "double" } });
    let on_array = path("items.1");
    let on_array = on_array.resolve(&root);
    assert_eq!(on_array.value(), Some(&json!("b")));
    assert_eq!(on_array.steps(), [PathStep::Key("items"), PathStep::Index(1)]);

    let on_mapping = path("map.0");
    let on_mapping = on_mapping.resolve(&root);
    assert_eq!(on_mapping.value(), Some(&json!("zero")));
    assert_eq!(on_mapping.steps(), [PathStep::Key("map"), PathStep::Key("0")]);

    assert_eq!(path("map.00").resolve(&root).value(), Some(&json!("double")));
}

#[test]
fn only_a_canonical_decimal_is_an_index() {
    let root = json!({ "items": ["a", "b"] });
    for segment in ["01", "00", "+1", "-1", "-", "1.0", "1e0", "٠", "name", " 1"] {
        let walked = PropertyPath::from_segments(["items", segment]).unwrap();
        let walked = walked.resolve(&root);
        assert_eq!(walked.stop(), ResolveStop::OutOfRange { len: 2 }, "{segment}");
        assert_eq!(walked.value(), None, "{segment}");
    }
}

#[test]
fn a_huge_index_is_out_of_range_rather_than_wrapping() {
    let root = json!({ "items": ["a"] });
    let huge = format!("items.{}", "9".repeat(40));
    assert_eq!(path(&huge).resolve(&root).stop(), ResolveStop::OutOfRange { len: 1 });
    // One past `u64::MAX` would wrap to 0.
    assert_eq!(path("items.18446744073709551616").resolve(&root).stop(), ResolveStop::OutOfRange { len: 1 });
    assert_eq!(path("items.1").resolve(&root).stop(), ResolveStop::OutOfRange { len: 1 });
}

#[test]
fn a_present_null_or_scalar_cannot_be_traversed_but_can_be_found() {
    let root = json!({ "gone": null, "name": "Ken", "n": 1, "flag": true });
    for (text, found) in [
        ("gone.x", ValueKind::Null),
        ("name.x", ValueKind::Text),
        ("n.0", ValueKind::Number),
        ("flag.x", ValueKind::Boolean),
    ] {
        assert_eq!(path(text).resolve(&root).stop(), ResolveStop::NotContainer { found }, "{text}");
    }
    let explicit_null = path("gone");
    assert_eq!(explicit_null.resolve(&root).value(), Some(&Value::Null));
}

#[test]
fn a_walk_reports_how_far_it_got() {
    let root = json!({ "a": { "b": {} } });
    let walked = path("a.b.c.d");
    let walked = walked.resolve(&root);
    assert_eq!(walked.stop(), ResolveStop::MissingKey);
    assert_eq!(walked.steps(), [PathStep::Key("a"), PathStep::Key("b")]);
    assert_eq!(walked.unresolved(), ["c", "d"]);
    assert_eq!(walked.pointer().as_pointer_string(), "/a/b");
}

#[test]
fn the_pointer_comes_from_the_typed_walk_and_escapes_its_keys() {
    let root = json!({ "a/b": [{ "c~d": 1 }], "x.y": 2, "": 3 });
    let walked = path("a/b.0.c~d");
    let walked = walked.resolve(&root);
    assert_eq!(walked.pointer().as_pointer_string(), "/a~1b/0/c~0d");
    assert_eq!(walked.pointer().segments(), ["a/b", "0", "c~d"]);

    assert_eq!(path(r"x\.y").resolve(&root).value(), Some(&json!(2)));
    let empty_key = path("");
    let empty_key = empty_key.resolve(&root);
    assert_eq!(empty_key.value(), Some(&json!(3)));
    assert_eq!(empty_key.pointer().segments(), [""]);
}

#[test]
fn a_value_root_and_a_map_root_walk_alike() {
    let root = json!({ "items": [{ "name": "a" }], "title": "Notes" });
    let map = top_level(root.clone());
    for text in ["items.0.name", "title", "items.9", "title.x", "absent", "absent.deeper"] {
        let walked = path(text);
        assert_eq!(walked.resolve(&root), walked.resolve_map(&map), "{text}");
    }
}

// ── set_at ──────────────────────────────────────────────────────────────────

#[test]
fn set_creates_missing_mapping_parents() {
    let mut root = top_level(json!({ "title": "Notes" }));
    set_at(&mut root, &path("author.name"), json!("Ken")).unwrap();
    assert_eq!(as_value(&root), json!({ "title": "Notes", "author": { "name": "Ken" } }));

    set_at(&mut root, &path("author.address.city"), json!("LA")).unwrap();
    assert_eq!(root["author"], json!({ "name": "Ken", "address": { "city": "LA" } }));
}

#[test]
fn a_number_under_a_missing_parent_is_refused_even_at_the_end() {
    for (text, failing) in [
        ("items.0.name", "items.0"),
        ("items.0", "items.0"),
        ("a.b.01", "a.b.01"),
        ("a.b.7.c", "a.b.7"),
    ] {
        let original = top_level(json!({ "title": "Notes" }));
        let mut root = original.clone();
        assert_eq!(
            set_at(&mut root, &path(text), json!(1)),
            Err(PathEditError::NumberUnderMissingParent { path: failing.to_string() }),
            "{text}"
        );
        assert_eq!(root, original, "{text} left a partial parent");
    }
}

#[test]
fn a_refusal_deep_in_an_existing_parent_creates_nothing() {
    let original = top_level(json!({ "a": { "keep": 1 } }));
    let mut root = original.clone();
    assert!(matches!(
        set_at(&mut root, &path("a.b.0.c"), json!(1)),
        Err(PathEditError::NumberUnderMissingParent { .. })
    ));
    assert_eq!(root, original);
}

#[test]
fn a_digit_key_on_an_existing_mapping_is_a_key() {
    let mut root = top_level(json!({ "map": {} }));
    set_at(&mut root, &path("map.0"), json!("zero")).unwrap();
    assert_eq!(root["map"], json!({ "0": "zero" }));
    // A new digit key's own children sit under a parent that is new, but only a
    // later digit segment is ambiguous.
    set_at(&mut root, &path("map.1.name"), json!("one")).unwrap();
    assert_eq!(root["map"]["1"], json!({ "name": "one" }));
}

#[test]
fn a_parent_that_cannot_hold_the_key_names_its_type() {
    for (text, parent, found) in [
        ("author.name", "author", ValueKind::Text),
        ("gone.x", "gone", ValueKind::Null),
        ("deep.n.x.y", "deep.n", ValueKind::Number),
    ] {
        let original = top_level(json!({ "author": "Ken", "gone": null, "deep": { "n": 1 } }));
        let mut root = original.clone();
        let error = set_at(&mut root, &path(text), json!(1)).unwrap_err();
        assert_eq!(error, PathEditError::ParentNotContainer { path: parent.to_string(), found }, "{text}");
        assert!(error.to_string().contains(&found.to_string()), "{error}");
        assert_eq!(root, original, "{text}");
    }
}

#[test]
fn naming_a_null_or_scalar_replaces_it() {
    let mut root = top_level(json!({ "author": "Ken", "gone": null, "a": { "n": 1 } }));
    set_at(&mut root, &path("author"), json!({ "name": "Bob" })).unwrap();
    set_at(&mut root, &path("gone"), json!(1)).unwrap();
    set_at(&mut root, &path("a.n"), json!(null)).unwrap();
    assert_eq!(as_value(&root), json!({ "author": { "name": "Bob" }, "gone": 1, "a": { "n": null } }));
}

#[test]
fn replacing_keeps_the_key_position_and_new_keys_append() {
    let mut root = top_level(json!({ "z": 1, "a": 2, "m": { "y": 1, "b": 2 } }));
    set_at(&mut root, &path("a"), json!(9)).unwrap();
    set_at(&mut root, &path("m.y"), json!(9)).unwrap();
    set_at(&mut root, &path("m.c"), json!(9)).unwrap();
    set_at(&mut root, &path("new"), json!(9)).unwrap();
    assert_eq!(root.keys().map(String::as_str).collect::<Vec<_>>(), ["z", "a", "m", "new"]);
    assert_eq!(keys(&root["m"]), ["y", "b", "c"]);
}

#[test]
fn set_edits_one_existing_array_element_and_never_appends() {
    let mut root = top_level(json!({ "items": [{ "name": "a" }, { "name": "b" }], "tags": ["x"] }));
    set_at(&mut root, &path("items.1.name"), json!("z")).unwrap();
    set_at(&mut root, &path("tags.0"), json!("y")).unwrap();
    assert_eq!(as_value(&root), json!({ "items": [{ "name": "a" }, { "name": "z" }], "tags": ["y"] }));

    let before = root.clone();
    for (text, failing, len) in [
        ("tags.1", "tags.1", 1),
        ("items.2.name", "items.2", 2),
        ("items.01.name", "items.01", 2),
        ("tags.-", "tags.-", 1),
    ] {
        assert_eq!(
            set_at(&mut root, &path(text), json!(1)),
            Err(PathEditError::IndexOutOfRange { path: failing.to_string(), len }),
            "{text}"
        );
    }
    let huge = format!("tags.{}", "9".repeat(40));
    assert!(matches!(set_at(&mut root, &path(&huge), json!(1)), Err(PathEditError::IndexOutOfRange { len: 1, .. })));
    assert_eq!(root, before);
}

#[test]
fn a_literal_dotted_key_differs_from_nested_segments() {
    let mut root = IndexMap::new();
    set_at(&mut root, &path(r"a\.b"), json!(1)).unwrap();
    assert_eq!(as_value(&root), json!({ "a.b": 1 }));
    set_at(&mut root, &path("a.b"), json!(2)).unwrap();
    assert_eq!(as_value(&root), json!({ "a.b": 1, "a": { "b": 2 } }));
}

#[test]
fn the_empty_key_is_a_top_level_key() {
    let mut root = IndexMap::new();
    set_at(&mut root, &path(""), json!(1)).unwrap();
    assert_eq!(as_value(&root), json!({ "": 1 }));
}

// ── removal ─────────────────────────────────────────────────────────────────

#[test]
fn removing_the_last_child_leaves_the_parent() {
    let mut root = top_level(json!({ "style": { "page": { "max-width": 96 } }, "items": ["a"] }));
    remove_at(&mut root, &[path("style.page.max-width"), path("items.0")]).unwrap();
    assert_eq!(as_value(&root), json!({ "style": { "page": {} }, "items": [] }));
}

#[test]
fn surviving_mapping_keys_keep_their_order() {
    let mut root = top_level(json!({ "a": 1, "b": 2, "c": 3, "m": { "x": 1, "y": 2, "z": 3 } }));
    remove_at(&mut root, &[path("a"), path("m.x")]).unwrap();
    assert_eq!(root.keys().map(String::as_str).collect::<Vec<_>>(), ["b", "c", "m"]);
    assert_eq!(keys(&root["m"]), ["y", "z"]);
}

#[test]
fn array_removals_use_original_indexes_in_either_order() {
    for order in [["items.0", "items.2"], ["items.2", "items.0"]] {
        let mut root = top_level(json!({ "items": ["a", "b", "c", "d"] }));
        let paths: Vec<_> = order.iter().map(|text| path(text)).collect();
        remove_at(&mut root, &paths).unwrap();
        assert_eq!(root["items"], json!(["b", "d"]), "{order:?}");
    }
}

#[test]
fn a_removal_inside_an_element_survives_removing_an_earlier_element() {
    for order in [["items.0", "items.2.x"], ["items.2.x", "items.0"]] {
        let mut root = top_level(json!({ "items": [1, 2, { "x": 1, "y": 2 }], "other": 1 }));
        let paths: Vec<_> = order.iter().map(|text| path(text)).collect();
        remove_at(&mut root, &paths).unwrap();
        assert_eq!(root["items"], json!([2, { "y": 2 }]), "{order:?}");
    }
}

#[test]
fn removal_order_is_consistent_across_unrelated_targets() {
    let mut root = top_level(json!({ "a": [1, 2, 3, 4, 5, 6], "b": { "c": 1, "d": 2 }, "e": 1 }));
    let paths = ["a.1", "e", "a.5", "b.d", "a.0", "a.3"].map(path);
    remove_at(&mut root, &paths).unwrap();
    assert_eq!(as_value(&root), json!({ "a": [3, 5], "b": { "c": 1 } }));
}

#[test]
fn a_target_that_does_not_exist_cancels_the_whole_removal() {
    let original = top_level(json!({ "a": { "b": 1 }, "items": ["x"], "name": "Ken" }));
    for (absent, expected) in [
        ("a.missing", PathEditError::Missing { path: "a.missing".to_string() }),
        ("gone.deeper", PathEditError::Missing { path: "gone".to_string() }),
        ("items.3", PathEditError::IndexOutOfRange { path: "items.3".to_string(), len: 1 }),
        ("items.00", PathEditError::IndexOutOfRange { path: "items.00".to_string(), len: 1 }),
        ("name.x", PathEditError::ParentNotContainer { path: "name".to_string(), found: ValueKind::Text }),
    ] {
        let mut root = original.clone();
        assert_eq!(remove_at(&mut root, &[path("a.b"), path(absent)]), Err(expected), "{absent}");
        assert_eq!(root, original, "{absent}");
    }
}

#[test]
fn a_duplicate_or_an_ancestor_with_its_descendant_is_a_conflict() {
    let original = top_level(json!({ "a": { "b": 1 }, "items": [{ "x": 1 }] }));
    for (first, second) in [
        ("a.b", "a.b"),
        ("a", "a.b"),
        ("a.b", "a"),
        ("items.0", "items.0.x"),
        (r"C:\\", r"C:\"),
    ] {
        let mut root = original.clone();
        root.insert(r"C:\".to_string(), json!(1));
        let unchanged = root.clone();
        assert_eq!(
            remove_at(&mut root, &[path(first), path(second)]),
            Err(PathEditError::RemovalConflict { first: first.to_string(), second: second.to_string() }),
            "{first} {second}"
        );
        assert_eq!(root, unchanged, "{first} {second}");
    }
}

#[test]
fn siblings_sharing_a_key_prefix_do_not_conflict() {
    let mut root = top_level(json!({ "a": 1, "ab": 2, "m": { "0": 1, "00": 2 } }));
    remove_at(&mut root, &[path("a"), path("ab"), path("m.0"), path("m.00")]).unwrap();
    assert_eq!(as_value(&root), json!({ "m": {} }));
}

#[test]
fn a_plan_exposes_typed_targets_in_argument_order() {
    let root = top_level(json!({ "items": ["a", "b"], "m": { "0": 1 } }));
    let plan = RemovalPlan::new(&root, &[path("m.0"), path("items.1")]).unwrap();
    assert_eq!(
        plan.targets().collect::<Vec<_>>(),
        [vec![PathStep::Key("m"), PathStep::Key("0")], vec![PathStep::Key("items"), PathStep::Index(1)]]
    );
}

#[test]
fn an_error_message_names_the_array_length() {
    let error = PathEditError::IndexOutOfRange { path: "items.9".to_string(), len: 2 };
    let message = error.to_string();
    assert!(message.contains("items.9") && message.contains('2'), "{message}");
}

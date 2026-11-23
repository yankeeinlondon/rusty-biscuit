//! The structural member admission shared by Darkmatter's registration
//! loader and Claudine's build (`simplified_schema::member_envelope`): each
//! field validator once, through `admit_member_text`'s public result, and
//! the closed key sets against the kind definitions they restate.
//!
//! Boundary: the crate's public API in-process; no files are read.

use simplified_schema::member_envelope::{MemberKind, admit_member_text, declared_name};

const TRIGGER: &str = "kind: schema-trigger\nmatch:\n    prompt: string(required)\n";
const YAML_SCHEMA: &str = "kind: yaml-schema\n$schema:\n    kind: literal(thing; required)\n";
const NAMESPACE: &str = "kind: namespace-schema\nnamespace: seq\n$schema: {}\n";
const LIBRARY: &str = "kind: schema\ntypes:\n    id: string\n";

/// `base` with `line` appended.
fn with(base: &str, line: &str) -> String {
    format!("{base}{line}\n")
}

/// `base` without its top-level `key` (and that key's indented lines).
fn without(base: &str, key: &str) -> String {
    let mut skipping = false;
    let mut kept = String::new();
    for line in base.lines() {
        if line.starts_with(&format!("{key}:")) {
            skipping = true;
            continue;
        }
        if skipping && line.starts_with(' ') {
            continue;
        }
        skipping = false;
        kept.push_str(line);
        kept.push('\n');
    }
    kept
}

/// The message of `text`'s refusal of the written `field`.
fn refused_at(text: &str, field: &str) -> String {
    let refusal = admit_member_text(text).expect_err(text);
    assert_eq!((refusal.field.as_deref(), refusal.absent), (Some(field), false), "{text:?}: {refusal:?}");
    refusal.message
}

/// The message of `text`'s refusal for omitting the required `field`: marked
/// absent, so a reader reports it at the mapping that lacks it.
fn refused_absent(text: &str, field: &str) -> String {
    let refusal = admit_member_text(text).expect_err(text);
    assert_eq!((refusal.field.as_deref(), refusal.absent), (Some(field), true), "{text:?}: {refusal:?}");
    refusal.message
}

#[test]
fn controls_of_every_kind_are_admitted_with_their_labels() {
    for (text, kind) in [
        (TRIGGER, MemberKind::Trigger),
        (YAML_SCHEMA, MemberKind::YamlSchema),
        (NAMESPACE, MemberKind::Namespace),
        (LIBRARY, MemberKind::Library),
    ] {
        let admitted = admit_member_text(text).unwrap_or_else(|refusal| panic!("{text:?}: {refusal:?}"));
        assert_eq!((admitted.kind, admitted.name), (kind, None), "{text:?}");
    }
    let named = admit_member_text(&with(LIBRARY, "name: shared-types")).expect("a named library");
    assert_eq!(named.name.as_deref(), Some("shared-types"));
}

#[test]
fn kind_absent_null_wrong_type_and_unknown() {
    assert!(refused_absent("types:\n    a: string\n", "kind").contains("declares no `kind`"));
    assert!(refused_at("kind:\ntypes:\n    a: string\n", "kind").contains("got null"));
    assert!(refused_at("kind: [schema]\n", "kind").contains("got sequence"));
    assert!(refused_at("kind: document\n", "kind").contains("`document`"));
    assert!(admit_member_text("- kind: schema\n").expect_err("a list").message.contains("not a YAML mapping"));
}

#[test]
fn every_kind_is_a_closed_envelope() {
    for base in [TRIGGER, YAML_SCHEMA, NAMESPACE, LIBRARY] {
        let message = refused_at(&with(base, "extra: 1"), "extra");
        assert!(message.contains("`extra` is not a field"), "{message}");
    }
    // A key another kind allows is still foreign to this one.
    refused_at(&with(NAMESPACE, "name: seq"), "name");
    refused_at(&with(YAML_SCHEMA, "name: thing"), "name");
    refused_at(&with(TRIGGER, "categories: [a]"), "categories");
    refused_at(&with(LIBRARY, "namespace: x"), "namespace");
}

#[test]
fn optional_scalars_may_be_omitted_but_never_null() {
    for base in [TRIGGER, LIBRARY] {
        refused_at(&with(base, "name:"), "name");
        refused_at(&with(base, "name: 5"), "name");
        assert!(refused_at(&with(base, "name: Not_Kebab"), "name").contains("kebab-case"));
    }
    for base in [TRIGGER, YAML_SCHEMA, NAMESPACE, LIBRARY] {
        refused_at(&with(base, "description: [x]"), "description");
        admit_member_text(&with(base, "description: What it is for.")).expect("a written description");
    }
    for base in [TRIGGER, NAMESPACE] {
        assert!(refused_at(&with(base, "description:"), "description").contains("must not be null"));
    }
    // The envelope contract of every `kind: schema` and `yaml-schema` load
    // path: a null `description` is an absent one.
    for base in [YAML_SCHEMA, LIBRARY] {
        admit_member_text(&with(base, "description:")).expect("a null description reads as absent");
    }
    // `not-empty` where the definition says so.
    for base in [YAML_SCHEMA, LIBRARY] {
        refused_at(&with(base, "description: \"\""), "description");
    }
    for base in [TRIGGER, NAMESPACE] {
        admit_member_text(&with(base, "description: \"\"")).expect("an empty description is a string");
    }
}

#[test]
fn trigger_match_and_payload() {
    assert!(refused_absent(&without(TRIGGER, "match"), "match").contains("non-null `match`"));
    refused_at("kind: schema-trigger\nmatch:\n", "match");
    refused_at("kind: schema-trigger\nmatch: text\n", "match");
    admit_member_text("kind: schema-trigger\nmatch:\n    - a: string(required)\n").expect("a match list");
    // A missing or null `$schema` is a matcher-only trigger.
    admit_member_text(&with(TRIGGER, "$schema:")).expect("a null payload");
    admit_member_text(&with(TRIGGER, "$schema: ./doc.yaml")).expect("a reference payload");
    refused_at(&with(TRIGGER, "$schema: {}"), "$schema");
    refused_at(&with(TRIGGER, "$schema: \" \""), "$schema");
    refused_at(&with(TRIGGER, "$schema: 5"), "$schema");
}

#[test]
fn yaml_schema_and_namespace_payloads() {
    for base in [YAML_SCHEMA, NAMESPACE] {
        refused_absent(&without(base, "$schema"), "$schema");
        refused_at(&with(&without(base, "$schema"), "$schema:"), "$schema");
        refused_at(&with(&without(base, "$schema"), "$schema: ./value.yaml"), "$schema");
    }
    admit_member_text(NAMESPACE).expect("an empty namespace payload is valid");
    refused_absent(&without(NAMESPACE, "namespace"), "namespace");
    refused_at("kind: namespace-schema\nnamespace:\n$schema: {}\n", "namespace");
    refused_at("kind: namespace-schema\nnamespace: 5\n$schema: {}\n", "namespace");
}

#[test]
fn types_tables_by_kind() {
    // Optional, never null, may be empty.
    for base in [TRIGGER, NAMESPACE] {
        admit_member_text(&with(base, "types: {}")).expect("an empty local table");
        refused_at(&with(base, "types:"), "types");
        refused_at(&with(base, "types: [a]"), "types");
        refused_at(&with(base, "types: 7"), "types");
    }
    // A yaml-schema's table is optional, never empty.
    admit_member_text(&with(YAML_SCHEMA, "types:")).expect("a null table reads as absent");
    admit_member_text(&with(YAML_SCHEMA, "types: null")).expect("a null table reads as absent");
    refused_at(&with(YAML_SCHEMA, "types: {}"), "types");
    refused_at(&with(YAML_SCHEMA, "types: 7"), "types");
    // A types-only library must supply a non-empty table.
    refused_absent("kind: schema\n", "types");
    for text in ["kind: schema\ntypes:\n", "kind: schema\ntypes: {}\n", "kind: schema\ntypes: [a]\n", "kind: schema\ntypes: 7\n"] {
        refused_at(text, "types");
    }
}

#[test]
fn library_payload_and_categories() {
    admit_member_text(&with(LIBRARY, "$schema:")).expect("the types-only arm's `$schema: null`");
    assert!(refused_at(&with(LIBRARY, "$schema:\n    a: string"), "$schema").contains("types-only"));
    for base in [YAML_SCHEMA, LIBRARY] {
        admit_member_text(&with(base, "categories: [media, text]")).expect("categories");
        refused_at(&with(base, "categories: []"), "categories");
        assert!(refused_at(&with(base, "categories: [media, 5]"), "categories").contains("categories[1]"));
        refused_at(&with(base, "categories: media"), "categories");
    }
}

/// The top-level properties `definition`'s `$schema` (or its arm `arm`)
/// declares.
fn declared_keys(definition: &str, arm: Option<usize>) -> Vec<String> {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(definition).expect("a definition");
    let schema = &value["$schema"];
    let schema = arm.map_or(schema, |index| &schema[index]);
    let mut keys: Vec<String> = schema
        .as_mapping()
        .expect("an object definition")
        .keys()
        .filter_map(|key| key.as_str())
        .filter(|key| *key != "$constraints")
        .map(str::to_string)
        .collect();
    keys.sort();
    keys
}

#[test]
fn the_closed_key_sets_are_the_definitions_keys() {
    for (kind, definition, arm) in [
        (MemberKind::Trigger, include_str!("../../../schemas/schema-trigger.yaml"), None),
        (MemberKind::YamlSchema, include_str!("../../../schemas/yaml-schema.yaml"), None),
        (MemberKind::Namespace, include_str!("../../../schemas/namespace-schema.yaml"), None),
        (MemberKind::Library, include_str!("../../../schemas/schema.yaml"), Some(1)),
    ] {
        let mut keys: Vec<String> = kind.keys().iter().map(|key| (*key).to_string()).collect();
        keys.sort();
        assert_eq!(keys, declared_keys(definition, arm), "{kind:?}");
    }
}

/// The lenient label a reader shows beside a refused member's path: a valid
/// `name` of a trigger or types-only library, even when the rest of the
/// envelope is refused; never an invalid name or another kind's.
#[test]
fn declared_name_reads_a_valid_label_leniently() {
    let named = |base: &str, name: &str| format!("name: {name}\n{base}");
    for (case, text, expected) in [
        ("trigger with an unexpected key", with(&named(TRIGGER, "inline-compose-schema"), "review-typo: 1"), Some("inline-compose-schema")),
        ("library", named(LIBRARY, "parts-list"), Some("parts-list")),
        ("unnamed trigger", TRIGGER.to_string(), None),
        ("invalid name", named(TRIGGER, "Not_Kebab"), None),
        ("library exporting $schema", with(&named(LIBRARY, "parts-list"), "$schema:\n    a: string"), None),
        ("namespace", named(NAMESPACE, "loop-schema"), None),
        ("yaml kind", named(YAML_SCHEMA, "kind-schema"), None),
        ("not YAML", "name: a\n  - [".to_string(), None),
    ] {
        assert_eq!(declared_name(&text).as_deref(), expected, "{case}");
    }
}

//! Identifying triggers through the public API: structural admission over
//! every combinator, and identification that tests only non-null presence
//! for positive required conditions while ordinary matching is unchanged.

use std::collections::BTreeSet;

use serde_json::{Value, json};
use simplified_schema::triggers::{
    IDENTIFY_TOO_BROAD_CODE, IdentifyTooBroad, IdentifyingTrigger, MatchConditions, PathSubject,
    admit_identifying_trigger, identifies, matches, parse_match_conditions, parse_trigger_envelope_from_str,
};

/// The match conditions of a `kind: schema-trigger` envelope.
fn envelope(source: &str) -> MatchConditions {
    parse_trigger_envelope_from_str(source)
        .expect("envelope parses")
        .expect("envelope is claimed")
        .conditions
}

/// The `match:` value alone. The envelope's vacuous-trigger lint refuses some
/// shapes (guard-only, `none:`-only) before admission could; this reaches
/// admission with them.
fn bare_match(source: &str) -> MatchConditions {
    let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).expect("YAML");
    parse_match_conditions(&value).expect("match grammar")
}

fn owned(names: &[&str]) -> BTreeSet<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

fn admit(conditions: &MatchConditions, names: &[&str]) -> Result<IdentifyingTrigger, IdentifyTooBroad> {
    admit_identifying_trigger(conditions, &owned(names))
}

fn refusal_field(conditions: &MatchConditions, names: &[&str]) -> String {
    admit(conditions, names).expect_err("refused as too broad").field
}

fn identifies_detached(trigger: &IdentifyingTrigger, frontmatter: &Value) -> bool {
    identifies(trigger, frontmatter, &PathSubject::detached())
}

/// Ordinary matching of the whole top-level list.
fn matches_detached(conditions: &MatchConditions, frontmatter: &Value) -> bool {
    conditions.0.iter().all(|condition| matches(condition, frontmatter, &PathSubject::detached()))
}

#[test]
fn identify_code_and_hint_name_the_repair() {
    assert_eq!(IDENTIFY_TOO_BROAD_CODE, "dm.host.identify_too_broad");
    let refusal = admit(&bare_match("initialize: object(required)"), &["initialize"]).unwrap_err();
    let rendered = refusal.to_string();
    assert!(rendered.starts_with("`match[0]`"), "{rendered}");
    assert!(rendered.contains(IdentifyTooBroad::hint()), "{rendered}");
}

#[test]
fn identify_all_needs_one_witnessing_child() {
    let list = envelope(
        "kind: schema-trigger\nmatch:\n  - initialize: object(required)\n  - sequence: object(required)\n",
    );
    assert!(admit(&list, &["initialize"]).is_ok(), "top-level list with one witness");

    let explicit = envelope(
        "kind: schema-trigger\nmatch:\n  all:\n    - initialize: object(required)\n    - sequence: object(required)\n",
    );
    assert!(admit(&explicit, &["initialize"]).is_ok(), "`all:` with one witness");

    let shared_only = envelope(
        "kind: schema-trigger\nmatch:\n  all:\n    - initialize: object(required)\n    - finalize: object(required)\n",
    );
    let refusal = admit(&shared_only, &["initialize", "finalize"]).unwrap_err();
    assert_eq!(refusal.field, "match[0].all[0]");
    assert!(refusal.message.contains("has no condition that identifies a host"), "{}", refusal.message);
}

#[test]
fn identify_group_requires_every_alternative_to_witness() {
    let all_witness = envelope(
        "kind: schema-trigger\nmatch:\n  group:\n    - sequence: object(required)\n    - start: object(required)\n",
    );
    assert!(admit(&all_witness, &["initialize"]).is_ok());

    let one_lacks = envelope(
        "kind: schema-trigger\nmatch:\n  group:\n    - sequence: object(required)\n    - start: object(required)\n    - initialize: object(required)\n",
    );
    assert_eq!(refusal_field(&one_lacks, &["initialize"]), "match[0].group[2]");
}

#[test]
fn identify_none_supplies_no_witness() {
    let none_only = bare_match("none:\n  - sequence: object(required)\n");
    assert_eq!(refusal_field(&none_only, &[]), "match[0]");

    let none_beside_witness = envelope(
        "kind: schema-trigger\nmatch:\n  - sequence: object(required)\n  - none:\n      - legacy: string(required)\n",
    );
    assert!(admit(&none_beside_witness, &[]).is_ok());
}

#[test]
fn identify_min_match_witnesses_only_above_the_unwitnessed_count() {
    // Two of the four children lack a witness (an owned key and a guard).
    // `bare_match`: counts 1 and 2 are vacuous to the envelope lint too.
    let at_count = |count: usize| {
        bare_match(&format!(
            "min-match:\n    count: {count}\n    of:\n      \
             - initialize: object(required)\n      - title: string\n      \
             - sequence: object(required)\n      - start: object(required)\n"
        ))
    };
    let refusal = admit(&at_count(2), &["initialize"]).unwrap_err();
    assert_eq!(refusal.field, "match[0]");
    assert!(refusal.message.contains("2 of 4 lack one"), "{}", refusal.message);
    assert!(admit(&at_count(3), &["initialize"]).is_ok(), "count == unwitnessed + 1");
    assert!(admit(&at_count(1), &["initialize"]).is_err());

    // The grammar refuses `count: 0` before admission sees it.
    let zero: serde_yaml_ng::Value =
        serde_yaml_ng::from_str("min-match:\n  count: 0\n  of:\n    - sequence: object(required)\n").unwrap();
    assert!(parse_match_conditions(&zero).is_err());
}

#[test]
fn identify_nested_combinators_compose() {
    let nested = envelope(
        "kind: schema-trigger\nmatch:\n  group:\n    - all:\n        - initialize: object(required)\n        \
         - sequence: object(required)\n    - min-match:\n        count: 2\n        of:\n          \
         - start: object(required)\n          - finalize: object(required)\n          - agent: string(required)\n",
    );
    assert!(admit(&nested, &["initialize", "finalize"]).is_ok());

    let nested_too_broad = envelope(
        "kind: schema-trigger\nmatch:\n  group:\n    - sequence: object(required)\n    - all:\n        \
         - initialize: object(required)\n        - group:\n            - finalize: object(required)\n            \
         - start: object(required)\n",
    );
    let refusal = admit(&nested_too_broad, &["initialize", "finalize"]).unwrap_err();
    assert_eq!(refusal.field, "match[0].group[1].all[0]");
}

#[test]
fn identify_refuses_shared_optional_and_path_only_expressions() {
    let shared = envelope("kind: schema-trigger\nmatch:\n  initialize: object(required)\n");
    let refusal = admit(&shared, &["initialize"]).unwrap_err();
    assert!(refusal.message.contains("`initialize`"), "{}", refusal.message);

    let optional = bare_match("sequence: object\n");
    let refusal = admit(&optional, &[]).unwrap_err();
    assert!(refusal.message.contains("does not require `sequence`"), "{}", refusal.message);

    let path_only = envelope("kind: schema-trigger\nmatch:\n  $path: \"prompts/**/*.md\"\n");
    let refusal = admit(&path_only, &[]).unwrap_err();
    assert!(refusal.message.contains("`$path`"), "{}", refusal.message);
}

#[test]
fn identify_nullable_and_unknown_atoms_supply_no_witness() {
    // A match condition takes one type (no `|` union), so a nullable gate is
    // spelled `null(required)` here.
    for declaration in ["x: unknown(required)\n", "x: null(required)\n"] {
        let conditions = bare_match(declaration);
        assert!(admit(&conditions, &[]).is_err(), "{declaration:?} must not witness");
    }
}

#[test]
fn identify_empty_combinators_are_refused_by_the_grammar() {
    for source in ["[]\n", "{}\n", "group: []\n", "all: []\n", "none: []\n"] {
        let value: serde_yaml_ng::Value = serde_yaml_ng::from_str(source).unwrap();
        assert!(parse_match_conditions(&value).is_err(), "{source:?} must not parse");
    }
    // Admission refuses an empty expression built without the grammar.
    let refusal = admit(&MatchConditions(Vec::new()), &[]).unwrap_err();
    assert_eq!(refusal.field, "match");
}

#[test]
fn identify_tests_non_null_presence_where_ordinary_matching_tests_type() {
    let conditions = envelope("kind: schema-trigger\nmatch:\n  sequence: object(required)\n");
    let trigger = admit(&conditions, &[]).unwrap();

    for value in [json!(3), json!("text"), json!(true), json!([1]), json!({})] {
        let document = json!({ "sequence": value });
        assert!(identifies_detached(&trigger, &document), "{document} identifies");
    }
    assert!(!identifies_detached(&trigger, &json!({ "sequence": null })));
    assert!(!identifies_detached(&trigger, &json!({})));

    // Contrast: ordinary matching still checks the declared type.
    assert!(!matches_detached(&conditions, &json!({ "sequence": 3 })));
    assert!(matches_detached(&conditions, &json!({ "sequence": {} })));
}

#[test]
fn identify_keeps_guard_and_none_semantics() {
    let conditions = envelope(
        "kind: schema-trigger\nmatch:\n  - sequence: object(required)\n  - title: string\n  \
         - none:\n      - legacy: string(required)\n",
    );
    let trigger = admit(&conditions, &[]).unwrap();

    assert!(identifies_detached(&trigger, &json!({ "sequence": 1 })));
    assert!(identifies_detached(&trigger, &json!({ "sequence": 1, "title": "Ada" })));
    // A guard on a wrongly typed value defeats, as in ordinary matching.
    assert!(!identifies_detached(&trigger, &json!({ "sequence": 1, "title": 3 })));
    // Under `none:` the required condition keeps its type test: a mistyped
    // `legacy` does not match, so the exclusion does not fire.
    assert!(identifies_detached(&trigger, &json!({ "sequence": 1, "legacy": 3 })));
    assert!(!identifies_detached(&trigger, &json!({ "sequence": 1, "legacy": "old" })));
}

#[test]
fn identify_leaves_ordinary_guard_matching_unchanged() {
    let conditions = envelope("kind: schema-trigger\nmatch:\n  - sequence: object(required)\n  - title: string\n");
    assert!(!matches_detached(&conditions, &json!({ "sequence": {}, "title": 3 })));
    assert!(matches_detached(&conditions, &json!({ "sequence": {}, "title": "Ada" })));
    assert!(matches_detached(&conditions, &json!({ "sequence": {} })));
}

#[test]
fn identify_admits_and_recognizes_a_claudine_style_host() {
    const HOST_KEYS: [&str; 9] = ["sequence", "start", "blocked", "success", "failure", "loop", "prompt", "agent", "model"];
    const OWNED: [&str; 8] = ["initialize", "cancelled", "finalize", "composed", "failed", "title", "summary", "tags"];

    let alternatives: String = HOST_KEYS.iter().map(|key| format!("    - {key}: object(required)\n")).collect();
    let conditions = envelope(&format!("kind: schema-trigger\nmatch:\n  group:\n{alternatives}"));
    let trigger = admit(&conditions, &OWNED).expect("every alternative names a host-only key");

    let non_null = [json!("text"), json!(3), json!(1.5), json!(true), json!([1]), json!({ "a": 1 })];
    for key in HOST_KEYS {
        for value in &non_null {
            let document = json!({ key: value });
            assert!(identifies_detached(&trigger, &document), "{document} identifies");
        }
        assert!(!identifies_detached(&trigger, &json!({ key: null })), "null `{key}` does not identify");
    }
    for key in ["initialize", "cancelled", "finalize"] {
        for value in &non_null {
            let document = json!({ key: value });
            assert!(!identifies_detached(&trigger, &document), "{document} must not identify");
        }
    }
    assert!(!identifies_detached(&trigger, &json!({ "unrelated": "x", "title": "Ada" })));
    assert!(!identifies_detached(&trigger, &json!({})));
}

#[test]
fn identify_path_never_admits_a_detached_subject() {
    // Passive: the path below does not exist and nothing reads it; a detached
    // subject carries no document, so `$path` cannot hold.
    let conditions = envelope(
        "kind: schema-trigger\nmatch:\n  - $path: \"prompts/**/*.md\"\n  - sequence: object(required)\n",
    );
    let trigger = admit(&conditions, &[]).expect("`$path` beside a witness is admitted");
    assert!(!identifies_detached(&trigger, &json!({ "sequence": {} })));
}

#[cfg(feature = "file")]
#[test]
fn identify_path_still_constrains_identification() {
    use biscuit_file::FileResolutionContext;
    use std::collections::HashMap;

    // Never created: `$path` is judged lexically against the known path.
    let root = std::env::temp_dir().join("simplified-schema-identify-absent-root");
    let context = FileResolutionContext::from_snapshot(&root, None, HashMap::new()).with_repository_root(&root);
    let conditions = envelope(
        "kind: schema-trigger\nmatch:\n  - $path: \"prompts/**/*.md\"\n  - sequence: object(required)\n",
    );
    let trigger = admit(&conditions, &[]).unwrap();
    let identifies_at = |document: &std::path::Path| {
        let document_context = context.for_cwd(document.parent().unwrap());
        let subject = PathSubject::new(document, &document_context).with_pattern_cwd(context.cwd());
        identifies(&trigger, &json!({ "sequence": 3 }), &subject)
    };
    assert!(identifies_at(&root.join("prompts/plan/task.md")));
    assert!(!identifies_at(&root.join("docs/task.md")));
    assert!(!root.exists(), "identification created nothing");
}

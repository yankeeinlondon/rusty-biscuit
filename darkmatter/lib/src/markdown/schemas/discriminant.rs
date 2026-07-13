//! Shared literal discriminant → union-arm selection.
//!
//! When two or more `anyOf` arms are inline-object schemas that share a
//! property typed as a `literal(x)` const (a *discriminant* key), an authored
//! instance can select exactly one arm by matching that key's typed value.
//! This is the single, presentation-neutral authority for that selection: the
//! library validation/reporting layer uses it to narrow diagnostics to the
//! chosen arm, and DMLS reuses it for sibling-key completion and union-narrowed
//! diagnostics — never a second, editor-only discriminant algorithm.
//!
//! Selection is deliberately conservative — it returns `Some(arm_index)` only
//! when narrowing is unambiguous, so every other case falls back to the
//! existing `anyOf` behavior byte-for-byte:
//!
//! - the discriminant key must appear as a literal const in **at least two**
//!   arms (a lone tagged arm is not a discriminated union);
//! - the instance must carry an **authored** (present, non-null) value for at
//!   least one discriminant key;
//! - **every** authored discriminant key must resolve to exactly **one** arm on
//!   its own (a *type-sensitive* match — the string `'2'` never satisfies a
//!   numeric `literal(2)`); a key whose value matches no arm (unknown) or more
//!   than one arm (a duplicate literal) abandons narrowing; and
//! - when the instance carries several discriminant keys, they must all resolve
//!   to the **same** arm — any disagreement abandons narrowing.
//!
//! Absent, unknown, duplicate, ambiguous, or conflicting discriminants all
//! yield `None`.

use std::collections::BTreeMap;
use std::collections::HashMap;

use serde_json::Value;

/// Selects the single union arm whose shared literal discriminant(s) match
/// `instance`, or `None` when no unambiguous narrowing applies.
///
/// `arms` is the raw `anyOf` array (the optional-nullable `{"type":"null"}`
/// sentinel, if present, is ignored — it declares no properties). The returned
/// index is into that same `arms` slice.
pub fn select_literal_discriminant_arm(arms: &[Value], instance: &Value) -> Option<usize> {
    let obj = instance.as_object()?;

    // Each arm's literal-const discriminant properties (by original index).
    let arm_consts: Vec<(usize, BTreeMap<String, &Value>)> = arms
        .iter()
        .enumerate()
        .filter_map(|(idx, arm)| literal_const_props(arm).map(|consts| (idx, consts)))
        .collect();

    // A discriminant key is one that appears as a literal const in >= 2 arms.
    let mut key_arm_count: HashMap<&str, usize> = HashMap::new();
    for (_, consts) in &arm_consts {
        for key in consts.keys() {
            *key_arm_count.entry(key.as_str()).or_default() += 1;
        }
    }
    let discriminant_keys: Vec<&str> = key_arm_count
        .into_iter()
        .filter_map(|(key, count)| (count >= 2).then_some(key))
        .collect();
    if discriminant_keys.is_empty() {
        return None;
    }

    // The instance must supply an authored value for at least one discriminant.
    let present: Vec<&str> = discriminant_keys
        .into_iter()
        .filter(|key| obj.get(*key).is_some_and(|value| !value.is_null()))
        .collect();
    if present.is_empty() {
        return None;
    }

    // Resolve each authored discriminant key independently, then require unanimity.
    // A key selects an arm only when exactly one arm declares it with a
    // type-sensitively equal const; zero matches (unknown value) or several
    // (a duplicate literal across arms) abandons narrowing. Keys that resolve
    // must all agree on the same arm.
    let mut selected: Option<usize> = None;
    for key in &present {
        let instance_value = obj.get(*key)?; // present by construction
        let mut key_arm: Option<usize> = None;
        for (idx, consts) in &arm_consts {
            if consts.get(*key) == Some(&instance_value) {
                if key_arm.is_some() {
                    return None; // duplicate literal → ambiguous
                }
                key_arm = Some(*idx);
            }
        }
        let arm = key_arm?; // unknown value → no arm
        match selected {
            Some(existing) if existing != arm => return None, // keys disagree
            _ => selected = Some(arm),
        }
    }

    selected
}

/// The literal-const discriminant properties of one arm, or `None` when the arm
/// is not an inline object with at least one `{"const": …}` property.
///
/// A property wrapped in the optional-nullable `{"anyOf":[{"type":"null"},
/// {"const": …}]}` form (an un-`required` inline-object literal) is looked
/// through so its const still counts as a discriminant.
fn literal_const_props(arm: &Value) -> Option<BTreeMap<String, &Value>> {
    let props = arm.get("properties")?.as_object()?;
    let mut consts = BTreeMap::new();
    for (key, prop) in props {
        if let Some(const_value) = const_of(prop) {
            consts.insert(key.clone(), const_value);
        }
    }
    (!consts.is_empty()).then_some(consts)
}

/// The `const` value of a property schema, looking through the two-arm
/// optional-nullable wrapper (`{"anyOf":[{"type":"null"}, {"const": …}]}`).
fn const_of(prop: &Value) -> Option<&Value> {
    if let Some(constant) = prop.get("const") {
        return Some(constant);
    }
    let arms = prop.get("anyOf").and_then(Value::as_array)?;
    if arms.len() == 2 && arms[0].get("type").and_then(Value::as_str) == Some("null") {
        return arms[1].get("const");
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Two inline-object arms tagged by a shared literal `kind` (plus a leading
    /// null sentinel, as the optional property union emits).
    fn created_deleted_union() -> Value {
        json!([
            { "type": "null" },
            {
                "type": "object",
                "properties": {
                    "kind": { "const": "created" },
                    "path": { "type": "string" }
                },
                "required": ["path"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "kind": { "const": "deleted" },
                    "reason": { "type": "string" }
                },
                "additionalProperties": false
            }
        ])
    }

    #[test]
    fn selects_matching_arm() {
        let arms = created_deleted_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "created" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), Some(1));
    }

    #[test]
    fn unknown_tag_selects_nothing() {
        let arms = created_deleted_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "renamed" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), None);
    }

    #[test]
    fn absent_discriminant_selects_nothing() {
        let arms = created_deleted_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "path": "a.md" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), None);
    }

    #[test]
    fn null_discriminant_is_not_authored() {
        let arms = created_deleted_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": null });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), None);
    }

    #[test]
    fn discriminant_is_type_sensitive() {
        // Numeric `literal(2)` vs string `literal(other)`; a string `'2'` must
        // not select the numeric arm.
        let arms = json!([
            {
                "type": "object",
                "properties": { "tag": { "const": 2 }, "a": { "type": "string" } },
                "required": ["a"],
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": { "tag": { "const": "other" }, "b": { "type": "string" } },
                "required": ["b"],
                "additionalProperties": false
            }
        ]);
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "tag": "2" })),
            None
        );
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "tag": 2 })),
            Some(0)
        );
    }

    #[test]
    fn lone_tagged_arm_is_not_a_discriminated_union() {
        // Only one arm carries a const `kind`, so `kind` is not a discriminant.
        let arms = json!([
            {
                "type": "object",
                "properties": { "kind": { "const": "created" } },
                "additionalProperties": false
            },
            { "type": "string" }
        ]);
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "kind": "created" })),
            None
        );
    }

    #[test]
    fn duplicate_tag_is_ambiguous() {
        let arms = json!([
            {
                "type": "object",
                "properties": { "kind": { "const": "created" }, "a": { "type": "string" } },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": { "kind": { "const": "created" }, "b": { "type": "string" } },
                "additionalProperties": false
            }
        ]);
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "kind": "created" })),
            None
        );
    }

    #[test]
    fn multiple_discriminants_must_agree() {
        // Arm 0: kind=a, mode=x. Arm 1: kind=b, mode=y. An instance agreeing on
        // both keys with arm 0 selects it; a conflicting pair selects nothing.
        let arms = json!([
            {
                "type": "object",
                "properties": {
                    "kind": { "const": "a" },
                    "mode": { "const": "x" }
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "kind": { "const": "b" },
                    "mode": { "const": "y" }
                },
                "additionalProperties": false
            }
        ]);
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "kind": "a", "mode": "x" })),
            Some(0)
        );
        // Conflicting: kind points at arm 0, mode at arm 1 → no agreement.
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "kind": "a", "mode": "y" })),
            None
        );
    }

    /// A sparse union whose two discriminant keys occur in *overlapping
    /// subsets* of the arms: `kind` tags arms 0 and 1, `mode` tags arms 0 and
    /// 2. Neither key alone covers every arm.
    fn sparse_overlapping_union() -> Value {
        json!([
            {
                "type": "object",
                "properties": {
                    "kind": { "const": "a" },
                    "mode": { "const": "x" }
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": { "kind": { "const": "b" } },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": { "mode": { "const": "y" } },
                "additionalProperties": false
            }
        ])
    }

    #[test]
    fn sparse_known_key_plus_unknown_key_abandons_narrowing() {
        // `kind: a` alone selects arm 0, but the same instance also carries
        // `mode: z`, a qualifying discriminant whose value matches no arm. An
        // unknown second discriminant must preserve ordinary union behavior even
        // though the arm declaring `kind` does not declare `mode`.
        let arms = sparse_overlapping_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "a", "mode": "z" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), None);
    }

    #[test]
    fn sparse_two_keys_select_different_arms_abandons_narrowing() {
        // `kind: b` selects arm 1; `mode: y` selects arm 2. The two qualifying
        // discriminants disagree, so narrowing is abandoned.
        let arms = sparse_overlapping_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "b", "mode": "y" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), None);
    }

    #[test]
    fn sparse_two_keys_agree_on_one_arm_narrows() {
        // `kind: a` and `mode: x` both point at arm 0 — full agreement narrows.
        let arms = sparse_overlapping_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "a", "mode": "x" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), Some(0));
    }

    #[test]
    fn sparse_single_present_key_declared_in_subset_narrows() {
        // Only `kind` is authored; `mode` (declared on a disjoint subset) is
        // absent. The lone present discriminant selects arm 0 unambiguously.
        let arms = sparse_overlapping_union();
        let arms = arms.as_array().unwrap();
        let instance = json!({ "kind": "a" });
        assert_eq!(select_literal_discriminant_arm(arms, &instance), Some(0));
    }

    #[test]
    fn looks_through_optional_nullable_const_wrapper() {
        // An un-`required` inline-object literal wraps its const in the
        // optional-nullable `anyOf` form; the discriminant is still found.
        let arms = json!([
            {
                "type": "object",
                "properties": {
                    "kind": { "anyOf": [ { "type": "null" }, { "const": "created" } ] }
                },
                "additionalProperties": false
            },
            {
                "type": "object",
                "properties": {
                    "kind": { "anyOf": [ { "type": "null" }, { "const": "deleted" } ] }
                },
                "additionalProperties": false
            }
        ]);
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!({ "kind": "created" })),
            Some(0)
        );
    }

    #[test]
    fn non_object_instance_selects_nothing() {
        let arms = created_deleted_union();
        let arms = arms.as_array().unwrap();
        assert_eq!(
            select_literal_discriminant_arm(arms, &json!("created")),
            None
        );
    }
}

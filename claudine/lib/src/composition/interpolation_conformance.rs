//! Shared interpolation conformance matrix (Phase 12).
//!
//! One matrix, two engines. The loop action renderer
//! (`looping::actions::render_action_value`, driven here through the public
//! `ActionStaging` `set` path) and the lifecycle DM2 substrate
//! (`darkmatter::markdown::compose::subtree::SubtreeCompose`) are the two
//! interpolation surfaces the frontmatter action grammar exposes. Both consume
//! the *same* Darkmatter expression core (`parse` / `evaluate` /
//! `ExpressionFinder` / `scalar_string`) over an `EvaluationLookup`; the loop is
//! not a parallel expression engine, only a loop-specific value renderer.
//!
//! [`overlap_cases`] enumerates the syntax both engines support and asserts they
//! produce the *same* value from the *same* input and state. The `divergence_*`
//! tests pin the two documented, intentional differences that keep the loop
//! renderer separate. See `docs/topics/flow-control/looping.md`
//! (§"When templates inside action values are rendered") and
//! `docs/topics/composition.md` (§"Loop vs lifecycle interpolation") for the
//! rationale.

use std::collections::HashMap;

use darkmatter::markdown::compose::subtree::{SubtreeCompose, SubtreeStrictness};
use darkmatter::markdown::compose::{EffectiveState, EffectiveStateBuilder};
use darkmatter::markdown::MarkdownError;
use serde_json::{Map, Value, json};

use super::looping::{ActionStaging, LoopAmbient, LoopExpressionLookup};
use super::CompositionError;
use super::LoopAction;

/// Render `value` through the loop action engine's public `set` path against
/// `frontmatter`, returning the stored value.
fn loop_render(value: &Value, frontmatter: &Map<String, Value>) -> Result<Value, CompositionError> {
    let ambient = LoopAmbient::new(1, true, false, "", 0);
    let lookup = LoopExpressionLookup::new(frontmatter, &ambient);
    let mut stage = ActionStaging::new(&Map::new(), 1, 1);
    stage.apply_action(
        &LoopAction::Set {
            prop: "out".into(),
            value: value.clone(),
        },
        1,
        Some(&lookup),
    )?;
    Ok(stage.commit_map().remove("out").expect("set stores `out`"))
}

/// Render `value` through the lifecycle DM2 substrate against the same
/// frontmatter and the given strictness. Lifecycle text runs in `Strict`.
fn dm2_render(
    value: &Value,
    frontmatter: &Map<String, Value>,
    strictness: SubtreeStrictness,
) -> Result<Value, MarkdownError> {
    let state = effective_state(frontmatter);
    SubtreeCompose::new(value, &state)
        .with_strictness(strictness)
        .compose()
}

fn effective_state(frontmatter: &Map<String, Value>) -> EffectiveState {
    let fm: HashMap<String, Value> = frontmatter
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .build()
        .expect("effective state builds")
}

fn obj(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// A single overlap case: the same input rendered against the same state must
/// produce `expected` from *both* engines.
struct OverlapCase {
    name: &'static str,
    input: Value,
    frontmatter: Map<String, Value>,
    expected: Value,
}

fn overlap_cases() -> Vec<OverlapCase> {
    vec![
        OverlapCase {
            name: "literal string, no template",
            input: json!("plain literal"),
            frontmatter: obj(json!({})),
            expected: json!("plain literal"),
        },
        OverlapCase {
            name: "whole-value number preserves type",
            input: json!("{{count}}"),
            frontmatter: obj(json!({ "count": 3 })),
            expected: json!(3),
        },
        OverlapCase {
            name: "whole-value bool preserves type",
            input: json!("{{flag}}"),
            frontmatter: obj(json!({ "flag": true })),
            expected: json!(true),
        },
        OverlapCase {
            name: "whole-value string preserves type",
            input: json!("{{name}}"),
            frontmatter: obj(json!({ "name": "alice" })),
            expected: json!("alice"),
        },
        OverlapCase {
            name: "whole-value null (known key) preserves null",
            input: json!("{{maybe}}"),
            frontmatter: obj(json!({ "maybe": null })),
            expected: json!(null),
        },
        OverlapCase {
            name: "whole-value object preserves type",
            input: json!("{{payload}}"),
            frontmatter: obj(json!({ "payload": { "a": 1 } })),
            expected: json!({ "a": 1 }),
        },
        OverlapCase {
            name: "whole-value array preserves type",
            input: json!("{{items}}"),
            frontmatter: obj(json!({ "items": [1, 2] })),
            expected: json!([1, 2]),
        },
        OverlapCase {
            name: "mixed text + template becomes string",
            input: json!("iter-{{count}}"),
            frontmatter: obj(json!({ "count": 3 })),
            expected: json!("iter-3"),
        },
        OverlapCase {
            name: "two templates joined by literal text becomes string",
            input: json!("{{a}}+{{b}}"),
            frontmatter: obj(json!({ "a": 1, "b": 2 })),
            expected: json!("1+2"),
        },
        OverlapCase {
            name: "doc namespace whole-value",
            input: json!("{{doc.count}}"),
            frontmatter: obj(json!({ "count": 3 })),
            expected: json!(3),
        },
        OverlapCase {
            name: "function call whole-value",
            input: json!("{{ Length('abcd') }}"),
            frontmatter: obj(json!({})),
            expected: json!(4),
        },
        OverlapCase {
            name: "string-literal escape inside expression",
            // Raw string: the expression text carries a backslash-escaped quote,
            // which both engines' shared lexer resolves to a literal apostrophe.
            input: Value::String(r#"{{ 'it\'s fine' }}"#.to_string()),
            frontmatter: obj(json!({})),
            expected: json!("it's fine"),
        },
        OverlapCase {
            name: "object walked recursively; non-string scalars pass through",
            input: json!({ "phase": "{{stage}}", "n": "{{count}}", "lit": 42 }),
            frontmatter: obj(json!({ "stage": "review", "count": 7 })),
            expected: json!({ "phase": "review", "n": 7, "lit": 42 }),
        },
        OverlapCase {
            name: "array walked recursively; static leaves pass through",
            input: json!(["{{x}}", "{{y}}", "static"]),
            frontmatter: obj(json!({ "x": 1, "y": 2 })),
            expected: json!([1, 2, "static"]),
        },
    ]
}

#[test]
fn loop_and_lifecycle_agree_on_shared_syntax() {
    for case in overlap_cases() {
        let loop_result = loop_render(&case.input, &case.frontmatter)
            .unwrap_or_else(|error| panic!("loop engine failed for `{}`: {error}", case.name));
        assert_eq!(
            loop_result, case.expected,
            "loop engine mismatch for `{}`",
            case.name
        );

        // Lifecycle text runs in strict mode; every overlap case uses only
        // known roots, so strict and lenient resolve identically here.
        let dm2_result = dm2_render(&case.input, &case.frontmatter, SubtreeStrictness::Strict)
            .unwrap_or_else(|error| panic!("DM2 engine failed for `{}`: {error}", case.name));
        assert_eq!(
            dm2_result, case.expected,
            "lifecycle DM2 mismatch for `{}`",
            case.name
        );
    }
}

/// Divergence 1 — the loop renderer re-parses a mixed-string result as JSON, so
/// a concatenation that happens to form valid JSON lands typed; the lifecycle
/// DM2 substrate keeps every mixed string as a string. Documented in
/// `looping.md` ("After rendering, the result is re-parsed as JSON").
#[test]
fn divergence_mixed_string_json_reparse() {
    let input = json!("{{a}}{{b}}");
    let frontmatter = obj(json!({ "a": 1, "b": 2 }));

    let loop_result = loop_render(&input, &frontmatter).expect("loop renders");
    assert_eq!(
        loop_result,
        json!(12),
        "loop re-parses the concatenated `12` as a JSON number"
    );

    let dm2_strict = dm2_render(&input, &frontmatter, SubtreeStrictness::Strict).expect("DM2 renders");
    assert_eq!(
        dm2_strict,
        json!("12"),
        "DM2 keeps the mixed string as a string"
    );
    let dm2_lenient =
        dm2_render(&input, &frontmatter, SubtreeStrictness::Lenient).expect("DM2 renders");
    assert_eq!(dm2_lenient, json!("12"), "DM2 mode does not change typing");
}

/// Divergence 2 — an unknown root in a mixed string is lenient in the loop
/// renderer (resolves empty, matching loop condition evaluation) but fails
/// closed in lifecycle DM2 strict mode (no side effect dispatches with an
/// unresolved reference).
#[test]
fn divergence_unknown_root_strictness() {
    let input = json!("x={{typo}}");
    let frontmatter = obj(json!({}));

    let loop_result = loop_render(&input, &frontmatter).expect("loop tolerates unknown root");
    assert_eq!(loop_result, json!("x="), "loop resolves the unknown root empty");

    // Lifecycle strict fails closed on the unknown root.
    let dm2_strict = dm2_render(&input, &frontmatter, SubtreeStrictness::Strict);
    let error = dm2_strict.expect_err("DM2 strict rejects the unknown root");
    assert!(
        error.to_string().contains("unknown root") && error.to_string().contains("typo"),
        "DM2 strict names the unknown root: {error}"
    );

    // DM2 lenient matches the loop's tolerant behavior.
    let dm2_lenient =
        dm2_render(&input, &frontmatter, SubtreeStrictness::Lenient).expect("DM2 lenient renders");
    assert_eq!(dm2_lenient, json!("x="), "DM2 lenient matches the loop engine");
}

/// Both engines fail closed on a malformed expression — the shared invariant —
/// but with the error type each surface needs: the loop renderer's contextual
/// `LoopActionExpressionInvalid` (carrying iteration/action index plus the typed
/// parse cause) and DM2's `Transform`.
#[test]
fn divergence_malformed_expression_both_fail_closed() {
    let input = json!("{{ >bad }}");
    let frontmatter = obj(json!({}));

    let loop_error = loop_render(&input, &frontmatter).expect_err("loop fails closed");
    assert!(
        matches!(
            loop_error,
            CompositionError::LoopActionExpressionInvalid { .. }
        ),
        "loop surfaces a contextual LoopActionExpressionInvalid: {loop_error}"
    );

    let dm2_error = dm2_render(&input, &frontmatter, SubtreeStrictness::Strict)
        .expect_err("DM2 strict fails closed");
    assert!(
        matches!(dm2_error, MarkdownError::Transform(_)),
        "DM2 surfaces a Transform error: {dm2_error}"
    );
    // Fail-closed on a malformed whole-value span holds in lenient mode too.
    assert!(
        dm2_render(&input, &frontmatter, SubtreeStrictness::Lenient).is_err(),
        "a malformed whole-value span is fatal in both DM2 modes"
    );
}

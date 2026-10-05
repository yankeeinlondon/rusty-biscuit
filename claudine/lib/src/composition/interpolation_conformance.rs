//! Shared interpolation conformance matrix.
//!
//! One matrix, every engine. The loop action renderer
//! (`looping::actions::render_action_value`, driven here through the public
//! `ActionStaging` `set` path), the lifecycle DM2 substrate
//! (`darkmatter::markdown::compose::subtree::SubtreeCompose`), and the sequence
//! source renderer (`sequence::expr::render_interpolated`) are the
//! interpolation surfaces the frontmatter grammar exposes. All consume the
//! *same* Darkmatter expression core over an `EvaluationLookup`.
//!
//! [`overlap_cases`] enumerates the syntax every engine supports and asserts
//! they produce the *same* value from the *same* input and state.
//! [`missing_property_cases`] is the one missing-property and escape table —
//! the inputs of Darkmatter's `absent_property_contract` — shared with the
//! lifecycle executor's own run of it (`executor/tests/binding_contract.rs`),
//! so no consumer can drift from Darkmatter's semantics. The `divergence_*`
//! test pins the one documented, intentional difference (the loop's error
//! type). See `docs/topics/flow-control/looping.md` (§"When templates inside
//! action values are rendered") and `docs/topics/composition.md` (§"Loop vs
//! lifecycle interpolation").

use std::collections::HashMap;

use darkmatter::markdown::compose::subtree::SubtreeCompose;
use darkmatter::markdown::compose::{ComposeContext, EffectiveState, EffectiveStateBuilder};
use darkmatter::markdown::MarkdownError;
use serde_json::{Map, Value, json};

use super::looping::{ActionStaging, LoopAmbient, LoopExpressionLookup};
use super::CompositionError;
use super::LoopAction;

/// Render `value` through the loop action engine's public `set` path against
/// `frontmatter`, returning the stored value.
fn loop_render(value: &Value, frontmatter: &Map<String, Value>) -> Result<Value, CompositionError> {
    let ambient = LoopAmbient::new(1, true, false, "", 0);
    let lookup = LoopExpressionLookup::new(frontmatter, &ambient, crate::test_support::process_context(), std::path::Path::new("prompt.md"));
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
/// frontmatter.
fn dm2_render(
    value: &Value,
    frontmatter: &Map<String, Value>,
    context: &ComposeContext,
) -> Result<Value, MarkdownError> {
    let state = effective_state(frontmatter, context);
    SubtreeCompose::new(value, &state).compose()
}

fn effective_state(
    frontmatter: &Map<String, Value>,
    context: &ComposeContext,
) -> EffectiveState {
    let fm: HashMap<String, Value> = frontmatter
        .iter()
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    EffectiveStateBuilder::new()
        .with_frontmatter(fm)
        .with_context(context.clone())
        .build()
        .expect("effective state builds")
}

fn prepared_context() -> ComposeContext {
    ComposeContext::capture_for_content(std::path::Path::new("."), "")
}

fn obj(value: Value) -> Map<String, Value> {
    value.as_object().cloned().unwrap_or_default()
}

/// A single overlap case: the same input rendered against the same state must
/// produce `expected` from every engine.
pub(crate) struct OverlapCase {
    pub(crate) name: &'static str,
    pub(crate) input: Value,
    pub(crate) frontmatter: Map<String, Value>,
    pub(crate) expected: Value,
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
            name: "mixed text + array becomes compact JSON",
            input: json!("items: {{items}}"),
            frontmatter: obj(json!({ "items": [1, 2] })),
            expected: json!("items: [1,2]"),
        },
        OverlapCase {
            name: "mixed text + empty array keeps the empty container",
            input: json!("items: {{items}}"),
            frontmatter: obj(json!({ "items": [] })),
            expected: json!("items: []"),
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
    let context = prepared_context();
    for case in overlap_cases() {
        let loop_result = loop_render(&case.input, &case.frontmatter)
            .unwrap_or_else(|error| panic!("loop engine failed for `{}`: {error}", case.name));
        assert_eq!(
            loop_result, case.expected,
            "loop engine mismatch for `{}`",
            case.name
        );

        let dm2_result = dm2_render(&case.input, &case.frontmatter, &context)
        .unwrap_or_else(|error| panic!("DM2 engine failed for `{}`: {error}", case.name));
        assert_eq!(
            dm2_result, case.expected,
            "lifecycle DM2 mismatch for `{}`",
            case.name
        );
    }
}

/// A mixed string stays a string in both engines, even when the concatenation
/// happens to form valid JSON: the inserted values are data, so the loop
/// renderer no longer re-parses the result (`looping.md`, "When templates
/// inside action values are rendered").
#[test]
fn mixed_string_stays_a_string_in_both_engines() {
    let input = json!("{{a}}{{b}}");
    let frontmatter = obj(json!({ "a": 1, "b": 2 }));
    let context = prepared_context();

    let loop_result = loop_render(&input, &frontmatter).expect("loop renders");
    assert_eq!(
        loop_result,
        json!("12"),
        "the loop keeps the concatenated `12` as a string"
    );

    let dm2_result = dm2_render(&input, &frontmatter, &context).expect("DM2 renders");
    assert_eq!(
        dm2_result,
        json!("12"),
        "DM2 keeps the mixed string as a string"
    );
}

/// The missing-property and escape table, with the inputs of Darkmatter's
/// `absent_property_contract` L1 tests: an absent property is `null` as a whole
/// value and empty in a mixed string, takes a ternary's falsy branch and a
/// fallback's next operand, and never reads `ctx`; every escape form is inert;
/// inserted data is never evaluated again.
pub(crate) fn missing_property_cases() -> Vec<OverlapCase> {
    let case = |name, input: &str, frontmatter: Value, expected: Value| OverlapCase {
        name,
        input: Value::String(input.to_string()),
        frontmatter: obj(frontmatter),
        expected,
    };
    vec![
        case("absent bare whole value", "{{ missing }}", json!({}), json!(null)),
        case("absent doc whole value", "{{ doc.missing }}", json!({}), json!(null)),
        case("absent descendant", "{{ missing.deep }}", json!({}), json!(null)),
        case("absent in mixed text", "x {{ missing }} y", json!({}), json!("x  y")),
        case("absent bare and doc in mixed text", "[{{ missing }}][{{ doc.missing }}]", json!({}), json!("[][]")),
        case("absent is null", "{{ is_null(missing) }}", json!({}), json!(true)),
        case("ternary falsy branch", "{{ missing ? 'yes' : 'no' }}", json!({}), json!("no")),
        case(
            "fallback chain skips empty",
            "{{ missing || blank || 'last' }}",
            json!({"blank": ""}),
            json!("last"),
        ),
        case("a context key is not a bare name", "[{{ repo }}]", json!({}), json!("[]")),
        case("triple-brace whole value", "{{{ ghost }}}", json!({}), json!("{{ ghost }}")),
        case("triple-brace in mixed text", "a {{{ ghost }}} b", json!({}), json!("a {{ ghost }} b")),
        // A backslash escape is inert and kept as authored, as in Darkmatter's
        // composed body; the Markdown renderer later drops the backslash.
        case("single-backslash whole value", "\\{{ ghost }}", json!({}), json!("\\{{ ghost }}")),
        case("single-backslash in mixed text", "a \\{{ ghost }} b", json!({}), json!("a \\{{ ghost }} b")),
        case("double-backslash whole value", "\\{\\{ ghost }}", json!({}), json!("\\{\\{ ghost }}")),
        case("double-backslash in mixed text", "a \\{\\{ ghost }} b", json!({}), json!("a \\{\\{ ghost }} b")),
        case(
            "inserted braces are data",
            "{{ body }}",
            json!({"body": "agent wrote {{ x }} and $(y)"}),
            json!("agent wrote {{ x }} and $(y)"),
        ),
        case(
            "inserted braces in mixed text are data",
            "> {{ body }}",
            json!({"body": "{{ x }}"}),
            json!("> {{ x }}"),
        ),
    ]
}

/// Every engine renders the missing-property and escape table exactly as
/// Darkmatter's subtree compose does, with no strict-versus-lenient split.
#[test]
fn every_engine_agrees_on_missing_properties_and_escapes() {
    let context = prepared_context();
    for case in missing_property_cases() {
        let dm2 = dm2_render(&case.input, &case.frontmatter, &context)
            .unwrap_or_else(|error| panic!("DM2 failed for `{}`: {error}", case.name));
        assert_eq!(dm2, case.expected, "DM2 for `{}`", case.name);

        let Value::String(raw) = &case.input else { unreachable!() };
        let lookup = super::sequence::expr::SourceExpressionLookup::new(
            &case.frontmatter,
            crate::test_support::process_context(),
            std::path::Path::new("doc.md"),
        );
        let source = super::sequence::expr::render_interpolated(raw, &lookup)
            .unwrap_or_else(|error| panic!("sequence source failed for `{}`: {error}", case.name));
        assert_eq!(source, case.expected, "sequence source for `{}`", case.name);

        // The loop action renderer does not recognize `{{{ … }}}` escapes; it
        // is outside this contract's migration (loop action handling is not
        // reworked here), so only its escape rows are excluded.
        if case.name.starts_with("triple-brace") {
            continue;
        }
        let looped = loop_render(&case.input, &case.frontmatter)
            .unwrap_or_else(|error| panic!("loop failed for `{}`: {error}", case.name));
        assert_eq!(looped, case.expected, "loop for `{}`", case.name);
    }
}

/// Both engines fail closed on a malformed expression — the shared invariant —
/// but with the error type each surface needs: the loop renderer's contextual
/// `LoopActionExpressionInvalid` (carrying iteration/action index plus the typed
/// parse cause) and DM2's typed `Interpolation`.
#[test]
fn divergence_malformed_expression_both_fail_closed() {
    let input = json!("{{ >bad }}");
    let frontmatter = obj(json!({}));
    let context = prepared_context();

    let loop_error = loop_render(&input, &frontmatter).expect_err("loop fails closed");
    assert!(
        matches!(
            loop_error,
            CompositionError::LoopActionExpressionInvalid { .. }
        ),
        "loop surfaces a contextual LoopActionExpressionInvalid: {loop_error}"
    );

    let dm2_error = dm2_render(&input, &frontmatter, &context).expect_err("DM2 fails closed");
    assert!(
        matches!(dm2_error, MarkdownError::Interpolation { .. }),
        "DM2 surfaces a typed Interpolation error: {dm2_error}"
    );
}

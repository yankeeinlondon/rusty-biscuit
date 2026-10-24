//! Darkmatter's lifecycle authoring errors, mapped onto their
//! [`CompositionError`] twins.
//!
//! Claudine's diagnostic catalog, block renderer, and tests are keyed on the
//! `CompositionError` variants, so each [`LifecycleError`] variant converts to
//! the same-named variant field by field and renders the same `Display`.
//!
//! The twins are Claudine's catalog projection of [`LifecycleError`] (and,
//! in `lifecycle_preflight.rs`, of `PreflightError`), not a second parser:
//! they are reached only through `From`. `tests/l1/lifecycle_one_implementation_guard.rs`
//! fails on a twin built anywhere else, except a reviewed Claudine grammar
//! that shares a variant.

use darkmatter::lifecycle::error::LifecycleError;

use super::CompositionError;

impl From<LifecycleError> for CompositionError {
    fn from(error: LifecycleError) -> Self {
        match error {
            LifecycleError::LifecycleInvalid {
                property,
                message,
                source_file,
                unknown_field,
                expected_fields,
            } => Self::LifecycleInvalid {
                property,
                message,
                source_file,
                unknown_field,
                expected_fields,
            },
            LifecycleError::LifecycleSayConflict(path) => Self::LifecycleSayConflict(path),
            LifecycleError::LifecycleUnknownEffect(path, name) => Self::LifecycleUnknownEffect(path, name),
            LifecycleError::LifecycleNestedSpanInLiteral {
                source_path,
                property,
                literal,
                nested,
                suggestion,
            } => Self::LifecycleNestedSpanInLiteral {
                source_path,
                property,
                literal,
                nested,
                suggestion,
            },
            LifecycleError::LifecycleCallRejected {
                source_path,
                property,
                expression,
                source,
            } => Self::LifecycleCallRejected {
                source_path,
                property,
                expression,
                source,
            },
            LifecycleError::RemovedGrammar {
                source_path,
                path,
                removed,
                replacement,
            } => Self::RemovedGrammar {
                source_path,
                path,
                removed,
                replacement,
            },
            LifecycleError::LifecycleStackInvalidShape {
                source_path,
                property,
                message,
            } => Self::LifecycleStackInvalidShape {
                source_path,
                property,
                message,
            },
            LifecycleError::LifecycleWhenExpressionInvalid {
                source_path,
                property,
                message,
                source,
            } => Self::LifecycleWhenExpressionInvalid {
                source_path,
                property,
                message,
                source,
            },
            LifecycleError::LifecycleActionInvalidShortForm {
                source_path,
                property,
                raw,
                message,
            } => Self::LifecycleActionInvalidShortForm {
                source_path,
                property,
                raw,
                message,
            },
            LifecycleError::LifecycleActionInvalidLongForm {
                source_path,
                property,
                action,
                message,
                source,
            } => Self::LifecycleActionInvalidLongForm {
                source_path,
                property,
                action,
                message,
                source,
            },
            LifecycleError::LifecycleUnknownVerb {
                source_path,
                property,
                verb,
                rewrite,
            } => Self::LifecycleUnknownVerb {
                source_path,
                property,
                verb,
                rewrite,
            },
            LifecycleError::LifecycleObjectDataThroughInterpolationPositional {
                source_path,
                property,
                verb,
            } => Self::LifecycleObjectDataThroughInterpolationPositional {
                source_path,
                property,
                verb,
            },
            LifecycleError::LifecycleSetPositionalRemoved {
                source_path,
                property,
                path,
            } => Self::LifecycleSetPositionalRemoved {
                source_path,
                property,
                path,
            },
            LifecycleError::LifecycleSetNotMapping {
                source_path,
                property,
                path,
                actual,
            } => Self::LifecycleSetNotMapping {
                source_path,
                property,
                path,
                actual,
            },
            LifecycleError::LifecycleSetInvalidKey {
                source_path,
                property,
                path,
                key,
                message,
            } => Self::LifecycleSetInvalidKey {
                source_path,
                property,
                path,
                key,
                message,
            },
            LifecycleError::LifecycleObjectDataThroughInterpolationParameter {
                source_path,
                property,
                verb,
                param,
            } => Self::LifecycleObjectDataThroughInterpolationParameter {
                source_path,
                property,
                verb,
                param,
            },
            LifecycleError::LifecycleWithNotMapping {
                source_path,
                property,
                path,
                actual,
            } => Self::LifecycleWithNotMapping {
                source_path,
                property,
                path,
                actual,
            },
            LifecycleError::LifecycleWithWholeMapping {
                source_path,
                property,
                path,
                raw,
            } => Self::LifecycleWithWholeMapping {
                source_path,
                property,
                path,
                raw,
            },
            LifecycleError::LifecycleWithDynamicKey {
                source_path,
                property,
                path,
                key,
            } => Self::LifecycleWithDynamicKey {
                source_path,
                property,
                path,
                key,
            },
            LifecycleError::LifecycleWithUnsupported {
                source_path,
                property,
                verb,
            } => Self::LifecycleWithUnsupported {
                source_path,
                property,
                verb,
            },
            LifecycleError::LifecycleWrongArity {
                source_path,
                property,
                verb,
                message,
            } => Self::LifecycleWrongArity {
                source_path,
                property,
                verb,
                message,
            },
            LifecycleError::LifecycleShortFormRemoved {
                source_path,
                property,
                raw,
                rewrite,
            } => Self::LifecycleShortFormRemoved {
                source_path,
                property,
                raw,
                rewrite,
            },
            LifecycleError::LifecycleExpressionFunctionKeyValueUnsupported {
                source_path,
                property,
                verb,
            } => Self::LifecycleExpressionFunctionKeyValueUnsupported {
                source_path,
                property,
                verb,
            },
            LifecycleError::LifecycleActionPlacement {
                source_path,
                property,
                action,
                event,
            } => Self::LifecycleActionPlacement {
                source_path,
                property,
                action,
                event,
            },
            LifecycleError::LifecycleRenamedEvent {
                source_path,
                authored,
                replacement,
            } => Self::LifecycleRenamedEvent {
                source_path,
                authored,
                replacement,
            },
            LifecycleError::LifecycleMultipleLifecycleActions {
                source_path,
                property,
            } => Self::LifecycleMultipleLifecycleActions {
                source_path,
                property,
            },
            LifecycleError::LifecycleActionOrder {
                source_path,
                property,
            } => Self::LifecycleActionOrder {
                source_path,
                property,
            },
            LifecycleError::LifecycleInvalidArgs {
                source_path,
                property,
                action,
                message,
            } => Self::LifecycleInvalidArgs {
                source_path,
                property,
                action,
                message,
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use darkmatter::lifecycle::error::ActionExprError;
    use darkmatter::markdown::compose::expression::parse_condition;

    use super::*;

    /// One instance of every [`LifecycleError`] variant, with the predicate
    /// its [`CompositionError`] twin must satisfy.
    #[allow(clippy::type_complexity)]
    fn every_variant() -> Vec<(LifecycleError, fn(&CompositionError) -> bool)> {
        let source_path = PathBuf::from("docs/prompt.md");
        let path = || source_path.clone();
        let parse_error = || parse_condition("(").expect_err("an unbalanced condition does not parse");
        let s = |text: &str| text.to_string();
        vec![
            (
                LifecycleError::LifecycleInvalid {
                    property: s("loop"),
                    message: s("`loop` must be an object, got string"),
                    source_file: path(),
                    unknown_field: Some(s("speak")),
                    expected_fields: vec![s("say")],
                },
                |e| matches!(e, CompositionError::LifecycleInvalid { .. }),
            ),
            (
                LifecycleError::LifecycleSayConflict(s("start")),
                |e| matches!(e, CompositionError::LifecycleSayConflict(_)),
            ),
            (
                LifecycleError::LifecycleUnknownEffect(s("start.effect"), s("kazoo")),
                |e| matches!(e, CompositionError::LifecycleUnknownEffect(..)),
            ),
            (
                LifecycleError::LifecycleNestedSpanInLiteral {
                    source_path: path(),
                    property: s("start[0].say.text"),
                    literal: s("'{{ x }}'"),
                    nested: s("{{ x }}"),
                    suggestion: Some(s("x")),
                },
                |e| matches!(e, CompositionError::LifecycleNestedSpanInLiteral { .. }),
            ),
            (
                LifecycleError::LifecycleCallRejected {
                    source_path: path(),
                    property: s("start[0].say.text"),
                    expression: s("min(flag, 5)"),
                    source: Box::new(rejected_call_error()),
                },
                |e| matches!(e, CompositionError::LifecycleCallRejected { .. }),
            ),
            (
                LifecycleError::RemovedGrammar {
                    source_path: path(),
                    path: s("start.stack"),
                    removed: s("stack"),
                    replacement: s("write a list"),
                },
                |e| matches!(e, CompositionError::RemovedGrammar { .. }),
            ),
            (
                LifecycleError::LifecycleStackInvalidShape {
                    source_path: path(),
                    property: s("start[0]"),
                    message: s("a stack item cannot be empty"),
                },
                |e| matches!(e, CompositionError::LifecycleStackInvalidShape { .. }),
            ),
            (
                LifecycleError::LifecycleWhenExpressionInvalid {
                    source_path: path(),
                    property: s("start[0].when"),
                    message: s("`when` is not a valid expression"),
                    source: parse_error(),
                },
                |e| matches!(e, CompositionError::LifecycleWhenExpressionInvalid { .. }),
            ),
            (
                LifecycleError::LifecycleActionInvalidShortForm {
                    source_path: path(),
                    property: s("start[0]"),
                    raw: s(" "),
                    message: s("action cannot be empty"),
                },
                |e| matches!(e, CompositionError::LifecycleActionInvalidShortForm { .. }),
            ),
            (
                LifecycleError::LifecycleActionInvalidLongForm {
                    source_path: path(),
                    property: s("start[0].shell"),
                    action: s("shell"),
                    message: s("`shell` requires a `command` parameter"),
                    source: Some(ActionExprError::Invalid(s("null values are not supported"))),
                },
                |e| {
                    matches!(
                        e,
                        CompositionError::LifecycleActionInvalidLongForm { source: Some(ActionExprError::Invalid(_)), .. }
                    )
                },
            ),
            (
                LifecycleError::LifecycleUnknownVerb {
                    source_path: path(),
                    property: s("start[0].sya"),
                    verb: s("sya"),
                    rewrite: s("; did you mean `say`?"),
                },
                |e| matches!(e, CompositionError::LifecycleUnknownVerb { .. }),
            ),
            (
                LifecycleError::LifecycleObjectDataThroughInterpolationPositional {
                    source_path: path(),
                    property: s("start[0].say"),
                    verb: s("say"),
                },
                |e| matches!(e, CompositionError::LifecycleObjectDataThroughInterpolationPositional { .. }),
            ),
            (
                LifecycleError::LifecycleSetPositionalRemoved {
                    source_path: path(),
                    property: s("start[0]"),
                    path: s("set"),
                },
                |e| matches!(e, CompositionError::LifecycleSetPositionalRemoved { .. }),
            ),
            (
                LifecycleError::LifecycleSetNotMapping {
                    source_path: path(),
                    property: s("start[0]"),
                    path: s("set"),
                    actual: s("string"),
                },
                |e| matches!(e, CompositionError::LifecycleSetNotMapping { .. }),
            ),
            (
                LifecycleError::LifecycleSetInvalidKey {
                    source_path: path(),
                    property: s("start[0]"),
                    path: s("set"),
                    key: String::new(),
                    message: s("destination keys must not be empty"),
                },
                |e| matches!(e, CompositionError::LifecycleSetInvalidKey { .. }),
            ),
            (
                LifecycleError::LifecycleObjectDataThroughInterpolationParameter {
                    source_path: path(),
                    property: s("start[0].ensure_file"),
                    verb: s("ensure_file"),
                    param: s("content"),
                },
                |e| matches!(e, CompositionError::LifecycleObjectDataThroughInterpolationParameter { .. }),
            ),
            (
                LifecycleError::LifecycleWithNotMapping {
                    source_path: path(),
                    property: s("failure[0].retry"),
                    path: s("with"),
                    actual: s("null"),
                },
                |e| matches!(e, CompositionError::LifecycleWithNotMapping { .. }),
            ),
            (
                LifecycleError::LifecycleWithWholeMapping {
                    source_path: path(),
                    property: String::new(),
                    path: s("with"),
                    raw: s("{{ payload }}"),
                },
                |e| matches!(e, CompositionError::LifecycleWithWholeMapping { .. }),
            ),
            (
                LifecycleError::LifecycleWithDynamicKey {
                    source_path: path(),
                    property: s("failure[0].proxy"),
                    path: s("with"),
                    key: s("{{ key }}"),
                },
                |e| matches!(e, CompositionError::LifecycleWithDynamicKey { .. }),
            ),
            (
                LifecycleError::LifecycleWithUnsupported {
                    source_path: path(),
                    property: s("failure[0].resume"),
                    verb: s("resume"),
                },
                |e| matches!(e, CompositionError::LifecycleWithUnsupported { .. }),
            ),
            (
                LifecycleError::LifecycleWrongArity {
                    source_path: path(),
                    property: s("start[0].stop"),
                    verb: s("stop"),
                    message: s("`stop` takes no arguments, got 1"),
                },
                |e| matches!(e, CompositionError::LifecycleWrongArity { .. }),
            ),
            (
                LifecycleError::LifecycleShortFormRemoved {
                    source_path: path(),
                    property: s("start[0]"),
                    raw: s("say(\"hi\")"),
                    rewrite: s("say: \"hi\""),
                },
                |e| matches!(e, CompositionError::LifecycleShortFormRemoved { .. }),
            ),
            (
                LifecycleError::LifecycleExpressionFunctionKeyValueUnsupported {
                    source_path: path(),
                    property: s("start[0].and"),
                    verb: s("and"),
                },
                |e| matches!(e, CompositionError::LifecycleExpressionFunctionKeyValueUnsupported { .. }),
            ),
            (
                LifecycleError::LifecycleActionPlacement {
                    source_path: path(),
                    property: s("start[0].skip"),
                    action: s("skip"),
                    event: s("start"),
                },
                |e| matches!(e, CompositionError::LifecycleActionPlacement { .. }),
            ),
            (
                LifecycleError::LifecycleRenamedEvent {
                    source_path: path(),
                    authored: s("composed"),
                    replacement: s("start"),
                },
                |e| matches!(e, CompositionError::LifecycleRenamedEvent { .. }),
            ),
            (
                LifecycleError::LifecycleMultipleLifecycleActions {
                    source_path: path(),
                    property: s("start[0]"),
                },
                |e| matches!(e, CompositionError::LifecycleMultipleLifecycleActions { .. }),
            ),
            (
                LifecycleError::LifecycleActionOrder {
                    source_path: path(),
                    property: s("success[0].stop"),
                },
                |e| matches!(e, CompositionError::LifecycleActionOrder { .. }),
            ),
            (
                LifecycleError::LifecycleInvalidArgs {
                    source_path: path(),
                    property: s("failure[0].proxy"),
                    action: s("proxy"),
                    message: s("`proxy` requires a `target` parameter"),
                },
                |e| matches!(e, CompositionError::LifecycleInvalidArgs { .. }),
            ),
        ]
    }

    /// The verdict Darkmatter's call check gives `min(true, 5)`.
    fn rejected_call_error() -> darkmatter::markdown::compose::expression::ExpressionError {
        use darkmatter::markdown::compose::expression::{DeclaredTypes, ExpressionError, TypeScope, check_expression_calls, parse_spanned};
        let (declared, expr) = (DeclaredTypes::new(None), parse_spanned("min(true, 5)").expect("parses"));
        let check = check_expression_calls(&expr, &TypeScope::unguarded(&declared)).remove(0);
        ExpressionError::CallCheck(Box::new(check))
    }

    #[test]
    fn every_lifecycle_error_converts_to_its_same_named_twin_with_the_same_display() {
        let variants = every_variant();
        assert_eq!(variants.len(), 28, "one case per `LifecycleError` variant");
        for (error, is_twin) in variants {
            let expected = error.to_string();
            let converted = CompositionError::from(error);
            assert!(is_twin(&converted), "{expected}: converted to {converted:?}");
            assert_eq!(converted.to_string(), expected);
        }
    }

    #[test]
    fn a_typed_cause_survives_the_conversion() {
        let error = LifecycleError::LifecycleWhenExpressionInvalid {
            source_path: PathBuf::from("doc.md"),
            property: "start[0].when".to_string(),
            message: "`when` is not a valid expression".to_string(),
            source: parse_condition("(").expect_err("does not parse"),
        };
        let converted = CompositionError::from(error);
        let source = std::error::Error::source(&converted).expect("a source");
        assert!(source.downcast_ref::<darkmatter::markdown::compose::expression::ParseError>().is_some());
    }
}

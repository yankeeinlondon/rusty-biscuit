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
            // Claudine renders a failed preparation as its error alone; it
            // has no failure report to show the earlier warnings in.
            LifecycleError::LifecycleCallRejected {
                source_path,
                property,
                expression,
                diagnostic,
                earlier: _,
                source,
            } => Self::LifecycleCallRejected {
                source_path,
                property,
                expression,
                diagnostic,
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
mod tests;

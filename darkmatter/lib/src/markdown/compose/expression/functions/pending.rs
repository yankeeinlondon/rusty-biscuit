//! Declared functions whose runtime implementation lands in a later phase of
//! `features/2026-09-09-more-context`.
//!
//! The descriptor catalog is the single source of truth for names and
//! signatures (plan Phase 4), and the registry requires every cataloged
//! function to have a binding. Each binding here fails loudly with
//! [`ExpressionError::Other`] instead of returning a plausible value. A phase
//! that implements a function moves its binding into the owning group;
//! `pending_bindings_are_exactly_the_unimplemented_functions` keeps this list
//! honest.

use serde_json::Value;

use super::{EvaluationMode, FunctionBinding, FunctionHandler};
use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

macro_rules! pending {
    ($name:literal, $phase:literal) => {
        FunctionBinding {
            canonical: $name,
            aliases: &[],
            evaluation: EvaluationMode::Context,
            handler: Some(FunctionHandler::Context({
                fn handler(_: &[Value], _: &ResolutionContext) -> Result<Value, ExpressionError> {
                    Err(ExpressionError::Other {
                        function: $name.to_string(),
                        message: concat!(
                            "is declared but not implemented yet (more-context phase ",
                            $phase,
                            ")"
                        )
                        .to_string(),
                    })
                }
                handler
            })),
        }
    };
}

pub(super) const BINDINGS: &[FunctionBinding] = &[
    pending!("package_area", "7"),
    pending!("package", "7"),
    pending!("recent_commits", "7"),
    pending!("ipv4", "7"),
    pending!("ipv6", "7"),
    pending!("has_alias", "7"),
    pending!("has_builtin_function", "7"),
    pending!("has_user_function", "7"),
    pending!("can_execute", "7"),
    pending!("has_agentic_cli", "7"),
    pending!("as_markdown", "8"),
    pending!("ping", "9"),
    pending!("ping_under", "9"),
];

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::BINDINGS;
    use crate::markdown::compose::expression::ResolutionContext;
    use super::super::{FunctionHandler, dispatch_fs};

    /// A ratchet, not a tolerance: the list is exact, and every entry must
    /// still fail. Implementing a function without removing it from this group
    /// fails `registration_names_aliases_and_signatures_are_unique` (duplicate
    /// canonical) or this test.
    #[test]
    fn pending_bindings_are_exactly_the_unimplemented_functions() {
        let names: Vec<&str> = BINDINGS.iter().map(|binding| binding.canonical).collect();
        assert_eq!(
            names,
            [
                "package_area", "package", "recent_commits", "ipv4", "ipv6", "has_alias",
                "has_builtin_function", "has_user_function", "can_execute", "has_agentic_cli",
                "as_markdown", "ping", "ping_under",
            ]
        );
        let context = ResolutionContext::default();
        for binding in BINDINGS {
            let Some(FunctionHandler::Context(handler)) = binding.handler else {
                panic!("{} must use a context handler", binding.canonical);
            };
            let message = handler(&[Value::Null], &context).unwrap_err().to_string();
            assert!(message.contains("not implemented yet"), "{}: {message}", binding.canonical);
            let dispatched = dispatch_fs(binding.canonical, &[Value::from("x")], &context);
            assert!(
                matches!(dispatched, Some(Err(_))),
                "{} must dispatch to its pending error",
                binding.canonical
            );
        }
    }
}

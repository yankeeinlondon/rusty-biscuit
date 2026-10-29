//! `as_markdown(content)`: composes a string through the calling request's
//! pipeline (R11, R20). The composition itself lives in
//! [`crate::markdown::compose::nested`].

use serde_json::Value;

use super::{BoundCall, FunctionBinding, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding::context("as_markdown", &[], as_markdown_fn)];

/// `null` passes through without composing anything.
fn as_markdown_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(content) = call.args().text(0) else {
        return Ok(Value::Null);
    };
    context.nested_compose.evaluate(content).map(Value::String)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::call_function;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};
    use crate::markdown::compose::nested::NestedComposeSlot;

    #[test]
    fn a_surface_without_a_compose_request_is_a_contract_violation() {
        let error = call_function("as_markdown", &[json!("# Title")], Some(&ResolutionContext::default()))
            .expect("as_markdown is a context function")
            .unwrap_err();

        assert!(
            matches!(&error, ExpressionError::ContractViolation { function, .. } if function == "as_markdown"),
            "{error:?}"
        );
        assert!(error.is_authoring_fatal());
    }

    /// Ledger row 111 (`as_markdown(content: string | null)`): `null` passes
    /// through without composing, a number arrives as its text, and a list or
    /// a wrong argument count fails at binding before the handler.
    #[test]
    fn arguments_bind_before_composing() {
        let context = ResolutionContext {
            nested_compose: NestedComposeSlot::discover(),
            ..ResolutionContext::default()
        };
        assert_eq!(call_function("as_markdown", &[Value::Null], Some(&context)).unwrap().unwrap(), Value::Null);
        for arguments in [vec![json!(["a"])], vec![]] {
            let error = call_function("as_markdown", &arguments, Some(&context)).unwrap().unwrap_err();
            assert!(matches!(error, ExpressionError::Call(_)), "{arguments:?}: {error:?}");
            assert!(error.to_string().contains("as_markdown"), "{error}");
        }
        assert!(context.nested_compose.take_discovered().is_empty());

        assert_eq!(call_function("as_markdown", &[json!(42)], Some(&context)).unwrap().unwrap(), json!(""));
        assert_eq!(context.nested_compose.take_discovered(), ["42"]);
    }

    /// Discovery records what it was given, empty content included, and
    /// composes nothing.
    #[test]
    fn discovery_records_content_and_returns_an_empty_string() {
        let context = ResolutionContext {
            nested_compose: NestedComposeSlot::discover(),
            ..ResolutionContext::default()
        };
        let clone = context.clone();

        for content in ["::shell echo hi", ""] {
            let value = call_function("as_markdown", &[json!(content)], Some(&clone)).unwrap().unwrap();
            assert_eq!(value, json!(""));
        }

        assert_eq!(context.nested_compose.take_discovered(), ["::shell echo hi", ""]);
        assert!(context.nested_compose.take_discovered().is_empty(), "draining empties the sink");
    }
}

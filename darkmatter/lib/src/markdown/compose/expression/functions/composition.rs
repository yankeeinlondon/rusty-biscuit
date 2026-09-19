//! `as_markdown(content)`: composes a string through the calling request's
//! pipeline (R11, R20). The composition itself lives in
//! [`crate::markdown::compose::nested`].

use serde_json::Value;

use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::expression::ExpressionError;

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding {
    canonical: "as_markdown",
    aliases: &[],
    evaluation: EvaluationMode::Context,
    handler: Some(FunctionHandler::Context(as_markdown_fn)),
}];

fn as_markdown_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let [content] = args else {
        return Err(ExpressionError::Other {
            function: "as_markdown".to_string(),
            message: format!("requires 1 argument, got {}", args.len()),
        });
    };
    let Some(content) = content.as_str() else {
        return Err(ExpressionError::ArgType {
            function: "as_markdown",
            index: 0,
            expected: "string",
            actual_type: super::git::value_type(content),
        });
    };
    context.nested_compose.evaluate(content).map(Value::String)
}

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};

    use super::super::dispatch_fs;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};
    use crate::markdown::compose::nested::NestedComposeSlot;

    #[test]
    fn a_surface_without_a_compose_request_is_a_contract_violation() {
        let error = dispatch_fs("as_markdown", &[json!("# Title")], &ResolutionContext::default())
            .expect("as_markdown is a context function")
            .unwrap_err();

        assert!(
            matches!(&error, ExpressionError::ContractViolation { function, .. } if function == "as_markdown"),
            "{error:?}"
        );
        assert!(error.is_authoring_fatal());
    }

    #[test]
    fn non_string_arguments_are_argument_type_errors() {
        let context = ResolutionContext {
            nested_compose: NestedComposeSlot::discover(),
            ..ResolutionContext::default()
        };
        for (argument, actual) in [(json!(42), "number"), (Value::Null, "null"), (json!(["a"]), "array")] {
            let error = dispatch_fs("as_markdown", &[argument], &context).unwrap().unwrap_err();
            assert!(
                matches!(error, ExpressionError::ArgType { function: "as_markdown", index: 0, expected: "string", actual_type } if actual_type == actual),
                "{actual}: {error:?}"
            );
        }
        assert!(context.nested_compose.take_discovered().is_empty());

        let arity = dispatch_fs("as_markdown", &[], &context).unwrap().unwrap_err();
        assert!(arity.to_string().contains("as_markdown"), "{arity}");
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
            let value = dispatch_fs("as_markdown", &[json!(content)], &clone).unwrap().unwrap();
            assert_eq!(value, json!(""));
        }

        assert_eq!(context.nested_compose.take_discovered(), ["::shell echo hi", ""]);
        assert!(context.nested_compose.take_discovered().is_empty(), "draining empties the sink");
    }
}

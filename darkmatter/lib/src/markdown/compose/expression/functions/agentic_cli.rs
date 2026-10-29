//! `has_agentic_cli(agent)`: whether a Claudine roster provider's CLI is on the
//! request's captured `InstalledAiClients` `PATH` index (R13, R19).

use serde_json::Value;
use sniff::programs::AiCli;

use super::agentic_cli_generated::AGENTIC_CLI_NAMES;
use super::{BoundCall, FunctionBinding, ResolutionContext};
use crate::markdown::compose::context::ContextGroup;
use crate::markdown::compose::expression::ExpressionError;

const FUNCTION: &str = "has_agentic_cli";

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding::context(FUNCTION, &[], has_agentic_cli_fn)];

/// The `agentic-cli` type admits only roster names, so an unknown name is a
/// binding error even in a request that never captured the `Agent` group;
/// `null` is `false` without reading the capture.
fn has_agentic_cli_fn(call: &BoundCall<'_>, context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let Some(name) = call.args().text(0) else {
        return Ok(Value::Bool(false));
    };
    let cli = agentic_cli(name)?;
    let installed = context.observations.agentic_clis().ok_or_else(|| {
        ExpressionError::FunctionContextNotCaptured {
            function: FUNCTION.to_string(),
            group: ContextGroup::Agent,
        }
    })?;
    Ok(Value::Bool(installed.is_installed(cli)))
}

/// The sniff variant a generated roster name or alias binds to. A name the
/// `agentic-cli` enum admitted but the generated table lacks is a catalog
/// parity defect, reported as a contract violation.
fn agentic_cli(name: &str) -> Result<AiCli, ExpressionError> {
    let Some((_, variant)) = AGENTIC_CLI_NAMES.iter().find(|(accepted, _)| *accepted == name) else {
        return Err(ExpressionError::ContractViolation {
            function: FUNCTION.to_string(),
            message: format!("agentic CLI {name:?} has no generated roster binding"),
        });
    };
    serde_json::from_value(Value::String((*variant).to_string())).map_err(|error| {
        ExpressionError::ContractViolation {
            function: FUNCTION.to_string(),
            message: format!("generated binding `{variant}` for {name:?} is not a sniff AiCli variant: {error}"),
        }
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use serde_json::{Value, json};
    use sniff::programs::{AiCli, ExecutableSource, InstalledAiClients};

    use super::super::call_function;
    use crate::markdown::compose::context::ContextGroup;
    use crate::markdown::compose::context::capture::CapturedObservations;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    fn context_with(installed: InstalledAiClients) -> ResolutionContext {
        ResolutionContext::default()
            .with_observations(CapturedObservations::for_test_agentic_clis(installed))
    }

    fn call(argument: Value, context: &ResolutionContext) -> Result<Value, ExpressionError> {
        call_function("has_agentic_cli", &[argument], Some(context)).expect("registered context function")
    }

    /// AC18: provider aliases agree with their slug in both outcomes.
    #[test]
    fn aliases_return_the_same_value_as_their_provider() {
        let kimi_only = context_with(InstalledAiClients::default().with_program(
            AiCli::KimiCli,
            PathBuf::from("/fixture/bin/kimi"),
            ExecutableSource::Path,
        ));
        let nothing = context_with(InstalledAiClients::default());

        for name in ["kimi", "kimi_code", "kimicode", "kimi-code"] {
            assert_eq!(call(json!(name), &kimi_only).unwrap(), json!(true), "{name}");
            assert_eq!(call(json!(name), &nothing).unwrap(), json!(false), "{name}");
        }
        for name in ["claude", "qwen", "qwen_code", "open-code"] {
            assert_eq!(call(json!(name), &kimi_only).unwrap(), json!(false), "{name}");
        }
    }

    /// AC18: an unknown name is an error, never `false`, whether or not the
    /// group was captured; only generated spellings resolve. The `agentic-cli`
    /// enum now rejects it at binding (ledger row 103), so the handler is
    /// never entered.
    #[test]
    fn unknown_names_are_binding_errors() {
        use crate::markdown::compose::expression::functions::test_hooks::{handler_entries, reset_handler_entries};

        let captured = context_with(InstalledAiClients::default());
        let uncaptured = ResolutionContext::default();
        reset_handler_entries();
        for name in ["not_a_provider", "Kimi", "kimi_cli", "KimiCli", " kimi", "", "aider"] {
            for context in [&captured, &uncaptured] {
                let error = call(json!(name), context).unwrap_err();
                assert!(matches!(error, ExpressionError::Call(_)), "{name}: {error:?}");
            }
        }
        assert_eq!(handler_entries("has_agentic_cli"), 0);
    }

    #[test]
    fn uncaptured_agent_group_is_fatal_and_null_is_false() {
        let error = call(json!("claude"), &ResolutionContext::default()).unwrap_err();
        assert!(matches!(
            error,
            ExpressionError::FunctionContextNotCaptured { group: ContextGroup::Agent, .. }
        ));
        assert!(error.is_authoring_fatal());

        // Ledger row 103: `null` is `false` (a predicate), even uncaptured;
        // a number converts to text that names no roster CLI, and a list
        // does not bind.
        assert_eq!(call(Value::Null, &ResolutionContext::default()).unwrap(), json!(false));
        let captured = context_with(InstalledAiClients::default());
        for argument in [json!(1), json!(["claude"])] {
            assert!(matches!(call(argument.clone(), &captured), Err(ExpressionError::Call(_))), "{argument}");
        }
    }

    /// Every clone of one capture shares a single `PATH` scan.
    #[test]
    fn a_captured_observation_is_shared_by_clones() {
        let first = context_with(InstalledAiClients::default().with_program(
            AiCli::Claude,
            PathBuf::from("/fixture/bin/claude"),
            ExecutableSource::Path,
        ));
        let second = first.clone();
        assert!(std::ptr::eq(
            first.observations.agentic_clis().unwrap(),
            second.observations.agentic_clis().unwrap(),
        ));
    }
}

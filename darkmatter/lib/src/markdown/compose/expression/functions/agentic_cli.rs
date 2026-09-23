//! `has_agentic_cli(agent)`: whether a Claudine roster provider's CLI is on the
//! request's captured `InstalledAiClients` `PATH` index (R13, R19).

use serde_json::Value;
use sniff::programs::AiCli;

use super::agentic_cli_generated::AGENTIC_CLI_NAMES;
use super::{EvaluationMode, FunctionBinding, FunctionHandler, ResolutionContext};
use crate::markdown::compose::context::ContextGroup;
use crate::markdown::compose::expression::ExpressionError;

const FUNCTION: &str = "has_agentic_cli";

pub(super) const BINDINGS: &[FunctionBinding] = &[FunctionBinding {
    canonical: FUNCTION,
    aliases: &[],
    evaluation: EvaluationMode::Context,
    handler: Some(FunctionHandler::Context(has_agentic_cli_fn)),
}];

/// The name is checked before the capture, so an unknown name is a compose
/// error even in a request that never captured the `Agent` group.
fn has_agentic_cli_fn(args: &[Value], context: &ResolutionContext) -> Result<Value, ExpressionError> {
    let name = match args.first() {
        Some(Value::String(name)) => name,
        other => {
            return Err(ExpressionError::ArgType {
                function: FUNCTION,
                index: 0,
                expected: "string",
                actual_type: super::git::value_type(other.unwrap_or(&Value::Null)),
            });
        }
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

/// The sniff variant a generated roster name or alias binds to.
fn agentic_cli(name: &str) -> Result<AiCli, ExpressionError> {
    let Some((_, variant)) = AGENTIC_CLI_NAMES.iter().find(|(accepted, _)| *accepted == name) else {
        let accepted = AGENTIC_CLI_NAMES.iter().map(|(accepted, _)| *accepted).collect::<Vec<_>>();
        return Err(ExpressionError::ContractViolation {
            function: FUNCTION.to_string(),
            message: format!("unknown agentic CLI {name:?}; expected one of: {}", accepted.join(", ")),
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

    use super::super::dispatch_fs;
    use crate::markdown::compose::context::ContextGroup;
    use crate::markdown::compose::context::capture::CapturedObservations;
    use crate::markdown::compose::expression::{ExpressionError, ResolutionContext};

    fn context_with(installed: InstalledAiClients) -> ResolutionContext {
        ResolutionContext::default()
            .with_observations(CapturedObservations::for_test_agentic_clis(installed))
    }

    fn call(argument: Value, context: &ResolutionContext) -> Result<Value, ExpressionError> {
        dispatch_fs("has_agentic_cli", &[argument], context).expect("registered context function")
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

    /// AC18: an unknown name is a compose error, never `false`, whether or not
    /// the group was captured; only generated spellings resolve.
    #[test]
    fn unknown_names_are_contract_violations() {
        let captured = context_with(InstalledAiClients::default());
        let uncaptured = ResolutionContext::default();
        for name in ["not_a_provider", "Kimi", "kimi_cli", "KimiCli", " kimi", "", "aider"] {
            for context in [&captured, &uncaptured] {
                let error = call(json!(name), context).unwrap_err();
                assert!(matches!(error, ExpressionError::ContractViolation { .. }), "{name}: {error:?}");
                assert!(error.is_authoring_fatal(), "{name}");
            }
        }
    }

    #[test]
    fn uncaptured_agent_group_is_fatal_and_non_strings_are_type_errors() {
        let error = call(json!("claude"), &ResolutionContext::default()).unwrap_err();
        assert!(matches!(
            error,
            ExpressionError::FunctionContextNotCaptured { group: ContextGroup::Agent, .. }
        ));
        assert!(error.is_authoring_fatal());

        let captured = context_with(InstalledAiClients::default());
        for argument in [Value::Null, json!(1), json!(["claude"])] {
            assert!(matches!(
                call(argument, &captured),
                Err(ExpressionError::ArgType { function: "has_agentic_cli", index: 0, .. })
            ));
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

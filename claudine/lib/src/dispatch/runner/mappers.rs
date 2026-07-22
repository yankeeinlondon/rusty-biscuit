use regex::Regex;
use tracing::warn;

use crate::actions::{CompiledMapper, HookDecision, HookResponse, Mapper};
use crate::error::Result;

#[derive(Debug)]
pub(super) struct CommandOutput {
    pub status: std::process::ExitStatus,
    pub stdout: String,
    pub stderr: String,
}

pub(super) fn apply_mapper(
    compiled_mapper: Option<&CompiledMapper>,
    fallback_mapper: Option<&Mapper>,
    output: &CommandOutput,
) -> Result<HookResponse> {
    if let Some(compiled_mapper) = compiled_mapper {
        return match compiled_mapper {
            CompiledMapper::JsonField { field } => map_json_field(field, output),
            CompiledMapper::JsonObject => map_json_object(output),
            CompiledMapper::ExitCode => Ok(map_exit_code(output)),
            CompiledMapper::Regex { pattern } => map_regex_with_compiled(pattern, output),
        };
    }

    match fallback_mapper.unwrap_or(&Mapper::ExitCode) {
        Mapper::ExitCode => Ok(map_exit_code(output)),
        Mapper::JsonField { field } => map_json_field(field, output),
        Mapper::JsonObject => map_json_object(output),
        Mapper::Regex { pattern } => {
            tracing::debug!(
                pattern,
                "apply_mapper: compiled_mapper not provided, falling back to per-call regex compilation"
            );
            let regex = Regex::new(pattern)?;
            map_regex_with_compiled(&regex, output)
        }
    }
}

/// Map a `Call` action's child exit status onto a `HookResponse`.
///
/// - `0` → [`HookDecision::Allow`].
/// - `2` → [`HookDecision::Deny`].
/// - Anything else → no decision (the caller falls through to the next
///   action). A `tracing::warn!` is emitted so operators can distinguish
///   "command crashed" (`139`), "not found" (`127`), "permission denied"
///   (`126`), and user-defined anomalous exits from a clean success.
pub(super) fn map_exit_code(output: &CommandOutput) -> HookResponse {
    let code = output.status.code().unwrap_or(1);
    let decision = match code {
        0 => Some(HookDecision::Allow),
        2 => Some(HookDecision::Deny),
        _ => {
            warn!(
                exit_code = code,
                stderr = %output.stderr,
                "map_exit_code: unexpected exit status — no decision emitted",
            );
            None
        }
    };

    let reason = if !output.stdout.is_empty() {
        Some(output.stdout.clone())
    } else if !output.stderr.is_empty() {
        Some(output.stderr.clone())
    } else {
        None
    };

    HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    }
}

pub(super) fn map_json_field(field: &str, output: &CommandOutput) -> Result<HookResponse> {
    let parsed: serde_json::Value = serde_json::from_str(&output.stdout)?;
    let value = super::dot_lookup(&parsed, field).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError(format!("mapper field not found: {field}"))
    })?;

    let decision = super::parse_decision(value);
    let reason = parsed
        .get("reason")
        .and_then(serde_json::Value::as_str)
        .map(ToOwned::to_owned);

    Ok(HookResponse {
        decision,
        reason,
        ..HookResponse::default()
    })
}

pub(super) fn map_json_object(output: &CommandOutput) -> Result<HookResponse> {
    if output.stdout.is_empty() {
        return Ok(HookResponse::default());
    }

    if let Ok(response) = serde_json::from_str::<HookResponse>(&output.stdout) {
        return Ok(response);
    }

    let raw = serde_json::from_str::<serde_json::Value>(&output.stdout)?;
    Ok(HookResponse {
        raw: Some(raw),
        ..HookResponse::default()
    })
}

pub(super) fn map_regex_with_compiled(
    regex: &Regex,
    output: &CommandOutput,
) -> Result<HookResponse> {
    let captures = regex.captures(&output.stdout).ok_or_else(|| {
        crate::error::ClaudineError::TemplateError("regex mapper produced no match".to_string())
    })?;

    let decision = captures
        .name("decision")
        .map(|capture| {
            super::parse_decision(&serde_json::Value::String(capture.as_str().to_string()))
        })
        .unwrap_or(None);
    let reason = captures
        .name("reason")
        .map(|capture| capture.as_str().to_string());

    Ok(HookResponse {
        decision,
        reason,
        additional_context: captures
            .name("context")
            .map(|capture| capture.as_str().to_string()),
        ..HookResponse::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Build an [`ExitStatus`](std::process::ExitStatus) reporting `code` as the
    /// process exit code.
    ///
    /// The two hosts' `ExitStatusExt::from_raw` take different things: Unix takes
    /// a `wait(2)` status, which carries the exit code in the second byte, while
    /// Windows takes the exit code itself. These tests care about the code, so
    /// the encoding difference belongs here rather than at each call site.
    fn exit_status(code: i32) -> std::process::ExitStatus {
        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code << 8)
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::ExitStatusExt;
            std::process::ExitStatus::from_raw(code as u32)
        }
    }

    #[test]
    fn mapper_exit_code_deny() {
        let output = CommandOutput {
            status: exit_status(2),
            stdout: "blocked".to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(None, None, &output).unwrap();
        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("blocked"));
    }

    #[test]
    fn mapper_exit_code_allow_on_zero() {
        let output = CommandOutput {
            status: exit_status(0),
            stdout: "ok".to_string(),
            stderr: String::new(),
        };
        let mapped = apply_mapper(None, None, &output).unwrap();
        assert_eq!(mapped.decision, Some(HookDecision::Allow));
    }

    #[test]
    fn mapper_exit_code_unknown_is_none_and_warns() {
        // Exit codes other than 0/2 (crashes, command-not-found, etc.)
        // must NOT silently map to Allow. The dispatcher instead emits
        // no decision and logs a warn!, so the caller falls through to
        // the next action.
        for code in [1_i32, 127, 139, 200] {
            let output = CommandOutput {
                status: exit_status(code),
                stdout: String::new(),
                stderr: format!("boom at {code}"),
            };
            let mapped = apply_mapper(None, None, &output).unwrap();
            assert_eq!(
                mapped.decision, None,
                "unexpected exit code {code} must not produce a decision",
            );
            assert_eq!(mapped.reason.as_deref(), Some(&*format!("boom at {code}")));
        }
    }

    #[test]
    fn mapper_json_field() {
        let output = CommandOutput {
            status: exit_status(0),
            stdout: r#"{"decision":"deny","reason":"nope"}"#.to_string(),
            stderr: String::new(),
        };

        let mapped = apply_mapper(
            None,
            Some(&Mapper::JsonField {
                field: "decision".to_string(),
            }),
            &output,
        )
        .unwrap();

        assert_eq!(mapped.decision, Some(HookDecision::Deny));
        assert_eq!(mapped.reason.as_deref(), Some("nope"));
    }
}

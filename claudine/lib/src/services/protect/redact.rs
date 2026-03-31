use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::error::{ClaudineError, Result};

use super::config::McpPolicy;

/// Redaction result for MCP text payloads.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpTextRedaction {
    pub text: String,
    pub redacted: bool,
    pub blocked_instruction_payload: bool,
    pub redactions_applied: u32,
}

/// Redaction result for MCP JSON payloads.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct McpJsonRedaction {
    pub value: Value,
    pub redacted: bool,
    pub blocked_instruction_payload: bool,
    pub redactions_applied: u32,
}

pub(crate) fn redact_text_with_policy(
    text: &str,
    mcp: &McpPolicy,
    secret_patterns: &[String],
) -> Result<McpTextRedaction> {
    let mut output = text.to_string();
    let mut redactions_applied = 0u32;
    let mut blocked_instruction_payload = false;

    if mcp.block_instruction_payloads && contains_instruction_payload(text) {
        blocked_instruction_payload = true;
        output = "[instruction-payload-removed]".to_string();
    }

    for pattern in mcp.redact_patterns.iter().chain(secret_patterns.iter()) {
        let regex = Regex::new(pattern).map_err(|source| ClaudineError::ProtectRuleParse {
            pattern: pattern.clone(),
            source,
        })?;

        let count = regex.find_iter(&output).count() as u32;
        if count > 0 {
            output = regex.replace_all(&output, "[REDACTED]").to_string();
            redactions_applied = redactions_applied.saturating_add(count);
        }
    }

    Ok(McpTextRedaction {
        text: output,
        redacted: redactions_applied > 0 || blocked_instruction_payload,
        blocked_instruction_payload,
        redactions_applied,
    })
}

pub(crate) fn redact_json_with_policy(
    value: &Value,
    mcp: &McpPolicy,
    secret_patterns: &[String],
) -> Result<McpJsonRedaction> {
    let mut redactions_applied = 0u32;
    let mut blocked_instruction_payload = false;

    fn visit(
        value: &Value,
        mcp: &McpPolicy,
        secret_patterns: &[String],
        redactions_applied: &mut u32,
        blocked_instruction_payload: &mut bool,
    ) -> Result<Value> {
        match value {
            Value::String(text) => {
                let redaction = redact_text_with_policy(text, mcp, secret_patterns)?;
                if redaction.redactions_applied > 0 {
                    *redactions_applied =
                        redactions_applied.saturating_add(redaction.redactions_applied);
                }
                if redaction.blocked_instruction_payload {
                    *blocked_instruction_payload = true;
                }
                Ok(Value::String(redaction.text))
            }
            Value::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(visit(
                        item,
                        mcp,
                        secret_patterns,
                        redactions_applied,
                        blocked_instruction_payload,
                    )?);
                }
                Ok(Value::Array(out))
            }
            Value::Object(map) => {
                let mut out = serde_json::Map::new();
                for (key, item) in map {
                    out.insert(
                        key.clone(),
                        visit(
                            item,
                            mcp,
                            secret_patterns,
                            redactions_applied,
                            blocked_instruction_payload,
                        )?,
                    );
                }
                Ok(Value::Object(out))
            }
            Value::Bool(_) | Value::Null | Value::Number(_) => Ok(value.clone()),
        }
    }

    let value = visit(
        value,
        mcp,
        secret_patterns,
        &mut redactions_applied,
        &mut blocked_instruction_payload,
    )?;

    Ok(McpJsonRedaction {
        value,
        redacted: redactions_applied > 0 || blocked_instruction_payload,
        blocked_instruction_payload,
        redactions_applied,
    })
}

pub(crate) fn contains_instruction_payload(text: &str) -> bool {
    let lowered = text.to_ascii_lowercase();
    [
        "ignore previous instructions",
        "system prompt",
        "developer instructions",
        "do not reveal",
        "tool instructions",
    ]
    .iter()
    .any(|needle| lowered.contains(needle))
}

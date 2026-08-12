//! Asset-classification helpers for malformed OpenCode assets (skills,
//! commands, agents, configs) and structured error-JSON summarization.

use serde_json::Value;
use tracing::debug;

use crate::stream::logs::opencode::events::{AssetType, LogClassification, OpenCodeLogRecord};

use super::{error_context, get_http_status_description, get_provider_code_description};

pub(super) fn classify_malformed_asset(record: &OpenCodeLogRecord) -> Option<LogClassification> {
    let err = record
        .tags
        .get("err")
        .or_else(|| record.tags.get("error"))
        .map(String::as_str)
        .unwrap_or("");

    let (asset_type, error_text) = if let Some(kind) = detect_asset_suffix(err) {
        (kind, err.to_string())
    } else {
        let kind = detect_asset_suffix(&record.message)?;
        (kind, record.message.clone())
    };

    let path = match asset_type {
        AssetType::Skill => record.tags.get("skill").cloned(),
        AssetType::Command => record.tags.get("command").cloned(),
        AssetType::Agent => record.tags.get("agent").cloned(),
        AssetType::Config => record
            .tags
            .get("path")
            .or_else(|| record.tags.get("file"))
            .cloned(),
        AssetType::Unknown => None,
    };

    Some(LogClassification::MalformedAsset {
        asset_type,
        path,
        error: error_text,
    })
}

fn detect_asset_suffix(value: &str) -> Option<AssetType> {
    let lowered = value.to_lowercase();
    if lowered.contains("failed to load skill") {
        Some(AssetType::Skill)
    } else if lowered.contains("failed to load command") {
        Some(AssetType::Command)
    } else if lowered.contains("failed to load agent") {
        Some(AssetType::Agent)
    } else if lowered.contains("failed to load config") {
        Some(AssetType::Config)
    } else {
        None
    }
}

pub(super) fn summarize_error_json(record: &OpenCodeLogRecord) -> String {
    let error_tag = match error_context(record) {
        Some(tag) => tag,
        None => return String::new(),
    };

    let root: Value = match serde_json::from_str(&error_tag) {
        Ok(v) => v,
        Err(err) => {
            // If it's not valid JSON, return the raw tag (truncated if huge).
            // The 1.17.8 flat `error.error="…"` form lands here verbatim.
            debug!(%err, "opencode error tag not valid JSON; falling back to raw");
            if error_tag.len() > 500 {
                let truncated: String = error_tag.chars().take(497).collect();
                return format!("{truncated}...");
            }
            return error_tag;
        }
    };

    let envelope = root.get("error").unwrap_or(&root);

    let error_name = envelope
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let status_code = envelope
        .get("statusCode")
        .and_then(|v| v.as_u64())
        .or_else(|| {
            envelope
                .get("errors")
                .and_then(|errors| errors.get(0))
                .and_then(|e| e.get("statusCode"))
                .and_then(|v| v.as_u64())
        })
        .or_else(|| {
            envelope
                .get("lastError")
                .and_then(|e| e.get("statusCode"))
                .and_then(|v| v.as_u64())
        });

    let provider_message = extract_provider_message(envelope);

    let mut parts = Vec::new();

    if let Some(code) = status_code {
        let desc = u16::try_from(code)
            .ok()
            .map(get_http_status_description)
            .filter(|d| !d.is_empty())
            .unwrap_or_default();
        if desc.is_empty() {
            parts.push(format!("{error_name} ({code})"));
        } else {
            parts.push(format!("{error_name} ({code}: {desc})"));
        }
    } else {
        parts.push(error_name.to_string());
    }

    if let Some(msg) = &provider_message {
        parts.push(msg.clone());
    }

    parts.join(": ")
}

pub(super) fn extract_provider_message(envelope: &Value) -> Option<String> {
    if let Some(msg) = envelope.get("message").and_then(|v| v.as_str())
        && !msg.is_empty()
    {
        return Some(msg.to_string());
    }

    if let Some(body_str) = envelope.get("responseBody").and_then(|v| v.as_str()) {
        match serde_json::from_str::<Value>(body_str) {
            Ok(body) => {
                if let Some(msg) = body
                    .get("error")
                    .and_then(|e| e.get("message"))
                    .and_then(|v| v.as_str())
                    && !msg.is_empty()
                {
                    return Some(msg.to_string());
                }

                if let Some(msg) = body.get("message").and_then(|v| v.as_str())
                    && !msg.is_empty()
                {
                    return Some(msg.to_string());
                }

                let code = body
                    .get("code")
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .or_else(|| {
                        body.get("code")
                            .and_then(|v| v.as_u64())
                            .map(|v| v.to_string())
                    });

                if let Some(code_str) = code {
                    let desc = get_provider_code_description(&code_str);
                    if !desc.is_empty() {
                        return Some(format!("{code_str}: {desc}"));
                    }
                }
            }
            Err(err) => {
                debug!(%err, "opencode responseBody not valid JSON; falling back to raw");
            }
        }
        if !body_str.is_empty() {
            return Some(body_str.to_string());
        }
    }

    for source in ["errors", "lastError", "data"] {
        let entries = match source {
            "errors" => envelope.get("errors").and_then(|v| v.as_array()).cloned(),
            _ => envelope.get(source).map(|v| vec![v.clone()]),
        };
        if let Some(arr) = entries {
            for entry in &arr {
                if let Some(msg) = extract_provider_message(entry) {
                    return Some(msg);
                }
            }
        }
    }

    None
}

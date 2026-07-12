//! Session, LLM-call, permission, and uncaught-error classifiers.
//!
//! Each function here handles a specific `service=` value's lifecycle
//! records, returning a [`LogClassification`] when the record matches the
//! expected shape.

use crate::stream::logs::opencode::events::{LogClassification, OpenCodeLogRecord};

use super::llm::has_trailing_keyword;

pub(super) fn classify_session(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    if !has_trailing_keyword(record, "created") {
        return None;
    }
    let id = record.tags.get("id").cloned()?;
    let parent_id = record.tags.get("parentID").cloned();
    Some(LogClassification::SessionCreated { id, parent_id })
}

pub(super) fn classify_llm_call(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    // Only match successful stream starts, not errors ("stream error" is handled
    // by the existing classify_llm_failure path).
    if !has_trailing_keyword(record, "stream") {
        return None;
    }
    let provider_id = record.tags.get("providerID").cloned()?;
    let model_id = record.tags.get("modelID").cloned()?;
    let mode = super::llm::tag_value_stripped(record, "mode", "stream").unwrap_or_default();
    Some(LogClassification::LlmCall {
        provider_id,
        model_id,
        mode,
        is_stream: true,
    })
}

pub(super) fn classify_session_prompt(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    let session_id = record.tags.get("session.id").cloned()?;

    // Check the longer keyword first; "exiting loop" ends with " loop"
    // and would otherwise match the shorter StepLoop branch.
    if has_trailing_keyword(record, "exiting loop") {
        return Some(LogClassification::StepExit { session_id });
    }

    if has_trailing_keyword(record, "loop") {
        let step = record
            .tags
            .get("step")
            .and_then(|s| s.parse::<u32>().ok())
            .unwrap_or(0);
        return Some(LogClassification::StepLoop { session_id, step });
    }

    None
}

pub(super) fn classify_permission(
    record: &OpenCodeLogRecord,
    _message: &str,
) -> Option<LogClassification> {
    if !has_trailing_keyword(record, "evaluated") {
        return None;
    }
    let permission = record.tags.get("permission").cloned().unwrap_or_default();
    let pattern = record.tags.get("pattern").cloned().unwrap_or_default();
    let action = record.tags.get("action").cloned().unwrap_or_default();
    Some(LogClassification::PermissionEvaluated {
        permission,
        pattern,
        action,
    })
}

pub(super) fn looks_like_uncaught_error(record: &OpenCodeLogRecord) -> bool {
    if let Some(name) = record.tags.get("name") {
        let head = name.split_whitespace().next().unwrap_or("");
        if matches!(
            head,
            "TypeError" | "ReferenceError" | "SyntaxError" | "RangeError" | "Error"
        ) {
            return true;
        }
    }
    if record.message.split_whitespace().any(|tok| tok == "fatal") {
        return true;
    }
    record
        .tags
        .values()
        .any(|value| value.split_whitespace().any(|tok| tok == "fatal"))
}

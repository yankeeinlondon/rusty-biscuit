use std::collections::HashMap;

use crate::events::AgenticEvent;

use super::types::{DailyToolStat, DerivedMetrics, SessionInfo, ToolActionClass};

/// Lightweight event slice for recovery-rate computation.
#[derive(Debug, Clone)]
pub(crate) struct RecoveryEvent {
    pub session_key: String,
    pub tool_name: Option<String>,
    pub event: AgenticEvent,
}

/// Normalize provider-specific tool names to canonical forms.
///
/// Different providers use different names for equivalent tools (e.g. OpenCode
/// uses `grep_search` for what Claude calls `Grep`). This merges them into a
/// single bucket for reporting.
pub fn normalize_tool_name(tool_name: &str) -> &str {
    match tool_name.trim().to_ascii_lowercase().as_str() {
        "grep_search" => "Grep",
        "list_dir" => "Glob",
        "file_search" => "Glob",
        "multi_edit" | "multiedit" => "Edit",
        "todo_write" | "todowrite" => "TodoWrite",
        "web_search" | "websearch" => "WebSearch",
        "web_fetch" | "webfetch" => "WebFetch",
        "taskcreate" => "TaskCreate",
        "taskupdate" => "TaskUpdate",
        _ => tool_name,
    }
}

/// Classify a tool into a coarse activity bucket.
pub fn classify_tool(tool_name: &str) -> ToolActionClass {
    match normalize_tool_name(tool_name)
        .to_ascii_lowercase()
        .as_str()
    {
        "read" | "grep" | "glob" | "websearch" | "webfetch" | "ls" | "cat" | "find" | "skill" => {
            ToolActionClass::Research
        }
        "edit" | "write" | "bash" | "shell" | "patch" | "todowrite" => ToolActionClass::Action,
        "agent" | "task" | "taskcreate" | "taskupdate" | "subagent" => ToolActionClass::Delegation,
        _ => ToolActionClass::Other,
    }
}

fn ratio(numerator: u64, denominator: u64) -> Option<f64> {
    if denominator == 0 {
        return None;
    }

    Some(numerator as f64 / denominator as f64)
}

/// Compute high-level report metrics from already aggregated inputs.
pub(crate) fn summarize_metrics(
    total_turns: u64,
    total_tool_calls: u64,
    total_tool_errors: u64,
    total_compactions: u64,
    sessions: &[SessionInfo],
    tools: &[DailyToolStat],
    recovery_events: &[RecoveryEvent],
) -> DerivedMetrics {
    let research_calls = tools
        .iter()
        .filter(|tool| tool.classification == ToolActionClass::Research)
        .map(|tool| tool.call_count)
        .sum();
    let action_calls = tools
        .iter()
        .filter(|tool| tool.classification == ToolActionClass::Action)
        .map(|tool| tool.call_count)
        .sum();
    let delegation_calls = tools
        .iter()
        .filter(|tool| tool.classification == ToolActionClass::Delegation)
        .map(|tool| tool.call_count)
        .sum();

    let total_duration_seconds: i64 = sessions
        .iter()
        .map(|session| session.duration_seconds)
        .sum();
    let session_efficiency = if total_duration_seconds <= 0 {
        None
    } else {
        Some(total_turns as f64 / (total_duration_seconds as f64 / 3600.0))
    };

    DerivedMetrics {
        autonomy_ratio: ratio(total_tool_calls, total_turns),
        research_vs_action_ratio: ratio(research_calls, action_calls),
        error_recovery_rate: error_recovery_rate(total_tool_errors, recovery_events),
        delegation_ratio: ratio(delegation_calls, total_tool_calls),
        session_efficiency,
        context_pressure_index: ratio(total_compactions, sessions.len() as u64),
    }
}

fn error_recovery_rate(total_tool_errors: u64, events: &[RecoveryEvent]) -> Option<f64> {
    if total_tool_errors == 0 {
        return None;
    }

    // Track open error counts per (session, tool) key and count each individual
    // recovery event rather than collapsing to unique (session, tool) pairs.
    let mut open_errors: HashMap<(String, Option<String>), u64> = HashMap::new();
    let mut recovered_count: u64 = 0;

    for event in events {
        let key = (event.session_key.clone(), event.tool_name.clone());
        match event.event {
            AgenticEvent::ToolError => {
                *open_errors.entry(key).or_default() += 1;
            }
            AgenticEvent::AfterTool => {
                if let Some(count) = open_errors.get_mut(&key)
                    && *count > 0
                {
                    *count -= 1;
                    recovered_count += 1;
                }
            }
            _ => {}
        }
    }

    ratio(recovered_count, total_tool_errors)
}

#[cfg(test)]
mod tests {
    use chrono::Utc;

    use super::*;
    use crate::events::Provider;
    use crate::reporting::types::SessionInfo;

    #[test]
    fn classifies_core_tools() {
        assert_eq!(classify_tool("Read"), ToolActionClass::Research);
        assert_eq!(classify_tool("Edit"), ToolActionClass::Action);
        assert_eq!(classify_tool("Agent"), ToolActionClass::Delegation);
    }

    #[test]
    fn computes_recovery_rate() {
        let events = vec![
            RecoveryEvent {
                session_key: "claude:s1".to_string(),
                tool_name: Some("Bash".to_string()),
                event: AgenticEvent::ToolError,
            },
            RecoveryEvent {
                session_key: "claude:s1".to_string(),
                tool_name: Some("Bash".to_string()),
                event: AgenticEvent::AfterTool,
            },
        ];

        let metrics = summarize_metrics(1, 2, 1, 0, &[], &[], &events);
        assert_eq!(metrics.error_recovery_rate, Some(1.0));
    }

    #[test]
    fn computes_session_efficiency() {
        let sessions = vec![SessionInfo {
            session_key: "claude:s1".to_string(),
            session_id: Some("s1".to_string()),
            provider: Provider::Claude,
            started_at: Utc::now(),
            ended_at: Utc::now(),
            duration_seconds: 1800,
            cwd: None,
            repo_name: None,
            repo_org: None,
            branch: None,
            package_area: None,
            package: None,
            model: None,
            permission_mode: None,
            hostname: None,
            primary_language: None,
            event_count: 0,
            turn_count: 0,
            tool_call_count: 0,
            tool_error_count: 0,
            turn_error_count: 0,
            subagent_count: 0,
            total_input_tokens: 0,
            total_output_tokens: 0,
            total_tokens: 0,
            total_cache_read_tokens: 0,
            total_cost_usd: 0.0,
        }];

        let metrics = summarize_metrics(3, 0, 0, 0, &sessions, &[], &[]);
        assert_eq!(metrics.session_efficiency, Some(6.0));
    }
}

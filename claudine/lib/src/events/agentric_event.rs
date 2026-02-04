use serde::{Deserialize, Serialize};
use std::fmt;

/// Normalized event names across all supported agentic CLI providers.
///
/// Each variant represents a lifecycle moment that at least 2 of the 5
/// supported providers expose. Provider adapters map their native events
/// to the appropriate variant.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AgenticEvent {
    /// Agent session started, resumed, or cleared.
    SessionStart,
    /// Agent session ended or terminated.
    SessionEnd,
    /// User prompt submitted, before agent processes it.
    BeforePrompt,
    /// Tool call created, before execution begins.
    BeforeTool,
    /// Tool call completed successfully.
    AfterTool,
    /// Tool call failed.
    ToolError,
    /// Agent is requesting user permission.
    PermissionRequest,
    /// Agent turn (request/response cycle) completed.
    TurnComplete,
    /// Agent turn failed with an error.
    TurnError,
    /// Sub-agent spawned.
    SubagentStart,
    /// Sub-agent finished.
    SubagentStop,
    /// Before sending prompt to the model.
    BeforeModel,
    /// After receiving response from the model.
    AfterModel,
    /// Before context compaction/summarization.
    BeforeCompact,
    /// Provider-specific notification.
    Notification,
}

impl fmt::Display for AgenticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = serde_json::to_value(self)
            .ok()
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_else(|| format!("{self:?}"));
        f.write_str(&s)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_json() {
        let event = AgenticEvent::BeforeTool;
        let json = serde_json::to_value(&event).unwrap();
        assert_eq!(json, serde_json::json!("before_tool"));
        let back: AgenticEvent = serde_json::from_value(json).unwrap();
        assert_eq!(back, AgenticEvent::BeforeTool);
    }

    #[test]
    fn all_variants_serialize_snake_case() {
        let cases = vec![
            (AgenticEvent::SessionStart, "session_start"),
            (AgenticEvent::SessionEnd, "session_end"),
            (AgenticEvent::BeforePrompt, "before_prompt"),
            (AgenticEvent::BeforeTool, "before_tool"),
            (AgenticEvent::AfterTool, "after_tool"),
            (AgenticEvent::ToolError, "tool_error"),
            (AgenticEvent::PermissionRequest, "permission_request"),
            (AgenticEvent::TurnComplete, "turn_complete"),
            (AgenticEvent::TurnError, "turn_error"),
            (AgenticEvent::SubagentStart, "subagent_start"),
            (AgenticEvent::SubagentStop, "subagent_stop"),
            (AgenticEvent::BeforeModel, "before_model"),
            (AgenticEvent::AfterModel, "after_model"),
            (AgenticEvent::BeforeCompact, "before_compact"),
            (AgenticEvent::Notification, "notification"),
        ];
        for (variant, expected) in cases {
            let json = serde_json::to_value(&variant).unwrap();
            assert_eq!(json.as_str().unwrap(), expected, "Failed for {variant:?}");
        }
    }

    #[test]
    fn display_uses_snake_case() {
        assert_eq!(AgenticEvent::BeforeTool.to_string(), "before_tool");
        assert_eq!(AgenticEvent::SessionStart.to_string(), "session_start");
    }

    #[test]
    fn can_use_as_hashmap_key() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(AgenticEvent::BeforeTool, "test");
        assert_eq!(map.get(&AgenticEvent::BeforeTool), Some(&"test"));
    }
}

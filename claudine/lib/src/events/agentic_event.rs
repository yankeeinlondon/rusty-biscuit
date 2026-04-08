use serde::{Deserialize, Serialize};
use std::fmt;

use super::provider::PROVIDERS_DISPLAY_ORDER;

/// Normalized event names across all supported agentic CLI providers.
///
/// Each variant represents a lifecycle moment that at least 2 of the 8
/// supported providers expose. Provider adapters map their native events
/// to the appropriate variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
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
    /// Agent is asking user a clarifying question (human-in-the-loop).
    HumanInTheLoop,
}

impl AgenticEvent {
    /// Returns all event variants in display order.
    pub const ALL: [AgenticEvent; 16] = [
        AgenticEvent::SessionStart,
        AgenticEvent::SessionEnd,
        AgenticEvent::BeforePrompt,
        AgenticEvent::BeforeTool,
        AgenticEvent::AfterTool,
        AgenticEvent::ToolError,
        AgenticEvent::PermissionRequest,
        AgenticEvent::HumanInTheLoop,
        AgenticEvent::TurnComplete,
        AgenticEvent::TurnError,
        AgenticEvent::SubagentStart,
        AgenticEvent::SubagentStop,
        AgenticEvent::BeforeModel,
        AgenticEvent::AfterModel,
        AgenticEvent::BeforeCompact,
        AgenticEvent::Notification,
    ];

    /// Returns the canonical snake_case identifier used in config files.
    pub const fn as_slug(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "session_start",
            AgenticEvent::SessionEnd => "session_end",
            AgenticEvent::BeforePrompt => "before_prompt",
            AgenticEvent::BeforeTool => "before_tool",
            AgenticEvent::AfterTool => "after_tool",
            AgenticEvent::ToolError => "tool_error",
            AgenticEvent::PermissionRequest => "permission_request",
            AgenticEvent::TurnComplete => "turn_complete",
            AgenticEvent::TurnError => "turn_error",
            AgenticEvent::SubagentStart => "subagent_start",
            AgenticEvent::SubagentStop => "subagent_stop",
            AgenticEvent::BeforeModel => "before_model",
            AgenticEvent::AfterModel => "after_model",
            AgenticEvent::BeforeCompact => "before_compact",
            AgenticEvent::Notification => "notification",
            AgenticEvent::HumanInTheLoop => "human_in_the_loop",
        }
    }

    /// Returns a human-readable display name for terminal presentation.
    pub const fn human_name(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "Session Start",
            AgenticEvent::SessionEnd => "Session End",
            AgenticEvent::BeforePrompt => "Before Prompt",
            AgenticEvent::BeforeTool => "Before Tool",
            AgenticEvent::AfterTool => "After Tool",
            AgenticEvent::ToolError => "Tool Error",
            AgenticEvent::PermissionRequest => "Permission Request",
            AgenticEvent::TurnComplete => "Turn Complete",
            AgenticEvent::TurnError => "Turn Error",
            AgenticEvent::SubagentStart => "Subagent Start",
            AgenticEvent::SubagentStop => "Subagent Stop",
            AgenticEvent::BeforeModel => "Before Model",
            AgenticEvent::AfterModel => "After Model",
            AgenticEvent::BeforeCompact => "Before Compact",
            AgenticEvent::Notification => "Notification",
            AgenticEvent::HumanInTheLoop => "HITL (human in the loop)",
        }
    }

    /// Returns a PascalCase display label for terminal presentation.
    pub const fn as_pascal_case(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "SessionStart",
            AgenticEvent::SessionEnd => "SessionEnd",
            AgenticEvent::BeforePrompt => "BeforePrompt",
            AgenticEvent::BeforeTool => "BeforeTool",
            AgenticEvent::AfterTool => "AfterTool",
            AgenticEvent::ToolError => "ToolError",
            AgenticEvent::PermissionRequest => "PermissionRequest",
            AgenticEvent::TurnComplete => "TurnComplete",
            AgenticEvent::TurnError => "TurnError",
            AgenticEvent::SubagentStart => "SubagentStart",
            AgenticEvent::SubagentStop => "SubagentStop",
            AgenticEvent::BeforeModel => "BeforeModel",
            AgenticEvent::AfterModel => "AfterModel",
            AgenticEvent::BeforeCompact => "BeforeCompact",
            AgenticEvent::Notification => "Notification",
            AgenticEvent::HumanInTheLoop => "HumanInTheLoop",
        }
    }

    /// Parse a canonical snake_case event name.
    pub fn from_slug(name: &str) -> Option<Self> {
        match name {
            "session_start" => Some(AgenticEvent::SessionStart),
            "session_end" => Some(AgenticEvent::SessionEnd),
            "before_prompt" => Some(AgenticEvent::BeforePrompt),
            "before_tool" => Some(AgenticEvent::BeforeTool),
            "after_tool" => Some(AgenticEvent::AfterTool),
            "tool_error" => Some(AgenticEvent::ToolError),
            "permission_request" => Some(AgenticEvent::PermissionRequest),
            "turn_complete" => Some(AgenticEvent::TurnComplete),
            "turn_error" => Some(AgenticEvent::TurnError),
            "subagent_start" => Some(AgenticEvent::SubagentStart),
            "subagent_stop" => Some(AgenticEvent::SubagentStop),
            "before_model" => Some(AgenticEvent::BeforeModel),
            "after_model" => Some(AgenticEvent::AfterModel),
            "before_compact" => Some(AgenticEvent::BeforeCompact),
            "notification" => Some(AgenticEvent::Notification),
            "human_in_the_loop" => Some(AgenticEvent::HumanInTheLoop),
            _ => None,
        }
    }

    /// Parse an event from canonical name or provider-native aliases.
    pub fn parse_name_or_alias(input: &str) -> Option<Self> {
        let normalized = normalize_event_identifier(input);
        if let Some(event) = Self::from_slug(&normalized) {
            return Some(event);
        }

        for provider in PROVIDERS_DISPLAY_ORDER {
            if let Some(event) = provider.event_from_shared_native_name(input) {
                return Some(event);
            }
            if let Some(event) = provider.event_from_shared_native_name(&normalized) {
                return Some(event);
            }
        }

        for provider in PROVIDERS_DISPLAY_ORDER {
            for event in AgenticEvent::ALL {
                let Some(native_name) = provider.native_event_name(&event) else {
                    continue;
                };
                if native_name.is_empty() {
                    continue;
                }
                if native_name.eq_ignore_ascii_case(input) {
                    return Some(event);
                }
                if normalize_event_identifier(native_name) == normalized {
                    return Some(event);
                }
            }
        }

        None
    }

    /// Returns a short abbreviation suitable for table column headers.
    ///
    /// These are kept to 4-5 characters to conserve horizontal space.
    pub fn abbrev(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "🎉",
            AgenticEvent::SessionEnd => "🛑",
            AgenticEvent::BeforePrompt => "🪄",
            AgenticEvent::BeforeTool => "←🔧",
            AgenticEvent::AfterTool => "🔧→",
            AgenticEvent::ToolError => "⚒️",
            AgenticEvent::PermissionRequest => "🪪",
            AgenticEvent::HumanInTheLoop => "🙋",
            AgenticEvent::TurnComplete => "⏎",
            AgenticEvent::TurnError => "⧳",
            AgenticEvent::SubagentStart => "🧑‍💼🎉",
            AgenticEvent::SubagentStop => "🧑‍💼🛑",
            AgenticEvent::BeforeModel => "←𝌭",
            AgenticEvent::AfterModel => "𝌭→",
            AgenticEvent::BeforeCompact => "🗜️",
            AgenticEvent::Notification => "💬",
        }
    }

    /// Returns a human-readable description of the event.
    pub fn description(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "Agent session started, resumed, or cleared",
            AgenticEvent::SessionEnd => "Agent session ended or terminated",
            AgenticEvent::BeforePrompt => "User prompt submitted, before agent processes it",
            AgenticEvent::BeforeTool => "Tool call created, before execution begins",
            AgenticEvent::AfterTool => "Tool call completed successfully",
            AgenticEvent::ToolError => "Tool call failed with an error",
            AgenticEvent::PermissionRequest => "Agent is requesting user permission",
            AgenticEvent::HumanInTheLoop => "Agent is asking user a clarifying question",
            AgenticEvent::TurnComplete => "Agent turn (request/response cycle) completed",
            AgenticEvent::TurnError => "Agent turn failed with an error",
            AgenticEvent::SubagentStart => "Sub-agent spawned",
            AgenticEvent::SubagentStop => "Sub-agent finished",
            AgenticEvent::BeforeModel => "Before sending prompt to the model",
            AgenticEvent::AfterModel => "After receiving response from the model",
            AgenticEvent::BeforeCompact => "Before context compaction/summarization",
            AgenticEvent::Notification => "Provider-specific notification or status update",
        }
    }

    /// Returns the schema of data provided to hooks when this event fires.
    ///
    /// Describes what fields are available in the event payload.
    pub fn response_schema(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "session_id, cwd",
            AgenticEvent::SessionEnd => "session_id",
            AgenticEvent::BeforePrompt => "prompt, session_id",
            AgenticEvent::BeforeTool => "tool_name, tool_input",
            AgenticEvent::AfterTool => "tool_name, tool_input, tool_response",
            AgenticEvent::ToolError => "tool_name, tool_input, error",
            AgenticEvent::PermissionRequest => "tool_name, tool_input",
            AgenticEvent::HumanInTheLoop => "questions, options",
            AgenticEvent::TurnComplete => "session_id",
            AgenticEvent::TurnError => "error, session_id",
            AgenticEvent::SubagentStart => "agent_type, session_id",
            AgenticEvent::SubagentStop => "agent_type, session_id",
            AgenticEvent::BeforeModel => "prompt (varies by provider)",
            AgenticEvent::AfterModel => "response (varies by provider)",
            AgenticEvent::BeforeCompact => "session_id",
            AgenticEvent::Notification => "notification_type, notification_message",
        }
    }

    /// Returns what the hook can return to influence agent behavior.
    ///
    /// Most hooks are fire-and-forget; blocking hooks can modify flow.
    pub fn return_schema(&self) -> &'static str {
        match self {
            AgenticEvent::SessionStart => "(none)",
            AgenticEvent::SessionEnd => "(none)",
            AgenticEvent::BeforePrompt => "modified prompt (blocking)",
            AgenticEvent::BeforeTool => "allow/deny, modified input (blocking)",
            AgenticEvent::AfterTool => "(none)",
            AgenticEvent::ToolError => "(none)",
            AgenticEvent::PermissionRequest => "allow/deny (blocking)",
            AgenticEvent::HumanInTheLoop => "answers (blocking)",
            AgenticEvent::TurnComplete => "(none)",
            AgenticEvent::TurnError => "(none)",
            AgenticEvent::SubagentStart => "(none)",
            AgenticEvent::SubagentStop => "(none)",
            AgenticEvent::BeforeModel => "modified prompt (blocking)",
            AgenticEvent::AfterModel => "(none)",
            AgenticEvent::BeforeCompact => "(none)",
            AgenticEvent::Notification => "(none)",
        }
    }
}

fn normalize_event_identifier(input: &str) -> String {
    let mut normalized = String::new();
    let mut prev_was_lower = false;

    for c in input.chars() {
        if c == '-' || c == '_' {
            if !normalized.ends_with('_') {
                normalized.push('_');
            }
            prev_was_lower = false;
        } else if c.is_uppercase() {
            if prev_was_lower && !normalized.ends_with('_') {
                normalized.push('_');
            }
            normalized.push(c.to_ascii_lowercase());
            prev_was_lower = false;
        } else {
            normalized.push(c.to_ascii_lowercase());
            prev_was_lower = c.is_lowercase();
        }
    }

    normalized
}

impl fmt::Display for AgenticEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_slug())
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
            (AgenticEvent::HumanInTheLoop, "human_in_the_loop"),
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
    fn as_pascal_case_returns_pascal_case() {
        assert_eq!(AgenticEvent::BeforeTool.as_pascal_case(), "BeforeTool");
        assert_eq!(AgenticEvent::SessionStart.as_pascal_case(), "SessionStart");
    }

    #[test]
    fn can_use_as_hashmap_key() {
        use std::collections::HashMap;
        let mut map = HashMap::new();
        map.insert(AgenticEvent::BeforeTool, "test");
        assert_eq!(map.get(&AgenticEvent::BeforeTool), Some(&"test"));
    }

    #[test]
    fn all_contains_all_variants() {
        // Verify ALL array has exactly 16 elements (all variants)
        assert_eq!(AgenticEvent::ALL.len(), 16);

        // Verify each variant is present
        let all_set: std::collections::HashSet<_> = AgenticEvent::ALL.iter().collect();
        assert!(all_set.contains(&AgenticEvent::SessionStart));
        assert!(all_set.contains(&AgenticEvent::SessionEnd));
        assert!(all_set.contains(&AgenticEvent::BeforePrompt));
        assert!(all_set.contains(&AgenticEvent::BeforeTool));
        assert!(all_set.contains(&AgenticEvent::AfterTool));
        assert!(all_set.contains(&AgenticEvent::ToolError));
        assert!(all_set.contains(&AgenticEvent::PermissionRequest));
        assert!(all_set.contains(&AgenticEvent::HumanInTheLoop));
        assert!(all_set.contains(&AgenticEvent::TurnComplete));
        assert!(all_set.contains(&AgenticEvent::TurnError));
        assert!(all_set.contains(&AgenticEvent::SubagentStart));
        assert!(all_set.contains(&AgenticEvent::SubagentStop));
        assert!(all_set.contains(&AgenticEvent::BeforeModel));
        assert!(all_set.contains(&AgenticEvent::AfterModel));
        assert!(all_set.contains(&AgenticEvent::BeforeCompact));
        assert!(all_set.contains(&AgenticEvent::Notification));
    }

    #[test]
    fn abbrev_returns_short_strings() {
        // Abbreviations should be short for table column headers (1-4 grapheme clusters)
        for event in AgenticEvent::ALL {
            let abbr = event.abbrev();
            let char_count = abbr.chars().count();
            assert!(
                char_count >= 1 && char_count <= 6,
                "Event {:?} abbreviation '{}' should be 1-6 chars (got {})",
                event,
                abbr,
                char_count
            );
        }
    }

    #[test]
    fn from_slug_parses_canonical_names() {
        assert_eq!(
            AgenticEvent::from_slug("turn_complete"),
            Some(AgenticEvent::TurnComplete)
        );
        assert_eq!(
            AgenticEvent::from_slug("human_in_the_loop"),
            Some(AgenticEvent::HumanInTheLoop)
        );
        assert_eq!(AgenticEvent::from_slug("unknown"), None);
    }

    #[test]
    fn parse_name_or_alias_parses_native_names() {
        assert_eq!(
            AgenticEvent::parse_name_or_alias("Stop"),
            Some(AgenticEvent::TurnComplete)
        );
        assert_eq!(
            AgenticEvent::parse_name_or_alias("PreToolUse"),
            Some(AgenticEvent::BeforeTool)
        );
        assert_eq!(
            AgenticEvent::parse_name_or_alias("permission.ask"),
            Some(AgenticEvent::PermissionRequest)
        );
    }

    #[test]
    fn parse_name_or_alias_is_case_and_separator_tolerant() {
        assert_eq!(
            AgenticEvent::parse_name_or_alias("TURN_COMPLETE"),
            Some(AgenticEvent::TurnComplete)
        );
        assert_eq!(
            AgenticEvent::parse_name_or_alias("turn-complete"),
            Some(AgenticEvent::TurnComplete)
        );
        assert_eq!(
            AgenticEvent::parse_name_or_alias("HumanInTheLoop"),
            Some(AgenticEvent::HumanInTheLoop)
        );
    }
}

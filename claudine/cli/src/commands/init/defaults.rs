//! Default configuration values and mappings for the init wizard.

use claudine::events::AgenticEvent;

/// Returns the recommended sound effect for an event.
///
/// These are curated defaults based on the semantic meaning of each event.
pub fn recommended_sound(event: &AgenticEvent) -> &'static str {
    match event {
        AgenticEvent::SessionStart => "high-up",
        AgenticEvent::SessionEnd => "phase-jump-1",
        AgenticEvent::BeforePrompt => "quick-blip-1",
        AgenticEvent::BeforeTool => "dit-hit-1",
        AgenticEvent::AfterTool => "dit-hit-2",
        AgenticEvent::ToolError => "sad-trombone",
        AgenticEvent::PermissionRequest => "doorbell",
        AgenticEvent::TurnComplete => "electronic-hit-fx1",
        AgenticEvent::TurnError => "error",
        AgenticEvent::SubagentStart => "power-up",
        AgenticEvent::SubagentStop => "power-down",
        AgenticEvent::BeforeModel => "sending",
        AgenticEvent::AfterModel => "receiving",
        AgenticEvent::BeforeCompact => "swoosh",
        AgenticEvent::Notification => "notification",
    }
}

/// Returns the default speak template for an event.
///
/// Templates support `{placeholder}` interpolation from the event's metadata.
pub fn default_speak_template(event: &AgenticEvent) -> &'static str {
    match event {
        AgenticEvent::SessionStart => "Session started",
        AgenticEvent::SessionEnd => "Session ended",
        AgenticEvent::BeforePrompt => "Processing prompt",
        AgenticEvent::BeforeTool => "Running {tool_name}",
        AgenticEvent::AfterTool => "{tool_name} completed",
        AgenticEvent::ToolError => "Tool {tool_name} failed",
        AgenticEvent::PermissionRequest => "Permission needed",
        AgenticEvent::TurnComplete => "Turn complete",
        AgenticEvent::TurnError => "Turn failed with error",
        AgenticEvent::SubagentStart => "Subagent started",
        AgenticEvent::SubagentStop => "Subagent finished",
        AgenticEvent::BeforeModel => "Sending to model",
        AgenticEvent::AfterModel => "Response received",
        AgenticEvent::BeforeCompact => "Compacting context",
        AgenticEvent::Notification => "Notification received",
    }
}

/// Returns a human-readable description for an event.
pub fn event_description(event: &AgenticEvent) -> &'static str {
    match event {
        AgenticEvent::SessionStart => "When an agent session begins, resumes, or clears",
        AgenticEvent::SessionEnd => "When an agent session terminates",
        AgenticEvent::BeforePrompt => "Before the agent processes a user prompt",
        AgenticEvent::BeforeTool => "Before a tool call executes",
        AgenticEvent::AfterTool => "After a tool call completes successfully",
        AgenticEvent::ToolError => "When a tool call fails",
        AgenticEvent::PermissionRequest => "When the agent requests user permission",
        AgenticEvent::TurnComplete => "When a request/response cycle completes",
        AgenticEvent::TurnError => "When a turn fails with an error",
        AgenticEvent::SubagentStart => "When a sub-agent is spawned",
        AgenticEvent::SubagentStop => "When a sub-agent finishes",
        AgenticEvent::BeforeModel => "Before sending a prompt to the LLM",
        AgenticEvent::AfterModel => "After receiving a response from the LLM",
        AgenticEvent::BeforeCompact => "Before context compaction/summarization",
        AgenticEvent::Notification => "Provider-specific notification",
    }
}

/// Returns the list of all events in a logical grouping order for display.
pub fn all_events_ordered() -> Vec<AgenticEvent> {
    vec![
        // Session lifecycle
        AgenticEvent::SessionStart,
        AgenticEvent::SessionEnd,
        // Turn lifecycle
        AgenticEvent::BeforePrompt,
        AgenticEvent::TurnComplete,
        AgenticEvent::TurnError,
        // Tool lifecycle
        AgenticEvent::BeforeTool,
        AgenticEvent::AfterTool,
        AgenticEvent::ToolError,
        // Model lifecycle
        AgenticEvent::BeforeModel,
        AgenticEvent::AfterModel,
        // Permission
        AgenticEvent::PermissionRequest,
        // Subagents
        AgenticEvent::SubagentStart,
        AgenticEvent::SubagentStop,
        // Other
        AgenticEvent::BeforeCompact,
        AgenticEvent::Notification,
    ]
}

/// Returns events that are commonly configured (recommended defaults).
pub fn recommended_events() -> Vec<AgenticEvent> {
    vec![
        AgenticEvent::SessionStart,
        AgenticEvent::TurnComplete,
        AgenticEvent::ToolError,
        AgenticEvent::PermissionRequest,
    ]
}

/// Available TTS providers with display names.
pub fn tts_providers() -> Vec<(&'static str, &'static str)> {
    vec![
        ("say", "macOS Say (built-in)"),
        ("espeak", "eSpeak (cross-platform)"),
        ("elevenlabs", "ElevenLabs (cloud, high quality)"),
        ("kokoro", "Kokoro (local, fast)"),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_events_have_sounds() {
        for event in all_events_ordered() {
            let sound = recommended_sound(&event);
            assert!(!sound.is_empty(), "Missing sound for {event:?}");
        }
    }

    #[test]
    fn all_events_have_templates() {
        for event in all_events_ordered() {
            let template = default_speak_template(&event);
            assert!(!template.is_empty(), "Missing template for {event:?}");
        }
    }

    #[test]
    fn all_events_have_descriptions() {
        for event in all_events_ordered() {
            let desc = event_description(&event);
            assert!(!desc.is_empty(), "Missing description for {event:?}");
        }
    }

    #[test]
    fn ordered_events_contains_all() {
        let ordered = all_events_ordered();
        assert_eq!(
            ordered.len(),
            15,
            "Should contain all 15 AgenticEvent variants"
        );
    }

    #[test]
    fn recommended_events_subset() {
        let recommended = recommended_events();
        let all = all_events_ordered();
        for event in recommended {
            assert!(
                all.contains(&event),
                "{event:?} should be in all_events_ordered"
            );
        }
    }
}

use super::AgenticEvent;
use crate::provider::{PROVIDERS_DISPLAY_ORDER, Provider};

/// A CLI-selectable TTS provider option.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TtsProviderOption {
    pub id: &'static str,
    pub display_name: &'static str,
}

/// Event ordering used by init-style UIs.
pub const INIT_EVENT_DISPLAY_ORDER: [AgenticEvent; 16] = [
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
    // Permission and user interaction
    AgenticEvent::PermissionRequest,
    AgenticEvent::HumanInTheLoop,
    // Subagents
    AgenticEvent::SubagentStart,
    AgenticEvent::SubagentStop,
    // Other
    AgenticEvent::BeforeCompact,
    AgenticEvent::Notification,
];

/// Events pre-selected by the init wizard.
pub const INIT_RECOMMENDED_EVENTS: [AgenticEvent; 4] = [
    AgenticEvent::SessionStart,
    AgenticEvent::TurnComplete,
    AgenticEvent::ToolError,
    AgenticEvent::PermissionRequest,
];

/// TTS provider options shown by init-style UIs.
pub const INIT_TTS_PROVIDERS: [TtsProviderOption; 4] = [
    TtsProviderOption {
        id: "say",
        display_name: "macOS Say (built-in)",
    },
    TtsProviderOption {
        id: "espeak",
        display_name: "eSpeak (cross-platform)",
    },
    TtsProviderOption {
        id: "elevenlabs",
        display_name: "ElevenLabs (cloud, high quality)",
    },
    TtsProviderOption {
        id: "kokoro",
        display_name: "Kokoro (local, fast)",
    },
];

/// Returns the recommended sound effect for an event.
pub fn recommended_sound(event: &AgenticEvent) -> &'static str {
    match event {
        AgenticEvent::SessionStart => "high-up",
        AgenticEvent::SessionEnd => "phase-jump-1",
        AgenticEvent::BeforePrompt => "dit-hit-1",
        AgenticEvent::BeforeTool => "dit-hit-2",
        AgenticEvent::AfterTool => "electronic-hit-fx2",
        AgenticEvent::ToolError => "sad-trombone",
        AgenticEvent::PermissionRequest => "doorbell",
        AgenticEvent::HumanInTheLoop => "doorbell-2",
        AgenticEvent::TurnComplete => "electronic-hit-fx1",
        AgenticEvent::TurnError => "space-alarm",
        AgenticEvent::SubagentStart => "phase-jump-2",
        AgenticEvent::SubagentStop => "high-down",
        AgenticEvent::BeforeModel => "two-tone",
        AgenticEvent::AfterModel => "phase-jump-3",
        AgenticEvent::BeforeCompact => "air-woosh",
        AgenticEvent::Notification => "doorbell-2",
    }
}

/// Returns the default speak template for an event.
pub fn default_speak_template(event: &AgenticEvent) -> &'static str {
    match event {
        AgenticEvent::SessionStart => "Session started",
        AgenticEvent::SessionEnd => "Session ended",
        AgenticEvent::BeforePrompt => "Processing prompt",
        AgenticEvent::BeforeTool => "Running {{tool_name}}",
        AgenticEvent::AfterTool => "{{tool_name}} completed",
        AgenticEvent::ToolError => "Tool {{tool_name}} failed",
        AgenticEvent::PermissionRequest => "Permission needed",
        AgenticEvent::HumanInTheLoop => "Question for you",
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

/// Providers included in quick-start defaults.
///
/// This stays data-driven by selecting providers that can register at least
/// one of the init recommended events via native hooks.
pub fn quick_start_supported_providers() -> Vec<Provider> {
    PROVIDERS_DISPLAY_ORDER
        .into_iter()
        .filter(|provider| {
            INIT_RECOMMENDED_EVENTS
                .iter()
                .any(|event| provider.supports_event_via_hook(event))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::HashSet;

    use super::*;

    #[test]
    fn init_event_order_contains_all_events() {
        let ordered: HashSet<_> = INIT_EVENT_DISPLAY_ORDER.into_iter().collect();
        let all: HashSet<_> = AgenticEvent::ALL.into_iter().collect();
        assert_eq!(ordered, all);
    }

    #[test]
    fn init_recommended_events_are_subset_of_display_order() {
        for event in INIT_RECOMMENDED_EVENTS {
            assert!(INIT_EVENT_DISPLAY_ORDER.contains(&event));
        }
    }

    #[test]
    fn all_events_have_sound_and_template() {
        for event in INIT_EVENT_DISPLAY_ORDER {
            assert!(!recommended_sound(&event).is_empty());
            assert!(!default_speak_template(&event).is_empty());
        }
    }

    #[test]
    fn all_recommended_sounds_exist_in_playa() {
        for event in INIT_EVENT_DISPLAY_ORDER {
            let sound = recommended_sound(&event);
            assert!(
                playa::SoundEffect::from_name(sound).is_some(),
                "Sound effect '{}' for {:?} does not exist in playa",
                sound,
                event
            );
        }
    }

    #[test]
    fn quick_start_provider_selection_matches_hook_support() {
        let providers = quick_start_supported_providers();
        assert_eq!(
            providers,
            vec![
                Provider::Claude,
                Provider::Codex,
                Provider::Gemini,
                Provider::OpenCode,
                // Kilo Code is an OpenCode fork with the same plugin-bus hooks.
                Provider::Kilo,
                // Antigravity registers a Stop hook for turn_complete (a
                // recommended init event) via ~/.gemini/config/hooks.json.
                Provider::Antigravity,
            ]
        );
    }

    #[test]
    fn init_tts_provider_ids_are_unique() {
        let mut ids = HashSet::new();
        for provider in INIT_TTS_PROVIDERS {
            assert!(ids.insert(provider.id), "duplicate TTS provider id");
        }
    }
}

//! Text-to-speech configuration types.

use serde::{Deserialize, Serialize};

// ============================================================================
// Gender
// ============================================================================

/// Gender preference for TTS voice selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Gender {
    Male,
    Female,
}

fn default_gender() -> Gender {
    Gender::Female
}

// ============================================================================
// VoiceSelection
// ============================================================================

/// Selects a TTS voice either as a single fixed voice ID or as gendered aliases.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum VoiceSelection {
    /// A single fixed voice ID string.
    Single(String),
    /// Separate voice IDs for male and female output.
    Gendered { male: String, female: String },
}

// ============================================================================
// TtsConfigSettings
// ============================================================================

/// Full TTS configuration when more than a boolean toggle is needed.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TtsConfigSettings {
    /// Which TTS provider to use (e.g., "say", "espeak", "elevenlabs").
    ///
    /// Absent selects the default provider, exactly as `"tts": true` does;
    /// an explicit `null` is rejected rather than read as absent. Resolved to
    /// a `TtsProvider` variant at runtime in the dispatch runner.
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub provider: Option<String>,

    /// Optional voice selection (single ID or gendered aliases).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub voice: Option<VoiceSelection>,

    /// Preferred gender when the voice is auto-selected.
    #[serde(default = "default_gender")]
    pub gender: Gender,

    /// Whether lifecycle speech is synthesized ahead of time into the audio
    /// cache. Absent means on; `null` is rejected rather than read as absent.
    ///
    /// Kept as an `Option` so a load/save round trip never writes the default
    /// into the user's file. Read it through [`TtsValue::prewarm_enabled`].
    #[serde(
        default,
        deserialize_with = "present",
        skip_serializing_if = "Option::is_none"
    )]
    pub prewarm: Option<bool>,
}

/// Deserialize a field that may be omitted but, when written, must hold a
/// value: `null` fails here instead of collapsing into `None` (absent).
fn present<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    T::deserialize(deserializer).map(Some)
}

// ============================================================================
// TtsValue
// ============================================================================

/// TTS configuration: either a simple boolean toggle or full settings.
///
/// The value governs hook `speak` actions and the configuration lifecycle
/// `say` plays with; it never decides whether lifecycle `say` plays.
///
/// - `"tts": true` enables hook speech with provider auto-detection.
/// - `"tts": false` (or absent) disables hook speech; lifecycle `say` still
///   plays with an auto-detected configuration.
/// - `"tts": { "provider": ..., "voice": ..., "gender": ..., "prewarm": ... }`
///   enables hook speech with explicit options; every key is optional, and an
///   object without `provider` uses the default provider as `true` does.
///
/// Pre-warm is on for every shape unless the object form says
/// `"prewarm": false` (see [`TtsValue::prewarm_enabled`]).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TtsValue {
    /// Boolean shorthand: enable or disable TTS.
    Boolean(bool),
    /// Full TTS settings.
    Config(TtsConfigSettings),
}

impl Default for TtsValue {
    fn default() -> Self {
        TtsValue::Boolean(false)
    }
}

impl TtsValue {
    /// Whether lifecycle speech may be synthesized ahead of time.
    ///
    /// Only an object with `"prewarm": false` turns it off: pre-warm follows
    /// lifecycle `say`, which plays under every shape, not hook speech.
    pub fn prewarm_enabled(&self) -> bool {
        match self {
            TtsValue::Boolean(_) => true,
            TtsValue::Config(settings) => settings.prewarm.unwrap_or(true),
        }
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::claudine_config::ClaudineConfig;

    #[test]
    fn tts_boolean_true_deserializes() {
        let config: ClaudineConfig =
            serde_json::from_value(serde_json::json!({ "preferred_agent": "claude", "tts": true }))
                .unwrap();
        assert!(matches!(config.tts, TtsValue::Boolean(true)));
    }

    #[test]
    fn tts_boolean_false_deserializes() {
        let config: ClaudineConfig = serde_json::from_value(
            serde_json::json!({ "preferred_agent": "claude", "tts": false }),
        )
        .unwrap();
        assert!(matches!(config.tts, TtsValue::Boolean(false)));
    }

    #[test]
    fn tts_config_settings_deserializes() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "say",
                "gender": "male"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => {
                assert_eq!(settings.provider.as_deref(), Some("say"));
                assert_eq!(settings.gender, Gender::Male);
                assert!(settings.voice.is_none());
            }
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    #[test]
    fn tts_config_with_single_voice() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "espeak",
                "voice": "Samantha"
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => match &settings.voice {
                Some(VoiceSelection::Single(v)) => assert_eq!(v, "Samantha"),
                other => panic!("expected Single voice, got {other:?}"),
            },
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    #[test]
    fn tts_config_with_gendered_voice() {
        let json = serde_json::json!({
            "preferred_agent": "claude",
            "tts": {
                "provider": "elevenlabs",
                "voice": { "male": "Alex", "female": "Samantha" }
            }
        });
        let config: ClaudineConfig = serde_json::from_value(json).unwrap();
        match &config.tts {
            TtsValue::Config(settings) => match &settings.voice {
                Some(VoiceSelection::Gendered { male, female }) => {
                    assert_eq!(male, "Alex");
                    assert_eq!(female, "Samantha");
                }
                other => panic!("expected Gendered voice, got {other:?}"),
            },
            other => panic!("expected Config variant, got {other:?}"),
        }
    }

    /// When no prior voice is set, selecting a voice for one gender should
    /// produce `VoiceSelection::Single`, not a `Gendered` with a placeholder.
    #[test]
    fn voice_selection_single_when_no_prior_voice() {
        // Simulate TUI behavior: voice is None, user picks a female voice
        let current_voice: Option<VoiceSelection> = None;
        let voice_name = "Samantha".to_string();

        // This matches the TUI code path: (_, GenderTab::Female) => Single
        let new_voice = match &current_voice {
            Some(VoiceSelection::Gendered { male, .. }) => VoiceSelection::Gendered {
                male: male.clone(),
                female: voice_name,
            },
            _ => VoiceSelection::Single(voice_name),
        };

        assert!(
            matches!(new_voice, VoiceSelection::Single(ref v) if v == "Samantha"),
            "should produce Single, not Gendered, when no prior voice is set"
        );
    }

    /// When voice is currently `Single` and user picks a different gender,
    /// the result should be a new `Single`, not `Gendered` with a placeholder.
    #[test]
    fn voice_selection_single_when_prior_was_single() {
        let current_voice = Some(VoiceSelection::Single("Alex".to_string()));
        let voice_name = "Samantha".to_string();

        // Matches TUI: (_, GenderTab::Female) arm — Single is not Gendered
        let new_voice = match &current_voice {
            Some(VoiceSelection::Gendered { male, .. }) => VoiceSelection::Gendered {
                male: male.clone(),
                female: voice_name,
            },
            _ => VoiceSelection::Single(voice_name),
        };

        assert!(
            matches!(new_voice, VoiceSelection::Single(ref v) if v == "Samantha"),
            "should produce Single when prior was also Single"
        );
    }

    /// When voice is `Gendered` and user updates one gender, the other
    /// gender's voice is preserved.
    #[test]
    fn voice_selection_preserves_gendered_when_updating_one_gender() {
        let current_voice = Some(VoiceSelection::Gendered {
            male: "Alex".to_string(),
            female: "Samantha".to_string(),
        });
        let new_female = "Karen".to_string();

        // Matches TUI: (Some(Gendered { male, .. }), GenderTab::Female) arm
        let new_voice = match &current_voice {
            Some(VoiceSelection::Gendered { male, .. }) => VoiceSelection::Gendered {
                male: male.clone(),
                female: new_female,
            },
            _ => unreachable!("current is Gendered"),
        };

        match new_voice {
            VoiceSelection::Gendered { male, female } => {
                assert_eq!(male, "Alex", "male voice should be preserved");
                assert_eq!(female, "Karen", "female voice should be updated");
            }
            _ => panic!("should still be Gendered"),
        }
    }
}

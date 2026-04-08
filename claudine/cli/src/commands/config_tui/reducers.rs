//! Pure reducer functions for TUI state mutations.
//!
//! These functions encapsulate the logic that was previously inline in modal
//! key handlers, making them independently testable without any App/terminal state.

use claudine::actions::HookAction;
use claudine::config::claudine_config::{MessengerProviderConfig, VoiceSelection};
use claudine::events::{AgenticEvent, recommended_sound};

use super::app::GenderTab;

/// Create the default sound effect action for an event.
///
/// Uses `recommended_sound()` to pick a contextually appropriate sound.
pub fn create_default_sound_action(event: &AgenticEvent) -> HookAction {
    HookAction::SoundEffect {
        effect: recommended_sound(event).to_string(),
        volume: 1.0,
        speed: 1.0,
    }
}

/// Compute the new `VoiceSelection` after a user picks a voice for one gender.
///
/// When the existing selection is already `Gendered`, the opposite gender is
/// preserved. Otherwise, a `Single` selection is returned (the user can always
/// come back and set the other gender, at which point the `Gendered` arms
/// handle promotion).
pub fn apply_voice_selection(
    current: Option<&VoiceSelection>,
    gender: GenderTab,
    voice: String,
) -> VoiceSelection {
    match (current, gender) {
        (Some(VoiceSelection::Gendered { male, .. }), GenderTab::Female) => {
            VoiceSelection::Gendered {
                male: male.clone(),
                female: voice,
            }
        }
        (Some(VoiceSelection::Gendered { female, .. }), GenderTab::Male) => {
            VoiceSelection::Gendered {
                male: voice,
                female: female.clone(),
            }
        }
        (_, GenderTab::Female) | (_, GenderTab::Male) => VoiceSelection::Single(voice),
    }
}

/// Create a skeleton `MessengerProviderConfig` for a given provider slug.
///
/// Returns `None` for unrecognized provider names. The returned config has
/// empty destination fields — the user must fill them in before the config
/// passes validation.
pub fn create_messenger_config(provider: &str) -> Option<MessengerProviderConfig> {
    match provider {
        "discord" => Some(MessengerProviderConfig::Discord {
            channel_id: String::new(),
            bot_token_env: "DISCORD_BOT_TOKEN".to_string(),
        }),
        "slack" => Some(MessengerProviderConfig::Slack {
            channel_id: String::new(),
            bot_token_env: "SLACK_BOT_TOKEN".to_string(),
        }),
        "signal" => Some(MessengerProviderConfig::Signal {
            recipient: String::new(),
            rpc_url_env: "SIGNAL_RPC_URL".to_string(),
            account_env: "SIGNAL_ACCOUNT".to_string(),
        }),
        "whatsapp" => Some(MessengerProviderConfig::Whatsapp {
            recipient: String::new(),
            access_token_env: "WHATSAPP_ACCESS_TOKEN".to_string(),
            phone_number_id_env: "WHATSAPP_PHONE_NUMBER_ID".to_string(),
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sound_action_uses_recommended_sound() {
        let action = create_default_sound_action(&AgenticEvent::SessionStart);
        match action {
            HookAction::SoundEffect {
                effect,
                volume,
                speed,
            } => {
                assert_eq!(effect, "high-up");
                assert_eq!(volume, 1.0);
                assert_eq!(speed, 1.0);
            }
            _ => panic!("expected SoundEffect"),
        }
    }

    #[test]
    fn sound_action_varies_by_event() {
        let start = create_default_sound_action(&AgenticEvent::SessionStart);
        let error = create_default_sound_action(&AgenticEvent::ToolError);
        match (&start, &error) {
            (
                HookAction::SoundEffect {
                    effect: start_fx, ..
                },
                HookAction::SoundEffect {
                    effect: error_fx, ..
                },
            ) => {
                assert_ne!(start_fx, error_fx);
            }
            _ => panic!("expected SoundEffect"),
        }
    }

    #[test]
    fn voice_single_when_no_existing() {
        let result = apply_voice_selection(None, GenderTab::Female, "Samantha".to_string());
        assert_eq!(result, VoiceSelection::Single("Samantha".to_string()));
    }

    #[test]
    fn voice_single_when_existing_single() {
        let current = VoiceSelection::Single("Alex".to_string());
        let result =
            apply_voice_selection(Some(&current), GenderTab::Male, "Daniel".to_string());
        assert_eq!(result, VoiceSelection::Single("Daniel".to_string()));
    }

    #[test]
    fn voice_gendered_preserves_male_when_setting_female() {
        let current = VoiceSelection::Gendered {
            male: "Daniel".to_string(),
            female: "Samantha".to_string(),
        };
        let result =
            apply_voice_selection(Some(&current), GenderTab::Female, "Karen".to_string());
        assert_eq!(
            result,
            VoiceSelection::Gendered {
                male: "Daniel".to_string(),
                female: "Karen".to_string(),
            }
        );
    }

    #[test]
    fn voice_gendered_preserves_female_when_setting_male() {
        let current = VoiceSelection::Gendered {
            male: "Daniel".to_string(),
            female: "Samantha".to_string(),
        };
        let result =
            apply_voice_selection(Some(&current), GenderTab::Male, "Tom".to_string());
        assert_eq!(
            result,
            VoiceSelection::Gendered {
                male: "Tom".to_string(),
                female: "Samantha".to_string(),
            }
        );
    }

    #[test]
    fn messenger_discord() {
        let config = create_messenger_config("discord").unwrap();
        match config {
            MessengerProviderConfig::Discord {
                channel_id,
                bot_token_env,
            } => {
                assert!(channel_id.is_empty());
                assert_eq!(bot_token_env, "DISCORD_BOT_TOKEN");
            }
            _ => panic!("expected Discord"),
        }
    }

    #[test]
    fn messenger_slack() {
        let config = create_messenger_config("slack").unwrap();
        assert!(matches!(config, MessengerProviderConfig::Slack { .. }));
    }

    #[test]
    fn messenger_signal() {
        let config = create_messenger_config("signal").unwrap();
        match config {
            MessengerProviderConfig::Signal {
                recipient,
                rpc_url_env,
                account_env,
            } => {
                assert!(recipient.is_empty());
                assert_eq!(rpc_url_env, "SIGNAL_RPC_URL");
                assert_eq!(account_env, "SIGNAL_ACCOUNT");
            }
            _ => panic!("expected Signal"),
        }
    }

    #[test]
    fn messenger_whatsapp() {
        let config = create_messenger_config("whatsapp").unwrap();
        assert!(matches!(config, MessengerProviderConfig::Whatsapp { .. }));
    }

    #[test]
    fn messenger_unknown_returns_none() {
        assert!(create_messenger_config("telegram").is_none());
        assert!(create_messenger_config("").is_none());
    }
}

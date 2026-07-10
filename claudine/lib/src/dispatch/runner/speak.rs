#[cfg(test)]
use biscuit_speaks::SpeedLevel;
use biscuit_speaks::{TtsConfig, TtsFailoverStrategy};
use tracing::warn;

use crate::config::claudine_config::ClaudineConfig;
use crate::config::tts::{Gender, TtsValue, VoiceSelection};
use crate::dispatch::template::interpolate;
use crate::events::EventMeta;

/// Speak a message using TTS with [`ClaudineConfig`]-aware voice resolution.
///
/// Voice selection priority:
/// 1. Action-level `voice` override (if present on the `Speak` action)
/// 2. Config-level voice resolved through gendered voice pairs
/// 3. Config-level single voice
/// 4. Auto-detect (no explicit voice)
pub(super) fn execute_speak_from_claudine(
    message_template: &str,
    voice_override: Option<&str>,
    gender_override: Option<Gender>,
    meta: &EventMeta,
    config: &ClaudineConfig,
) {
    let text = interpolate(message_template, meta);
    if text.is_empty() {
        return;
    }

    if matches!(config.tts, TtsValue::Boolean(false)) {
        return;
    }

    let tts = tts_config_from_claudine(config, voice_override, gender_override);

    tokio::spawn(async move {
        if let Err(error) = biscuit_speaks::Speak::new(text)
            .with_config(tts)
            .play()
            .await
        {
            warn!(%error, "TTS playback failed");
        }
    });
}

/// Build a [`TtsConfig`] from [`ClaudineConfig`] with optional per-action overrides.
///
/// Resolution order for voice:
/// 1. `voice_override` from the action (always wins)
/// 2. Gendered voice pair from config, using action `gender_override` → config default gender
/// 3. Single voice from config
/// 4. No explicit voice (auto-detect)
pub(super) fn tts_config_from_claudine(
    config: &ClaudineConfig,
    voice_override: Option<&str>,
    gender_override: Option<Gender>,
) -> TtsConfig {
    let mut tts = TtsConfig::new();

    if let Some(voice) = voice_override {
        tts = tts.with_voice(voice);
    }

    match &config.tts {
        TtsValue::Boolean(false) => {}
        TtsValue::Boolean(true) => {}
        TtsValue::Config(settings) => {
            if voice_override.is_none() {
                if let Some(provider) = biscuit_speaks::parse_provider_name(&settings.provider) {
                    tts = tts.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
                }

                let gender = gender_override.unwrap_or(settings.gender);
                match &settings.voice {
                    Some(VoiceSelection::Single(v)) => {
                        tts = tts.with_voice(v);
                    }
                    Some(VoiceSelection::Gendered { male, female }) => {
                        let voice = match gender {
                            Gender::Male => male.as_str(),
                            Gender::Female => female.as_str(),
                        };
                        tts = tts.with_voice(voice);
                    }
                    None => {}
                }
            } else if let Some(provider) = biscuit_speaks::parse_provider_name(&settings.provider) {
                tts = tts.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
            }
        }
    }

    tts
}

/// Speak a message via biscuit-speaks TTS (fire-and-forget).
#[cfg(test)]
fn tts_config_from_settings(settings: Option<&crate::events::TtsSettings>) -> TtsConfig {
    let mut config = TtsConfig::new();
    let Some(settings) = settings else {
        return config;
    };

    if let Some(voice) = settings.voice.as_deref() {
        config = config.with_voice(voice);
    }

    if let Some(rate) = settings.rate {
        config = config.with_speed(SpeedLevel::Explicit(rate));
    }

    if let Some(provider) = settings.provider.as_deref() {
        if let Some(provider) = biscuit_speaks::parse_provider_name(provider) {
            config = config.with_failover(TtsFailoverStrategy::SpecificProvider(provider));
        } else {
            warn!(
                provider,
                "Unknown TTS provider in settings; falling back to automatic provider selection"
            );
        }
    }

    config
}

#[cfg(test)]
mod tests {
    use biscuit_speaks::{HostTtsProvider, TtsFailoverStrategy, TtsProvider};
    use std::collections::HashMap;

    use super::*;
    use crate::config::claudine_config::ClaudineConfig;
    use crate::config::tts::{Gender, TtsConfigSettings, TtsValue, VoiceSelection};
    use crate::events::TtsSettings;

    fn claudine_config_with_tts(tts: TtsValue) -> ClaudineConfig {
        ClaudineConfig {
            tts,
            messenger: None,
            logging: true,
            protect: Default::default(),
            actions: HashMap::new(),
            matchers: HashMap::new(),
            preferred_agent: Some(crate::provider::Provider::Claude),
            canonical_provider: None,
            models: HashMap::new(),
            default_sounds: Default::default(),
            prompt_for_missing: true,
            harvest_unmatched: false,
            exit_expressions: None,
            guard_settings: Default::default(),
        }
    }

    #[test]
    fn tts_config_applies_provider_voice_and_rate() {
        let settings = TtsSettings {
            provider: Some("say".to_string()),
            voice: Some("Samantha".to_string()),
            rate: Some(1.4),
        };

        let config = tts_config_from_settings(Some(&settings));

        assert_eq!(config.requested_voice.as_deref(), Some("Samantha"));
        assert_eq!(config.speed, biscuit_speaks::SpeedLevel::Explicit(1.4));
        assert!(matches!(
            config.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(HostTtsProvider::Say))
        ));
    }

    #[test]
    fn tts_config_keeps_default_failover_for_unknown_provider() {
        let settings = TtsSettings {
            provider: Some("not-a-provider".to_string()),
            voice: None,
            rate: None,
        };

        let config = tts_config_from_settings(Some(&settings));
        assert!(matches!(
            config.failover_strategy,
            TtsFailoverStrategy::FirstAvailable
        ));
    }

    #[test]
    fn tts_config_from_claudine_boolean_true_is_auto_detect() {
        let config = claudine_config_with_tts(TtsValue::Boolean(true));
        let tts = tts_config_from_claudine(&config, None, None);
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::FirstAvailable
        ));
        assert!(tts.requested_voice.is_none());
    }

    #[test]
    fn tts_config_from_claudine_boolean_false_returns_default() {
        let config = claudine_config_with_tts(TtsValue::Boolean(false));
        let tts = tts_config_from_claudine(&config, None, None);
        assert!(tts.requested_voice.is_none());
    }

    #[test]
    fn tts_config_from_claudine_single_voice() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Samantha"));
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(_)
        ));
    }

    #[test]
    fn tts_config_from_claudine_gendered_voice_female_default() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Samantha"));
    }

    #[test]
    fn tts_config_from_claudine_gendered_voice_male_default() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Male,
        }));
        let tts = tts_config_from_claudine(&config, None, None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Alex"));
    }

    #[test]
    fn tts_config_from_claudine_gender_override_selects_male_voice() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Gendered {
                male: "Alex".to_string(),
                female: "Samantha".to_string(),
            }),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, None, Some(Gender::Male));
        assert_eq!(tts.requested_voice.as_deref(), Some("Alex"));
    }

    #[test]
    fn tts_config_from_claudine_voice_override_wins_over_config() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, Some("Karen"), None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Karen"));
    }

    #[test]
    fn tts_config_from_claudine_voice_override_sets_provider() {
        let config = claudine_config_with_tts(TtsValue::Config(TtsConfigSettings {
            provider: "say".to_string(),
            voice: Some(VoiceSelection::Single("Samantha".to_string())),
            gender: Gender::Female,
        }));
        let tts = tts_config_from_claudine(&config, Some("Karen"), None);
        assert_eq!(tts.requested_voice.as_deref(), Some("Karen"));
        assert!(matches!(
            tts.failover_strategy,
            TtsFailoverStrategy::SpecificProvider(_)
        ));
    }
}

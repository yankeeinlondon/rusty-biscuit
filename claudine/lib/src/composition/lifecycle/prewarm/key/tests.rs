//! A key separates candidates by text and by every playback config field.

use biscuit_speaks::{
    Gender, HostTtsProvider, Language, SpeedLevel, TtsConfig, TtsFailoverStrategy, TtsProvider,
    VolumeLevel,
};

use super::PrewarmKey;

#[test]
fn equal_text_and_config_give_equal_keys() {
    let config = TtsConfig::new().with_voice("af_heart");
    assert_eq!(
        PrewarmKey::new("build is done", &config),
        PrewarmKey::new("build is done", &config.clone())
    );
}

#[test]
fn different_text_gives_a_different_key() {
    let config = TtsConfig::new();
    assert_ne!(
        PrewarmKey::new("build is done", &config),
        PrewarmKey::new("build is Done", &config)
    );
}

#[test]
fn changing_any_config_field_gives_a_different_key() {
    // Destructured so a field added to `TtsConfig` fails to compile here
    // until it gets a row below.
    let TtsConfig {
        requested_voice: _,
        requested_model: _,
        gender: _,
        language: _,
        volume: _,
        speed: _,
        failover_strategy: _,
    } = TtsConfig::default();

    let base = TtsConfig::default();
    let variants: [(&str, TtsConfig); 7] = [
        ("requested_voice", TtsConfig { requested_voice: Some("af_heart".into()), ..base.clone() }),
        ("requested_model", TtsConfig { requested_model: Some("eleven_v2".into()), ..base.clone() }),
        ("gender", TtsConfig { gender: Gender::Female, ..base.clone() }),
        ("language", TtsConfig { language: Language::Custom("fr".into()), ..base.clone() }),
        ("volume", TtsConfig { volume: VolumeLevel::Explicit(0.0), ..base.clone() }),
        ("speed", TtsConfig { speed: SpeedLevel::Explicit(1.25), ..base.clone() }),
        (
            "failover_strategy",
            TtsConfig {
                failover_strategy: TtsFailoverStrategy::SpecificProvider(TtsProvider::Host(
                    HostTtsProvider::KokoroTts,
                )),
                ..base.clone()
            },
        ),
    ];
    let base_key = PrewarmKey::new("same text", &base);
    for (field, config) in variants {
        assert_ne!(PrewarmKey::new("same text", &config), base_key, "{field}");
    }
}
